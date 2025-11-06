```bash
#!/bin/bash

pkill -9 qemu

FLAG_FILE=$(mktemp)
echo "$FLAG1" > $FLAG_FILE
FLAG_FILE2=$(mktemp)
echo "$FLAG2" > $FLAG_FILE2

qemu-system-aarch64 \
  -M virt \
  -kernel ./Image \
  -cpu cortex-a76 \
  -m 128M \
  -smp 2 \
  -net nic,netdev=net0 \
  -netdev user,id=net0 \
  -drive file=./hamal.ext4,format=raw,readonly=on \
  -drive file=$FLAG_FILE,format=raw \
  -drive file=$FLAG_FILE2,format=raw \
  -initrd ./rootfs.cpio.gz  \
  -append "console=ttyAMA0 kaslr quiet oops=panic panic_on_warn=1 panic=-1" \
  -monitor /dev/null \
  -no-reboot \
  -nographic

rm $FLAG_FILE $FLAG_FILE2
reset
```
