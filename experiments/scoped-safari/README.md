# Experimental Safari folder-scoped access (not distributed)

**Goal:** Find out whether Rindle can replace Full Disk Access (FDA) with a macOS `NSOpenPanel` consent for **only** `~/Library/Safari`, persisted via an app-scoped security-scoped bookmark. This is an exploratory, self-contained macOS app and is **not part of the released CLI, LaunchAgent or extension**.

The sandbox entitlements grant user-selected read/write file access and app-scoped bookmarks. There are no network, automation, accessibility or camera entitlements. The binary's only database operation is `SELECT COUNT(*) FROM history_items`; it never reads or logs URLs and never modifies browser history.

## Try on a Mac

```sh
./experiments/scoped-safari/build.sh
open /tmp/rindle-scoped-safari-experiment/RindleScopedProbe.app
```

The Open panel asks for **exactly** `~/Library/Safari`. It refuses a broader selection and stores a bookmark **only if a read-only database probe succeeds**. To test persistence after the picker app exits:

```sh
/tmp/rindle-scoped-safari-experiment/RindleScopedProbe.app/Contents/MacOS/RindleScopedProbe --verify
```

To test that the *selected Safari folder* is writable (without changing Safari
history), use `--write-check`. The helper opens `History.db` with SQLite's
read/write flag and `PRAGMA query_only=ON`, then creates, verifies and deletes
a uniquely named temporary file in `~/Library/Safari`. No browser records or
websites are opened or modified.

```sh
/tmp/rindle-scoped-safari-experiment/RindleScopedProbe.app/Contents/MacOS/RindleScopedProbe --write-check
```

Read `probe.log` from the helper's own App Sandbox application support container if you need diagnostics. No history URLs are recorded. For local testing with a stable signing identity you control, set `CODESIGN_IDENTITY` before building; ad-hoc signatures won't reliably retain bookmark identity across updated binaries.

**Important limitations:** Apple's Open panel extends an app's sandbox to a selected folder, but macOS mandatory access controls (TCC) may still deny `~/Library/Safari` despite selection. Apple explicitly documents this caveat. A successful local read test plus a **separate LaunchAgent test with a persisted bookmark and no FDA** are required before this can replace Rindle's current direct SQLite adapter. Do not claim success or ship this as a no-FDA solution until both tests pass. A production integration would additionally need a signed, sandboxed broker to perform Rindle's Safari SQLite read/write operations and exchange narrowly scoped data with the unprivileged Rust scheduler.

## Results on macOS 26.3.1 (October 24, 2026)

- **Read-only access: PASS.** The saved bookmark restored successfully both in a manually launched helper and in an independent LaunchAgent without Full Disk Access.
- **Read/write access: PASS.** The same sandboxed helper could open the actual `History.db` with `SQLITE_OPEN_READWRITE`, create/read/delete an unrelated temporary file under the selected folder, and clean up.
- **SQL write authorization: PASS, without persisted changes.** An independent LaunchAgent used the sandboxed helper to execute a real `BEGIN IMMEDIATE`, insert one synthetic Safari history item and visit, then `ROLLBACK`. The synthetic record was absent afterward.
- **Unprivileged Rindle integration: READ verified.** Set `RINDLE_SCOPED_SAFARI_HELPER` to the signed executable path to opt into the Rust backend adapter. A separate LaunchAgent ran `rindle sync --dry-run --limit 1` and read Safari history through the bookmark without FDA.
- **Live iCloud history transfer with this helper:** not yet verified. Safari was open in an existing user session during the attempted full test, so no live write was made. Rindle's **existing, FDA-enabled** installation was deliberately left unchanged.

### Local-only JSON helper protocol

The experimental sandboxed helper also supports `--serve` for one local JSON request on stdin, one response on stdout, then exit. Supported actions: `checkpoint`, `inspect`, `visit_generation`, and `insert`. The `insert` action accepts `"dry_run":true` to exercise the real Safari database transaction and roll it back, leaving no changed history records. Only the saved, explicitly selected `~/Library/Safari` folder is accessible; callers cannot supply arbitrary database paths.

From a developer checkout, try the opt-in Rust adapter **without replacing the installed job**:

```sh
export RINDLE_SCOPED_SAFARI_HELPER=/tmp/rindle-scoped-safari-experiment/RindleScopedProbe.app/Contents/MacOS/RindleScopedProbe
cargo run -- status
cargo run -- sync --dry-run --limit 1
```

This is a proof of implementation, not yet a supported installer feature. Release-quality integration still needs a permanent helper bundle, stable Developer ID code signing, notarization, and a staged end-to-end iPhone test before FDA can safely be revoked.

**Important:** The environment variable above affects only the invoked Rindle
process. The existing `xyz.rindle.sync` LaunchAgent still uses its current
direct SQLite/FDA backend unless a separate test LaunchAgent supplies the
variable. `rindle status` explicitly distinguishes these. Do **not** revoke
Full Disk Access for a functioning installation until the scoped backend has
passed an end-to-end upload test and the scheduled service is migrated.

References: [Apple: Accessing files from the macOS App Sandbox](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox), [Apple: Security-scoped bookmarks](https://developer.apple.com/documentation/professional-video-applications/enabling-security-scoped-bookmark-and-url-access), [Apple: Code Signing Requirements](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements).
