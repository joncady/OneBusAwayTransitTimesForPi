#!/bin/sh
# Restore the last known compatible app/config snapshot through normal activation.
set -eu
PM3_SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PM3_TARGET=${1:?Usage: rollback-update.sh root@DEVICE_HOST}
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" 'umask 077; mkdir /init/transit-prototype/maintenance.lock'
cleanup() { ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" 'rmdir /init/transit-prototype/maintenance.lock' || true; }
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
ssh -o BatchMode=yes "$PM3_TARGET" 'sh -s' <<'REMOTE'
set -eu
cd /init/transit-prototype
snapshot=$(cat last-good-deployment.local.txt)
# Only app-owned direct child snapshots, never arbitrary paths.
case "$snapshot" in deployments/*) ;;
 *) exit 1;;
esac
name=${snapshot#deployments/}
case "$name" in ''|*[!0-9TZ-]*) exit 1;; esac
test -d "$snapshot" && test ! -L deployments && test ! -L "$snapshot"
grep -qx 'PM3-deployment-1' "$snapshot/format"
test ! -L update-incoming
rm -rf update-incoming
umask 077
mkdir update-incoming
while IFS= read -r file; do
 case "$file" in prototype|restart-display.sh|start-display.sh|transit.local.json|display.local.json|api.env|qr.local.json|actions.local.json) ;;
  *) exit 1;;
 esac
 test -f "$snapshot/$file" && test ! -L "$snapshot/$file"
 cp -p "$snapshot/$file" "update-incoming/$file"
done < "$snapshot/present"
cp "$snapshot/absent" update-incoming/remove
REMOTE
ssh -o BatchMode=yes "$PM3_TARGET" 'sh -s -- 3600' < "$PM3_SOURCE_DIR/activate-update.sh"
cleanup
trap - EXIT HUP INT TERM
python3 "$PM3_SOURCE_DIR/maintain-device.py" "$PM3_TARGET" --apply --keep-releases "${PM3_KEEP_RELEASES:-3}" || \
  echo 'Rollback succeeded; retention cleanup needs a retry.' >&2
