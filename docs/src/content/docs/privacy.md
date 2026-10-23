---
title: Privacy and permissions
description: Why Rindle needs Safari data access and how we are testing a narrower alternative.
order: 5
category: Reference
summary: Understand macOS Full Disk Access, Safari history protection, and sandboxed folder permissions.
---

Rindle reads history URLs locally. It does not upload browsing history to a Rindle server, does not read webpage contents, and never reopens the pages you previously visited to manufacture history entries. Dia/Chrome/Edge communicate with a one-shot local Native Messaging helper and Safari history is synced through the Apple iCloud account that is already configured on your devices.

## No Full Disk Access in the standard installation

Apple protects `~/Library/Safari/History.db` using macOS privacy controls (TCC). A standalone, unsandboxed LaunchAgent usually cannot open this database without Full Disk Access. Rindle instead uses a small, App Sandbox-enabled Swift helper that asks you to select exactly `~/Library/Safari` through the system folder picker and stores a persistent security-scoped bookmark.

The background Rust CLI can read the local Rindle ledger and connect to the sandboxed helper through one-shot local stdio, but it does not itself receive blanket Safari database access. The signed helper limits its file access to the user-selected Safari folder and its own app container.

The legacy direct-SQLite fallback explicitly requires `rindle setup --allow-full-disk-access`. That is *not recommended*: FDA can expose unrelated private Mail, Messages, and other app data. It is not restricted to Safari, though it does not grant microphone or camera access.

To turn off automatic syncing entirely:

```sh
rindle setup --browser dia --no-schedule
```

You can revoke an earlier Rindle FDA grant in System Settings after verifying the scoped scheduler with `rindle status`. Rindle never bypasses TCC or grants itself privacy permissions.

## Scoped access verified on a real Mac

Apple supports opening a system folder picker (`NSOpenPanel`) in a sandboxed app and preserving user-granted access with a security-scoped bookmark. Rindle's [sandboxed Safari helper](https://github.com/amxv/rindle/tree/main/apps/scoped-safari) lets users select only `~/Library/Safari` once, then restores that permission for later scheduled runs.

**What we have verified (October 24, 2026, macOS 26.3.1):** The user selected `~/Library/Safari` through a sandboxed folder picker, then the same signed helper successfully restored its bookmark in later processes. Independent, sandboxed LaunchAgents without any Full Disk Access grant read the actual Safari `History.db`, opened it with SQLite's read/write flag, and exercised a real `BEGIN IMMEDIATE`/insert/`ROLLBACK` transaction. The synthetic test record was absent afterward. A Rust Rindle LaunchAgent that also had no FDA successfully delegated its read-only sync preview to this sandboxed helper.

**Subsequent results:** The sandboxed helper committed real Safari history entries, a user verified one of those URLs in iPhone Safari autocomplete, and an independent macOS LaunchAgent succeeded. The actual three-hour Rindle scheduler was also migrated and completed successfully using the folder-scoped helper. A separate unique iPhone Safari visit traveled back into Mac Safari and was acknowledged by Dia's extension. Both directions were tested on October 24, 2026.

**Distribution:** Rindle bundles its ad-hoc-signed, sandboxed `.app` with the CLI in a SHA-256-verified GitHub Release tarball. Its shell installer uses HTTPS `curl` and `tar`, which do not normally assign `com.apple.quarantine`. It never disables Gatekeeper or removes a quarantine attribute. Manually downloaded `.app` bundles may still be blocked, and an ad-hoc signature does not establish a verified publisher identity.

## Code signing and updates

macOS tracks sandbox bookmarks and privacy authorizations using a program's code signing identity. Ad-hoc signatures work for Rindle's folder-scoped helper but may not preserve authorization across every update. The installer verifies saved access after replacing the `.app` and opens the folder picker again if needed; noninteractive auto-upgrades fail without changing the installed version if reauthorization is required. The release workflow checks the ad-hoc code signature and SHA-256 integrity, not Apple notarization or publisher identity.

Sources: [Apple: Full Disk Access](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox), [Apple: Security-scoped bookmarks](https://developer.apple.com/documentation/professional-video-applications/enabling-security-scoped-bookmark-and-url-access), [Apple: Code signing requirements](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements).
