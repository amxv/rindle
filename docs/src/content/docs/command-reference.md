---
title: Command reference
description: All user and developer commands for Rindle.
order: 4
category: Reference
summary: Setup, history sync, diagnostics, and CLI updates.
---

## Browser and history sync

| Command | Effect |
| --- | --- |
| `rindle setup --browser dia` | Configure Dia and enable background sync every three hours |
| `rindle setup --browser chrome` | Also register Google Chrome |
| `rindle setup --browser edge` | Also register Microsoft Edge |
| `rindle setup --all` | Register all three browsers |
| `rindle setup --browser dia --no-schedule` | Register without recurring background runs |
| `rindle sync --dry-run` | Report potential work without changing history |
| `rindle sync` | Stage iCloud URLs, write new Safari records, trigger hidden upload |
| `rindle status` | Show registrations, last extension contact, and queue counts |
| `rindle uninstall` | Remove helpers and schedule; retain history and deduplication data |

Browser extension updates are embedded in the Rust binary and extracted by setup or the background sync. No Node runtime is needed after installation.

## Upgrades

```sh
rindle upgrade --check
rindle upgrade
rindle auto-update on
rindle auto-update status
rindle auto-update off
```

Automatic **CLI upgrades** are separate from browser-history sync. They remain off until explicitly enabled and require a published GitHub Release.

## Development

```sh
just check-fast
just check
just extension-test
just build
just docs-check
just release-plan
```

Do not create a release tag until Dia's Native Messaging host and Safari iCloud autocomplete have passed end-to-end testing.
