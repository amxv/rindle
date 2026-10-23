---
title: Living with Rindle
description: What to expect after setup, and how to make the most of Rindle.
order: 3
category: Everyday use
summary: Use Safari and Dia normally while your browsing history catches up in the background.
---

The nicest thing about Rindle is that you don't have to think about it much.

## Keep your usual routine

Use Safari on your iPhone. Use Dia on your Mac. Search the address bar in either browser for websites you found in the other. Rindle makes those earlier visits discoverable without juggling links, tabs, or another app.

There's no separate Rindle inbox or dashboard to manage.

## Let it catch up

Rindle checks Safari history on your Mac every **three hours**. Dia checks its incoming queue at startup and periodically when open. iCloud syncing isn't instantaneous, so a visit may not appear right away.

If you've just installed Rindle, expect the first history backfill to take longer: entries are transferred in small batches. More recent visits get priority.

## Want to check progress?

In Terminal, run:

```sh
~/.local/bin/rindle status
```

You'll see whether Safari is accessible, whether Dia is connected, and how many history entries are queued. You can also open the Rindle extension popup in Dia and click **Sync now** to make the extension check its queue.

To see what Rindle would transfer without making changes:

```sh
~/.local/bin/rindle sync --dry-run
```

Or trigger a small manual sync on your Mac:

```sh
~/.local/bin/rindle sync --limit 3
```

The manual sync may open Safari discreetly to refresh iCloud history. It never opens websites from your browsing history.

## Keep it updated

Check for a new Rindle version with:

```sh
~/.local/bin/rindle upgrade --check
```

CLI auto-updates are **off by default**. If you'd like a daily automatic check, see [Command reference](/docs/command-reference). Daily software updates and the three-hour history sync are separate features.

## Need a hand?

Visit [Troubleshooting](/docs/troubleshooting) for common connection and history issues, or [Other browsers](/docs/other-browsers) if you'd like to use Chrome or Edge too.
