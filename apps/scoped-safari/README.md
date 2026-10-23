# Scoped Safari history helper

Rindle uses a small, sandboxed Swift `.app` for Safari's protected history database. A one-time macOS `NSOpenPanel` authorization selects **only** `~/Library/Safari` and persists that permission as an app-scoped security-scoped bookmark. **The Rust CLI does not need Full Disk Access.**

The helper has user-selected read/write and bookmark entitlements, without network, automation, accessibility or camera entitlements. `ScopedSafariService.swift` provides bounded one-shot JSON-over-stdio operations to inspect iCloud-sourced history, read generation checkpoints and insert new history rows. It never opens the visited webpages. Writes validate the schema, create a consistent SQLite backup and use a transaction.

## Development build

```sh
./apps/scoped-safari/build.sh
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

Read `probe.log` from the helper's own App Sandbox application support container if you need diagnostics. The log reports permission checks but not history URLs. For local development, set `CODESIGN_IDENTITY` if you have a stable local signing identity; the release archive uses ad-hoc signing and the installer rechecks saved access after updates.

**Compatibility:** Safari's private SQLite schema and macOS privacy policy are not documented integration interfaces. The helper rejects schema incompatibilities and an inaccessible selected folder instead of requesting broader access. A manually downloaded `.app` can be subject to Gatekeeper; Rindle's verified command-line archive installer never disables Gatekeeper or strips quarantine.

## Results on macOS 26.3.1 (October 24, 2026)

- **Read-only access: PASS.** The saved bookmark restored successfully both in a manually launched helper and in an independent LaunchAgent without Full Disk Access.
- **Read/write access: PASS.** The same sandboxed helper could open the actual `History.db` with `SQLITE_OPEN_READWRITE`, create/read/delete an unrelated temporary file under the selected folder, and clean up.
- **SQL write authorization: PASS, without persisted changes.** An independent LaunchAgent used the sandboxed helper to execute a real `BEGIN IMMEDIATE`, insert one synthetic Safari history item and visit, then `ROLLBACK`. The synthetic record was absent afterward.
- **Unprivileged Rindle integration: PASS.** The LaunchAgent delegates all Safari read and write operations to the sandboxed helper with no Full Disk Access authorization for the CLI.
- **Live two-way iCloud sync: PASS.** The helper committed a synthetic URL, the user verified its appearance in iPhone Safari, and a separate iPhone visit reached Mac Safari and Dia through the queued Native Messaging protocol.

### Local-only JSON helper protocol

The experimental sandboxed helper also supports `--serve` for one local JSON request on stdin, one response on stdout, then exit. Supported actions: `checkpoint`, `inspect`, `visit_generation`, and `insert`. The `insert` action accepts `"dry_run":true` to exercise the real Safari database transaction and roll it back, leaving no changed history records. Only the saved, explicitly selected `~/Library/Safari` folder is accessible; callers cannot supply arbitrary database paths.

From a developer checkout, you can test the Rust adapter without changing an existing scheduled job:

```sh
export RINDLE_SCOPED_SAFARI_HELPER=/tmp/rindle-scoped-safari-experiment/RindleScopedProbe.app/Contents/MacOS/RindleScopedProbe
cargo run -- status
cargo run -- sync --dry-run --limit 1
```

The release packager, `scripts/package-release.sh`, embeds this helper into both native macOS architecture archives alongside the Rust CLI. The installer validates the checksum and ad-hoc code signature, then opens the folder picker only if the persisted bookmark is unavailable.

**Important:** Setting the environment variable above affects only that process. Use `rindle setup --scoped-helper /absolute/path/to/RindleScopedProbe` to update the three-hour LaunchAgent. `rindle status` identifies the installed backend; do not revoke an old Full Disk Access grant until the scheduled service uses the scoped helper successfully.

References: [Apple: Accessing files from the macOS App Sandbox](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox), [Apple: Security-scoped bookmarks](https://developer.apple.com/documentation/professional-video-applications/enabling-security-scoped-bookmark-and-url-access), [Apple: Code Signing Requirements](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements).
