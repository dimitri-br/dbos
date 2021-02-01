@echo off
cargo build
cargo bootimage
qemu-system-x86_64 -drive format=raw,file=target/x86_64-dbos/debug/bootimage-dbos.bin -device isa-debug-exit,iobase=0xf4,iosize=0x04 -serial stdio