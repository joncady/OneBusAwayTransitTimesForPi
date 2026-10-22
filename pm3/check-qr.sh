#!/bin/sh
# Temporary QR trial. Restores UART/power settings; never prints scanned contents.
set -eu
PM3_TARGET=${1:?Usage: check-qr.sh root@DEVICE_HOST}
ssh -o BatchMode=yes "$PM3_TARGET" 'sh -s' <<'REMOTE'
set -eu
if systemctl is-active --quiet nx || systemctl is-active --quiet init-abtproxy; then
  echo 'Stock fare software is active; stop and review first.' >&2; exit 1
fi
umask 077
trial=$(mktemp -d /init/transit-prototype/qr-check.XXXXXX)
original=$(stty -F /dev/ttyUSB0 -g)
changed=0
reader=''
cleanup() {
  trap - EXIT HUP INT TERM
  if [ -n "$reader" ]; then kill "$reader" 2>/dev/null || true; wait "$reader" 2>/dev/null || true; fi
  stty -F /dev/ttyUSB0 "$original" || true
  if [ "$changed" = 1 ] && NxExe prox2 portinfo | grep -q '^barcode:[[:space:]]*1$'; then
    NxExe prox2 barcode
  fi
  rm -f "$trial/scan"
  rmdir "$trial"
  echo 'QR trial finished; UART settings and scanner power restored.'
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
state=$(NxExe prox2 portinfo | sed -n 's/^barcode:[[:space:]]*\([01]\)$/\1/p')
case "$state" in
  0) changed=1; NxExe prox2 barcode;;
  1) ;;
  *) echo 'Unknown scanner power state; not toggling.' >&2; exit 1;;
esac
stty -F /dev/ttyUSB0 raw -echo 115200 cs8 -cstopb -parenb -ixon -ixoff -crtscts clocal cread min 1 time 0
cat /dev/ttyUSB0 > "$trial/scan" &
reader=$!
echo 'Scanner listening for PM3-QR-CHECK for up to 180 seconds. Present the test QR at the code-reader window.'
seconds=0
last=0
while [ "$seconds" -lt 180 ]; do
  if grep -q 'PM3-QR-CHECK' "$trial/scan"; then
    echo 'SUCCESS: the scanner decoded the expected test QR.'
    exit 0
  fi
  count=$(wc -c < "$trial/scan")
  if [ "$count" -gt "$last" ]; then
    echo "Scanner received $count bytes; content omitted (waiting for test QR)."
    last=$count
  fi
  if [ "$count" -gt 16384 ]; then echo 'Scan size limit reached.' >&2; exit 1; fi
  sleep 1
  seconds=$((seconds + 1))
done
echo 'No matching test QR was received before timeout.'
exit 1
REMOTE
