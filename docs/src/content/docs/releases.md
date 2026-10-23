---
title: Releases and installers
description: Native macOS distribution through GitHub Releases and cargo-dist.
order: 3
category: Distribution
summary: Mac installers, checksums, built-in upgrades, and release validation.
---

Rindle currently supports **macOS only**. `cargo-dist` packages both Apple Silicon (`aarch64-apple-darwin`) and Intel (`x86_64-apple-darwin`) archives with checksums and a shell installer. The unpacked extension is embedded in the Rust executable.

## Install after the first release

```sh
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
rindle setup --browser dia
```

**The first release is not yet published.** For now, use the [source quickstart](/docs/quickstart).

## Upgrade

```sh
rindle upgrade --check
rindle upgrade
rindle auto-update on
rindle auto-update off
```

Daily automatic CLI updates are opt-in and independent of history synchronization. The history sync schedule runs every three hours after `rindle setup`, unless `--no-schedule` was supplied.

## Release process

```sh
just check
just docs-check
dist plan
```

The generated workflow `.github/workflows/release.yml` must be regenerated with `dist generate` whenever `dist-workspace.toml` changes. We will not tag or publish the first release until Dia Native Messaging and iCloud autocomplete have passed the real-device acceptance test.
