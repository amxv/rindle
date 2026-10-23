//! Durable import ledger. The two browsers never need to be open simultaneously.
use crate::cli::Browser;
use crate::urls;
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserVisit {
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, rename = "lastVisitTime")]
    pub last_visit_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedUrl {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct OutboundUrl {
    pub url: String,
    pub title: String,
}

pub struct Store {
    pub conn: Connection,
}

pub fn app_dir() -> Result<PathBuf> {
    if let Some(value) = env::var_os("RINDLE_HOME") {
        return Ok(PathBuf::from(value));
    }
    let home = PathBuf::from(env::var_os("HOME").context("HOME is not set")?);
    #[cfg(target_os = "macos")]
    return Ok(home.join("Library/Application Support/Rindle"));
    #[cfg(not(target_os = "macos"))]
    Ok(home.join(".config/rindle"))
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl Store {
    pub fn open() -> Result<Self> {
        let base = app_dir()?;
        fs::create_dir_all(&base).with_context(|| format!("Cannot create {}", base.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&base, fs::Permissions::from_mode(0o700))?;
        }
        Self::open_at(&base.join("state.sqlite"))
    }

    pub fn open_at(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Cannot open Rindle state at {}", path.display()))?;
        conn.busy_timeout(Duration::from_secs(10))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS enabled_browsers (
                 browser TEXT PRIMARY KEY NOT NULL
             );
             CREATE TABLE IF NOT EXISTS extension_contacts (
                 browser TEXT PRIMARY KEY NOT NULL,
                 last_seen INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS browser_urls (
                 browser TEXT NOT NULL,
                 url TEXT NOT NULL,
                 title TEXT NOT NULL DEFAULT '',
                 first_seen INTEGER NOT NULL,
                 last_visit_ms INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY (browser,url)
             );
             CREATE TABLE IF NOT EXISTS safari_imports (
                 url TEXT PRIMARY KEY NOT NULL,
                 generation INTEGER,
                 imported_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS browser_queue (
                 browser TEXT NOT NULL,
                 url TEXT NOT NULL,
                 queued_at INTEGER NOT NULL,
                 acked_at INTEGER,
                 PRIMARY KEY (browser,url)
             );
             CREATE INDEX IF NOT EXISTS browser_queue_pending
                 ON browser_queue(browser,acked_at,queued_at);",
        )?;
        // Existing 0.1 development installs may already have a ledger without
        // visit timestamps. Upgrade it in place without discarding the queue.
        let mut columns = conn.prepare("PRAGMA table_info(browser_urls)")?;
        let has_last_visit = columns
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|column| column == "last_visit_ms");
        drop(columns);
        if !has_last_visit {
            conn.execute(
                "ALTER TABLE browser_urls ADD COLUMN last_visit_ms INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
        Ok(Self { conn })
    }

    pub fn enable(&self, browser: Browser) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO enabled_browsers(browser) VALUES (?1)",
            [browser.as_str()],
        )?;
        Ok(())
    }

    pub fn disable_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM enabled_browsers", [])?;
        Ok(())
    }

    pub fn enabled(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT browser FROM enabled_browsers ORDER BY browser")?;
        stmt.query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn has_browser(&self, browser: Browser) -> Result<bool> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM enabled_browsers WHERE browser=?1",
                [browser.as_str()],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false))
    }

    pub fn touch(&self, browser: Browser) -> Result<()> {
        self.conn.execute(
            "INSERT INTO extension_contacts(browser,last_seen) VALUES (?1,?2)
             ON CONFLICT(browser) DO UPDATE SET last_seen=excluded.last_seen",
            params![browser.as_str(), now()],
        )?;
        Ok(())
    }

    pub fn last_contact(&self, browser: Browser) -> Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT last_seen FROM extension_contacts WHERE browser=?1",
                [browser.as_str()],
                |r| r.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn ingest(&mut self, browser: Browser, visits: &[BrowserVisit]) -> Result<usize> {
        if visits.len() > 500 {
            bail!("Native Messaging batch exceeds 500 URLs");
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut accepted = 0;
        {
            let mut stmt = tx.prepare("INSERT OR IGNORE INTO browser_urls(browser,url,title,first_seen,last_visit_ms) VALUES (?1,?2,?3,?4,?5)")?;
            let mut update = tx.prepare(
                "UPDATE browser_urls SET last_visit_ms=MAX(last_visit_ms,?3),
                 title=CASE WHEN ?4!='' THEN ?4 ELSE title END
                 WHERE browser=?1 AND url=?2 AND (last_visit_ms < ?3 OR (title='' AND ?4!=''))",
            )?;
            for visit in visits {
                if let Some(url) = urls::clean(&visit.url) {
                    let title: String = visit.title.chars().take(512).collect();
                    let visited_at = visit.last_visit_time.unwrap_or(0).max(0);
                    let new =
                        stmt.execute(params![browser.as_str(), url, title, now(), visited_at])?;
                    if new == 0 {
                        update.execute(params![browser.as_str(), url, visited_at, title])?;
                    }
                    accepted += new;
                }
            }
        }
        tx.commit()?;
        Ok(accepted)
    }

    pub fn acknowledge(&mut self, browser: Browser, urls: &[String]) -> Result<usize> {
        if urls.len() > 500 {
            bail!("Native Messaging acknowledgement exceeds 500 URLs");
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut acknowledged = 0;
        {
            let mut update = tx.prepare("UPDATE browser_queue SET acked_at=?1 WHERE browser=?2 AND url=?3 AND acked_at IS NULL")?;
            let mut include = tx.prepare("INSERT OR IGNORE INTO browser_urls(browser,url,title,first_seen) VALUES (?1,?2,'',?3)")?;
            for url in urls {
                if let Some(url) = urls::clean(url) {
                    let changed = update.execute(params![now(), browser.as_str(), url])?;
                    if changed != 0 {
                        // Only acknowledge URLs that were actually queued by
                        // Safari. A malformed or stale ack cannot invent a
                        // browser visit and accidentally suppress future syncs.
                        acknowledged += changed;
                        include.execute(params![browser.as_str(), url, now()])?;
                    }
                }
            }
        }
        tx.commit()?;
        Ok(acknowledged)
    }

    pub fn next_for_browser(&self, browser: Browser, limit: usize) -> Result<Vec<QueuedUrl>> {
        let mut stmt = self.conn.prepare(
            "SELECT url FROM browser_queue WHERE browser=?1 AND acked_at IS NULL ORDER BY queued_at,url LIMIT ?2",
        )?;
        stmt.query_map(params![browser.as_str(), limit.min(100) as i64], |r| {
            Ok(QueuedUrl { url: r.get(0)? })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
    }

    pub fn enqueue_remote(&mut self, urls: &[String]) -> Result<usize> {
        let browsers = self.enabled()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut accepted = 0;
        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO browser_queue(browser,url,queued_at)
                 SELECT ?1,?2,?3 WHERE NOT EXISTS (
                   SELECT 1 FROM browser_urls WHERE browser=?1 AND url=?2
                 ) AND NOT EXISTS (
                   SELECT 1 FROM safari_imports WHERE url=?2
                 )",
            )?;
            for raw in urls {
                if let Some(url) = urls::clean(raw) {
                    for browser in &browsers {
                        accepted += stmt.execute(params![browser, url, now()])?;
                    }
                }
            }
        }
        tx.commit()?;
        Ok(accepted)
    }

    pub fn pending_outbound(&self, max: usize) -> Result<Vec<OutboundUrl>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.url,MAX(b.title) FROM browser_urls b
             LEFT JOIN safari_imports s ON s.url=b.url
             WHERE s.url IS NULL AND NOT EXISTS (
               SELECT 1 FROM browser_queue q WHERE q.url=b.url
             ) GROUP BY b.url
             ORDER BY MAX(b.last_visit_ms) DESC,MIN(b.first_seen) DESC,b.url LIMIT ?1",
        )?;
        stmt.query_map([max as i64], |r| {
            Ok(OutboundUrl {
                url: r.get(0)?,
                title: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
    }

    pub fn mark_in_safari(&mut self, imports: &[(String, Option<i64>)]) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        for (url, generation) in imports {
            tx.execute("INSERT OR IGNORE INTO safari_imports(url,generation,imported_at) VALUES (?1,?2,?3)", params![url, generation, now()])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn latest_pending_generation(&self, checkpoint: i64) -> Result<Option<i64>> {
        Ok(self.conn.query_row(
            "SELECT MAX(generation) FROM safari_imports WHERE generation > ?1",
            [checkpoint],
            |r| r.get(0),
        )?)
    }

    pub fn counts(&self) -> Result<(i64, i64, i64)> {
        Ok((
            self.conn
                .query_row("SELECT COUNT(*) FROM browser_urls", [], |r| r.get(0))?,
            self.conn.query_row(
                "SELECT COUNT(*) FROM browser_queue WHERE acked_at IS NULL",
                [],
                |r| r.get(0),
            )?,
            self.conn
                .query_row("SELECT COUNT(*) FROM safari_imports", [], |r| r.get(0))?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotent_bidirectional_transfer_and_ack() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open_at(&temp.path().join("ledger.sqlite"))?;
        store.enable(Browser::Dia)?;
        store.enable(Browser::Chrome)?;
        let items = vec![BrowserVisit {
            url: "https://example.com/a".into(),
            title: "A".into(),
            last_visit_time: Some(123456),
        }];
        assert_eq!(store.ingest(Browser::Dia, &items)?, 1);
        assert_eq!(store.ingest(Browser::Dia, &items)?, 0);
        let sent = store.pending_outbound(10)?;
        assert_eq!(sent.len(), 1);
        store.mark_in_safari(&[(sent[0].url.clone(), Some(12))])?;
        assert!(store.pending_outbound(10)?.is_empty());
        assert_eq!(store.enqueue_remote(&["https://example.com/a".into()])?, 0);
        assert_eq!(store.enqueue_remote(&["https://example.org/b".into()])?, 2);
        assert_eq!(store.enqueue_remote(&["https://example.org/b".into()])?, 0);
        assert_eq!(store.next_for_browser(Browser::Dia, 10)?.len(), 1);
        assert_eq!(
            store.acknowledge(Browser::Dia, &["https://example.org/b".into()])?,
            1
        );
        assert!(store.next_for_browser(Browser::Dia, 10)?.is_empty());
        assert!(
            store.pending_outbound(10)?.is_empty(),
            "Safari-sourced URLs must not be bounced back into Safari"
        );
        assert_eq!(store.next_for_browser(Browser::Chrome, 10)?.len(), 1);
        assert_eq!(store.counts()?, (2, 1, 1));
        Ok(())
    }

    #[test]
    fn unknown_ack_does_not_invent_a_visit_or_block_future_imports() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open_at(&temp.path().join("ledger.sqlite"))?;
        store.enable(Browser::Dia)?;
        let url = "https://example.org/from-phone".to_string();
        assert_eq!(
            store.acknowledge(Browser::Dia, std::slice::from_ref(&url))?,
            0
        );
        assert_eq!(store.counts()?, (0, 0, 0));
        assert_eq!(store.enqueue_remote(std::slice::from_ref(&url))?, 1);
        assert_eq!(
            store.acknowledge(Browser::Dia, std::slice::from_ref(&url))?,
            1
        );
        assert_eq!(
            store.acknowledge(Browser::Dia, std::slice::from_ref(&url))?,
            0
        );
        assert!(store.pending_outbound(10)?.is_empty());
        assert_eq!(store.counts()?, (1, 0, 0));
        Ok(())
    }

    #[test]
    fn newest_browser_visits_take_priority_without_duplicate_rows() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open_at(&temp.path().join("ledger.sqlite"))?;
        store.enable(Browser::Dia)?;
        let old = BrowserVisit {
            url: "https://example.com/old".into(),
            title: "Old".into(),
            last_visit_time: Some(1000),
        };
        let recent = BrowserVisit {
            url: "https://example.com/recent".into(),
            title: "Recent".into(),
            last_visit_time: Some(5000),
        };
        assert_eq!(
            store.ingest(Browser::Dia, &[old.clone(), recent.clone()])?,
            2
        );
        assert_eq!(store.pending_outbound(1)?[0].url, recent.url);
        let updated_old = BrowserVisit {
            last_visit_time: Some(9000),
            ..old
        };
        assert_eq!(store.ingest(Browser::Dia, &[updated_old])?, 0);
        assert_eq!(store.pending_outbound(1)?[0].url, "https://example.com/old");
        assert_eq!(store.counts()?.0, 2);
        Ok(())
    }

    #[test]
    fn migrates_previous_development_ledger_without_losing_urls() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("ledger.sqlite");
        let old = Connection::open(&path)?;
        old.execute_batch(
            "CREATE TABLE browser_urls (
                browser TEXT NOT NULL,
                url TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                first_seen INTEGER NOT NULL,
                PRIMARY KEY(browser,url)
             );
             INSERT INTO browser_urls VALUES('dia','https://example.com/kept','Kept',1234);",
        )?;
        drop(old);
        let mut store = Store::open_at(&path)?;
        assert_eq!(store.counts()?.0, 1);
        assert_eq!(
            store.conn.query_row(
                "SELECT last_visit_ms FROM browser_urls WHERE url='https://example.com/kept'",
                [],
                |row| row.get::<_, i64>(0),
            )?,
            0
        );
        assert_eq!(
            store.ingest(
                Browser::Dia,
                &[BrowserVisit {
                    url: "https://example.com/kept".into(),
                    title: "Kept".into(),
                    last_visit_time: Some(6000),
                }]
            )?,
            0
        );
        assert_eq!(
            store.pending_outbound(1)?[0].url,
            "https://example.com/kept"
        );
        assert_eq!(store.counts()?.0, 1);
        Ok(())
    }
}
