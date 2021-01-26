#!/bin/sh
#cargo bootimage
dd if=./target/x86_64-dbos/debug/bootimage-dbos.bin of=./target/x86_64-dbos/debug/dbos.img bs=100M conv=sync
rm ./target/x86_64-dbos/debug/dbos.vdi
VBoxManage convertdd ./target/x86_64-dbos/debug/dbos.img ./target/x86_64-dbos/debug/dbos.vdi --format VDI