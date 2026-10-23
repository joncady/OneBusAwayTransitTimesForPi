#!/bin/sh
# Continuous live mode for transit-display.service. No power or watchdog changes.
set -eu
PM3_HOME=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if systemctl is-active --quiet nx || systemctl is-active --quiet init-abtproxy; then
  echo 'Stock fare software is active; refusing to share the reader/display.' >&2
  exit 1
fi
set -- --continuous --enable-card-detection --reader-lights \
  --cards-file "$PM3_HOME/cards.local.json" \
  --actions-file "$PM3_HOME/actions.local.json" \
  --reader-info-file "$PM3_HOME/reader-info.local.bin" \
  --transit-file "$PM3_HOME/transit.local.json" \
  --display-file "$PM3_HOME/display.local.json" --env-file "$PM3_HOME/api.env"
# QR is opt-in; keep its album tokens in private configuration.
# Remove reader sound without changing the independent WAV/audio settings.
if [ ! -f "$PM3_HOME/buzzer.disabled" ]; then
  set -- "$@" --reader-buzzer
fi
if [ -f "$PM3_HOME/qr.local.json" ]; then
  set -- "$@" --qr-file "$PM3_HOME/qr.local.json"
fi
exec "$PM3_HOME/prototype" "$@"
