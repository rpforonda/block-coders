#!/bin/sh
# Builds dist/block-coders-<version>-linux-x86_64.tar.gz, ready to copy to
# another Linux computer. Run from anywhere: packaging/make-package.sh
set -eu
cd "$(dirname "$0")/.."

version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
name="block-coders-$version-linux-x86_64"
out="dist/$name"

echo "==> Testing"
cargo test --release --quiet
echo "==> Building"
cargo build --release --quiet
bin=target/release/block-coders

# Older distros can't run binaries that need a newer glibc than they have.
needed=$(objdump -T "$bin" | grep -o 'GLIBC_[0-9.]*' | cut -d_ -f2 | sort -uV | tail -n1)
limit=2.34
if [ "$(printf '%s\n%s\n' "$limit" "$needed" | sort -V | tail -n1)" != "$limit" ]; then
    echo "ERROR: binary needs glibc $needed, newer than the $limit we promise." >&2
    echo "       Find the new symbol with: objdump -T $bin | grep GLIBC_$needed" >&2
    exit 1
fi
echo "    needs glibc $needed (ok, limit $limit)"

echo "==> Packaging"
rm -rf "$out" "dist/$name.tar.gz"
mkdir -p "$out"
cp "$bin" "$out/block-coders"
"$bin" --write-icon "$out/block-coders.png"
cp packaging/block-coders.desktop packaging/install.sh packaging/uninstall.sh packaging/README.txt "$out/"
# The fonts are built into the game; their license travels with it.
mkdir -p "$out/licenses"
cp assets/fonts/OFL-*.txt "$out/licenses/"
chmod 755 "$out/install.sh" "$out/uninstall.sh" "$out/block-coders"
tar -C dist -czf "dist/$name.tar.gz" "$name"

echo "==> Done: dist/$name.tar.gz ($(du -h "dist/$name.tar.gz" | cut -f1))"
