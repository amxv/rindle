//! Narrow macOS Safari History.db adapter. The sync engine refuses unknown
//! schemas, keeps consistent backups, and never opens an imported URL.
use crate::state::{self, OutboundUrl};
use crate::urls;
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const COCOA_EPOCH_SECONDS: f64 = 978_307_200.0;
const MAX_BACKUPS: usize = 3;

pub struct Safari {
    db: Connection,
}

pub fn database_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os("RINDLE_SAFARI_HISTORY_DB") {
        return Ok(PathBuf::from(path));
    }
    Ok(PathBuf::from(env::var_os("HOME").context("HOME not set")?)
        .join("Library/Safari/History.db"))
}

fn validate_schema(db: &Connection) -> Result<()> {
    for (table, expected) in [
        (
            "history_items",
            &[
                "id",
                "url",
                "domain_expansion",
                "visit_count",
                "daily_visit_counts",
                "visit_count_score",
                "status_code",
            ][..],
        ),
        (
            "history_visits",
            &[
                "history_item",
                "visit_time",
                "title",
                "origin",
                "generation",
                "attributes",
                "score",
            ][..],
        ),
        ("metadata", &["key", "value"][..]),
    ] {
        let mut stmt = db.prepare(&format!("PRAGMA table_info({table})"))?;
        let columns = stmt
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for column in expected {
            if !columns.iter().any(|x| x == column) {
                bail!("Safari History.db has changed; missing {table}.{column}. Refusing to write")
            }
        }
    }
    if db.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))? != "ok" {
        bail!("Safari History.db integrity check failed: refusing to write")
    }
    let _: i64 = db
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM metadata WHERE key='current_generation'",
            [],
            |row| row.get(0),
        )
        .context("Safari history has no current_generation metadata")?;
    Ok(())
}

impl Safari {
    /// Poll iCloud history's upload checkpoint without acquiring a write
    /// connection or running an expensive integrity check against Safari's
    /// actively syncing database. Writers are validated separately.
    pub fn synced_generation_read_only() -> Result<i64> {
        let path = database_path()?;
        Self::synced_generation_at(&path)
    }

    /// Safari increments its native history generation asynchronously after
    /// launch/visits. A heartbeat recorded before the generation assigned to
    /// an out-of-band import may not carry that imported record into iCloud.
    pub fn current_generation_read_only() -> Result<i64> {
        let path = database_path()?;
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .context("Cannot read Safari current history generation")?;
        conn.busy_timeout(Duration::from_secs(2))?;
        conn.query_row(
            "SELECT CAST(value AS INTEGER) FROM metadata WHERE key='current_generation'",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    /// Read the generation of one Safari visit without opening the writable
    /// database or performing expensive integrity checks while Safari runs.
    pub fn visit_generation_read_only(url: &str) -> Result<Option<i64>> {
        let path = database_path()?;
        Self::visit_generation_at(&path, url)
    }

    fn visit_generation_at(path: &Path, url: &str) -> Result<Option<i64>> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .context("Cannot read Safari visit generation")?;
        conn.busy_timeout(Duration::from_secs(2))?;
        conn.query_row(
            "SELECT MAX(v.generation) FROM history_visits v JOIN history_items i ON i.id=v.history_item WHERE i.url=?1",
            [url],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    fn synced_generation_at(path: &Path) -> Result<i64> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .context("Cannot read Safari history sync checkpoint")?;
        conn.busy_timeout(Duration::from_secs(2))?;
        let value = conn.query_row(
            "SELECT CAST(value AS INTEGER) FROM metadata WHERE key='last_synced_generation'",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(value)
    }

    pub fn open() -> Result<Self> {
        Self::open_at(&database_path()?)
    }

    pub fn open_at(path: &Path) -> Result<Self> {
        if !path.is_file() {
            bail!(
                "Safari history not found at {}. Open Safari once with iCloud Safari enabled",
                path.display()
            )
        }
        let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .with_context(|| {
                format!(
                    "Cannot open Safari history at {} (macOS Full Disk Access may be required)",
                    path.display()
                )
            })?;
        db.busy_timeout(Duration::from_secs(10))?;
        validate_schema(&db)?;
        Ok(Self { db })
    }

    pub fn generations(&self) -> Result<(i64, i64)> {
        let current: i64 = self.db.query_row(
            "SELECT CAST(value AS INTEGER) FROM metadata WHERE key='current_generation'",
            [],
            |r| r.get(0),
        )?;
        let synced: i64 = self
            .db
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM metadata WHERE key='last_synced_generation'",
                [],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0);
        Ok((current, synced))
    }

    /// Incoming from iCloud: any remote device, not uniquely identifiable as iPhone.
    pub fn remote_urls(&self) -> Result<Vec<String>> {
        let mut stmt = self.db.prepare(
            "SELECT i.url FROM history_items i
             JOIN history_visits v ON v.history_item=i.id WHERE v.origin=1
             GROUP BY i.id ORDER BY MAX(v.visit_time) DESC",
        )?;
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for url in stmt.query_map([], |r| r.get::<_, String>(0))? {
            if let Some(url) = urls::clean(&url?)
                && seen.insert(url.clone())
            {
                result.push(url);
            }
        }
        Ok(result)
    }

    fn backup(&self, directory: &Path) -> Result<PathBuf> {
        fs::create_dir_all(directory)?;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = directory.join(format!("safari-{timestamp}-{}.sqlite", std::process::id()));
        if path.exists() {
            bail!("Refusing to overwrite Safari history backup")
        }
        self.db
            .execute("VACUUM INTO ?1", [path.to_string_lossy().as_ref()])
            .context("Unable to create a consistent Safari history backup")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        Ok(path)
    }

    /// Returns `(url, generation)` for every known or newly inserted URL.
    /// `None` means Safari already had that URL; no visit was generated.
    pub fn add_urls(
        &mut self,
        urls: &[OutboundUrl],
        backups: &Path,
    ) -> Result<Vec<(String, Option<i64>)>> {
        if urls.is_empty() {
            return Ok(Vec::new());
        }
        validate_schema(&self.db)?;
        let mut results = Vec::new();
        let mut missing = Vec::new();
        {
            let mut known = self
                .db
                .prepare("SELECT 1 FROM history_items WHERE url=?1 LIMIT 1")?;
            for item in urls {
                if let Some(url) = urls::clean(&item.url) {
                    if known.exists([&url])? {
                        results.push((url, None));
                    } else {
                        missing.push((url, item.title.chars().take(512).collect::<String>()));
                    }
                }
            }
        }
        if missing.is_empty() {
            return Ok(results);
        }
        self.backup(backups)?;
        let next_gen = self.generations()?.0 + 1;
        let now_cocoa =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64() - COCOA_EPOCH_SECONDS;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        {
            let mut known = tx.prepare("SELECT 1 FROM history_items WHERE url=?1 LIMIT 1")?;
            let mut add_item = tx.prepare(
                "INSERT INTO history_items(url,domain_expansion,visit_count,daily_visit_counts,weekly_visit_counts,
                 autocomplete_triggers,should_recompute_derived_visit_counts,visit_count_score,status_code)
                 VALUES (?1,?2,1,?3,NULL,NULL,0,100,0)",
            )?;
            let mut add_visit = tx.prepare(
                "INSERT INTO history_visits(history_item,visit_time,title,load_successful,http_non_get,
                 synthesized,origin,generation,attributes,score)
                 VALUES (?1,?2,?3,1,0,0,0,?4,0,100)",
            )?;
            for (url, title) in missing {
                // The history daemon may have inserted this URL since our first read.
                if known.exists([&url])? {
                    results.push((url, None));
                    continue;
                }
                add_item.execute(params![
                    url,
                    urls::safari_domain_expansion(&url),
                    100_u32.to_le_bytes().to_vec()
                ])?;
                let id = tx.last_insert_rowid();
                add_visit.execute(params![id, now_cocoa, title, next_gen])?;
                results.push((url, Some(next_gen)));
            }
        }
        tx.commit()?;
        prune_backups(backups)?;
        Ok(results)
    }
}

fn prune_backups(directory: &Path) -> Result<()> {
    let mut paths = fs::read_dir(directory)?
        .map(|e| e.map(|v| v.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|p| {
        p.is_file()
            && p.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("safari-"))
    });
    paths.sort();
    for path in paths.iter().take(paths.len().saturating_sub(MAX_BACKUPS)) {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn backup_directory() -> Result<PathBuf> {
    Ok(state::app_dir()?.join("backups"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(path: &Path) -> Result<()> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT);
             INSERT INTO metadata VALUES('current_generation','100');
             INSERT INTO metadata VALUES('last_synced_generation','99');
             CREATE TABLE history_items (id INTEGER PRIMARY KEY,url TEXT UNIQUE,domain_expansion TEXT,
                visit_count INTEGER,daily_visit_counts BLOB,weekly_visit_counts BLOB,
                autocomplete_triggers BLOB,should_recompute_derived_visit_counts INTEGER,
                visit_count_score INTEGER,status_code INTEGER);
             CREATE TABLE history_visits (id INTEGER PRIMARY KEY,history_item INTEGER,visit_time REAL,
                title TEXT,load_successful INTEGER,http_non_get INTEGER,synthesized INTEGER,
                redirect_source INTEGER,redirect_destination INTEGER,origin INTEGER,generation INTEGER,
                attributes INTEGER,score INTEGER);
             INSERT INTO history_items(url,visit_count) VALUES('https://from-iphone.example/a',1);
             INSERT INTO history_visits(history_item,visit_time,origin,generation) VALUES(1,100,1,99);",
        )?;
        Ok(())
    }

    #[test]
    fn safely_adds_one_history_visit_with_backup_and_no_duplicates() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("History.db");
        fixture(&path)?;
        assert_eq!(Safari::synced_generation_at(&path)?, 99);
        assert_eq!(
            Safari::visit_generation_at(&path, "https://from-iphone.example/a")?,
            Some(99)
        );
        let mut safari = Safari::open_at(&path)?;
        assert_eq!(safari.remote_urls()?, vec!["https://from-iphone.example/a"]);
        let items = vec![
            OutboundUrl {
                url: "https://www.mozilla.org/article".into(),
                title: "Article".into(),
            },
            OutboundUrl {
                url: "https://from-iphone.example/a".into(),
                title: "Existing".into(),
            },
        ];
        let first = safari.add_urls(&items, &dir.path().join("backups"))?;
        assert_eq!(
            first,
            vec![
                ("https://from-iphone.example/a".into(), None),
                ("https://www.mozilla.org/article".into(), Some(101))
            ]
        );
        let second = safari.add_urls(&items, &dir.path().join("backups"))?;
        assert!(second.iter().all(|(_, generation)| generation.is_none()));
        let visits: i64 = safari
            .db
            .query_row("SELECT COUNT(*) FROM history_visits", [], |r| r.get(0))?;
        assert_eq!(visits, 2);
        let origin: i64 = safari.db.query_row(
            "SELECT origin FROM history_visits WHERE history_item=2",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(origin, 0);
        assert_eq!(
            Safari::visit_generation_at(&path, "https://www.mozilla.org/article")?,
            Some(101)
        );
        assert_eq!(Safari::synced_generation_at(&path)?, 99);
        assert_eq!(fs::read_dir(dir.path().join("backups"))?.count(), 1);
        Ok(())
    }

    #[test]
    fn rejects_changed_schema_without_writing() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("History.db");
        Connection::open(&path)?.execute_batch("CREATE TABLE history_items(url TEXT)")?;
        assert!(Safari::open_at(&path).is_err());
        Ok(())
    }
}
