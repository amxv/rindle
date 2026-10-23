# Rindle agent instructions

Rindle is a macOS-only Rust CLI that synchronizes Safari/iCloud history with Dia, Google Chrome, and Microsoft Edge through a local Native Messaging extension. Keep browser history local, never navigate a user's visited URLs, and do not tag a first release until the extension works end to end with Dia and iPhone Safari. No release has been published.

## Layout

- `Cargo.toml`: package identity and Rust lints; `Cargo.lock` is committed.
- `src/main.rs`: process error and exit boundary.
- `src/cli.rs`: clap argument model.
- `src/lib.rs`: command dispatch.
- `src/macos/`: Safari SQLite adapter, unique generation-aligned Apple.com heartbeat(s), setup, and a user LaunchAgent.
- `src/macos/backend.rs` and `apps/scoped-safari/`: a sandboxed Swift helper with a user-approved security-scoped bookmark for ONLY `~/Library/Safari`; this is the default unattended backend. The old FDA-dependent direct SQLite adapter requires explicit opt-in. Never disable the sandbox or remove its signing or entitlements in a public release.
- `src/state.rs`: local deduplicated browser history queue.
- `src/native.rs`: browser-allowlisted, one-shot Native Messaging bridge.
- `src/urls.rs`: privacy-oriented URL validation.
- `extension/`: unpacked Manifest V3 Chromium extension, bundled in the CLI binary.
- `src/commands/`: command behavior. `upgrade` and `auto_update` are retained release infrastructure.
- `tests/`: integration tests that execute the CLI.
- `scripts/package-release.sh`: per-architecture CLI and sandboxed app packaging.
- `.github/workflows/release.yml`: release builds, installer smoke tests and publishing.
- `docs/`: isolated Astro/ZueDocs site and direct installers.
- `docs/public/install.sh`: the only website-hosted installer, macOS only.

## Local checks

```bash
just check-fast
just check
just build
just docs-test
just docs-check
just release-plan
```

The pinned Rust version is in `rust-toolchain.toml`. Use Rust 2024, no unsafe code, and obey Clippy's denied lints. Keep product logic in modules with tests, not in `main.rs`.

## Releases and docs

Direct distribution produces **Apple Silicon and Intel macOS** per-architecture tarballs, each containing the Rust CLI and ad-hoc-signed, sandboxed `ScopedSafariHelper.app`, plus SHA-256 checksums and a website-hosted shell installer. No Developer ID, notarization, automatic Gatekeeper disabling or quarantine stripping. Browser downloads of an `.app` can still be blocked; the verified curl/tar installation path normally has no browser quarantine attribute. No Windows/Linux artifacts, npm/crates.io publishing or menu bar app. Built-in `upgrade` and optional daily `auto-update` are separate from three-hour history sync and must preserve user opt-outs.

For releases, follow `.agents/skills/release/SKILL.md`. The hand-authored `.github/workflows/release.yml` is the authoritative release graph; update and test it directly (no cargo-dist generation).

The site has Vercel project root `docs`, with GitHub connected and `git.deploymentEnabled: false` in `docs/vercel.json`. `.github/workflows/docs.yml` builds and deploys the site only for `docs/**` changes on `main`, using a GitHub Actions token secret and project ID variables. Do not add a Vercel `ignoreCommand` or re-enable Git auto-deploys unless the deploy strategy is redesigned. For routine local validation, use `just docs-test` and `just docs-check`, not an extra Astro production build. Keep the site's canonical URL `https://rindle.ashray.xyz`.
