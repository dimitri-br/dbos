@echo off

rem build our binary
cargo build --release

rem add the bootloader to the binary
cargo bootimage --release

rem compile for virtualbox
wsl "./bin-to-vdi-release.sh"

rem run qemu
qemu-system-x86_64 -drive format=raw,file=target/x86_64-dbos/release/bootimage-dbos.bin -device isa-debug-exit,iobase=0xf4,iosize=0x04 -serial stdio

rem open the vdi file location
cd target/x86_64-dbos/release/
explorer .
cd ../../..