#!/bin/sh
# Installs Block Coders.
#   ./install.sh            install for the current user (no password needed)
#   sudo ./install.sh --system   install for everyone on this computer
set -eu
here=$(cd "$(dirname "$0")" && pwd)

if [ "${1:-}" = "--system" ]; then
    bindir=/usr/local/bin
    datadir=/usr/local/share
else
    bindir="$HOME/.local/bin"
    datadir="${XDG_DATA_HOME:-$HOME/.local/share}"
fi

# Warn (but don't stop) if this computer is missing something the game needs.
if ! ldconfig -p 2>/dev/null | grep -q 'libasound.so.2'; then
    echo "Note: the sound library (libasound2 / alsa-lib) wasn't found. The game will"
    echo "      still need it - install it with your package manager, for example:"
    echo "      sudo apt install libasound2   or   sudo dnf install alsa-lib"
fi
glibc=$(ldd --version 2>/dev/null | head -n1 | grep -o '[0-9]\+\.[0-9]\+$' || true)
if [ -n "$glibc" ] && [ "$(printf '2.34\n%s\n' "$glibc" | sort -V | head -n1)" != "2.34" ]; then
    echo "Warning: this system has glibc $glibc, but Block Coders needs 2.34 or newer"
    echo "         (Ubuntu 22.04+, Debian 12+, Fedora 35+). It may not start."
fi

install -Dm755 "$here/block-coders" "$bindir/block-coders"
install -Dm644 "$here/block-coders.png" "$datadir/icons/hicolor/256x256/apps/block-coders.png"
mkdir -p "$datadir/applications"
sed "s|^Exec=.*|Exec=$bindir/block-coders|" "$here/block-coders.desktop" > "$datadir/applications/block-coders.desktop"
chmod 644 "$datadir/applications/block-coders.desktop"

# Refresh menus/icons where those tools exist; harmless if they don't.
update-desktop-database "$datadir/applications" >/dev/null 2>&1 || true
gtk-update-icon-cache -q -t "$datadir/icons/hicolor" >/dev/null 2>&1 || true

echo "Block Coders is installed! Find it in your app menu, or run: $bindir/block-coders"
