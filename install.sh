#!/bin/sh
# Installs the matrical command line.
#
#   curl -fsSL https://raw.githubusercontent.com/blossomstack/matrical-cli/main/install.sh | sh
#
# MATRICAL_VERSION      a release tag such as v0.1.0; the newest release if unset
# MATRICAL_INSTALL_DIR  where to put the binary; ~/.local/bin if unset
set -eu

repo=blossomstack/matrical-cli
install_dir=${MATRICAL_INSTALL_DIR:-$HOME/.local/bin}

fail() {
  echo "matrical install: $*" >&2
  exit 1
}

case "$(uname -s)" in
  Linux) os=unknown-linux-musl ;;
  Darwin) os=apple-darwin ;;
  *) fail "no build for $(uname -s); build from source with cargo instead" ;;
esac

case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  arm64 | aarch64) arch=aarch64 ;;
  *) fail "no build for $(uname -m); build from source with cargo instead" ;;
esac

# An x86_64 shell under Rosetta reports x86_64 on an Apple silicon Mac.
if [ "$os" = apple-darwin ] && [ "$arch" = x86_64 ] &&
  [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || echo 0)" = 1 ]; then
  arch=aarch64
fi

asset="matrical-$arch-$os.tar.gz"
if [ -n "${MATRICAL_DOWNLOAD_URL:-}" ]; then
  base=$MATRICAL_DOWNLOAD_URL
elif [ -n "${MATRICAL_VERSION:-}" ]; then
  base="https://github.com/$repo/releases/download/$MATRICAL_VERSION"
else
  base="https://github.com/$repo/releases/latest/download"
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "Downloading $asset"
curl -fsSL "$base/$asset" -o "$tmp/$asset" || fail "could not download $base/$asset"
curl -fsSL "$base/$asset.sha256" -o "$tmp/$asset.sha256" ||
  fail "could not download $base/$asset.sha256"

expected=$(cut -d ' ' -f 1 "$tmp/$asset.sha256")
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$tmp/$asset" | cut -d ' ' -f 1)
else
  actual=$(shasum -a 256 "$tmp/$asset" | cut -d ' ' -f 1)
fi
[ "$expected" = "$actual" ] || fail "checksum mismatch for $asset"

tar -xzf "$tmp/$asset" -C "$tmp"
mkdir -p "$install_dir"
# Renamed into place within the same folder, so a running copy is not corrupted.
cp "$tmp/matrical" "$install_dir/.matrical.new"
chmod +x "$install_dir/.matrical.new"
mv -f "$install_dir/.matrical.new" "$install_dir/matrical"

echo "Installed $("$install_dir/matrical" --version) to $install_dir/matrical"
case ":$PATH:" in
  *":$install_dir:"*) ;;
  *) echo "Add $install_dir to your PATH to run it as 'matrical'." ;;
esac
