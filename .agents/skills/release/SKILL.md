---
name: release
description: Use this skill when the user says to cut, ship, publish, or create a new release for this Rust CLI.
allowed-tools: Bash, Read, Write, Edit
---

# Release this Rust CLI

Use this skill whenever the user asks for a new release.

`Cargo.toml` owns the package version. `docs/src/content/docs/changelog.md` owns curated release notes. `dist` owns release artifacts, GitHub Release creation, attestations, and the **macOS-only direct shell installer**.

**Mandatory Rindle release gate:** The primary CLI depends on a second, sandboxed Swift `.app` that provides scoped access to `~/Library/Safari`. The custom `global-artifacts-jobs = ["./build-scoped-helper"]` workflow must build a universal `rindle-scoped-helper-universal.zip`, sign it with a **Developer ID Application** certificate, notarize it, staple the ticket, verify Gatekeeper acceptance, and produce its SHA-256 sidecar. The `host` job must wait for this step to succeed. Never publish an ad-hoc/local-development-signed app or a CLI-only release.

Do not create a tag before these GitHub Actions secrets exist: `RINDLE_APPLE_DEVELOPER_ID_P12_B64`, `RINDLE_APPLE_DEVELOPER_ID_P12_PASSWORD`, `RINDLE_APPLE_NOTARY_KEY_P8_B64`, `RINDLE_APPLE_NOTARY_KEY_ID`, and `RINDLE_APPLE_NOTARY_ISSUER_ID`. Inspect only whether they exist. Never print or copy their values.

## 1. Inspect release state

Start from the repository root and read:

```text
Cargo.toml
dist-workspace.toml
docs/src/content/docs/changelog.md
```

Inspect Git/release state:

```bash
git status --short --branch
git fetch origin --tags
git tag --sort=-version:refname | head -n 10
gh release list --limit 10
git log --oneline --decorate -n 30
```

Identify the most recent release tag and review the commits/diff since it. Understand any uncommitted changes before editing release metadata.

Normally release from `main` with the branch pushed and synchronized with `origin/main` before the final tag is created.

## 2. Choose the version

If the user supplied an exact version, use it.

Otherwise choose the smallest SemVer bump justified by user-visible changes since the previous release:

- breaking compatibility change: major bump
- new backward-compatible functionality: minor bump
- fixes, performance improvements, or maintenance: patch bump
- before `1.0.0`, use a minor bump for breaking changes and a patch bump for compatible fixes

Do not reuse an existing tag, GitHub Release version, or npm version (when npm is enabled).

Update `package.version` in `Cargo.toml`, then run Cargo once without `--locked` so the root package entry in `Cargo.lock` follows the new version. Confirm the resolved package version:

```bash
cargo metadata --no-deps --format-version 1 \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])'
```

The value must exactly match the version about to be tagged.

## 3. Write the changelog

Update `docs/src/content/docs/changelog.md` manually using the actual changes since the previous release.

The new entry should:

- be first in the changelog
- use the exact version and current release date
- explain user-visible behavior, fixes, performance changes, compatibility changes, and installation changes
- combine related commits into concise release notes
- omit internal refactors and docs-only styling/navigation work unless they materially affect users
- avoid copying raw commit subjects verbatim

For a maintenance-only release with no direct user-visible behavior change, say so plainly rather than inventing features.

## 4. Verify release configuration

Read the pinned `cargo-dist` version from `dist-workspace.toml` and use that exact version.

If `dist-workspace.toml` changed since the previous committed release configuration, regenerate the release workflow before validation:

```bash
dist generate
```

The generated `.github/workflows/release.yml` should match the committed dist configuration.

When npm publishing is configured, confirm the GitHub secret exists without reading its value:

```bash
gh secret list --json name --jq 'any(.[]; .name == "NPM_TOKEN")'
```

A missing required publishing secret is a release blocker; configure it through the user's established secure credential path before tagging.

## 5. Validate

Run from the repository root. The Astro check validates documentation changes;
the deployment platform owns its production build:

```bash
just check
just docs-check
dist plan
git diff --check
```

`dist plan` must list **only** the Intel and Apple Silicon macOS CLI archives, checksums, and `rindle-installer.sh`. The extra helper zip/sidecar come from the separately gated custom global artifacts workflow, not `dist plan` itself. Reject Windows/Linux artifacts, PowerShell installers, and npm. Verify the generated `release.yml` waits for `custom-build-scoped-helper` before the `host` job.

Test the developer-only universal archive locally with `./scripts/package-scoped-helper.sh --development`; it must contain both `arm64` and `x86_64` Mach-O slices and pass `codesign --verify`. This local mode does **not** notarize or qualify an artifact for release. The release workflow must run `--release` with the Developer ID and notary credentials.

If crates.io publishing is enabled in `Cargo.toml` and this release is intended for crates.io too, also run:

```bash
cargo publish --dry-run
```

Treat crates.io publication as a separate publication channel with its own authentication/trusted-publisher state.

## Optional Swift macOS menu bar release

For direct projects containing apps/menubar, dist-workspace.toml must retain
the custom post-announce job pointing to .github/workflows/publish-menubar.yml.
The latter publishes two ad-hoc-signed Swift .app + Rust CLI archives and
matching checksum sidecars after cargo-dist creates the release. The website's
macOS install.sh should download this paired archive, install it with no
quarantine metadata, verify its signature and relaunch the app if previously
running. Check both architecture assets and use dist generate --check; do not
hand-edit the generated main release.yml. Without the flag, none of these app
artifacts/jobs should exist.

## 6. Commit and push release preparation

Review the diff. Release preparation normally includes:

- `Cargo.toml`
- `Cargo.lock`
- `docs/src/content/docs/changelog.md`
- `dist-workspace.toml` and generated release workflow only when distribution configuration intentionally changed

Commit and push those changes to the release branch before tagging.

Confirm the pushed commit is the commit intended for release.

## 7. Tag the Cargo version

Create the tag through the repository helper:

```bash
just release-tag ${VERSION}
```

The helper verifies that `${VERSION}` matches Cargo metadata before pushing `v${VERSION}`.

Do not create an alternate tag merely to recover from a failed workflow. Fix the actual failure and retry the same release workflow/tag when possible.

## 8. Watch the release workflow

Find the run created by the pushed tag:

```bash
gh run list --workflow release.yml --limit 5
```

Watch the specific run through completion:

```bash
gh run watch <run-id> --exit-status
```

If it fails, inspect the failed job and fix the underlying issue before declaring the release complete.

## 9. Verify published outputs

Verify the GitHub Release and its assets:

```bash
gh release view "v${VERSION}" --json tagName,name,url,assets
```

**Only when npm publishing is configured**, verify npm publication using the package identity from `dist-workspace.toml`:

```bash
npm view "<npm-package>" version dist-tags.latest --json
```

For direct distribution, verify that the website-hosted **macOS-only** `/install.sh` resolves to the GitHub Release's `rindle-installer.sh` **and** downloads the same-version `rindle-scoped-helper-universal.zip` and `.sha256`. Test the installer plus `upgrade --check` and `auto-update on|off|status` using an isolated/mock scheduler. The helper's Developer ID signature, notarization and SHA-256 verification must be enforced before it is installed. The installer should launch the `~/Library/Safari` folder picker and set up the scoped LaunchAgent without Full Disk Access. Confirm the GitHub Release contains both macOS CLI architectures **and** the universal helper archive with checksum.

If crates.io publication was requested, publish/verify it separately and report its status separately from GitHub/npm.

## 10. Report completion

Report:

- released version and tag
- release commit
- changelog summary
- validation results
- GitHub Actions release result
- GitHub Release URL and artifact set
- npm package/version **or** direct install/upgrade verification
- crates.io status when applicable
- any non-blocking warnings
