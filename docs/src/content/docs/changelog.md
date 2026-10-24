---
title: Changelog
description: Rindle development and releases.
order: 99
category: Reference
summary: Changes in the first release and future updates.
---

## 0.1.2 — October 24, 2026

- Immediately re-hide Safari after its harmless Apple.com heartbeat so the window no longer remains visible during the background iCloud sync wait. No additional macOS privacy permissions are required.
- Preserve the working AppleScript heartbeat, iCloud checkpoint waiting, and user-owned Safari window behavior.

## 0.1.1 — October 24, 2026

- Increased the unattended macOS history backfill limit from 100 to 300 URLs per direction every three hours. Manual sync limits remain unchanged.

## 0.1.0 — October 24, 2026

- Verified both directions of Safari ↔ Dia history synchronization with an iPhone, including Chromium URL redirects and an automatic background run.
- Replaced the default Full Disk Access workflow with a sandboxed, folder-scoped Safari helper and a one-time macOS folder picker. The direct FDA backend is now explicit opt-in only.
- Packaged the Rust CLI and ad-hoc-signed, sandboxed Safari app in a checksummed tarball for each macOS architecture; no Developer ID, Full Disk Access, or Gatekeeper override required for standard setup.
- Added local-first Safari iCloud ↔ Dia history synchronization, with optional Chrome and Edge support.
- Embedded a Manifest V3 extension, a one-shot Rust Native Messaging helper, and a durable SQLite deduplication ledger.
- Added a user-owned three-hour macOS LaunchAgent, hidden Safari iCloud heartbeat, URL privacy filtering, schema checks, and consistent backups.
- Retained direct GitHub Release installation, `rindle upgrade`, and separately opt-in automatic CLI updates. Distribution targets Intel and Apple Silicon Macs.
- Validated the native Dia extension handshake and real-device autocomplete on an iPhone.
