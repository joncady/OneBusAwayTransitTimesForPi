#!/bin/sh
# Run on the PM3. Recreates a transient unit even after systemd collected it.
# Normal display/card mode only; does not enable any module maintenance trial.
set -eu
PM3_DURATION=${1:-3600}
PM3_MODE=${2:-live}
case "$PM3_DURATION" in ''|*[!0-9]*) echo 'Duration must be numeric.' >&2; exit 1;; esac
[ "$PM3_DURATION" -ge 1 ] && [ "$PM3_DURATION" -le 86400 ] || exit 1
PM3_HOME=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if systemctl is-active --quiet nx || systemctl is-active --quiet init-abtproxy; then
  echo 'Stock fare software is active; stop and review first.' >&2
  exit 1
fi
set -- --seconds "$PM3_DURATION" --enable-card-detection \
  --reader-lights \
  --cards-file "$PM3_HOME/cards.local.json" \
  --actions-file "$PM3_HOME/actions.local.json" \
  --reader-info-file "$PM3_HOME/reader-info.local.bin"
# Remove reader sound without changing the independent WAV/audio settings.
if [ ! -f "$PM3_HOME/buzzer.disabled" ]; then
  set -- "$@" --reader-buzzer
fi
if [ -f "$PM3_HOME/qr.local.json" ]; then
  set -- "$@" --qr-file "$PM3_HOME/qr.local.json"
fi
case "$PM3_MODE" in
  live)
    set -- "$@" --transit-file "$PM3_HOME/transit.local.json" \
      --display-file "$PM3_HOME/display.local.json" --env-file "$PM3_HOME/api.env"
    "$PM3_HOME/prototype" --check-config "$@"
    ;;
  sample) ;;
  *) echo 'Mode must be sample or live.' >&2; exit 1;;
esac
if systemctl show transit-prototype.service --property=LoadState --value 2>/dev/null | grep -q '^loaded$'; then
  systemctl stop transit-prototype.service
fi
umask 077
if [ "$PM3_MODE" = live ] && [ -f "$PM3_HOME/autostart.enabled" ] &&
   systemctl show transit-display.service --property=LoadState --value 2>/dev/null | grep -q '^loaded$'; then
  systemctl restart transit-display.service
  systemctl show transit-display.service --property=MainPID --value > "$PM3_HOME/prototype.pid"
  sleep 1
  systemctl is-active --quiet transit-display.service
  echo 'Persistent live service restarted; duration applies only to transient sessions.'
  exit 0
fi
# Sample sessions temporarily replace the persistent display until live is resumed.
if systemctl is-active --quiet transit-display.service; then
  systemctl stop transit-display.service
fi
systemd-run --unit=transit-prototype --collect \
  --property="RuntimeMaxSec=$((PM3_DURATION + 10))" \
  --property="StandardOutput=append:$PM3_HOME/prototype.log" \
  --property=StandardError=inherit "$PM3_HOME/prototype" "$@"
systemctl show transit-prototype.service --property=MainPID --value > "$PM3_HOME/prototype.pid"
sleep 1
systemctl is-active --quiet transit-prototype.service
