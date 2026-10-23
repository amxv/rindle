---
title: Releases and installers
description: Native macOS distribution using ad-hoc signed apps and GitHub Release tarballs.
order: 3
category: Distribution
summary: Mac installers, checksums, built-in upgrades, and release validation.
---

Rindle supports **macOS only**. GitHub Actions builds architecture-specific tarballs for Apple Silicon (`aarch64-apple-darwin`) and Intel (`x86_64-apple-darwin`) Macs. **Each archive contains the Rust CLI and an ad-hoc-signed sandboxed Safari `.app`.** A SHA-256 sidecar accompanies every archive. The unpacked Chromium extension is embedded in the Rust executable.

## Install after the first release

```sh
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
```

The website installer uses HTTPS `curl`, verifies the archive's SHA-256 checksum, extracts it with `tar`, checks the ad-hoc code signature for integrity, asks users to select only `~/Library/Safari` once, and registers the three-hour background sync automatically. It does not request Full Disk Access or disable Gatekeeper. Command-line extraction does not normally attach browser-download quarantine metadata; manually downloaded app bundles may be subject to Gatekeeper.

**Before the first release:** Build and smoke-test both architectures through the manual GitHub Actions release workflow, then tag the tested commit. No Apple Developer account is needed.

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
just release-plan
just release-build-local
```

The hand-authored [release workflow](https://github.com/amxv/rindle/blob/main/.github/workflows/release.yml) builds and smoke-tests the checksummed CLI+app archives on macOS 15 arm64 and macOS 15 Intel runners. `workflow_dispatch` tests packaging without publishing. A version tag publishes both archives and a copy of the website installer.

There are **no Apple Developer ID, certificate-export, or notarization secrets**. Gatekeeper itself is never disabled; users who choose to download the `.app` through a browser rather than the verified installer may encounter Gatekeeper restrictions.

To test both target archives locally without publishing:

```sh
just release-build-local
(cd target/distrib && shasum -a 256 -c rindle-aarch64-apple-darwin.tar.gz.sha256)
(cd target/distrib && shasum -a 256 -c rindle-x86_64-apple-darwin.tar.gz.sha256)
```

Release archives are ad-hoc signed by design. GitHub Release SHA-256 checksums provide integrity checks; this should not be confused with Apple's notarized publisher verification.
