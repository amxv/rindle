# Rindle

**Your browsing history, wherever you are.**

Safari on your iPhone. Dia on your Mac. Rindle makes the websites you've visited available in both address bars—without copying tabs, reopening pages, or changing how you browse.

**[Website](https://rindle.ashray.xyz)** · **[Get started](https://rindle.ashray.xyz/docs/quickstart)** · **[How it works](https://rindle.ashray.xyz/docs/how-it-works)** · **[Releases](https://github.com/amxv/rindle/releases)**

## Install

Requires macOS 14 or later (Apple Silicon or Intel).

```sh
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
```

The installer will ask you to select **`~/Library/Safari`**. Then open `dia://extensions`, enable Developer mode, and **Load unpacked** from:

```text
~/Library/Application Support/Rindle/extension
```

That's it. Rindle keeps history moving between Safari and Dia in the background.

## Why Rindle?

- **One history across devices.** Find the links you visited in either browser.
- **Private by design.** Local syncing through your Mac and existing iCloud account. No Rindle cloud or account.
- **Quietly automatic.** A lightweight macOS background job and browser extension; no need to keep both browsers open.
- **Only the access it needs.** A sandboxed Safari helper uses a folder-specific permission—**no Full Disk Access** for normal installation.

Rindle also supports Chrome and Edge on macOS. It transfers history URLs, not cookies, passwords, page contents, or open tabs. iCloud and background sync can introduce delays; changes aren't always immediate.

For usage, troubleshooting, privacy details, and the CLI reference, see the **[documentation](https://rindle.ashray.xyz/docs)**.

## Open source

Written in Rust and Swift, with a small Chromium extension. Distributed as checksummed macOS releases containing an ad-hoc-signed sandboxed helper. Rindle doesn't disable Gatekeeper or alter quarantine settings. See [the license](LICENSE) (Apache-2.0) and [contribution guide](CONTRIBUTORS.md).
