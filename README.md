# Rindle

An early-stage Rust CLI. The project is intentionally a minimal scaffold: no application-specific commands have been implemented yet.

**[Documentation](https://rindle.ashray.xyz)** · **[Source](https://github.com/amxv/rindle)**

## Develop

Install [Rust](https://rustup.rs/) and [just](https://github.com/casey/just), then:

```bash
git clone https://github.com/amxv/rindle.git
cd rindle
just check
cargo run -- --help
```

The repository pins Rust 1.99.1 and uses edition 2024. Its application structure is deliberately small:

```text
src/main.rs       process exit and errors
src/cli.rs        clap argument parser
src/lib.rs        command dispatch
src/commands/     command implementations
tests/            CLI integration tests
docs/             Astro/ZueDocs website
```

`just check-fast` performs format, check, and Clippy validation. `just check` runs format, Clippy, and tests. `just build` builds the binary.

## Distribution

Rindle is configured for **direct GitHub Releases**, not npm or crates.io. The [cargo-dist](https://opensource.axo.dev/cargo-dist/) configuration targets Apple Silicon/Intel macOS, ARM64/x64 Linux, and x64 Windows. Releases will include checksummed native archives and shell/PowerShell installers. No macOS menu bar app is included.

When the first release is published, its installers will be accessible at:

```bash
curl -fsSL https://rindle.ashray.xyz/install.sh | sh
```

Windows: `https://rindle.ashray.xyz/install.ps1`

**There is no published Rindle release yet.** These website scripts are present, but cannot install a binary until a tagged release exists.

The distribution scaffold includes `rindle upgrade [--check]` and `rindle auto-update on|off|status`. Automatic daily updates are disabled until a user explicitly enables them. The CLI has no other commands yet.

See [Releases and installers](https://rindle.ashray.xyz/docs/releases) for the planned release process.

## Documentation

The site lives entirely under `docs/` and requires [Bun](https://bun.sh/):

```bash
just docs-install
just docs-dev
just docs-test
just docs-check
```

Production runs on Vercel, with `rindle.ashray.xyz` configured through Cloudflare. Vercel's Git auto-deployments are disabled; `.github/workflows/docs.yml` deploys only for changes to `docs/**` on `main`, avoiding deployments for Rust-only commits.

## License

Apache-2.0. See [LICENSE](LICENSE).
