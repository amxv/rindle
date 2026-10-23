---
title: Releases and installers
description: Native macOS distribution through GitHub Releases and cargo-dist.
order: 3
category: Distribution
summary: Mac installers, checksums, built-in upgrades, and release validation.
---

Rindle supports **macOS only**. `cargo-dist` packages Apple Silicon (`aarch64-apple-darwin`) and Intel (`x86_64-apple-darwin`) archives, checksums and a shell installer. The unpacked Chromium extension is embedded in the Rust executable. A required parallel artifact job builds a **universal sandboxed Safari helper** and its own checksum and performs Developer ID signing and notarization before the GitHub Release is created.

## Install after the first release

```sh
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
```

The website installer checks the downloaded archive's SHA-256 checksum, validates the Developer ID signature and Gatekeeper assessment, extracts the helper, asks users to select only `~/Library/Safari` once, and registers the three-hour background sync automatically. It does not request Full Disk Access.

**The first release is not yet published** because Developer ID and notarization secrets must be configured in the repository. For now, use the [source quickstart](/docs/quickstart).

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

Before the first release, configure the GitHub Actions secrets `RINDLE_APPLE_DEVELOPER_ID_P12_B64`, `RINDLE_APPLE_DEVELOPER_ID_P12_PASSWORD`, `RINDLE_APPLE_NOTARY_KEY_P8_B64`, `RINDLE_APPLE_NOTARY_KEY_ID`, and `RINDLE_APPLE_NOTARY_ISSUER_ID` from an authorized Apple Developer account. The custom macOS artifact workflow fails closed if any are absent. It uses Apple's notarization service and includes `rindle-scoped-helper-universal.zip` and its SHA-256 sidecar in the same GitHub Release as the CLI.

To test universal helper packaging locally without attempting to publish an unsigned app:

```sh
./scripts/package-scoped-helper.sh --development
shasum -a 256 -c target/distrib/rindle-scoped-helper-universal.zip.sha256
```

Development mode is never suitable for public distribution. The actual release requires `--release`, a Developer ID identity, and App Store Connect notarization credentials.
