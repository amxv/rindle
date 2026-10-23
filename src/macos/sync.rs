//! Three-hour macOS sync task: queue remote Safari URLs for extensions, inject
//! new Chromium URLs into Safari, and use only Apple's home page as the hidden
//! iCloud sync heartbeat. No user browsing URL is ever navigated.
use crate::macos::{safari, setup};
use crate::state::{self, Store};
use anyhow::{Context, Result, bail};
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

const EXPORT_LIMIT_PER_RUN: usize = 1000;
const HEARTBEAT_URL: &str = "https://www.apple.com/?rindle-sync=1";
const MAX_SYNC_WAIT: Duration = Duration::from_secs(150);
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
    created_heartbeat: bool,
}

impl HiddenSafari {
    fn start(trigger_upload: bool) -> Result<Self> {
        if safari_is_running() {
            bail!("Safari is already running; refusing to interfere with an existing session")
        }
        exec_success("open", &["-gj", "-a", "Safari"])?;
        let session = Self {
            created_heartbeat: trigger_upload,
        };
        if trigger_upload {
            // This literal is intentionally not constructed from any user history.
            let script = format!(
                "tell application \"Safari\" to make new document with properties {{URL:\"{HEARTBEAT_URL}\"}}"
            );
            exec_success("osascript", &["-e", &script])?;
        }
        Ok(session)
    }
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
        if self.created_heartbeat {
            // Closing the heartbeat tab avoids cluttering Safari's restored
            // windows. The only navigated URL was this fixed literal.
            let script = r#"tell application "Safari"
repeat with w in windows
  try
    set pageURL to URL of current tab of w
    if pageURL is not missing value and pageURL contains "rindle-sync=1" then
      close w
      exit repeat
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
    while start.elapsed() < MAX_SYNC_WAIT {
        thread::sleep(Duration::from_secs(2));
        let safari = safari::Safari::open()?;
        if safari.generations()?.1 >= generation {
            return Ok(true);
        }
    }
    Ok(false)
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
        let safari = safari::Safari::open()?;
        let remote = safari.remote_urls()?;
        let outbound = store.pending_outbound(limit)?;
        println!(
            "Dry run: {} Safari iCloud URLs visible, {} Chromium URLs eligible for Safari import (first {}), targets: {}",
            remote.len(),
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
    let mut safari = safari::Safari::open()?;
    let remote = safari.remote_urls()?;
    let mut queued = store.enqueue_remote(&remote)?;
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
        let _session = HiddenSafari::start(pending.is_some())?;
        if let Some(generation) = pending {
            if wait_for_checkpoint(generation)? {
                println!("iCloud history sync checkpoint reached generation {generation}.");
            } else {
                println!(
                    "iCloud hasn't acknowledged generation {generation} yet. Rindle will retry on its next scheduled run."
                );
            }
        } else {
            thread::sleep(ICLOUD_FETCH_WARMUP);
        }
        // Safari's launch pulls new iCloud history into the local SQLite store.
        // Stage the refreshed records after the fetch, not only before launch.
        let refreshed = safari::Safari::open()?.remote_urls()?;
        queued += store.enqueue_remote(&refreshed)?;
    }
    println!("Safari → browsers: {queued} newly queued after iCloud refresh.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_never_contains_user_url_or_token() {
        assert_eq!(HEARTBEAT_URL, "https://www.apple.com/?rindle-sync=1");
        assert_eq!(EXPORT_LIMIT_PER_RUN, 1000);
    }
}
