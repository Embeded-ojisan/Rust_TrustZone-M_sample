#!/usr/bin/env bash
set -eu

# 1) secure / nonsecure をビルド
cargo build -p secure  --release
cargo build -p nonsecure --release

# 2) 出力パス
SEC=target/thumbv8m.main-none-eabi/release/secure
NSC=target/thumbv8m.main-none-eabi/release/nonsecure

# 3) QEMU 実行

/usr/local/bin/qemu-system-arm -machine mps2-an505 -cpu cortex-m33 -nographic -semihosting \
  -device loader,file=target/thumbv8m.main-none-eabi/release/secure,addr=0x10000000 \
  -device loader,file=target/thumbv8m.main-none-eabi/release/nonsecure,addr=0x00200000