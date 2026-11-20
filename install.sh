#!/bin/sh
# Install a verified Eita GitHub Release on Linux or macOS.
# Usage: curl -fsSL https://raw.githubusercontent.com/rodkisten/eita/main/install.sh | sh
# Optional: EITA_VERSION=main-<sha>|pr-<number> EITA_INSTALL_DIR=/path/to/bin
set -eu

die() {
  printf 'eita: %s\n' "$*" >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || die "curl is required"
command -v tar >/dev/null 2>&1 || die "tar is required"
command -v install >/dev/null 2>&1 || die "install is required"

case "$(uname -s)" in
  Linux) os=unknown-linux-musl ;;
  Darwin) os=apple-darwin ;;
  *) die "unsupported operating system: $(uname -s)" ;;
esac

case "$(uname -m)" in
  x86_64|amd64) arch=x86_64 ;;
  aarch64|arm64) arch=aarch64 ;;
  *) die "unsupported CPU architecture: $(uname -m)" ;;
esac

[ -n "${HOME:-}" ] || die "HOME must be set (or use EITA_INSTALL_DIR)"
install_dir="${EITA_INSTALL_DIR:-$HOME/.local/bin}"
plugin_dir="${EITA_PLUGIN_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/eita}"
archive="eita-$arch-$os.tar.gz"
release="${EITA_VERSION:-latest}"

case "$release" in
  latest)
    base="https://github.com/rodkisten/eita/releases/latest/download"
    ;;
  main-*|pr-*)
    case "$release" in
      *[!A-Za-z0-9._-]*|'main-'|'pr-') die "invalid release tag: $release" ;;
    esac
    base="https://github.com/rodkisten/eita/releases/download/$release"
    ;;
  *)
    die "EITA_VERSION must be latest, main-<commit>, or pr-<number>"
    ;;
esac

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/eita-install.XXXXXX")" || die "cannot create temp directory"
trap 'rm -rf "$tmpdir"' 0
trap 'exit 130' 2
trap 'exit 143' 15

printf '🦀 Installing Eita (%s, %s)\n' "$release" "$arch-$os"
curl -fLsS --retry 3 "$base/$archive" -o "$tmpdir/$archive" || die "unable to download $archive"
curl -fLsS --retry 3 "$base/$archive.sha256" -o "$tmpdir/$archive.sha256" || die "unable to download checksum"

(
  cd "$tmpdir"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "$archive.sha256"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c "$archive.sha256"
  else
    die "sha256sum or shasum is required for verification"
  fi
) || die "checksum verification failed"

tar -xzf "$tmpdir/$archive" -C "$tmpdir" || die "invalid release archive"
[ -f "$tmpdir/eita" ] || die "release archive does not contain eita"
[ -f "$tmpdir/integrations/zsh/eita.plugin.zsh" ] || die "release archive does not contain Zsh plugin"

mkdir -p "$install_dir" "$plugin_dir"
install -m 0755 "$tmpdir/eita" "$install_dir/eita"
install -m 0644 "$tmpdir/integrations/zsh/eita.plugin.zsh" "$plugin_dir/eita.plugin.zsh"

if ! "$install_dir/eita" --version; then
  die "installed binary did not launch correctly; installation failed"
fi
printf '\n✓ Installed: %s\n' "$install_dir/eita"
printf '✓ Zsh plugin: %s\n' "$plugin_dir/eita.plugin.zsh"
printf '\nRun: %s suggest --line "git comit"\n' "$install_dir/eita"
printf 'Popup test: %s popup --line "git com"\n' "$install_dir/eita"
printf 'For Ctrl+Space completion, add the install directory to PATH and source the plugin in ~/.zshrc.\n'
