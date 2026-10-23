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
export PM3_QR_CONFIG
"$PM3_SOURCE_DIR/deploy-update.sh" "$PM3_TARGET" "$PM3_DURATION"
echo 'QR-enabled app launched; existing transit, media and card settings preserved.'
