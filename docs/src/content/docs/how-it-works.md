---
title: How Rindle works
description: A friendly explanation of how your browsing history travels between devices.
order: 2
category: Start here
summary: See how Safari, iCloud, your Mac, and Dia fit together—without the technical jargon.
---

Think of Rindle as a small bridge between two browsers you already use.

You might discover an interesting article on your iPhone during lunch, then want to find it again from Dia on your Mac. Normally, those two address bars don't share a memory. Rindle makes that handoff possible.

## From your iPhone to your Mac

1. You visit a website in **Safari on your iPhone**.
2. **iCloud Safari** carries that visit to Safari history on your Mac.
3. **Rindle** notices it and passes the address to the **Dia extension**.
4. The next time you type part of that address in Dia, it can appear in the usual history suggestions.

You don't need to reopen the webpage or leave Safari running on your Mac all day.

## From your Mac to your iPhone

1. You visit a website in **Dia on your Mac**.
2. The extension shares its history entry with Rindle on your Mac.
3. Rindle adds the missing address to Mac Safari's history.
4. Safari's existing iCloud history sync carries it to your iPhone.

Rindle may launch Safari in the background to encourage iCloud syncing. It hides Safari again immediately after opening its harmless, uniquely tagged Apple.com heartbeat, then leaves the app hidden while iCloud sync runs. It never opens sites from your history.

## What moves—and what doesn't

**History URLs move.** Open tabs, cookies, passwords, login sessions, page contents, and bookmarks do not. Visiting a link in one browser does not automatically open a window in the other.

Rindle also filters out local/private addresses and URLs that look like they contain credentials. It's built to be a quiet aid to everyday browsing, not a browser backup or surveillance service.

## It isn't always instant

The Mac background job runs every three hours, the browser extension checks periodically, and iCloud has its own timing. A new visit can arrive sooner, but it can also take a while. The initial history import is gradual so Rindle doesn't flood Safari with thousands of entries at once.

If a link is missing, start with [Troubleshooting](/docs/troubleshooting). For the database-level explanation, see [Architecture](/docs/customize-cli).
