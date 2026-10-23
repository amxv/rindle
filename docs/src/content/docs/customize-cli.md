---
title: Architecture and development
description: How Rindle's Rust history engine and browser extension work.
order: 2
category: Development
summary: Native Messaging, Safari history injection, durable queues, and safeguards.
---

## Components

| Component | Job |
| --- | --- |
| `src/macos/safari.rs` | Read iCloud-origin visits, validate Safari history schema, and back up before inserting |
| `src/macos/sync.rs` | Stage URL transfers and trigger a fresh, generation-checked hidden Apple.com heartbeat when needed |
| `src/macos/setup.rs` | Install embedded extension files, browser-native hosts, and launchd schedule |
| `src/state.rs` | Deduplication ledger, browser-specific queues, acknowledged imports |
| `src/native.rs` | One-shot Native Messaging protocol with a fixed extension ID |
| `src/urls.rs` | Canonical URLs, filtering of local addresses and credential-bearing links |
| `extension/` | JavaScript MV3 service worker, browser history API, and connection popup |

The LaunchAgent works without Dia running. It stores incoming iCloud URLs in SQLite; the extension consumes these URLs on startup and every five minutes while the browser is open. It calls `chrome.history.getVisits()` before `chrome.history.addUrl()` so a crash or retry does not duplicate a visit.

The reverse direction also sends history records, not page loads. When necessary, Rindle launches Safari hidden and opens a fresh `https://www.apple.com/?rindle-sync-heartbeat=<unique nonce>` URL to trigger the normal iCloud history upload. If Safari records that heartbeat before advancing to the generation of imported visits, it sends one more fresh Apple.com heartbeat **after** the generation catches up. It waits through the approximate 90-second idle-sync interval. None of the pages found in Chromium history are opened.

## Privacy and resilience

- Safari `origin=1` distinguishes remote iCloud visits, not a specific iPhone.
- Rindle preserves meaningful URL paths and queries but discards fragments, private hosts, and recognized credentials in query parameters.
- Native Messaging is explicitly allowlisted to Rindle's stable extension ID; extension does not inject content scripts or call third-party APIs.
- A durable ledger prevents bouncing URLs between browsers even if iCloud reflects an entry.
- Safari's private history schema is undocumented. Rindle validates known columns, makes SQLite-consistent backups, and fails closed when macOS changes the schema.
- A `last_synced_generation` checkpoint is a **local progress marker**, not an acknowledgment from iPhone Safari. Rindle must not claim successful device delivery from it. macOS Safari has logged failed Cloud History saves while the marker advanced. Apple provides a user-facing supported history import workflow at **Safari → File → Import Browsing Data from File or Folder**, but it has no supported headless scripting API.
- Only up to three backups are retained, stored locally. Uninstall never deletes actual browser history.

## Tests

```sh
just check
just docs-check
dist plan
```

Rust tests use isolated SQLite databases, Native Messaging framing tests, and CLI process tests. Real-browser testing is still required for compatibility and final iPhone autocomplete behavior.
