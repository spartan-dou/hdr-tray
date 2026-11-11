#!/bin/sh
# Compile HDRTray (x64 et ARM64) puis les installeurs. Lancé dans le conteneur (voir Dockerfile).
# Les noms x64 restent sans suffixe : la mise à jour auto de la v1.0.0 télécharge HDRTray-Setup.exe.
set -e
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
mkdir -p dist

build() { # build <cible rust> <suffixe> <arch nsis>
  cargo build --release --target "$1"
  cp "/build/$1/release/hdrtray.exe" "dist/HDRTray$2.exe"
  echo "DLL importées (HDRTray$2.exe) :"
  llvm-objdump -p "dist/HDRTray$2.exe" | grep "DLL Name"
  makensis -V2 -DVERSION="$VERSION" -DARCH="$3" -DSUFFIX="$2" installer.nsi
}

build x86_64-pc-windows-gnu "" x64
build aarch64-pc-windows-gnullvm "-arm64" arm64
echo "$VERSION" > dist/version.txt
ls -la dist
