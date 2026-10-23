#!/bin/sh
# Rindle macOS installer: HTTPS download, SHA-256 check, tar extraction, and
# ad-hoc-signed sandboxed Safari helper installed alongside the CLI.
# We never disable Gatekeeper or strip quarantine attributes.
set -eu

[ "$(uname -s)" = Darwin ] || { echo 'Rindle requires macOS.' >&2; exit 1; }
mode=${1:-install}
case "$mode" in install|upgrade|--check) ;; *) echo 'Usage: install.sh [upgrade|--check]' >&2; exit 2 ;; esac
for cmd in curl tar shasum codesign lipo install open ditto; do
  command -v "$cmd" >/dev/null 2>&1 || { echo "Missing required macOS tool: $cmd" >&2; exit 1; }
done

repo=https://github.com/amxv/rindle
case "$(uname -m)" in
  arm64|aarch64) target=aarch64-apple-darwin ;;
  x86_64) target=x86_64-apple-darwin ;;
  *) echo 'Unsupported macOS CPU architecture.' >&2; exit 1 ;;
esac

if [ "$mode" = --check ]; then
  if command -v rindle >/dev/null 2>&1; then rindle --version; else echo 'Rindle is not installed on PATH.'; fi
  curl --proto '=https' --tlsv1.2 -fsSIL -o /dev/null -w 'Latest release: %{url_effective}\n' "$repo/releases/latest"
  exit 0
fi

if [ -n "${CLI_UPGRADE_TARGET:-}" ]; then
  case "$CLI_UPGRADE_TARGET" in /*) ;; *) echo 'Upgrade target must be an absolute path.' >&2; exit 1 ;; esac
  [ -f "$CLI_UPGRADE_TARGET" ] && [ ! -L "$CLI_UPGRADE_TARGET" ] || { echo 'Invalid existing Rindle executable.' >&2; exit 1; }
  RINDLE_INSTALL_DIR=$(dirname "$CLI_UPGRADE_TARGET")
fi
install_dir=${RINDLE_INSTALL_DIR:-"$HOME/.local/bin"}
app_dir=${RINDLE_APP_DIR:-"$HOME/Library/Application Support/Rindle"}
case "$install_dir" in /*) ;; *) echo 'Install directory must be absolute.' >&2; exit 1 ;; esac
case "$app_dir" in /*) ;; *) echo 'App directory must be absolute.' >&2; exit 1 ;; esac

umask 077
stage=$(mktemp -d "${TMPDIR:-/tmp}/rindle-install.XXXXXXXX")
cli_target="$install_dir/rindle"
helper_target="$app_dir/ScopedSafariHelper.app"
cli_next="$install_dir/.rindle-install.$$"
cli_old="$install_dir/.rindle-previous.$$"
helper_next="$app_dir/.ScopedSafariHelper.new.$$"
helper_old="$app_dir/.ScopedSafariHelper.old.$$"
installed_cli=0
installed_helper=0

finish() {
  result=$?
  if [ "$result" -ne 0 ]; then
    echo 'Rindle installation did not complete; restoring the previous CLI and helper.' >&2
    if [ "$installed_cli" -eq 1 ]; then rm -f "$cli_target"; fi
    if [ "$installed_helper" -eq 1 ]; then rm -rf "$helper_target"; fi
    if [ -f "$cli_old" ]; then mv "$cli_old" "$cli_target"; fi
    if [ -d "$helper_old" ]; then mv "$helper_old" "$helper_target"; fi
  else
    rm -f "$cli_old"
    rm -rf "$helper_old"
  fi
  rm -f "$cli_next"
  rm -rf "$helper_next" "$stage"
}
trap finish EXIT

archive_name="rindle-${target}.tar.gz"
archive="$stage/$archive_name"
checksum="$stage/$archive_name.sha256"
if [ -n "${RINDLE_LOCAL_ARCHIVE:-}" ]; then
  # Explicit local QA mode. Standard installs always use GitHub HTTPS assets.
  cp "$RINDLE_LOCAL_ARCHIVE" "$archive"
  cp "$RINDLE_LOCAL_ARCHIVE.sha256" "$checksum"
else
  if [ -n "${CLI_VERSION:-}" ]; then
    case "$CLI_VERSION" in *[!0-9.]*|'') echo 'Unsupported release version.' >&2; exit 1 ;; esac
    release_base="$repo/releases/download/v$CLI_VERSION"
  else
    release_base="$repo/releases/latest/download"
  fi
  curl --proto '=https' --tlsv1.2 -fsSL --retry 3 "$release_base/$archive_name" -o "$archive"
  curl --proto '=https' --tlsv1.2 -fsSL --retry 3 "$release_base/$archive_name.sha256" -o "$checksum"
fi

(cd "$stage" && shasum -a 256 -c "$archive_name.sha256")
tar -tzf "$archive" | awk -v top="rindle-${target}" '
  /^\// || /(^|\/)\.\.($|\/)/ { bad=1 }
  $0 != top && index($0, top "/") != 1 { bad=1 }
  END { exit bad ? 1 : 0 }
' || { echo 'Release archive contains an unsafe path.' >&2; exit 1; }
mkdir "$stage/extract"
tar -xzf "$archive" -C "$stage/extract"
source_dir="$stage/extract/rindle-${target}"
source_cli="$source_dir/rindle"
source_app="$source_dir/ScopedSafariHelper.app"
[ -f "$source_cli" ] && [ ! -L "$source_cli" ] && [ -x "$source_cli" ] || { echo 'Release is missing the Rindle CLI.' >&2; exit 1; }
[ -d "$source_app" ] && [ ! -L "$source_app" ] || { echo 'Release is missing the scoped Safari app.' >&2; exit 1; }
[ -z "$(find "$source_dir" -type l -print -quit)" ] || { echo 'Release contains unexpected symlinks.' >&2; exit 1; }
codesign --verify --strict --deep "$source_app"
codesign -dv --verbose=2 "$source_app" 2>&1 | grep -q 'Signature=adhoc' || { echo 'Release app must be ad-hoc signed.' >&2; exit 1; }
case "$target" in aarch64-apple-darwin) arch=arm64 ;; *) arch=x86_64 ;; esac
lipo "$source_cli" -verify_arch "$arch"
lipo "$source_app/Contents/MacOS/RindleScopedProbe" -verify_arch "$arch"

mkdir -p "$install_dir" "$app_dir"
[ ! -L "$install_dir" ] && [ ! -L "$app_dir" ] || { echo 'Refusing symlinked installation directories.' >&2; exit 1; }
chmod 700 "$app_dir"
for path in "$cli_next" "$cli_old" "$helper_next" "$helper_old"; do
  [ ! -e "$path" ] || { echo "Refusing existing temporary install path: $path" >&2; exit 1; }
done
[ ! -L "$cli_target" ] && [ ! -L "$helper_target" ] || { echo 'Refusing to replace a symlinked Rindle installation.' >&2; exit 1; }
install -m 0755 "$source_cli" "$cli_next"
ditto "$source_app" "$helper_next"
codesign --verify --strict --deep "$helper_next"

if [ -f "$cli_target" ]; then mv "$cli_target" "$cli_old"; fi
mv "$cli_next" "$cli_target"
installed_cli=1
if [ -d "$helper_target" ]; then mv "$helper_target" "$helper_old"; fi
mv "$helper_next" "$helper_target"
installed_helper=1

if [ "${RINDLE_INSTALL_SKIP_SETUP:-0}" != 1 ]; then
  helper_bin="$helper_target/Contents/MacOS/RindleScopedProbe"
  if ! "$helper_bin" --verify >/dev/null 2>&1; then
    if [ "${RINDLE_NONINTERACTIVE:-0}" = 1 ]; then
      echo 'Saved Safari permission needs renewal; cannot prompt during an unattended upgrade.' >&2
      exit 1
    fi
    echo 'Select ~/Library/Safari in the Rindle Safari Access folder picker.'
    open -W "$helper_target"
    "$helper_bin" --verify || { echo 'Safari folder permission was not granted.' >&2; exit 1; }
  fi
  "$cli_target" setup --browser dia --scoped-helper "$helper_bin"
fi

echo "Rindle installed: $cli_target"
echo "Safari helper installed: $helper_target"
if [ "${RINDLE_INSTALL_SKIP_SETUP:-0}" != 1 ]; then
  echo 'Load the unpacked browser extension from ~/Library/Application Support/Rindle/extension.'
fi
