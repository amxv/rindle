# Rindle

**One browsing history across your Mac and iPhone.** Rindle privately synchronizes Safari's iCloud history with Dia, Chrome, or Microsoft Edge. Use Safari on your iPhone and Dia on your Mac; both address bars learn URLs you've visited on the other device.

[Website](https://rindle.ashray.xyz) · [Documentation](https://rindle.ashray.xyz/docs) · [Source](https://github.com/amxv/rindle)

Rindle is currently **macOS-only and unreleased**. The repository is ready for local testing before its first GitHub Release.

**Safari ↔ iPhone syncing is experimental.** Although Rindle can insert records in Mac Safari history and trigger hidden sync attempts, Safari's private database may advance its local sync checkpoint even when iCloud reports an upload error. We observed this on macOS 26.3.1. Do not treat a completed local checkpoint as proof the URLs appeared in iPhone Safari. A one-entry test using the supported `History.json` importer also did not produce an iPhone suggestion, so it is **not a verified workaround**. Rindle now replicates the earlier user-confirmed hidden Safari approach with a fresh, generation-aligned Apple.com heartbeat. Unattended iPhone delivery still requires device-side verification.

## Privacy and macOS permissions

The current unattended integration accesses Safari's protected `~/Library/Safari/History.db`. macOS requires **Full Disk Access (FDA)** for a standalone background process to do that via direct SQLite access. **FDA is not restricted to Safari history**: it can expose other apps' sensitive data, including Mail and Messages. Rindle's own code only accesses the required browser databases and its local sync ledger, but its current unsandboxed binary has wider operating-system authority when granted FDA. You should understand and accept that distinction before enabling background sync. The one-shot Dia native host, source validation and history ledger work without FDA.

For users who do not grant FDA, keep the automated job disabled with `rindle setup --browser dia --no-schedule`. You can revoke the installed binary's grant at any time under **System Settings → Privacy & Security → Full Disk Access**. Rindle then detects the scheduled read failure and does not alter Safari history.

We have **verified a narrower alternative** on macOS 26.3.1: a sandboxed, locally signed helper gets one-time `NSOpenPanel` access to only `~/Library/Safari` and persists it via a security-scoped bookmark. Separate unattended LaunchAgents have demonstrated read-only access and a real SQLite insert followed by rollback, **without Full Disk Access**. An opt-in experimental Rust adapter delegates Safari operations through the helper via `RINDLE_SCOPED_SAFARI_HELPER`. The source is in [`experiments/scoped-safari`](experiments/scoped-safari), but it **is not a supported public install**. Before replacing FDA we still need end-to-end iCloud verification of the scoped helper's committed writes, stable Developer ID signing, notarization, and automatic helper packaging.

## How it works

- **Safari → Dia:** iCloud syncs iPhone visits to Mac Safari. Rindle queues remote-origin URLs locally. When Dia opens, its Manifest V3 extension checks for existing visits, then imports missing URLs through `chrome.history.addUrl()` without opening the websites. It catches up on startup and periodically while running.
- **Dia → Safari:** The extension sends Dia's history to a local Native Messaging host. A Rust LaunchAgent runs every three hours, adds missing URL records to Safari's history after a consistent backup, and opens Safari hidden with a **new, uniquely tagged Apple.com heartbeat**. It verifies the heartbeat's Safari generation is at least that of the injected records, sends a second harmless heartbeat if the first ran too early, and keeps Safari hidden through its idle-sync window. **No actual Dia history URL is opened in Safari.**
- **Duplicates:** A durable SQLite ledger and destination-history checks prevent bounce-back loops, even if a URL originally came from a different iCloud device.

The macOS background job runs every three hours and transfers up to **100 URLs per direction per run**, newest first. This deliberately staggers the initial history backfill rather than injecting thousands of visits into Safari at once. For a one-time manual import, use `rindle sync --limit N` (maximum 1,000).

**One-time macOS permission:** The scheduled executable needs Full Disk Access to read Safari's protected `History.db`. Open **System Settings → Privacy & Security → Full Disk Access**, click **+**, use `Cmd+Shift+G` to select your installed `rindle` executable (typically `~/.cargo/bin/rindle`), and enable it. You must approve this manually; granting your Terminal or desktop agent access does **not** authorize an independent macOS LaunchAgent. Then restart the job with `launchctl kickstart -k gui/$(id -u)/xyz.rindle.sync`. `rindle status` detects failed scheduled runs and reports when this permission is missing. Full Disk Access is a broad macOS permission; grant it only if you trust the locally installed Rindle executable.

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
