---
title: Changelog
description: Rindle development and releases.
order: 99
category: Distribution
summary: Unreleased two-way Safari / Chromium history sync.
---

## Unreleased

- Verified both directions of Safari ↔ Dia history synchronization with an iPhone, including Chromium URL redirects and an automatic background run.
- Replaced the default Full Disk Access workflow with a sandboxed, folder-scoped Safari helper and a one-time macOS folder picker. The direct FDA backend is now explicit opt-in only.
- Added release-gating Developer ID signing, Apple notarization and a universal macOS helper archive that the website installer verifies before setup.
- Added local-first Safari iCloud ↔ Dia history synchronization, with optional Chrome and Edge support.
- Embedded a Manifest V3 extension, a one-shot Rust Native Messaging helper, and a durable SQLite deduplication ledger.
- Added a user-owned three-hour macOS LaunchAgent, hidden Safari iCloud heartbeat, URL privacy filtering, schema checks, and consistent backups.
- Retained direct GitHub Release installation, `rindle upgrade`, and separately opt-in automatic CLI updates. Distribution targets Intel and Apple Silicon Macs.
- No tagged release yet. The native Dia extension handshake and real-device autocomplete still need acceptance testing.
