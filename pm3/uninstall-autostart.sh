#!/bin/sh
# Disable/remove only our boot service. Keep app, private files and USB bootstrap.
set -eu
PM3_TARGET=${1:?Usage: uninstall-autostart.sh root@DEVICE_HOST}
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" 'bash -s' <<'REMOTE'
set -euo pipefail
cd /init/transit-prototype
test -f /etc/systemd/system/transit-display.service
root_ro=0
if grep -q ' / ext4 ro,' /proc/mounts; then root_ro=1; fi
cleanup() {
 trap - EXIT HUP INT TERM
 sync
 if [ "$root_ro" = 1 ]; then mount -o remount,ro / || true; fi
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
if [ "$root_ro" = 1 ]; then mount -o remount,rw /; fi
systemctl disable --now transit-display.service
rm -f /etc/systemd/system/transit-display.service autostart.enabled
if [ -f abtproxy.before-autostart.local.txt ] && grep -qx enabled abtproxy.before-autostart.local.txt; then
  systemctl enable init-abtproxy.service
fi
systemctl daemon-reload
systemctl reset-failed transit-display.service 2>/dev/null || true
echo 'Automatic startup removed. App/private files/bootstrap preserved; no fare service started.'
echo 'Use restart-display.sh for a manual bounded session.'
REMOTE
