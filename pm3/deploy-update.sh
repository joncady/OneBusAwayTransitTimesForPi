#!/bin/sh
# Shared preserving deployment. Caller builds first; optional configs are explicit.
set -eu
PM3_SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PM3_TARGET=${1:?Usage: deploy-update.sh root@DEVICE_HOST [seconds]}
PM3_DURATION=${2:-3600}
case "$PM3_DURATION" in ''|*[!0-9]*) exit 1;; esac
[ "$PM3_DURATION" -ge 1 ] && [ "$PM3_DURATION" -le 86400 ]
PM3_BINARY=${PM3_BINARY:-"$PM3_SOURCE_DIR/target/armv7-unknown-linux-musleabihf/release/pm3-transit-prototype"}
test -f "$PM3_BINARY"
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" '
 set -eu
 if systemctl is-active --quiet nx || systemctl is-active --quiet init-abtproxy; then
   echo "Stock fare software is active; refusing deployment." >&2; exit 1
 fi
 umask 077
 mkdir -p /init/transit-prototype
 cd /init/transit-prototype
 mkdir maintenance.lock || { echo "Another maintenance operation holds the lock." >&2; exit 1; }
'
cleanup() { ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" 'rmdir /init/transit-prototype/maintenance.lock' || true; }
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
ssh -o BatchMode=yes "$PM3_TARGET" '
 set -eu
 cd /init/transit-prototype
 test ! -L update-incoming
 rm -rf update-incoming
 umask 077
 mkdir update-incoming
'
upload() {
 ssh -o BatchMode=yes "$PM3_TARGET" "umask 077; cat > /init/transit-prototype/update-incoming/$2" < "$1"
}
upload "$PM3_BINARY" prototype
for PM3_FILE in start-display.sh restart-display.sh; do upload "$PM3_SOURCE_DIR/$PM3_FILE" "$PM3_FILE"; done
if [ -n "${PM3_TRANSIT_DIR:-}" ]; then
 upload "$PM3_TRANSIT_DIR/transit.json" transit.local.json
 upload "$PM3_TRANSIT_DIR/display.json" display.local.json
 upload "$PM3_TRANSIT_DIR/api.env" api.env
fi
if [ -n "${PM3_DISPLAY_CONFIG:-}" ]; then upload "$PM3_DISPLAY_CONFIG" display.local.json; fi
if [ -n "${PM3_QR_CONFIG:-}" ]; then upload "$PM3_QR_CONFIG" qr.local.json; fi
if [ -n "${PM3_ACTIONS_CONFIG:-}" ]; then upload "$PM3_ACTIONS_CONFIG" actions.local.json; fi
ssh -o BatchMode=yes "$PM3_TARGET" "sh -s -- $PM3_DURATION" < "$PM3_SOURCE_DIR/activate-update.sh"
# Release the publication lock before retention takes the same lock.
cleanup
trap - EXIT HUP INT TERM
python3 "$PM3_SOURCE_DIR/maintain-device.py" "$PM3_TARGET" --apply --keep-releases "${PM3_KEEP_RELEASES:-3}" || \
  echo 'Update succeeded; retention cleanup needs a retry.' >&2
