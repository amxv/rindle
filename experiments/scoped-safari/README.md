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

Read `probe.log` from the helper's own App Sandbox application support container if you need diagnostics. No history URLs are recorded. For local testing with a stable signing identity you control, set `CODESIGN_IDENTITY` before building; ad-hoc signatures won't reliably retain bookmark identity across updated binaries.

**Important limitations:** Apple's Open panel extends an app's sandbox to a selected folder, but macOS mandatory access controls (TCC) may still deny `~/Library/Safari` despite selection. Apple explicitly documents this caveat. A successful local read test plus a **separate LaunchAgent test with a persisted bookmark and no FDA** are required before this can replace Rindle's current direct SQLite adapter. Do not claim success or ship this as a no-FDA solution until both tests pass. A production integration would additionally need a signed, sandboxed broker to perform Rindle's Safari SQLite read/write operations and exchange narrowly scoped data with the unprivileged Rust scheduler.

References: [Apple: Accessing files from the macOS App Sandbox](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox), [Apple: Security-scoped bookmarks](https://developer.apple.com/documentation/professional-video-applications/enabling-security-scoped-bookmark-and-url-access), [Apple: Code Signing Requirements](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements).
