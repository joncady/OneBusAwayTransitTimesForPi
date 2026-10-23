#!/bin/bash
# Snapshot audited PM3 disks while internal filesystems are read-only, then copy privately.
set -euo pipefail
PM3_SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PM3_TARGET=${1:?Usage: backup-device.sh root@DEVICE_HOST}
PM3_BACKUP_ID="boot-$(date -u +%Y%m%dT%H%M%SZ)"
PM3_LOCAL="$PM3_SOURCE_DIR/recovery.local/$PM3_BACKUP_ID"
umask 077
mkdir -p "$PM3_SOURCE_DIR/recovery.local"
mkdir "$PM3_LOCAL"
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" "bash -s -- $PM3_BACKUP_ID" <<'REMOTE'
set -euo pipefail
backup="/init/efad/mp_sd/transit-recovery/$1"
grep -q ' / ext4 ro,' /proc/mounts || { echo 'Root must be read-only before imaging.' >&2; exit 1; }
grep -q '^root=/dev/sda1 ' /proc/cmdline || { echo 'Unexpected root device; review storage mapping.' >&2; exit 1; }
grep -q '^/dev/sdb1 /init ' /proc/mounts || { echo 'Unexpected data device; review storage mapping.' >&2; exit 1; }
grep -q '^/dev/sdc /init/efad/mp_sd ' /proc/mounts || { echo 'Expected USB mount missing.' >&2; exit 1; }
set -- "$1" $(df -Pk /init/efad/mp_sd | tail -1)
[ "$5" -gt 2100000 ] || { echo 'Need at least 2100000 KiB USB space before imaging.' >&2; exit 1; }
mkdir -p /init/efad/mp_sd/transit-recovery
mkdir "$backup"
was_active=''
for unit in transit-display.service transit-prototype.service; do
  if systemctl is-active --quiet "$unit"; then was_active="$unit"; break; fi
done
readonly=0
lock_held=0
app_stopped=0
cleanup() {
 trap - EXIT HUP INT TERM
 if [ "$readonly" = 1 ]; then mount -o remount,rw /init || true; fi
 if [ "$lock_held" = 1 ]; then rmdir /init/transit-prototype/maintenance.lock || true; fi
 if [ "$app_stopped" != 1 ]; then return; fi
 if [ "$was_active" = transit-display.service ]; then systemctl start "$was_active" || true;
 elif [ "$was_active" = transit-prototype.service ]; then /init/transit-prototype/restart-display.sh 3600 live || true; fi
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir /init/transit-prototype/maintenance.lock
lock_held=1
if [ -n "$was_active" ]; then app_stopped=1; systemctl stop "$was_active"; fi
sync
mount -o remount,ro /init
readonly=1
for spec in 'root-card:/dev/sda' 'data-card:/dev/sdb' 'spi:/dev/mtd0ro'; do
 name=${spec%%:*}; device=${spec#*:}
 echo "Saving $name recovery image"
 dd if="$device" bs=1048576 2> "$backup/$name.read.log" | gzip -1 > "$backup/$name.img.gz.partial"
 mv "$backup/$name.img.gz.partial" "$backup/$name.img.gz"
 gzip -t "$backup/$name.img.gz"
 sha256sum "$backup/$name.img.gz" >> "$backup/images.sha256"
done
tar --exclude='./transit-recovery' -czf "$backup/usb-files.tar.gz" -C /init/efad/mp_sd .
sha256sum "$backup/usb-files.tar.gz" >> "$backup/images.sha256"
printf 'Root=/dev/sda1; root-card=/dev/sda; data-card=/dev/sdb; SPI=/dev/mtd0ro\nBoth internal filesystems read-only during imaging. USB files not frozen.\nImages contain private keys/settings; keep private.\n' > "$backup/README.private.txt"
sync
echo 'Images saved and gzip integrity checked; resuming app.'
REMOTE
ssh -o BatchMode=yes "$PM3_TARGET" "tar -cf - -C /init/efad/mp_sd/transit-recovery/$PM3_BACKUP_ID ." | tar -xf - -C "$PM3_LOCAL"
python3 "$PM3_SOURCE_DIR/verify-recovery.py" "$PM3_LOCAL"
chmod -R go-rwx "$PM3_LOCAL"
echo "Private recovery copies: $PM3_LOCAL"
echo 'Imaging/integrity checks are not a physical restore test.'
