---
title: Quickstart
description: Sync Safari iCloud history with Dia on your Mac.
order: 1
category: Start
summary: Register the native helper, load Dia's extension, and enable background sync.
---

Rindle lets you keep using **Safari on your iPhone** and **Dia on your Mac**, while syncing their visited URLs for autocomplete. It transfers history records, not webpage visits.

**Caution:** The macOS Safari history write path is an experimental, unsupported interface. A local iCloud sync checkpoint does not prove the iPhone has received an imported URL. In testing, Safari sometimes logged upload errors even when that checkpoint advanced. A separate one-entry test through Safari's native `History.json` importer also failed the iPhone autocomplete check. The initial release remains on hold until the corrected hidden-heartbeat approach passes end-to-end device verification.

**The first Rindle release has not been published yet.** Use the [source instructions](#test-from-source) until then.

## 1. Install Rindle

After the first release:

```sh
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
rindle setup --browser dia
```

Setup installs the native host and a macOS LaunchAgent that synchronizes every three hours. It also extracts the browser extension to `~/Library/Application Support/Rindle/extension`.

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

This test setup does **not** enable a recurring job. After installing the extension and verifying the local connection, run `rindle setup --browser dia` to turn on unattended syncing. macOS may require Full Disk Access for the Rindle executable or its launching terminal.

If Rindle displays **“Specified native messaging host not found”**, rerun `rindle setup --browser dia --no-schedule` to repair Dia's Chrome-compatible native-host registration, then reload the extension if needed. This does not launch Safari or change browser history. The shared registration also works when Chrome is configured separately.

Before enabling unattended syncing, try `rindle sync --limit 3` to transfer a small batch in each direction and verify suggestions on your iPhone. The standard schedule processes up to 1,000 URLs per direction, per run.
