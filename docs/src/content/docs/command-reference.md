---
title: CLI reference
description: Every Rindle command and option, in one place.
order: 10
category: Reference
summary: Setup flags, status, manual syncing, uninstall, and software upgrades.
---

The standard installer places the command at `~/.local/bin/rindle`. If that directory is on your PATH, you can use `rindle` directly.

## Setup

| Command | Purpose |
| --- | --- |
| `rindle setup --browser dia` | Connect Dia and enable the three-hour background job |
| `rindle setup --browser chrome` | Connect Google Chrome |
| `rindle setup --browser edge` | Connect Microsoft Edge |
| `rindle setup --all` | Connect all three supported browsers |
| `rindle setup --browser dia --no-schedule` | Register the browser without background history syncing |
| `rindle setup --scoped-helper /absolute/path/to/RindleScopedProbe` | Use a particular authorized sandboxed Safari helper |
| `rindle setup --allow-full-disk-access` | Legacy direct database access; not recommended |

The included Safari helper is the default. Running `setup` again repairs Native Messaging host registration without deleting history.

## Sync and status

| Command | Purpose |
| --- | --- |
| `rindle status` | Show Safari access, connected browsers, and queue progress |
| `rindle sync --dry-run` | Preview potential work without changing history or opening Safari |
| `rindle sync` | Sync history between Mac Safari and connected browsers |
| `rindle sync --limit 3` | Limit a manual sync to at most three URLs per direction |
| `rindle uninstall` | Remove background sync and Native Messaging registrations; keep browsing history |

The manual sync limit is **1–1,000** URLs per direction. The unattended job is deliberately capped at **100 URLs per direction every three hours**.

## Software updates

| Command | Purpose |
| --- | --- |
| `rindle upgrade --check` | Show whether a new stable release exists |
| `rindle upgrade` | Install the latest release |
| `rindle auto-update on` | Opt into daily automatic CLI update checks |
| `rindle auto-update status` | Show your saved update preference |
| `rindle auto-update off` | Disable automatic CLI updates |

Automatic software updates are **off until you enable them**. They are separate from the three-hour history sync schedule.

## Browser extension

The extension is installed at `~/Library/Application Support/Rindle/extension` and includes a popup with **Sync now**. It is bundled with the Rust executable—there's no separate Node runtime needed after installation.

## More information

For issues with permissions or connection status, see [Troubleshooting](/docs/troubleshooting). For the technical protocols, see [Security and architecture](/docs/customize-cli).
