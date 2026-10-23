---
title: Privacy and permissions
description: Why Rindle needs Safari data access and how we are testing a narrower alternative.
order: 5
category: Reference
summary: Understand macOS Full Disk Access, Safari history protection, and sandboxed folder permissions.
---

Rindle reads history URLs locally. It does not upload browsing history to a Rindle server, does not read webpage contents, and never reopens the pages you previously visited to manufacture history entries. Dia/Chrome/Edge communicate with a one-shot local Native Messaging helper and Safari history is synced through the Apple iCloud account that is already configured on your devices.

## Why Full Disk Access is requested today

Apple protects `~/Library/Safari/History.db` using macOS privacy controls (TCC). A standalone background LaunchAgent cannot normally open this database. The current Rust CLI reads and updates it directly using SQLite after creating a local backup, which is why unattended syncing currently requires **Full Disk Access** granted to the installed `rindle` executable.

Full Disk Access is **not** a permission to just Safari history: it can allow an app to access other private data including other apps' databases, mail, messages, and files. It does not grant microphone or camera access. Because the current Rindle executable is unsandboxed, granting FDA gives it broader filesystem authority than the actions it normally performs. We do not consider this an ideal long-term onboarding flow.

macOS requires users to make this choice in **System Settings → Privacy & Security → Full Disk Access**. If you would rather not grant it, keep the automated job disabled using:

```sh
rindle setup --browser dia --no-schedule
```

You can revoke the grant in System Settings whenever you want. The CLI will report when its background job is blocked by missing permission. Rindle never bypasses TCC or grants itself privacy permissions.

## A narrowly scoped alternative under investigation

Apple supports opening a system folder picker (`NSOpenPanel`) in a sandboxed app and preserving user-granted access with a security-scoped bookmark. We have a **separate, experimental**, read-only proof-of-concept in [`experiments/scoped-safari`](https://github.com/amxv/rindle/tree/main/experiments/scoped-safari). The goal is a user selecting only `~/Library/Safari` once, with a signed helper that can access only that folder on later scheduled runs.

**What we have verified (October 24, 2026, macOS 26.3.1):** The user selected `~/Library/Safari` through a sandboxed folder picker, then the same signed helper successfully restored its bookmark in later processes. Independent, sandboxed LaunchAgents without any Full Disk Access grant read the actual Safari `History.db`, opened it with SQLite's read/write flag, and exercised a real `BEGIN IMMEDIATE`/insert/`ROLLBACK` transaction. The synthetic test record was absent afterward. A Rust Rindle LaunchAgent that also had no FDA successfully delegated its read-only sync preview to this sandboxed helper.

**What remains:** The sandboxed helper's real-history insertion and hidden Safari → iCloud delivery have not yet been tested together. That needs a safe interval when Safari is closed. The current proof-of-concept is available as an *opt-in* backend for developer testing through `RINDLE_SCOPED_SAFARI_HELPER`; it has not replaced the user-installed scheduler, which still uses the original FDA-enabled backend. Public distribution additionally requires packaging the helper with a stable Developer ID signature and notarization. We will not ask users to remove their existing FDA permission until the complete scoped path succeeds.

## Code signing and updates

macOS tracks privacy authorizations using a program's code signing identity. Ad-hoc signatures used by local builds do not reliably identify new binary versions as the same trusted application. Before a broad public release, Rindle should use a stable Developer ID signature and Apple's notarization, regardless of which permission approach succeeds.

Sources: [Apple: Full Disk Access](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox), [Apple: Security-scoped bookmarks](https://developer.apple.com/documentation/professional-video-applications/enabling-security-scoped-bookmark-and-url-access), [Apple: Code signing requirements](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements).
