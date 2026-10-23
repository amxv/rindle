#!/bin/sh
# Local ad-hoc-signed distribution needs no Developer ID or notarization.
# Check consistency and the source state before pushing a release tag.
set -eu
cd "$(dirname "$0")/.."

command -v git >/dev/null 2>&1 || exit 1
command -v gh >/dev/null 2>&1 || exit 1
if [ -n "$(git status --porcelain)" ]; then
  echo 'Commit all changes before tagging Rindle.' >&2
  exit 1
fi
git fetch --quiet origin main
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
  echo 'Push Rindle main to origin before tagging.' >&2
  exit 1
fi

version=$(sed -n '/^\[package\]/,/^\[/s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
if [ -n "${1:-}" ] && [ "$1" != "$version" ]; then
  echo "Requested release $1 does not match Cargo $version" >&2
  exit 1
fi
if git tag -l "v$version" | grep -q .; then
  echo "Git tag v$version already exists" >&2
  exit 1
fi
if git ls-remote --tags origin "refs/tags/v$version" | grep -q .; then
  echo "Remote tag v$version already exists" >&2
  exit 1
fi

# Do not regress to the Developer ID/notarization-only setup path.
grep -q 'codesign --force --sign -' scripts/package-release.sh
grep -q 'rindle-aarch64-apple-darwin.tar.gz' .github/workflows/release.yml
grep -q 'rindle-x86_64-apple-darwin.tar.gz' .github/workflows/release.yml
grep -q 'shasum -a 256 -c' docs/public/install.sh
grep -q 'RINDLE_INSTALL_SKIP_SETUP' docs/public/install.sh
echo "Rindle v$version release preflight passed; no Apple Developer ID or notarization required."
