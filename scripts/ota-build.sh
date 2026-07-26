#!/bin/sh
set -e
cargo build --release
espflash save-image --chip=esp32 target/xtensa-esp32-none-elf/release/auto-water firmware.bin
