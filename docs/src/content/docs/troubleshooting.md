---
title: Troubleshooting
description: Simple ways to fix missing history suggestions or a disconnected browser.
order: 5
category: Everyday use
summary: Get Safari, iCloud, and Dia talking again with a few focused checks.
---

Most Rindle hiccups come from one of three places: iCloud hasn't synced yet, the Dia extension isn't connected, or macOS hasn't granted access to Safari history.

## A website isn't showing up

**First, give it a little time.** Rindle runs its background sync every three hours and iCloud sync has its own timing. Initial history import can take several runs.

Then check:

1. **On both devices**, make sure Safari is enabled in iCloud settings.
2. **On your Mac**, open the Rindle extension popup in Dia and click **Sync now**.
3. Run `~/.local/bin/rindle status` and check that Safari is accessible and Dia is connected.
4. If you're testing with a website that redirects, search for the **final address**. For example, a site might add `/en-US/` after you open it.

## Dia says “native messaging host not found”

Rindle's helper registration may need refreshing. Run:

```sh
~/.local/bin/rindle setup --browser dia
```

Then reload the Rindle extension in `dia://extensions` and click **Sync now**. Existing history isn't deleted.

## Safari history isn't accessible

Rindle uses a sandboxed helper and a one-time macOS folder selection. It should **not** ask for Full Disk Access. If the folder permission was revoked or lost during an update, run the installer again and select **`~/Library/Safari`** when the folder picker appears.

If you still see an error, use `~/.local/bin/rindle status` to check which Safari backend is active. It should say **sandboxed, folder-scoped helper**.

## The extension is installed but not connecting

Make sure the extension is enabled in Dia. The extension files live at:

```text
~/Library/Application Support/Rindle/extension
```

Try **Sync now**, then restart Dia. Rindle keeps its local queue until the browser is ready to receive it.

## Rindle isn't doing anything in the background

Run:

```sh
~/.local/bin/rindle status
```

Look for **Background sync: configured** and a successful or running background job. If it isn't configured, run `~/.local/bin/rindle setup --browser dia`.

## Still stuck?

Open a [GitHub issue](https://github.com/amxv/rindle/issues) with your macOS version, browser version, Rindle version, and the error text from `rindle status`. **Don't attach your Safari history database, full browsing history, cookies, tokens, or private URLs.**
