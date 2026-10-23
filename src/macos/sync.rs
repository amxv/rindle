//! Three-hour macOS sync task: queue remote Safari URLs for extensions, inject
//! new Chromium URLs into Safari, and use only Apple's home page as the hidden
//! iCloud sync heartbeat. No user browsing URL is ever navigated.
use crate::macos::{backend, safari, setup};
use crate::state::{self, Store};
use anyhow::{Context, Result, bail};
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const EXPORT_LIMIT_PER_RUN: usize = 1000;
const HEARTBEAT_PREFIX: &str = "https://www.apple.com/?rindle-sync-heartbeat=";
const MAX_SYNC_WAIT: Duration = Duration::from_secs(150);
// Safari's Cloud History idle-sync delay is about 90 seconds on tested macOS.
// An early local checkpoint is not an iCloud delivery acknowledgment.
const MIN_IDLE_AFTER_HEARTBEAT: Duration = Duration::from_secs(110);
const ICLOUD_FETCH_WARMUP: Duration = Duration::from_secs(15);

struct SyncLock(PathBuf);

impl SyncLock {
    fn acquire() -> Result<Option<Self>> {
        let path = state::app_dir()?.join("sync.lock");
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => Ok(Some(Self(path))),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if fs::metadata(&path)?
                    .modified()?
                    .elapsed()
                    .unwrap_or_default()
                    > Duration::from_secs(45 * 60)
                {
                    fs::remove_file(&path)?;
                    return Self::acquire();
                }
                Ok(None)
            }
            Err(error) => Err(error).context("Unable to acquire sync lock"),
        }
    }
}

impl Drop for SyncLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn safari_is_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "Safari"])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn exec_success(program: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Unable to launch {program}"))?;
    if !output.status.success() {
        bail!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
    Ok(())
}

/// Own only Safari sessions started by Rindle. A user who activates Safari
/// while the helper is running should be allowed to keep using it.
struct HiddenSafari {
    created_heartbeats: usize,
}

impl HiddenSafari {
    fn start(pending_generation: Option<i64>) -> Result<Self> {
        if safari_is_running() {
            bail!("Safari is already running; refusing to interfere with an existing session")
        }
        exec_success("open", &["-gj", "-a", "Safari"])?;
        let mut session = Self {
            created_heartbeats: 0,
        };
        if let Some(generation) = pending_generation {
            session.trigger_upload(generation)?;
        }
        Ok(session)
    }

    /// Open ONLY a newly tagged Apple.com URL, never an item from browser
    /// history. Each invocation must produce a distinct visit in Safari.
    fn heartbeat(&mut self) -> Result<String> {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let url = format!("{HEARTBEAT_PREFIX}{}-{nonce}", std::process::id());
        let script = format!(
            "tell application \"Safari\" to make new document with properties {{URL:\"{url}\"}}"
        );
        exec_success("osascript", &["-e", &script])?;
        self.created_heartbeats += 1;
        Ok(url)
    }

    fn recorded_heartbeat_generation(url: &str) -> Result<i64> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(30) {
            if let Ok(Some(generation)) = backend::Backend::visit_generation_read_only(url) {
                return Ok(generation);
            }
            thread::sleep(Duration::from_secs(2));
        }
        bail!("Safari did not record the harmless Apple.com heartbeat")
    }

    fn trigger_upload(&mut self, target_generation: i64) -> Result<()> {
        let first = self.heartbeat()?;
        let first_generation = Self::recorded_heartbeat_generation(&first)?;
        if !heartbeat_is_early(first_generation, target_generation) {
            println!("Harmless Safari heartbeat recorded at generation {first_generation}.");
            return Ok(());
        }

        // Proven successful tests had the imported URL AND the heartbeat at
        // the same generation. In the failed Rindle test the heartbeat was
        // one generation *behind* the import. Safari bumps current_generation
        // asynchronously, so wait for it before emitting a fresh heartbeat.
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(15) {
            if backend::Backend::current_generation_read_only()
                .is_ok_and(|current| current >= target_generation)
            {
                break;
            }
            thread::sleep(Duration::from_secs(2));
        }
        if !backend::Backend::current_generation_read_only()
            .is_ok_and(|current| current >= target_generation)
        {
            bail!(
                "Safari's history generation has not caught up; refusing to send an early heartbeat"
            )
        }

        let second = self.heartbeat()?;
        let second_generation = Self::recorded_heartbeat_generation(&second)?;
        if heartbeat_is_early(second_generation, target_generation) {
            bail!(
                "Safari heartbeat generation {second_generation} is behind imported generation {target_generation}"
            )
        }
        println!(
            "Safari's first heartbeat was early (generation {first_generation}); a new harmless heartbeat recorded at generation {second_generation}."
        );
        Ok(())
    }
}

fn heartbeat_is_early(heartbeat_generation: i64, imported_generation: i64) -> bool {
    heartbeat_generation < imported_generation
}

impl Drop for HiddenSafari {
    fn drop(&mut self) {
        if !safari_is_running() {
            return;
        }
        // If somebody brought Safari to the foreground, don't close their app.
        let still_hidden = Command::new("lsappinfo")
            .args(["info", "-only", "hidden", "Safari"])
            .output()
            .ok()
            .is_some_and(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).contains("\"Hidden\"=true")
            });
        if !still_hidden {
            return;
        }
        if self.created_heartbeats > 0 {
            // Close only the unique heartbeat windows created by Rindle. Other
            // tabs, windows and Safari browsing history are untouched.
            let script = r#"tell application "Safari"
repeat with n from (count of windows) to 1 by -1
  try
    set pageURL to URL of current tab of window n
    if pageURL is not missing value and pageURL contains "rindle-sync-heartbeat=" then
      close window n
    end if
  end try
end repeat
end tell"#;
            let _ = Command::new("osascript").args(["-e", script]).output();
        }
        let _ = Command::new("osascript")
            .args(["-e", "tell application \"Safari\" to quit"])
            .output();
    }
}

fn wait_for_checkpoint(generation: i64) -> Result<bool> {
    let start = Instant::now();
    let mut checkpoint_seen = false;
    while start.elapsed() < MAX_SYNC_WAIT {
        thread::sleep(Duration::from_secs(5));
        if backend::Backend::synced_generation_read_only().is_ok_and(|last| last >= generation) {
            checkpoint_seen = true;
        }
        if checkpoint_seen && start.elapsed() >= MIN_IDLE_AFTER_HEARTBEAT {
            return Ok(true);
        }
    }
    Ok(checkpoint_seen)
}

pub fn run(dry_run: bool, limit: usize) -> Result<()> {
    if !(1..=EXPORT_LIMIT_PER_RUN).contains(&limit) {
        bail!("Sync limit must be between 1 and {EXPORT_LIMIT_PER_RUN}")
    }
    let mut store = Store::open()?;
    let targets = store.enabled()?;
    if targets.is_empty() {
        println!("No browsers registered. Run 'rindle setup --browser dia' first.");
        return Ok(());
    }
    if dry_run {
        let safari = backend::Backend::open()?;
        let remote = safari.remote_urls()?;
        let outbound = store.pending_outbound(limit)?;
        println!(
            "Dry run: {} Safari iCloud URLs visible (up to {} newest per browser will be queued), {} Chromium URLs eligible for Safari import (up to {}), targets: {}",
            remote.len(),
            limit,
            outbound.len(),
            limit,
            targets.join(", ")
        );
        println!("No history was modified, no website opened, and no browser launched.");
        return Ok(());
    }
    let Some(_lock) = SyncLock::acquire()? else {
        println!("Another Rindle sync is already running; skipping this invocation.");
        return Ok(());
    };
    setup::refresh_if_present()?;
    let mut safari = backend::Backend::open()?;
    let remote = safari.remote_urls()?;
    let mut queued = store.enqueue_remote(&remote.into_iter().take(limit).collect::<Vec<_>>())?;
    let outbound = store.pending_outbound(limit)?;

    let mut inserted = 0;
    let mut known = 0;
    if !outbound.is_empty() {
        if safari_is_running() {
            println!(
                "Safari is already running; queued Chromium URLs will be imported during the next idle sync."
            );
        } else {
            let results = safari.add_urls(&outbound, &safari::backup_directory()?)?;
            inserted = results
                .iter()
                .filter(|(_, generation)| generation.is_some())
                .count();
            known = results.len() - inserted;
            store.mark_in_safari(&results)?;
        }
    }

    let (_, last_synced) = safari.generations()?;
    let pending = store.latest_pending_generation(last_synced)?;
    drop(safari);
    println!(
        "Safari → browsers: {queued} queued; Chromium → Safari: {inserted} new, {known} already present."
    );

    // Refresh incoming iPhone history even when Chromium has nothing to upload.
    // The independent Safari history daemon does not consistently fetch remote
    // changes while Safari's application hasn't launched.
    if safari_is_running() {
        if pending.is_some() {
            println!(
                "Safari is active; leaving iCloud upload to Safari rather than interrupting it."
            );
        }
    } else {
        let _session = HiddenSafari::start(pending)?;
        if let Some(generation) = pending {
            if wait_for_checkpoint(generation)? {
                println!(
                    "Safari's LOCAL sync checkpoint reached generation {generation}; this does NOT confirm delivery to your iPhone."
                );
            } else {
                println!(
                    "Safari's LOCAL sync checkpoint has not reached generation {generation}. Rindle will retry a hidden heartbeat on its next scheduled run."
                );
            }
        } else {
            thread::sleep(ICLOUD_FETCH_WARMUP);
        }
        // Safari's launch pulls new iCloud history into the local SQLite store.
        // Stage the refreshed records after the fetch, not only before launch.
        let refreshed = backend::Backend::open()?.remote_urls()?;
        queued += store.enqueue_remote(&refreshed.into_iter().take(limit).collect::<Vec<_>>())?;
    }
    println!("Safari → browsers: {queued} newly queued after iCloud refresh.");
    println!(
        "Note: Safari history database changes use an unsupported macOS interface; an iCloud checkpoint alone cannot prove that an iPhone received those URLs."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_never_contains_user_url_or_token() {
        assert_eq!(
            HEARTBEAT_PREFIX,
            "https://www.apple.com/?rindle-sync-heartbeat="
        );
        assert!(MAX_SYNC_WAIT >= MIN_IDLE_AFTER_HEARTBEAT);
        assert_eq!(EXPORT_LIMIT_PER_RUN, 1000);
    }

    #[test]
    fn an_early_heartbeat_requires_a_generation_aligned_retry() {
        // Matching the pre-launch generation is insufficient when Safari
        // imported rows were stamped with current_generation + 1.
        assert!(heartbeat_is_early(1208, 1209));
        assert!(!heartbeat_is_early(1209, 1209));
        assert!(!heartbeat_is_early(1210, 1209));
    }
}
