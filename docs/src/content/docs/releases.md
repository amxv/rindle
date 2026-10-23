---
title: Releases and installers
description: Direct distribution using GitHub Releases and cargo-dist.
order: 3
category: Distribution
summary: Supported targets, install scripts, and the eventual first release.
---

Rindle uses **direct distribution**, not npm or crates.io. `dist-workspace.toml` configures `cargo-dist`; `.github/workflows/release.yml` is generated from that configuration.

## Platforms

The release configuration targets Apple Silicon and Intel macOS, ARM64 and x64 Linux, and x64 Windows. Releases will contain native archives and checksum files, plus shell and PowerShell installers.

## Installing a release

Once the first tagged release is published, users will be able to install with:

```bash
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
```

For Windows, the PowerShell installer is available at `https://rindle.ashray.xyz/install.ps1`.

**There is no published release yet.** The site hosts installer entry points now, but they cannot download a Rindle binary until the first release exists.

The installed CLI includes `rindle upgrade` and `rindle upgrade --check`. Optional daily updates use launchd on macOS, systemd user timers on Linux, and Task Scheduler on Windows. Background updates are **off by default**.

## Publishing

The package version in `Cargo.toml` must match the tag. Before the first release, implement and test the application behavior, update the [changelog](/docs/changelog), and then run:

```bash
just check
just docs-check
dist plan
just release-tag 0.1.0
```

The tag triggers the GitHub Actions release workflow. Follow `.agents/skills/release/SKILL.md` for the complete procedure. Change `dist-workspace.toml` and run `dist generate` when modifying release configuration; never edit the generated workflow by hand.
