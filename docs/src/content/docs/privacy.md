---
title: Privacy, simply explained
description: What Rindle can access, what it never collects, and why it needs a folder permission.
order: 4
category: Everyday use
summary: Your history stays on your devices, with no Rindle account or cloud service.
---

Your browsing history is personal. Rindle is designed to connect the browsers you use **without making your history someone else's data**.

## No extra cloud. No account.

Rindle runs locally on your Mac. It works with **Safari's existing iCloud history sync** to reach your iPhone. There isn't a Rindle-hosted service, account, or analytics database collecting the websites you visit.

## Only the history it needs

The browser extension uses Chromium's History API to exchange visited website addresses. The Mac helper works with Safari history entries, not your passwords, cookies, or webpage content. It doesn't copy open tabs or read what you type into a page.

Rindle ignores internal/private network addresses and recognizes some credential-bearing URLs so they aren't passed between browsers. No automatic URL filter is perfect, so treat your browser history as sensitive data.

## You choose the folder

On first installation, macOS asks you to choose **`~/Library/Safari`** in a folder picker. That creates a saved, folder-specific permission for Rindle's sandboxed Safari helper.

**You don't have to grant Full Disk Access.** It's a much broader permission, and the normal Rindle installation doesn't use it. You can stop the background job with `~/.local/bin/rindle setup --browser dia --no-schedule`.

## What about Gatekeeper?

Rindle's macOS helper is ad-hoc signed and distributed in a checksummed archive that the command-line installer downloads and extracts. This installation method normally doesn't attach browser-download quarantine metadata. Rindle never turns off Gatekeeper or removes existing quarantine tags.

An app downloaded separately through Safari or Finder may be subject to Gatekeeper prompts, and ad-hoc signing is **not** verified publisher identity. You can inspect [all the source code](https://github.com/amxv/rindle) before installing.

## What stays on your Mac?

Rindle keeps a small local record of links already synchronized and a queue of links waiting for a browser to open. It also makes local Safari history backups before writing new entries. Uninstalling Rindle removes its integrations and background schedule, but does not erase history already synced into your browsers.

For the exact permissions, data flow, and storage details, see [Security and architecture](/docs/customize-cli).
