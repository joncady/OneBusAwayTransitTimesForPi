#!/bin/sh
# Opt-in QR app/config deployment. Preserves current transit, API, NFC and media files.
set -eu
PM3_SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PM3_TARGET=${1:?Usage: deploy-qr.sh root@DEVICE_HOST [seconds] [--skip-build]}
PM3_DURATION=${2:-3600}
case "$PM3_DURATION" in ''|*[!0-9]*) echo 'Duration must be numeric.' >&2; exit 1;; esac
[ "$PM3_DURATION" -ge 1 ] && [ "$PM3_DURATION" -le 86400 ] || exit 1
PM3_QR_CONFIG=${PM3_QR_CONFIG:-"$PM3_SOURCE_DIR/qr.local.json"}
python3 - "$PM3_SOURCE_DIR" "$PM3_QR_CONFIG" <<'PY'
import sys
sys.path.insert(0, sys.argv[1])
from qr_config import load
load(sys.argv[2])
print('Private QR configuration valid')
PY
case "${3:-}" in
  '') "$PM3_SOURCE_DIR/build.sh";;
  --skip-build) ;;
  *) echo 'Unknown option.' >&2; exit 1;;
esac
PM3_BINARY="$PM3_SOURCE_DIR/target/armv7-unknown-linux-musleabihf/release/pm3-transit-prototype"
test -f "$PM3_BINARY" || { echo 'Build the app first.' >&2; exit 1; }
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" '
  set -eu
  if systemctl is-active --quiet nx || systemctl is-active --quiet init-abtproxy; then
    echo "Stock fare software is active; stop and review first." >&2; exit 1
  fi
  test -d /init/transit-prototype
  umask 077
  mkdir -p /init/transit-prototype/qr-incoming
'
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" 'umask 077; cat > /init/transit-prototype/qr-incoming/prototype' < "$PM3_BINARY"
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" 'umask 077; cat > /init/transit-prototype/qr-incoming/qr.local.json' < "$PM3_QR_CONFIG"
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" 'umask 077; cat > /init/transit-prototype/qr-incoming/restart-display.sh' < "$PM3_SOURCE_DIR/restart-display.sh"
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" 'umask 077; cat > /init/transit-prototype/qr-incoming/start-display.sh' < "$PM3_SOURCE_DIR/start-display.sh"
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" "sh -s -- $PM3_DURATION" <<'REMOTE'
set -eu
cd /init/transit-prototype
chmod 700 qr-incoming/prototype qr-incoming/restart-display.sh qr-incoming/start-display.sh
# Pure configuration validation before stopping the working app: no UART/audio access.
qr-incoming/prototype --check-config --qr-file qr-incoming/qr.local.json \
  --actions-file actions.local.json --transit-file transit.local.json \
  --display-file display.local.json --env-file api.env
for unit in transit-display.service transit-prototype.service; do
  if systemctl show "$unit" --property=LoadState --value 2>/dev/null | grep -q '^loaded$'; then
    systemctl stop "$unit"
  fi
done
umask 077
cp -p prototype prototype.before-qr
cp -p restart-display.sh restart-display.before-qr.sh
had_start=0
if [ -f start-display.sh ]; then
  cp -p start-display.sh start-display.before-qr.sh
  had_start=1
fi
had_qr=0
if [ -f qr.local.json ]; then
  cp -p qr.local.json qr.before-qr.local.json
  had_qr=1
fi
mv qr-incoming/prototype prototype
mv qr-incoming/restart-display.sh restart-display.sh
mv qr-incoming/start-display.sh start-display.sh
mv qr-incoming/qr.local.json qr.local.json
if ! ./restart-display.sh "$1" live; then
  echo 'QR startup failed; attempting to restore the previous app and QR settings.' >&2
  systemctl stop transit-display.service transit-prototype.service 2>/dev/null || true
  cp -p prototype.before-qr prototype.restore
  mv prototype.restore prototype
  cp -p restart-display.before-qr.sh restart-display.sh
  if [ "$had_start" = 1 ]; then
    cp -p start-display.before-qr.sh start-display.sh
  else
    rm -f start-display.sh
  fi
  if [ "$had_qr" = 1 ]; then
    cp -p qr.before-qr.local.json qr.local.json
  else
    rm -f qr.local.json
  fi
  ./restart-display.sh "$1" live || echo 'Previous app needs manual recovery.' >&2
  exit 1
fi
REMOTE
echo 'QR-enabled app launched; existing transit, media and card settings preserved.'
echo 'Physical scanner/label verification is still required. See pm3/README.md.'
