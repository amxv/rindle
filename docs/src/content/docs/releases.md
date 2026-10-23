---
title: Downloads and updates
description: macOS releases, installing Rindle, and keeping it up to date.
order: 12
category: Reference
summary: Release downloads, the one-command installer, and safe software updates.
---

Rindle is available for **macOS 14 or later**, on Apple Silicon and Intel Macs.

## Install

```sh
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
```

The installer selects the correct architecture, checks the archive's SHA-256 checksum, verifies the sandboxed Safari helper's ad-hoc code signature, and installs everything into user-writable directories. It then asks you to select `~/Library/Safari` and registers the browser integration and background schedule.

It does **not** ask for Full Disk Access, disable Gatekeeper, or strip quarantine attributes. If you download the `.app` manually in a web browser, macOS may treat it differently from a command-line installation.

You can also [browse the release assets on GitHub](https://github.com/amxv/rindle/releases/latest).

## What's in a release?

Each GitHub Release contains two checksummed `.tar.gz` archives—one for Apple Silicon and one for Intel. Each archive contains the native Rust CLI and a matching sandboxed Swift Safari helper. A copy of the installer script is included.

There is no installer that requires administrator privileges, a separate JavaScript runtime, or an Apple Developer Program membership.

## Upgrade

```sh
~/.local/bin/rindle upgrade --check
~/.local/bin/rindle upgrade
```

Rindle checks the newest stable GitHub Release. The update installs the CLI and helper together, then checks whether the Safari folder permission is still valid. If the helper needs you to select the folder again, the interactive installer will ask.

You can choose automatic daily CLI updates:

```sh
~/.local/bin/rindle auto-update on
```

To turn them off:

```sh
~/.local/bin/rindle auto-update off
```

Software updates are distinct from the three-hour background history sync. Updates are off by default.

## Source and release process

Rindle's [source code](https://github.com/amxv/rindle) includes the packaging scripts, SHA-256 checks, and macOS CI workflow. The project publishes separate archives for Apple Silicon and Intel with the same versioned source. See the [changelog](/docs/changelog) for what's new.
