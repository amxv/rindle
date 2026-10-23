//! End-to-end framing through the real CLI, using only an isolated test HOME.
#![cfg(target_os = "macos")]

use anyhow::{Result, ensure};
use rindle::native::EXTENSION_ID;
use rindle::state::Store;
use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

#[test]
fn setup_and_native_host_round_trip_without_touching_real_browsers() -> Result<()> {
    let root = tempfile::tempdir()?;
    let home = root.path().join("home");
    let data = root.path().join("rindle");
    fs::create_dir_all(&home)?;
    let exe = env!("CARGO_BIN_EXE_rindle");
    let output = Command::new(exe)
        .args(["setup", "--all", "--no-schedule"])
        .env("HOME", &home)
        .env("RINDLE_HOME", &data)
        .output()?;
    ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let manifest = home.join(
        "Library/Application Support/Dia/User Data/NativeMessagingHosts/xyz.rindle.history.json",
    );
    let contents: serde_json::Value = serde_json::from_slice(&fs::read(manifest)?)?;
    ensure!(contents["allowed_origins"][0] == format!("chrome-extension://{EXTENSION_ID}/"));
    let fallback = home
        .join("Library/Application Support/Chromium/NativeMessagingHosts/xyz.rindle.history.json");
    let fallback_manifest: serde_json::Value = serde_json::from_slice(&fs::read(&fallback)?)?;
    ensure!(
        fallback_manifest["path"] == data.join("native-host-shared").to_string_lossy().as_ref()
    );
    ensure!(contents["path"] == data.join("native-host-dia").to_string_lossy().as_ref());
    ensure!(data.join("extension/background.js").exists());
    ensure!(data.join("extension/manifest.json").exists());
    let google_manifest = home.join(
        "Library/Application Support/Google/Chrome/NativeMessagingHosts/xyz.rindle.history.json",
    );
    ensure!(google_manifest.exists());
    let google_config: serde_json::Value = serde_json::from_slice(&fs::read(&google_manifest)?)?;
    ensure!(
        google_config == fallback_manifest,
        "Dia and Chrome must use the same shared host"
    );
    ensure!(
        home.join(
            "Library/Application Support/Google/ChromeForTesting/NativeMessagingHosts/xyz.rindle.history.json"
        )
        .exists()
    );
    ensure!(home.join("Library/Application Support/Microsoft Edge/NativeMessagingHosts/xyz.rindle.history.json").exists());
    ensure!(
        !home
            .join("Library/LaunchAgents/xyz.rindle.sync.plist")
            .exists()
    );

    let mut state = Store::open_at(&data.join("state.sqlite"))?;
    state.enqueue_remote(&["https://from-iphone.example/interesting".to_string()])?;
    drop(state);

    let mut child = Command::new(contents["path"].as_str().unwrap())
        .arg(format!("chrome-extension://{EXTENSION_ID}/"))
        .env("HOME", &home)
        .env("RINDLE_HOME", &data)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let input = serde_json::json!({
        "version": 1,
        "browser": "dia",
        "visits": [{"url":"https://from-dia.example/page", "title":"From Dia"}],
        "ack": [],
    });
    let bytes = serde_json::to_vec(&input)?;
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(&(bytes.len() as u32).to_ne_bytes())?;
        stdin.write_all(&bytes)?;
    }
    let output = child.wait_with_output()?;
    ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut buffer = output.stdout.as_slice();
    let mut header = [0_u8; 4];
    buffer.read_exact(&mut header)?;
    let len = u32::from_ne_bytes(header) as usize;
    ensure!(buffer.len() == len);
    let response: serde_json::Value = serde_json::from_slice(buffer)?;
    ensure!(response["received"] == 1);
    ensure!(response["import"][0]["url"] == "https://from-iphone.example/interesting");

    let store = Store::open_at(&data.join("state.sqlite"))?;
    ensure!(store.enabled()?.len() == 3);
    ensure!(store.last_contact(rindle::cli::Browser::Dia)?.is_some());
    ensure!(
        store
            .pending_outbound(10)?
            .iter()
            .any(|v| v.url == "https://from-dia.example/page")
    );

    let unauthorized = Command::new(contents["path"].as_str().unwrap())
        .arg("chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/")
        .env("HOME", &home)
        .env("RINDLE_HOME", &data)
        .output()?;
    ensure!(!unauthorized.status.success());

    // The exact same shared executable dispatches both Dia and Chrome requests
    // when both are installed. Registration order must not break either one.
    for browser in ["dia", "chrome"] {
        let mut child = Command::new(google_config["path"].as_str().unwrap())
            .arg(format!("chrome-extension://{EXTENSION_ID}/"))
            .env("HOME", &home)
            .env("RINDLE_HOME", &data)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let bytes = serde_json::to_vec(&serde_json::json!({
            "version": 1, "browser": browser, "visits": [], "ack": []
        }))?;
        {
            let mut stdin = child.stdin.take().unwrap();
            stdin.write_all(&(bytes.len() as u32).to_ne_bytes())?;
            stdin.write_all(&bytes)?;
        }
        let output = child.wait_with_output()?;
        ensure!(
            output.status.success(),
            "shared host rejected {browser}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: serde_json::Value = serde_json::from_slice(&output.stdout[4..])?;
        ensure!(result["ok"] == true);
    }
    Ok(())
}

#[test]
fn installing_chrome_after_dia_does_not_replace_the_shared_host_with_a_chrome_only_host()
-> Result<()> {
    let root = tempfile::tempdir()?;
    let home = root.path().join("home");
    let data = root.path().join("rindle");
    fs::create_dir_all(&home)?;
    let exe = env!("CARGO_BIN_EXE_rindle");
    for browser in ["dia", "chrome", "dia"] {
        let output = Command::new(exe)
            .args(["setup", "--browser", browser, "--no-schedule"])
            .env("HOME", &home)
            .env("RINDLE_HOME", &data)
            .output()?;
        ensure!(
            output.status.success(),
            "setup {browser}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let manifest = home.join("Library/Application Support/Google/Chrome/NativeMessagingHosts/xyz.rindle.history.json");
        let contents: serde_json::Value = serde_json::from_slice(&fs::read(&manifest)?)?;
        ensure!(contents["path"] == data.join("native-host-shared").to_string_lossy().as_ref());
    }
    Ok(())
}

#[test]
fn dry_run_reads_an_isolated_safari_history_without_modifying_it() -> Result<()> {
    let root = tempfile::tempdir()?;
    let home = root.path().join("home");
    let data = root.path().join("rindle");
    let history_path = root.path().join("History.db");
    fs::create_dir_all(&home)?;

    // Use a synthetic Safari schema: a real browser and the user's iCloud
    // database are not involved in this integration test.
    let conn = rusqlite::Connection::open(&history_path)?;
    conn.execute_batch(
        "CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT);
         INSERT INTO metadata VALUES ('current_generation', '42');
         INSERT INTO metadata VALUES ('last_synced_generation', '41');
         CREATE TABLE history_items (
             id INTEGER PRIMARY KEY, url TEXT UNIQUE, domain_expansion TEXT,
             visit_count INTEGER, daily_visit_counts BLOB, visit_count_score INTEGER,
             status_code INTEGER
         );
         CREATE TABLE history_visits (
             id INTEGER PRIMARY KEY, history_item INTEGER, visit_time REAL,
             title TEXT, origin INTEGER, generation INTEGER, attributes INTEGER, score INTEGER
         );
         INSERT INTO history_items (url) VALUES ('https://iphone.example/research');
         INSERT INTO history_visits (history_item, origin, generation)
             VALUES (1, 1, 42);",
    )?;
    drop(conn);

    let exe = env!("CARGO_BIN_EXE_rindle");
    let setup = Command::new(exe)
        .args(["setup", "--browser", "dia", "--no-schedule"])
        .env("HOME", &home)
        .env("RINDLE_HOME", &data)
        .output()?;
    ensure!(
        setup.status.success(),
        "{}",
        String::from_utf8_lossy(&setup.stderr)
    );

    let status = Command::new(exe)
        .arg("status")
        .env("HOME", &home)
        .env("RINDLE_HOME", &data)
        .env("RINDLE_SAFARI_HISTORY_DB", &history_path)
        .output()?;
    ensure!(status.status.success());
    ensure!(
        String::from_utf8(status.stdout)?.contains("iCloud delivery is NOT verified"),
        "Local checkpoint must never be mistaken for phone delivery"
    );

    let before = fs::read(&history_path)?;
    let output = Command::new(exe)
        .args(["sync", "--dry-run"])
        .env("HOME", &home)
        .env("RINDLE_HOME", &data)
        .env("RINDLE_SAFARI_HISTORY_DB", &history_path)
        .output()?;
    ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout)?;
    ensure!(text.contains("1 Safari iCloud URLs visible"));
    ensure!(text.contains("No history was modified"));
    ensure!(fs::read(&history_path)? == before);
    ensure!(!data.join("sync.lock").exists());
    ensure!(
        !home
            .join("Library/LaunchAgents/xyz.rindle.sync.plist")
            .exists()
    );
    Ok(())
}

#[test]
fn unattended_setup_refuses_unscoped_full_disk_access_by_default() -> Result<()> {
    let root = tempfile::tempdir()?;
    let home = root.path().join("home");
    let state = root.path().join("state");
    fs::create_dir_all(&home)?;
    let exe = env!("CARGO_BIN_EXE_rindle");
    let attempted = Command::new(exe)
        .args(["setup", "--browser", "dia"])
        .env("HOME", &home)
        .env("RINDLE_HOME", &state)
        .output()?;
    ensure!(!attempted.status.success());
    ensure!(String::from_utf8(attempted.stderr)?.contains("--allow-full-disk-access"));
    ensure!(
        !home
            .join("Library/LaunchAgents/xyz.rindle.sync.plist")
            .exists()
    );
    ensure!(!state.join("state.sqlite").exists());
    Ok(())
}
