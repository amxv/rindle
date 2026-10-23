#!/bin/sh
# Website-hosted installer. cargo-dist publishes and verifies release archives.
set -eu
if [ "$(uname -s)" != "Darwin" ]; then
  echo 'Rindle requires macOS (Safari and iCloud history sync).' >&2
  exit 1
fi
cli='rindle'
repo='https://github.com/amxv/rindle'
mode=${1:-install}
case "$mode" in install|upgrade|--check) ;; *) echo 'Usage: install.sh [upgrade|--check]' >&2; exit 2;; esac
for command in curl sh ditto codesign spctl shasum grep open; do command -v "$command" >/dev/null 2>&1 || { echo "$command is required" >&2; exit 1; }; done
if [ "$mode" = --check ]; then
  printf 'Installed: '
  if command -v "$cli" >/dev/null 2>&1; then "$cli" --version; else echo '(not installed)'; fi
  curl --proto '=https' --tlsv1.2 -fsSIL -o /dev/null -w 'Latest release: %{url_effective}\n' "$repo/releases/latest"
  exit 0
fi

if [ "$mode" = upgrade ] && [ -n "${CLI_UPGRADE_TARGET:-}" ]; then
  case "$CLI_UPGRADE_TARGET" in /*) ;; *) echo 'Upgrade target must be absolute' >&2; exit 1;; esac
  if [ ! -f "$CLI_UPGRADE_TARGET" ] || [ -L "$CLI_UPGRADE_TARGET" ]; then
    echo 'Upgrade requires a regular installed executable' >&2; exit 1
  fi
  RINDLE_INSTALL_DIR=$(dirname "$CLI_UPGRADE_TARGET")
  export RINDLE_INSTALL_DIR
fi
umask 077
stage=$(mktemp -d "${TMPDIR:-/tmp}/${cli}-install.XXXXXXXX")
trap 'rm -rf "$stage"' EXIT HUP INT TERM
# cargo-dist uses sha256sum for release verification. Make its verifier
# available on macOS, where shasum is installed instead.
if ! command -v sha256sum >/dev/null 2>&1; then
  command -v shasum >/dev/null 2>&1 || { echo 'shasum or sha256sum is required' >&2; exit 1; }
  printf '#!/bin/sh\nexec shasum -a 256 "$@"\n' > "$stage/sha256sum"
  chmod 755 "$stage/sha256sum"
  PATH="$stage:$PATH"
  export PATH
fi
if [ -n "${CLI_VERSION:-}" ]; then
  case "$CLI_VERSION" in
    *[!0-9A-Za-z._-]*|'') echo 'Invalid upgrade version' >&2; exit 1 ;;
  esac
  installer_url="$repo/releases/download/v${CLI_VERSION}/${cli}-installer.sh"
else
  installer_url="$repo/releases/latest/download/${cli}-installer.sh"
fi
curl --proto '=https' --tlsv1.2 -fsSL --retry 3 \
  "$installer_url" -o "$stage/installer.sh"
sh "$stage/installer.sh"

# The Rust binary alone cannot read protected Safari history without an
# overbroad Full Disk Access grant. Every published installer MUST include the
# separately notarized, sandboxed helper. Fail closed if that asset is absent.
if [ -n "${CLI_UPGRADE_TARGET:-}" ]; then
  rindle_bin=$CLI_UPGRADE_TARGET
elif [ -n "${RINDLE_INSTALL_DIR:-}" ]; then
  rindle_bin=$RINDLE_INSTALL_DIR/rindle
else
  rindle_bin=${CARGO_HOME:-"$HOME/.cargo"}/bin/rindle
fi
if [ ! -x "$rindle_bin" ]; then
  echo "The Rindle executable was not found at $rindle_bin" >&2
  exit 1
fi
version=$("$rindle_bin" --version)
case "$version" in 'rindle '*) version=${version#rindle } ;; *) echo 'Invalid installed Rindle version' >&2; exit 1 ;; esac
if ! printf '%s\n' "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$'; then
  echo 'Refusing to download a helper for an invalid version' >&2
  exit 1
fi
base_url="$repo/releases/download/v$version"
archive=rindle-scoped-helper-universal.zip
curl --proto '=https' --tlsv1.2 -fsSL --retry 3 "$base_url/$archive" -o "$stage/$archive"
curl --proto '=https' --tlsv1.2 -fsSL --retry 3 "$base_url/$archive.sha256" -o "$stage/$archive.sha256"
(cd "$stage" && shasum -a 256 -c "$archive.sha256")

mkdir "$stage/extracted"
ditto -x -k "$stage/$archive" "$stage/extracted"
helper="$stage/extracted/ScopedSafariHelper.app"
if [ ! -d "$helper" ] || [ -L "$helper" ]; then
  echo 'The Rindle scoped Safari helper archive is invalid' >&2
  exit 1
fi
codesign --verify --strict --deep "$helper"
if ! codesign -dv --verbose=3 "$helper" 2>&1 | grep -q '^Authority=Developer ID Application:'; then
  echo 'Refusing to install a helper without a real Developer ID Application signature' >&2
  exit 1
fi
spctl --assess --type execute "$helper" || {
  echo 'The scoped Safari helper failed macOS Gatekeeper assessment' >&2
  exit 1
}

app_root="$HOME/Library/Application Support/Rindle"
mkdir -p "$app_root"
chmod 700 "$app_root"
destination="$app_root/ScopedSafariHelper.app"
next="$app_root/.ScopedSafariHelper.app.new.$$"
rollback="$app_root/.ScopedSafariHelper.app.old.$$"
if [ -L "$destination" ] || [ -e "$next" ] || [ -e "$rollback" ]; then
  echo 'Refusing to overwrite an unusual existing Rindle helper path' >&2
  exit 1
fi
ditto "$helper" "$next"
codesign --verify --strict --deep "$next"
if [ -d "$destination" ]; then mv "$destination" "$rollback"; fi
if ! mv "$next" "$destination"; then
  if [ -d "$rollback" ]; then mv "$rollback" "$destination"; fi
  echo 'Could not install the scoped Safari helper' >&2
  exit 1
fi
rm -rf "$rollback"

scoped_bin="$destination/Contents/MacOS/RindleScopedProbe"
# If this is an update from an identically signed build, the user's saved
# bookmark should work immediately. Otherwise show the standard one-time
# folder chooser, *only* for ~/Library/Safari (never Full Disk Access).
if ! RINDLE_SCOPED_SAFARI_HELPER="$scoped_bin" "$rindle_bin" status 2>/dev/null \
  | grep -q 'Safari history: accessible via sandboxed, folder-scoped helper'; then
  printf '\nOne-time macOS permission: please choose your Safari history folder.\n'
  printf 'Select ~/Library/Safari in the system folder picker. Rindle will not request Full Disk Access.\n'
  open -W "$destination"
fi
"$rindle_bin" setup --browser dia --scoped-helper "$scoped_bin"
printf '\nRindle is installed with the scoped Safari helper.\n'
printf 'Load the extension in Dia using the directory shown by setup.\n'
