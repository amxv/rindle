# Rindle

**One browsing history across your Mac and iPhone.** Rindle privately synchronizes Safari's iCloud history with Dia, Chrome, or Microsoft Edge. Use Safari on your iPhone and Dia on your Mac; both address bars learn URLs you've visited on the other device.

[Website](https://rindle.ashray.xyz) · [Documentation](https://rindle.ashray.xyz/docs) · [Source](https://github.com/amxv/rindle)

Rindle is currently **macOS-only and unreleased**. The repository is ready for local testing before its first GitHub Release.

**Safari ↔ iPhone syncing is experimental.** Although Rindle can insert records in Mac Safari history and trigger hidden sync attempts, Safari's private database may advance its local sync checkpoint even when iCloud reports an upload error. We observed this on macOS 26.3.1. Do not treat a completed local checkpoint as proof the URLs appeared in iPhone Safari. The supported alternative is importing a `History.json` through Safari's File menu. Unattended delivery to iPhone Safari has not been reliably verified.

## How it works

- **Safari → Dia:** iCloud syncs iPhone visits to Mac Safari. Rindle queues remote-origin URLs locally. When Dia opens, its Manifest V3 extension checks for existing visits, then imports missing URLs through `chrome.history.addUrl()` without opening the websites. It catches up on startup and periodically while running.
- **Dia → Safari:** The extension sends Dia's history to a local Native Messaging host. A Rust LaunchAgent runs every three hours, adds missing URL records to Safari's history after a consistent backup, and triggers iCloud sync by launching Safari hidden and visiting only `https://www.apple.com/?rindle-sync=1`. **No actual Dia history URL is opened in Safari.**
- **Duplicates:** A durable SQLite ledger and destination-history checks prevent bounce-back loops, even if a URL originally came from a different iCloud device.

No permanent server or third-party cloud service is involved. The extension does not read webpage content. Internal/private URLs, fragments, and recognized token-bearing queries are excluded. The local Safari adapter retains up to three consistent history backups.

## Install (after the first release)

```sh
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
rindle setup --browser dia
```

Enable Safari under iCloud on both Mac and iPhone. Open `dia://extensions`, enable Developer mode, choose **Load unpacked**, and select `~/Library/Application Support/Rindle/extension` (or the directory printed by setup). This is a one-time manual browser step. Once loaded, the extension will keep catching up automatically when Dia is running.

**No release has been published yet**, so the installer cannot download Rindle until the first tagged GitHub Release.

## Test locally now

On macOS, with [Rust](https://rustup.rs/) and Node.js 22+:

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

Load the unpacked extension and click **Sync now** from its popup, then confirm `rindle status` says Dia connected. To enable truly unattended three-hour sync, rerun `rindle setup --browser dia` without `--no-schedule` after testing. Safari's protected database may require granting Full Disk Access to the executable or the app launching it; Rindle reports access errors instead of bypassing macOS protections.

If Dia says **“Specified native messaging host not found”**, run `rindle setup --browser dia --no-schedule` again. Rindle installs its native host in **Google Chrome's host directory**, which Dia uses according to AdGuard's browser compatibility registry, as well as Dia and Chromium fallback locations. Reload the unpacked extension if needed. `rindle status` reports host registrations and whether Dia has connected. The Google Chrome and Chromium paths share a browser-aware helper, so configuring Chrome later won't disable Dia.

For an initial small-scale test after loading the extension, run `rindle sync --limit 3` to transfer at most three URLs in either direction rather than starting with a full 1,000-URL batch.

## Chrome and Edge

```sh
rindle setup --browser chrome
rindle setup --browser edge
# Or all three browsers:
rindle setup --all
```

Load the same unpacked extension from `chrome://extensions` or `edge://extensions` and select **Google Chrome** or **Microsoft Edge** in the extension popup. Each browser has its own Native Messaging registration and queue. Dia is the primary target; its native host integration must pass real-browser acceptance testing before the first release.

## Commands

| Command | Function |
| --- | --- |
| `rindle setup [--browser dia\|chrome\|edge] [--no-schedule]` | Register browser extension host, with optional background schedule |
| `rindle setup --all` | Configure Dia, Chrome and Edge |
| `rindle sync --dry-run` | Inspect potential transfers without changing history or launching Safari |
| `rindle sync` | Queue remote Safari URLs, inject new Chromium URLs, sync iCloud hidden |
| `rindle status` | Inspect browser registrations and pending queues |
| `rindle uninstall` | Remove native hosts and background sync without deleting history |
| `rindle upgrade [--check]` | Update the CLI from GitHub Releases after the first release |
| `rindle auto-update on\|off\|status` | Optional daily CLI upgrades, separate from history sync |

The standalone Rust binary embeds the entire extension and native host; Node.js and Cargo are not required after installation.

## Development and releases

```sh
just check
just docs-check
dist plan
```

The scaffold's `cargo-dist` release automation packages checksummed native installers for Intel and Apple Silicon macOS. No release will be tagged until real Dia and iPhone Safari autocomplete have been tested. Apache-2.0 license.
