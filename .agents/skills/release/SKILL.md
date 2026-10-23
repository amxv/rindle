---
name: release
description: Prepare, test and publish a macOS-only Rindle release without Apple Developer ID.
allowed-tools: Bash, Read, Write, Edit
---

# Release Rindle

**Canonical source of truth:** `Cargo.toml` for version, `.github/workflows/release.yml` for publishing, `scripts/package-release.sh` for archives, and `docs/public/install.sh` for installation. This is a hand-authored release workflow, **not cargo-dist**.

Do not introduce a paid Apple Developer ID, notarization, a Gatekeeper override, a quarantine-stripping step, a manual Finder app-install step, or Full Disk Access for standard setup. These are not required for the tested Rindle use case.

The release pipeline uses `codesign --force --sign -` to ad-hoc sign the sandboxed `.app` and includes it alongside the Rust CLI in a checksummed per-architecture GitHub Release tarball. The shell installer uses HTTPS `curl` and `tar` in a user-writable directory. This path does not normally assign a browser-download quarantine attribute. Users downloading the `.app` directly in Safari/Finder may still encounter Gatekeeper. The ad-hoc signature is an integrity check, not publisher authentication.

## Preflight

1. Inspect `git status`, `git log`, tags, and existing GitHub releases. Work from clean, up-to-date `main`.
2. Choose the next SemVer release in `Cargo.toml`, update `Cargo.lock` with Cargo if necessary, and update `docs/src/content/docs/changelog.md` with real user-visible changes.
3. Run `just check`, `just docs-check`, `just release-plan`, `actionlint .github/workflows/release.yml`, and `shellcheck -S warning docs/public/install.sh scripts/package-release.sh scripts/release-preflight.sh`. Also run `git diff --check`.
4. No signing secrets, Apple Developer account or notarization credentials are needed. Require both Apple Silicon and Intel artifacts in the release workflow, with `.tar.gz.sha256` sidecars and the `rindle-installer.sh` release asset.

## Local packaging and installer acceptance

```sh
just release-build-local
shasum -a 256 -c target/distrib/rindle-aarch64-apple-darwin.tar.gz.sha256
shasum -a 256 -c target/distrib/rindle-x86_64-apple-darwin.tar.gz.sha256
```

Inspect each archive. Every architecture must contain exactly a native executable named `rindle`, a sandboxed `ScopedSafariHelper.app` with ad-hoc code signature and security-scoped-bookmark entitlements, `README.md`, and `LICENSE`. Validate both expected Mach-O architectures, the code signature using `codesign --verify --strict`, and the matching application version. No user history files, persistent bookmarks, keys, or runtime state may be packaged.

Run the website installer in an **isolated** test directory with `RINDLE_LOCAL_ARCHIVE`, `RINDLE_INSTALL_DIR`, `RINDLE_APP_DIR`, and `RINDLE_INSTALL_SKIP_SETUP=1`. Verify the checksum, CLI executable, `.app`, version, and absence of modifications to real user files/LaunchAgents. Verify the installer fails closed on corrupted archives or invalid signatures. Never remove `com.apple.quarantine` or disable Gatekeeper.

## GitHub CI packaging smoke and publication

The workflow `release.yml` is triggered both by manual `workflow_dispatch` and version tags. Before the first release, use a **manual workflow_dispatch** to test both `macos-15` (arm64) and `macos-15-intel` jobs without publishing. Require both builds and installer smoke tests to pass. Review downloaded test artifacts if needed.

When ready, commit and push main, then use:

```sh
just release-tag VERSION
```

The tag helper runs `scripts/release-preflight.sh VERSION`, validates clean synchronized main and package version, and pushes `vVERSION`. No duplicate tags. Watch the new Release workflow through completion, then inspect `gh release view vVERSION --json url,assets,tagName`.

The GitHub Release must contain `rindle-aarch64-apple-darwin.tar.gz`, `rindle-x86_64-apple-darwin.tar.gz`, each SHA-256 sidecar, and `rindle-installer.sh`. Verify the website `/install.sh` deploys and resolves latest GitHub Release assets.

## Permissions, upgrade, and post-release

Default install is **not** Full Disk Access. A one-time macOS system folder picker authorizes exactly `~/Library/Safari`, using a sandboxed security-scoped bookmark. On updates, the installer verifies saved access and prompts again if an ad-hoc signature change requires it. Noninteractive automatic upgrades must fail safely and preserve the old binary/helper when reauthorization is needed.

Keep `rindle upgrade` and optional daily `rindle auto-update` working, including `CLI_UPGRADE_TARGET` and `CLI_VERSION` upgrade paths. The normal history sync LaunchAgent remains an independent, three-hour schedule bounded at 100 URLs per direction per run. Preserve the user's existing ledger and browser extension identity.

Report version, tag, source commit, release URL, two archive/checksum assets, installer smoke test, CI result, source/permission caveats, and any unresolved blockers. Never claim a release succeeded before its workflow and assets are verified.
