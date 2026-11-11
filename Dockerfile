FROM rust:1.88-slim-bookworm
RUN apt-get update \
 && apt-get install -y --no-install-recommends gcc-mingw-w64-x86-64 binutils-mingw-w64-x86-64 nsis curl xz-utils ca-certificates \
 && rm -rf /var/lib/apt/lists/*
# llvm-mingw : éditeur de liens et windres pour Windows ARM64
ARG LLVM_MINGW=20261020
RUN curl -fsSL "https://github.com/mstorsjo/llvm-mingw/releases/download/${LLVM_MINGW}/llvm-mingw-${LLVM_MINGW}-ucrt-ubuntu-22.04-$(uname -m).tar.xz" \
  | tar -xJ -C /opt \
 && mv /opt/llvm-mingw-* /opt/llvm-mingw
RUN rustup target add x86_64-pc-windows-gnu aarch64-pc-windows-gnullvm
ENV PATH="${PATH}:/opt/llvm-mingw/bin" \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    CARGO_TARGET_AARCH64_PC_WINDOWS_GNULLVM_LINKER=aarch64-w64-mingw32-clang \
    CARGO_TARGET_DIR=/build
WORKDIR /src
CMD ["sh", "build.sh"]
