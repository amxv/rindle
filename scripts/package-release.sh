#!/usr/bin/env bash
# Package the ad-hoc-signed sandboxed Safari helper and the Rust CLI in
# an architecture-specific .tar.gz archive, with a SHA-256 sidecar.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

target="${1:?Usage: scripts/package-release.sh aarch64-apple-darwin|x86_64-apple-darwin}"
case "$target" in
  aarch64-apple-darwin) arch=arm64 ;;
  x86_64-apple-darwin) arch=x86_64 ;;
  *) echo "Unsupported Rindle release target: $target" >&2; exit 2 ;;
esac

version="$(sed -n '/^\[package\]/,/^\[/s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Invalid Cargo version' >&2; exit 1; }
archive_name="rindle-${target}"
out_dir="$root/target/distrib"
staged="$out_dir/$archive_name"
mkdir -p "$out_dir"
[[ ! -e "$staged" ]] || rm -rf "$staged"
mkdir -p "$staged/ScopedSafariHelper.app/Contents/MacOS" "$staged/ScopedSafariHelper.app/Contents/Resources"

# Explicit Rust target avoids accidentally publishing a host-architecture CLI.
cargo build --locked --release --target "$target" --bin rindle
install -m 0755 "target/$target/release/rindle" "$staged/rindle"
cp LICENSE README.md "$staged/"

app="$staged/ScopedSafariHelper.app"
cp apps/scoped-safari/Info.plist "$app/Contents/Info.plist"
bundle_version="${version%%[-+]*}"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $bundle_version" "$app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $bundle_version" "$app/Contents/Info.plist"

sdk="$(xcrun --sdk macosx --show-sdk-path)"
swiftc -O -sdk "$sdk" -target "${arch}-apple-macosx14.0" \
  -framework AppKit -framework Foundation -lsqlite3 \
  apps/scoped-safari/main.swift \
  apps/scoped-safari/ScopedSafariService.swift \
  -o "$app/Contents/MacOS/RindleScopedProbe"
chmod 755 "$app/Contents/MacOS/RindleScopedProbe"

# Preserve the sandbox entitlements and the stable CFBundleIdentifier. No
# Apple Developer Program, notarization, Gatekeeper modification or quarantine
# stripping. We validate this signature for integrity, not publisher identity.
codesign --force --sign - --timestamp=none \
  --entitlements apps/scoped-safari/ScopedSafariProbe.entitlements "$app"
codesign --verify --strict --deep "$app"
signature=$(codesign -dv --verbose=2 "$app" 2>&1)
grep -q 'Signature=adhoc' <<< "$signature"
lipo "$staged/rindle" -verify_arch "$arch"
lipo "$app/Contents/MacOS/RindleScopedProbe" -verify_arch "$arch"

tar -C "$out_dir" -czf "$out_dir/$archive_name.tar.gz" "$archive_name"
(
  cd "$out_dir"
  shasum -a 256 "$archive_name.tar.gz" > "$archive_name.tar.gz.sha256"
  shasum -a 256 -c "$archive_name.tar.gz.sha256"
)
echo "Packaged $archive_name.tar.gz: ad-hoc signed scoped .app + CLI + SHA-256"
