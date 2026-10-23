#!/bin/sh
# Manual, time-limited launch only. This does not install a boot service.
set -eu
PM3_SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PM3_TARGET=${1:?Usage: run-prototype.sh root@DEVICE_HOST [seconds] [sample|live]}
PM3_DURATION=${2:-600}
PM3_LIVE_ARGS=''
case "${3:-sample}" in
  sample) ;;
  live) PM3_LIVE_ARGS='--transit-file /init/transit-prototype/transit.local.json --display-file /init/transit-prototype/display.local.json --env-file /init/transit-prototype/api.env';;
  *) echo 'Mode must be sample or live.' >&2; exit 1;;
esac
case "$PM3_DURATION" in ''|*[!0-9]*) echo 'Duration must be numeric.' >&2; exit 1;; esac
if [ "$PM3_DURATION" -lt 1 ] || [ "$PM3_DURATION" -gt 86400 ]; then
  echo 'Duration must be between 1 and 86400 seconds.' >&2
  exit 1
fi
PM3_BINARY="$PM3_SOURCE_DIR/target/armv7-unknown-linux-musleabihf/release/pm3-transit-prototype"
test -f "$PM3_BINARY" || { echo 'Run pm3/build.sh first.' >&2; exit 1; }
ssh -o BatchMode=yes "$PM3_TARGET" '
  if systemctl is-active --quiet nx || systemctl is-active --quiet init-abtproxy; then
    echo "Stock fare software is running; stop and review before using this prototype." >&2
    exit 1
  fi
  if systemctl is-active --quiet transit-prototype.service || systemctl is-active --quiet transit-display.service; then
    echo "Prototype service is already running." >&2
    exit 1
  fi
  if [ -e /init/transit-prototype/prototype.pid ]; then
    pid=$(cat /init/transit-prototype/prototype.pid)
    case "$pid" in ""|*[!0-9]*) exit 1;; esac
    if [ -e "/proc/$pid" ]; then
      echo "Previous prototype PID is still present; stop it before deploying." >&2
      exit 1
    fi
  fi
  mkdir -p /init/transit-prototype
'
# Use an SSH stream because the device does not provide a modern SFTP server.
ssh -o BatchMode=yes "$PM3_TARGET" 'cat > /init/transit-prototype/prototype.new' < "$PM3_BINARY"
ssh -o BatchMode=yes "$PM3_TARGET" 'cat > /init/transit-prototype/restart-display.sh.new' < "$PM3_SOURCE_DIR/restart-display.sh"
ssh -o BatchMode=yes "$PM3_TARGET" "
  chmod 755 /init/transit-prototype/prototype.new
  if [ -n '$PM3_LIVE_ARGS' ]; then
    /init/transit-prototype/prototype.new --check-config $PM3_LIVE_ARGS || exit 1
  fi
  if [ -f /init/transit-prototype/prototype ]; then
    cp /init/transit-prototype/prototype /init/transit-prototype/prototype.previous
  fi
  mv /init/transit-prototype/prototype.new /init/transit-prototype/prototype
  chmod 700 /init/transit-prototype/restart-display.sh.new
  mv /init/transit-prototype/restart-display.sh.new /init/transit-prototype/restart-display.sh
  /init/transit-prototype/restart-display.sh $PM3_DURATION ${3:-sample}
"
ssh -o BatchMode=yes "$PM3_TARGET" '
  sleep 1
  if ! systemctl is-active --quiet transit-prototype.service && ! systemctl is-active --quiet transit-display.service; then
    journalctl -u transit-prototype.service -u transit-display.service -n 30 --no-pager >&2
    exit 1
  fi
'
echo "Prototype launched for $PM3_DURATION seconds. Logs: journalctl -u transit-prototype.service -u transit-display.service"
