#!/bin/sh
# Removes Block Coders. Use --system if it was installed with --system.
# Players' saved progress (~/.local/share/block-coders) is kept, unless you add --remove-saves.
set -eu
bindir="$HOME/.local/bin"
datadir="${XDG_DATA_HOME:-$HOME/.local/share}"
for arg in "$@"; do
    case "$arg" in
        --system) bindir=/usr/local/bin; datadir=/usr/local/share ;;
        --remove-saves) rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/block-coders"; echo "Saved progress removed." ;;
    esac
done
rm -f "$bindir/block-coders" \
      "$datadir/applications/block-coders.desktop" \
      "$datadir/icons/hicolor/256x256/apps/block-coders.png"
update-desktop-database "$datadir/applications" >/dev/null 2>&1 || true
echo "Block Coders has been removed."
