#!/bin/bash
# Aperture installer — downloads the latest release binaries.
# Usage: curl -fsSL https://raw.githubusercontent.com/yourusername/aperture/main/scripts/install.sh | bash
set -euo pipefail

REPO="yourusername/aperture"
INSTALL_DIR="${APERTURE_INSTALL_DIR:-/usr/local/bin}"

info()  { printf '\033[1;34m==> %s\033[0m\n' "$*"; }
error() { printf '\033[1;31mERROR: %s\033[0m\n' "$*" >&2; exit 1; }

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
  x86_64|amd64) ARTIFACT="aperture-linux-amd64" ;;
  aarch64|arm64) ARTIFACT="aperture-linux-arm64" ;;
  *) error "Unsupported architecture: $ARCH" ;;
esac

OS=$(uname -s)
[ "$OS" = "Linux" ] || error "Aperture agent requires Linux (got $OS). The aggregator and CLI can run anywhere."

# Fetch latest release tag
info "Fetching latest release..."
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | cut -d'"' -f4)
[ -n "$LATEST" ] || error "Could not determine latest release. Check https://github.com/${REPO}/releases"

info "Installing Aperture $LATEST ($ARTIFACT)"

# Download and extract
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
URL="https://github.com/${REPO}/releases/download/${LATEST}/${ARTIFACT}.tar.gz"
info "Downloading $URL"
curl -fsSL "$URL" -o "$TMPDIR/aperture.tar.gz"
tar xzf "$TMPDIR/aperture.tar.gz" -C "$TMPDIR"

# Install binaries
info "Installing to $INSTALL_DIR (may require sudo)"
for bin in aperture-aggregator aperture-cli; do
  if [ -f "$TMPDIR/$bin" ]; then
    if [ -w "$INSTALL_DIR" ]; then
      install -m 755 "$TMPDIR/$bin" "$INSTALL_DIR/$bin"
    else
      sudo install -m 755 "$TMPDIR/$bin" "$INSTALL_DIR/$bin"
    fi
    info "  Installed $bin"
  fi
done

info "Installation complete!"
echo ""
echo "  aperture-aggregator --help    # Start the aggregation server"
echo "  aperture-cli --help           # CLI for querying and profiling"
echo ""
echo "  For the agent (requires root + eBPF):"
echo "    See https://github.com/${REPO}/blob/main/docs/RUN-EXAMPLES.md"
