---
title: Quickstart
description: Sync Safari iCloud history with Dia on your Mac.
order: 1
category: Start
summary: Register the native helper, load Dia's extension, and enable background sync.
---

Rindle lets you keep using **Safari on your iPhone** and **Dia on your Mac**, while syncing their visited URLs for autocomplete. It transfers history records, not webpage visits.

**Compatibility:** The generation-aligned Safari history sync was verified end to end with iPhone Safari and Dia on macOS 26.3.1. It still uses Safari's private, undocumented history format. Rindle fails closed on schema changes and backs up each write batch, but future macOS changes may require maintenance.

**The first Rindle release has not been published yet.** Use the [source instructions](#test-from-source) until then.

## 1. Install Rindle

After the first release:

```sh
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
```

The installer downloads and verifies the Intel/Apple Silicon CLI and a signed, notarized universal Safari helper. It installs the native host and a macOS LaunchAgent that synchronizes every three hours, and extracts the browser extension to `~/Library/Application Support/Rindle/extension`.

### One-time Safari folder permission on macOS

The installer opens Apple's normal folder picker. Select **`~/Library/Safari`**. Only the sandboxed Safari helper receives persistent access to this folder, using a security-scoped bookmark. Rindle does **not** require Full Disk Access for its standard automatic sync.

To verify:

```sh
rindle status
```

The Safari status should identify the **sandboxed, folder-scoped helper**. The initial release will not ship until its Developer ID signing and Apple notarization pass the release pipeline. See [Privacy and permissions](/docs/privacy).

## 2. Load the extension in Dia

1. Open `dia://extensions` or Dia's Extensions settings.
2. Enable **Developer mode**.
3. Choose **Load unpacked** and select the extension directory shown by setup.
4. Open the Rindle popup and click **Sync now**.

The extension needs History, Storage, Alarms and Native Messaging permissions. It does not inject scripts into websites.

## 3. Enable iCloud Safari on both devices

Enable Safari in iCloud Settings on your Mac and iPhone. Mac Safari is only Rindle's bridge, not your day-to-day browser.

```sh
rindle status
rindle sync --dry-run
rindle sync
```

The dry run does not modify history. A normal sync opens a **unique Apple.com heartbeat URL** while Safari is hidden if an iCloud upload is pending. If Safari records it one history generation behind the imported URLs, Rindle waits and opens a second harmless Apple.com heartbeat to avoid silently skipping the imported batch. Safari remains hidden through its idle-sync period. Dia can remain closed until you next use it; its extension catches up then.

## Test from source

With macOS, Rust and Node.js installed:

```sh
git clone https://github.com/amxv/rindle.git
cd rindle
cargo build
cargo test
node --test extension/utils.test.mjs
./target/debug/rindle setup --browser dia --no-schedule
./target/debug/rindle status
./target/debug/rindle sync --dry-run
```

This test setup does **not** enable a recurring job. To enable unattended sync from source, build and sign `experiments/scoped-safari`, select `~/Library/Safari` in its folder picker, and then use `rindle setup --browser dia --scoped-helper /absolute/path/to/RindleScopedProbe`. Rindle refuses to enable the legacy Full Disk Access backend without the explicit `--allow-full-disk-access` option.

If Rindle displays **“Specified native messaging host not found”**, rerun `rindle setup --browser dia --no-schedule` to repair Dia's Chrome-compatible native-host registration, then reload the extension if needed. This does not launch Safari or change browser history. The shared registration also works when Chrome is configured separately.

Before enabling unattended syncing, try `rindle sync --limit 3` to transfer a small batch in each direction and verify suggestions on your iPhone. The three-hour background schedule is intentionally bounded to **100 URLs per direction per run**, newest first. Manual `rindle sync --limit N` permits up to 1,000 URLs.
