---
title: Security and architecture
description: The local data flow, Safari helper, privacy boundaries, and failure safeguards.
order: 11
category: Reference
summary: A closer look at Native Messaging, sandboxed Safari access, SQLite backups, and iCloud refresh.
---

This page is for developers and curious readers. You don't need it to install or use Rindle. Start with [How Rindle works](/docs/how-it-works) for the simpler explanation.

## Components

| Component | Responsibility |
| --- | --- |
| `extension/` | Manifest V3 extension; reads and writes Chromium history through `chrome.history` |
| `src/native.rs` | One-shot browser Native Messaging bridge with an allowlisted extension ID |
| `src/state.rs` | Local SQLite ledger, browser-specific queues, and acknowledgements |
| `src/macos/backend.rs` | Selects the sandboxed Safari helper, with an explicit legacy direct adapter |
| `apps/scoped-safari/` | Small Swift App Sandbox helper with a security-scoped bookmark to only `~/Library/Safari` |
| `src/macos/sync.rs` | Bounded transfers, hidden Safari heartbeat, iCloud refresh, and locking |
| `src/urls.rs` | URL normalization and filtering |
| `src/macos/setup.rs` | Browser host registration and user LaunchAgent configuration |

Rindle stores its local ledger at `~/Library/Application Support/Rindle/state.sqlite`. It holds deduplication records and pending URLs. There is no remote Rindle history endpoint.

## Safari's folder permission

macOS protects Safari's `History.db`. Instead of asking the unsandboxed CLI for Full Disk Access, the installer opens a system folder picker in the Swift helper. The user selects exactly `~/Library/Safari`.

The helper stores an **app-scoped security-scoped bookmark** and restores it whenever the background job needs to read or write Safari history. The helper's entitlements permit only user-selected files and saved bookmarks; it has no network, camera, microphone, Accessibility, or Apple Events entitlement.

The Rust CLI launches the helper as a short-lived child with JSON over standard input and output. Supported actions include `inspect`, `checkpoint`, `visit_generation`, and `insert`. Request sizes and batches are bounded; callers cannot choose an arbitrary SQLite path.

## Chromium side

The extension exports visited URLs through `chrome.history.search()` and imports missing entries through `chrome.history.addUrl()`. Before importing, it uses `chrome.history.getVisits()` to avoid duplicating existing visits. It persists its scan cursor across browser restarts.

Dia uses a Chrome-compatible Native Messaging host registration. Rindle also supports dedicated Chrome and Edge host directories. The host verifies the browser identity and extension allowlist before accepting a transfer.

A per-browser queue and acknowledgement protocol means Dia can be closed while the Mac background job runs. Imports resume when Dia opens.

## Safari side and iCloud

New Chromium URLs are added to the local Safari history database **as history records**; Rindle doesn't navigate to them in Safari. The helper checks the expected schema, creates a consistent backup using SQLite, and performs bounded transactional inserts. By default, it retains up to three backups in its private container.

The background job can launch Safari discreetly to encourage iCloud history upload and download. It visits only a uniquely tagged Apple.com URL as a heartbeat. It checks that the heartbeat belongs to the same or newer local history generation than the inserted records, allowing a second harmless heartbeat if the first was too early.

A local `last_synced_generation` checkpoint is **not** proof that the URL reached an iPhone. Apple controls the iCloud delivery timeline. Our acceptance test checked the resulting history suggestion on an actual iPhone, then tested a fresh visit in the opposite direction.

## Privacy and compatibility limits

- `origin=1` identifies an iCloud-sourced visit, not a specific iPhone or individual device.
- Rindle filters internal URLs and recognizable credential-bearing query strings; it does not promise to detect every secret-bearing URL.
- Safari's history schema is undocumented. Rindle validates the known layout before writing and fails closed if the schema changes.
- The Rindle CLI and Safari helper are ad-hoc signed in checksummed macOS release archives. That verifies integrity, not verified publisher identity. Installing the `.app` manually through a browser can trigger Gatekeeper.
- A changed ad-hoc signature can require reselecting `~/Library/Safari`. The installer rechecks the saved bookmark after upgrades.

## Contributing

The repository has Rust, Swift, and extension tests. Run:

```sh
just check
just docs-check
just release-plan
```

Read [Contributing on GitHub](https://github.com/amxv/rindle/blob/main/CONTRIBUTORS.md) for the test and release workflow. Never use your actual Safari or Dia history database as a test fixture.
