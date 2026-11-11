#!/bin/sh
# Compile HDRTray.exe puis l'installeur. Lancé dans le conteneur (voir Dockerfile).
set -e
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cargo build --release --target x86_64-pc-windows-gnu
mkdir -p dist
cp /build/x86_64-pc-windows-gnu/release/hdrtray.exe dist/HDRTray.exe
echo "DLL importées :"
x86_64-w64-mingw32-objdump -p dist/HDRTray.exe | grep "DLL Name"
makensis -V2 -DVERSION="$VERSION" installer.nsi
echo "$VERSION" > dist/version.txt
ls -la dist
