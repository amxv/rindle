---
title: Get started
description: Set up Safari and Dia history sync on your Mac in a few minutes.
order: 1
category: Start here
summary: Install Rindle, give it access to your Safari history, and connect Dia.
---

Rindle keeps the sites you visit in **Safari on your iPhone** available in **Dia on your Mac**—and vice versa. You can keep using both browsers exactly as you do today.

You only need to set it up once.

## Before you begin

You'll need a Mac running **macOS 14 or later** (Apple Silicon or Intel), an iPhone, and [Dia](https://www.diabrowser.com/) installed on your Mac.

Make sure **Safari is enabled in iCloud** on *both* devices. On your iPhone, open Settings → your Apple Account → iCloud and check Safari. Do the same in System Settings on your Mac. The exact menu wording can vary by macOS and iOS version.

## 1. Install Rindle

Open Terminal on your Mac and paste:

```sh
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
```

The installer downloads a checksummed macOS release, installs a small command-line app and a Safari helper, and configures background sync. No Rust or build tools are required.

Prefer to inspect the script first? It's [open source on GitHub](https://github.com/amxv/rindle/blob/main/docs/public/install.sh).

## 2. Choose your Safari history folder

During installation, macOS will show a normal folder picker. Select:

```text
~/Library/Safari
```

You can press **Command + Shift + G** in the picker and paste the path. This grants the small, sandboxed helper access to *that folder only*.

**Do not enable Full Disk Access.** Rindle doesn't need it for normal installation. Learn more in [Privacy and permissions](/docs/privacy).

## 3. Add the extension to Dia

Rindle installs the extension files, but Dia needs you to load them once:

1. Open **`dia://extensions`** in Dia.
2. Turn on **Developer mode**.
3. Click **Load unpacked**.
4. Select **`~/Library/Application Support/Rindle/extension`**. You can paste that path in the file picker with Command + Shift + G.
5. Open the Rindle extension popup and choose **Sync now**.

That's the entire browser setup. You won't have to repeat it on every launch.

## 4. Check that everything is connected

Back in Terminal, run:

```sh
~/.local/bin/rindle status
```

Look for **“dia: connected”**, an accessible Safari history, and the **sandboxed, folder-scoped helper**. If you've added `~/.local/bin` to your shell's PATH, you can simply run `rindle status`.

For a simple test, visit a unique webpage in Safari on your iPhone, then search for it in Dia's address bar once iCloud and Rindle have had time to catch up. You can also visit a page in Dia and check its address in iPhone Safari later.

Rindle transfers history entries—not open tabs. A first sync can take longer as older history is brought across in small batches.

## You're all set

Keep browsing. Your Mac's background sync runs **every three hours**, and the Dia extension checks for new history on startup and periodically while Dia is open. Apple's iCloud sync may introduce additional delay, so changes aren't always instant.

Next, read [Living with Rindle](/docs/everyday-use) for what to expect, or [Troubleshooting](/docs/troubleshooting) if a suggestion doesn't appear.
