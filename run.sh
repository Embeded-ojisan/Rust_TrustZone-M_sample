#!/usr/bin/env bash
set -eu

rm -f qemu_debug.log
rm -rf ./temp && mkdir -p ./temp

rustup override set nightly-2025-07-15

# 1) secure / nonsecure をビルド
(cd secure && cargo build --release)
(cd nonsecure && cargo build --release)

# 2) 出力パス
SEC=target/thumbv8m.main-none-eabi/release/secure
NSC=target/thumbv8m.main-none-eabi/release/nonsecure

# 3) QEMU 実行 (MPS2-AN521 / Cortex-M33)
#
# 注意:
# - QEMU のリセットは Flash ベース(0x1000_0000)のベクタを見に行く。
# - TF-MのBL2配置(0x1000_0000)は「BL2が居る世界」の話。
#   Rust単体PoCではBL2が居ないので secure を 0x1000_0000 に置く。
qemu-system-arm \
  -M mps2-an521 \
  -cpu cortex-m33 \
  -nographic \
  -serial mon:stdio \
  -semihosting \
  -device loader,file=${SEC},addr=0x10000000 \
  -device loader,file=${NSC},addr=0x00100000
