//! Install the embedded extension, native host manifests, and a user LaunchAgent.
//! Does not modify Safari or Chromium's existing history.
use crate::cli::Browser;
use crate::native::{EXTENSION_ID, HOST_NAME};
use crate::state::{self, Store};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const SYNC_JOB: &str = "xyz.rindle.sync";
const SYNC_INTERVAL: u32 = 3 * 60 * 60;
const MANIFEST: &str = include_str!("../../extension/manifest.json");
const EXTENSION_FILES: [(&str, &str); 4] = [
    (
        "background.js",
        include_str!("../../extension/background.js"),
    ),
    ("utils.mjs", include_str!("../../extension/utils.mjs")),
    ("popup.html", include_str!("../../extension/popup.html")),
    ("popup.js", include_str!("../../extension/popup.js")),
];

pub fn extension_dir() -> Result<PathBuf> {
    Ok(state::app_dir()?.join("extension"))
}

fn home() -> Result<PathBuf> {
    Ok(PathBuf::from(
        env::var_os("HOME").context("HOME is missing")?,
    ))
}

fn browser_paths(browser: Browser) -> Result<Vec<PathBuf>> {
    let dir = home()?.join("Library/Application Support");
    Ok(match browser {
        // Chromium forks vary in which user-data root they consult. Register
        // both Dia roots with the same constrained, per-browser host wrapper.
        Browser::Dia => vec![
            dir.join("Dia/User Data/NativeMessagingHosts"),
            dir.join("Dia/NativeMessagingHosts"),
        ],
        Browser::Chrome => vec![
            dir.join("Google/Chrome/NativeMessagingHosts"),
            // Since Chrome 146, Chrome for Testing has a separate lookup path.
            // This registration enables isolated, headless extension QA.
            dir.join("Google/ChromeForTesting/NativeMessagingHosts"),
        ],
        Browser::Edge => vec![dir.join("Microsoft Edge/NativeMessagingHosts")],
    })
}

pub fn write_file(path: &Path, contents: &[u8], executable: bool) -> Result<()> {
    let dir = path.parent().context("File has no parent directory")?;
    fs::create_dir_all(dir)?;
    if let Ok(meta) = fs::symlink_metadata(path)
        && !meta.is_file()
    {
        bail!("Refusing to replace nonregular path {}", path.display());
    }
    if fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    let tmp = dir.join(format!(
        ".rindle-write-{}-{}",
        std::process::id(),
        state::now()
    ));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new().create_new(true).write(true).open(&tmp)?;
        file.set_permissions(fs::Permissions::from_mode(if executable {
            0o700
        } else {
            0o600
        }))?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}

pub fn write_extension_bundle() -> Result<PathBuf> {
    let dir = extension_dir()?;
    fs::create_dir_all(&dir)?;
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    for (name, contents) in EXTENSION_FILES {
        write_file(&dir.join(name), contents.as_bytes(), false)?;
    }
    let mut manifest: Value = serde_json::from_str(MANIFEST)?;
    manifest["version"] = json!(env!("CARGO_PKG_VERSION"));
    write_file(
        &dir.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest)?,
        false,
    )?;
    Ok(dir)
}

fn shell_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "'\\''"))
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn host_wrapper(binary: &Path, browser: Browser) -> String {
    format!(
        "#!/bin/sh\nexec {} host --browser {} \"$@\"\n",
        shell_quote(&binary.to_string_lossy()),
        browser.as_str(),
    )
}

fn host_manifest(wrapper: &Path) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(&json!({
        "name": HOST_NAME,
        "description": "Rindle private Safari / Chromium history bridge",
        "path": wrapper.to_string_lossy(),
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{EXTENSION_ID}/")],
    }))?)
}

fn register_host(browser: Browser, binary: &Path) -> Result<()> {
    let wrapper = state::app_dir()?.join(format!("native-host-{}", browser.as_str()));
    write_file(&wrapper, host_wrapper(binary, browser).as_bytes(), true)?;
    for path in browser_paths(browser)? {
        write_file(
            &path.join(format!("{HOST_NAME}.json")),
            &host_manifest(&wrapper)?,
            false,
        )?;
    }
    Ok(())
}

fn job_path() -> Result<PathBuf> {
    Ok(home()?.join(format!("Library/LaunchAgents/{SYNC_JOB}.plist")))
}

fn job_xml(binary: &Path, logfile: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>{SYNC_JOB}</string>
<key>ProgramArguments</key><array><string>{}</string><string>sync</string></array>
<key>RunAtLoad</key><true/>
<key>StartInterval</key><integer>{SYNC_INTERVAL}</integer>
<key>StandardOutPath</key><string>{}</string>
<key>StandardErrorPath</key><string>{}</string>
</dict></plist>
"#,
        escape_xml(&binary.to_string_lossy()),
        escape_xml(&logfile.to_string_lossy()),
        escape_xml(&logfile.to_string_lossy()),
    )
}

fn uid() -> Result<String> {
    let output = Command::new("id").arg("-u").output()?;
    if !output.status.success() {
        bail!("Cannot determine macOS user UID")
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn launchctl(args: &[&str]) -> Result<()> {
    let output = Command::new("launchctl")
        .args(args)
        .output()
        .context("launchctl is unavailable")?;
    if !output.status.success() {
        bail!(
            "launchctl {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
    Ok(())
}

fn install_schedule(binary: &Path) -> Result<()> {
    let logdir = home()?.join("Library/Logs/Rindle");
    fs::create_dir_all(&logdir)?;
    let plist = job_path()?;
    write_file(
        &plist,
        job_xml(binary, &logdir.join("sync.log")).as_bytes(),
        false,
    )?;
    let target = format!("gui/{}", uid()?);
    let label = format!("{target}/{SYNC_JOB}");
    let _ = launchctl(&["bootout", &label]);
    launchctl(&["bootstrap", &target, &plist.to_string_lossy()])?;
    Ok(())
}

fn remove_schedule() -> Result<()> {
    let plist = job_path()?;
    if plist.exists() {
        let target = format!("gui/{}/{SYNC_JOB}", uid()?);
        let _ = launchctl(&["bootout", &target]);
        fs::remove_file(&plist)?;
    }
    Ok(())
}

pub fn run(browser: Browser, all: bool, no_schedule: bool) -> Result<()> {
    let binary = env::current_exe()?.canonicalize()?;
    let store = Store::open()?;
    let extension = write_extension_bundle()?;
    let selected = if all {
        Browser::ALL.to_vec()
    } else {
        vec![browser]
    };
    for browser in selected {
        register_host(browser, &binary)?;
        store.enable(browser)?;
        println!(
            "Registered {} native messaging integration",
            browser.as_str()
        );
    }
    if !no_schedule {
        install_schedule(&binary)?;
        println!("Background sync: every 3 hours via macOS launchd");
    } else {
        remove_schedule()?;
        println!("Background schedule: not enabled (--no-schedule)");
    }
    println!("\nExtension directory: {}", extension.display());
    println!(
        "Open dia://extensions (or chrome://extensions / edge://extensions), enable Developer mode, and Load unpacked from that directory."
    );
    println!("In Chrome and Edge, select the corresponding browser in Rindle's extension popup.");
    println!("After installing, check the extension popup and run 'rindle status'.");
    Ok(())
}

/// Sync can refresh a previously installed extension after the CLI upgrades,
/// without ever installing a new extension or enabling another browser.
pub fn refresh_if_present() -> Result<()> {
    if extension_dir()?.join("manifest.json").is_file() {
        write_extension_bundle()?;
    }
    Ok(())
}

pub fn status() -> Result<()> {
    let store = Store::open()?;
    let browsers = store.enabled()?;
    let counts = store.counts()?;
    println!("Rindle {}", env!("CARGO_PKG_VERSION"));
    println!("State: {}", state::app_dir()?.display());
    println!("Extension: {}", extension_dir()?.display());
    println!(
        "Background sync: {}",
        if job_path()?.exists() {
            "configured (3 hours)"
        } else {
            "off"
        }
    );
    match crate::macos::safari::Safari::open() {
        Ok(safari) => {
            let (_, checkpoint) = safari.generations()?;
            println!("Safari history: accessible (last iCloud sync generation {checkpoint})");
        }
        Err(error) => println!("Safari history: unavailable ({error:#})"),
    }
    for browser in browsers {
        if let Some(item) = Browser::ALL.iter().find(|b| b.as_str() == browser) {
            match store.last_contact(*item)? {
                Some(timestamp) => {
                    println!("{}: connected (last contact Unix {timestamp})", browser)
                }
                None => println!("{}: registered; extension has not connected yet", browser),
            }
        }
    }
    println!(
        "Known Chromium URLs: {}; queued Safari → browsers: {}; known Safari imports: {}",
        counts.0, counts.1, counts.2
    );
    Ok(())
}

pub fn uninstall() -> Result<()> {
    remove_schedule()?;
    for browser in Browser::ALL {
        for dir in browser_paths(browser)? {
            let path = dir.join(format!("{HOST_NAME}.json"));
            if let Ok(contents) = fs::read_to_string(&path)
                && contents.contains(EXTENSION_ID)
            {
                fs::remove_file(path)?;
            }
        }
        let wrapper = state::app_dir()?.join(format!("native-host-{}", browser.as_str()));
        if wrapper.is_file() {
            fs::remove_file(wrapper)?;
        }
    }
    Store::open()?.disable_all()?;
    println!("Removed Rindle launchd sync and native host registrations.");
    println!(
        "Safari/Dia history and your local sync ledger were preserved. Remove the extension manually from your browser if desired."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_manifest_and_job_are_constrained() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let exe = dir.path().join("rindle 'cli' & test");
        let wrapper = host_wrapper(&exe, Browser::Dia);
        assert!(wrapper.contains("exec '/"));
        assert!(wrapper.contains("'\\''"));
        assert!(wrapper.contains("--browser dia \"$@\""));
        let parsed: Value = serde_json::from_slice(&host_manifest(&exe)?)?;
        assert_eq!(
            parsed["allowed_origins"][0],
            format!("chrome-extension://{EXTENSION_ID}/")
        );
        let plist = job_xml(&exe, &dir.path().join("rindle.log"));
        assert!(plist.contains("&amp;"));
        assert!(plist.contains("<integer>10800</integer>"));
        Ok(())
    }

    #[test]
    fn embedded_extension_declares_stable_id_and_minimal_permissions() -> Result<()> {
        let manifest: Value = serde_json::from_str(MANIFEST)?;
        assert_eq!(manifest["manifest_version"], 3);
        assert_eq!(
            manifest["permissions"],
            json!(["history", "nativeMessaging", "storage", "alarms"])
        );
        assert_eq!(EXTENSION_ID.len(), 32);
        Ok(())
    }
}
