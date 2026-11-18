FROM rust:1.88-slim-bookworm
RUN apt-get update \
 && apt-get install -y --no-install-recommends gcc-mingw-w64-x86-64 binutils-mingw-w64-x86-64 nsis curl xz-utils ca-certificates \
 && rm -rf /var/lib/apt/lists/*
# llvm-mingw : éditeur de liens et windres pour Windows ARM64
ARG LLVM_MINGW=20261020
# Empreintes relevées sur l'archive publiée (à revérifier à chaque changement de version)
ARG LLVM_MINGW_SHA256_X86_64=55064b60b991a1e1e8090b8c2f84e82b22cfe5903d12c159d65064cf5e8ae335
ARG LLVM_MINGW_SHA256_AARCH64=ca16bb0f4f485876496fa46ad8d353c525eba57d0cf9e9e49ec1738fe4810151
RUN set -e; ARCH=$(uname -m); \
    case "$ARCH" in x86_64) SHA="$LLVM_MINGW_SHA256_X86_64";; aarch64) SHA="$LLVM_MINGW_SHA256_AARCH64";; esac; \
    curl -fsSL -o /tmp/llvm-mingw.tar.xz "https://github.com/mstorsjo/llvm-mingw/releases/download/${LLVM_MINGW}/llvm-mingw-${LLVM_MINGW}-ucrt-ubuntu-22.04-${ARCH}.tar.xz"; \
    echo "$SHA  /tmp/llvm-mingw.tar.xz" | sha256sum -c -; \
    tar -xJf /tmp/llvm-mingw.tar.xz -C /opt && mv /opt/llvm-mingw-* /opt/llvm-mingw && rm /tmp/llvm-mingw.tar.xz
RUN rustup target add x86_64-pc-windows-gnu aarch64-pc-windows-gnullvm
ENV PATH="${PATH}:/opt/llvm-mingw/bin" \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    CARGO_TARGET_AARCH64_PC_WINDOWS_GNULLVM_LINKER=aarch64-w64-mingw32-clang \
    CARGO_TARGET_DIR=/build
WORKDIR /src
CMD ["sh", "build.sh"]
