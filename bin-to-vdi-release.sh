#!/bin/sh
#cargo bootimage
cd /mnt/d/dbos/target/x86_64-dbos/release/
dd if=bootimage-dbos.bin of=dbos.img bs=100M conv=sync
rm dbos.vdi
VBoxManage convertdd dbos.img dbos.vdi --format VDI