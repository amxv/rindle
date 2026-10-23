# Rindle

**One browsing history across your Mac and iPhone.** Rindle privately synchronizes Safari's iCloud history with Dia, Chrome, or Microsoft Edge. Use Safari on your iPhone and Dia on your Mac; both address bars learn URLs you've visited on the other device.

[Website](https://rindle.ashray.xyz) · [Documentation](https://rindle.ashray.xyz/docs) · [Source](https://github.com/amxv/rindle)

Rindle is currently **macOS-only and unreleased**. The repository is ready for local testing before its first GitHub Release.

**Safari ↔ iPhone syncing has been confirmed on a real iPhone and Dia (October 24, 2026).** Safari's history database is nevertheless a private, undocumented interface, and future macOS updates may require compatibility fixes. An advancing local iCloud checkpoint alone is not proof of delivery: Rindle verifies the heartbeat generation and allows the normal idle-sync window, but iPhone delivery must ultimately be assessed on the device.

## Privacy and macOS permissions

**Rindle's recommended background integration does not need Full Disk Access.** A small, sandboxed and signed macOS helper asks you to select only `~/Library/Safari` through Apple's folder picker. A persistent security-scoped bookmark lets that helper access Safari's history on subsequent scheduled runs. The normal Rust CLI and browser extension do not receive blanket disk privileges.

The old direct SQLite backend is still available **only with explicit `rindle setup --allow-full-disk-access`** for advanced users. FDA is broad: it could expose unrelated private Mail, Messages, and other app data. It is not an appropriate default for a history-sync tool. If you previously granted it to Rindle, revoke that grant in System Settings after `rindle status` confirms the scoped backend is healthy.

The scoped helper has passed real SQLite read, write, rollback, backup, and one-entry iCloud-to-iPhone tests. An actual unattended Rindle LaunchAgent has also completed its three-hour batch using the saved folder permission. The public installer will ship this helper with a stable Developer ID signature and notarization; the first release remains blocked until those Apple signing credentials are configured.

## How it works

- **Safari → Dia:** iCloud syncs iPhone visits to Mac Safari. Rindle queues remote-origin URLs locally. When Dia opens, its Manifest V3 extension checks for existing visits, then imports missing URLs through `chrome.history.addUrl()` without opening the websites. It catches up on startup and periodically while running.
- **Dia → Safari:** The extension sends Dia's history to a local Native Messaging host. A Rust LaunchAgent runs every three hours, adds missing URL records to Safari's history after a consistent backup, and opens Safari hidden with a **new, uniquely tagged Apple.com heartbeat**. It verifies the heartbeat's Safari generation is at least that of the injected records, sends a second harmless heartbeat if the first ran too early, and keeps Safari hidden through its idle-sync window. **No actual Dia history URL is opened in Safari.**
- **Duplicates:** A durable SQLite ledger and destination-history checks prevent bounce-back loops, even if a URL originally came from a different iCloud device.

The macOS background job runs every three hours and transfers up to **100 URLs per direction per run**, newest first. This deliberately staggers the initial history backfill rather than injecting thousands of visits into Safari at once. For a one-time manual import, use `rindle sync --limit N` (maximum 1,000).

**One-time macOS permission:** The official installer opens a folder picker and asks you to select **only `~/Library/Safari`**. This allows the sandboxed helper to read and update that folder on later launches. **Do not grant Full Disk Access** for the normal installation.

No permanent server or third-party cloud service is involved. The extension does not read webpage content. Internal/private URLs, fragments, and recognized token-bearing queries are excluded. The local Safari adapter retains up to three consistent history backups.

## Install (after the first release)

```sh
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
```

The installer downloads a checksummed, Developer ID-signed and notarized universal Swift helper, installs the CLI, requests access to only `~/Library/Safari`, and registers the three-hour LaunchAgent. Enable Safari under iCloud on both Mac and iPhone. Open `dia://extensions`, enable Developer mode, choose **Load unpacked**, and select `~/Library/Application Support/Rindle/extension`. This is a one-time manual browser step. Once loaded, the extension will keep catching up automatically when Dia is running.

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

Load the unpacked extension and click **Sync now** from its popup, then confirm `rindle status` says Dia connected. To enable unattended three-hour sync from a source checkout, first build/sign the sandboxed helper under `experiments/scoped-safari`, authorize the Safari folder using its macOS folder picker, and run `rindle setup --browser dia --scoped-helper /absolute/path/to/RindleScopedProbe`. Rindle will refuse to silently schedule the legacy Full Disk Access backend.

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
| `rindle setup [--browser dia\|chrome\|edge] [--no-schedule]` | Register browser extension host and scoped Safari background scheduler |
| `rindle setup --scoped-helper /absolute/path/to/RindleScopedProbe` | Configure a particular signed, authorized folder-scoped helper |
| `rindle setup --allow-full-disk-access` | Explicitly choose the legacy, overprivileged direct SQLite backend |
| `rindle setup --all` | Configure Dia, Chrome and Edge |
| `rindle sync --dry-run` | Inspect potential transfers without changing history or launching Safari |
| `rindle sync` | Queue remote Safari URLs, inject new Chromium URLs, sync iCloud hidden |
| `rindle status` | Inspect browser registrations and pending queues |
| `rindle uninstall` | Remove native hosts and background sync without deleting history |
| `rindle upgrade [--check]` | Update the CLI from GitHub Releases after the first release |
| `rindle auto-update on\|off\|status` | Optional daily CLI upgrades, separate from history sync |

The Rust binary embeds the browser extension and Native Messaging host. The installer additionally downloads a separately signed/notarized universal Swift helper. Node.js and Cargo are not required after installation.

## Development and releases

```sh
just check
just docs-check
dist plan
```

The `cargo-dist` automation builds Intel and Apple Silicon macOS CLI installers. A required pre-host job builds, signs, notarizes, and attaches the universal sandboxed helper and its checksum. No release will be tagged until the Apple Developer ID/notarization secrets are configured and the combined installer passes checks. Apache-2.0 license.
