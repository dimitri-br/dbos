@echo off
cargo build
qemu-system-x86_64 -drive format=raw,file=target/x86_64-blog_os/debug/dbos.bin