#!/bin/sh
# Called with the shared maintenance lock held, after a new USB release is uploaded.
set -eu
cd /init/transit-prototype
test -d maintenance.lock
test -f actions.local.json.new && test ! -L actions.local.json.new
set -- --check-config --transit-file transit.local.json --display-file display.local.json --env-file api.env --actions-file actions.local.json.new
if [ -f qr.local.json ]; then set -- "$@" --qr-file qr.local.json; fi
./prototype "$@"
cp -p actions.local.json actions.local.json.rollback
changed=0
finished=0
rollback() {
 trap - EXIT HUP INT TERM
 if [ "$changed" = 1 ] && [ "$finished" != 1 ]; then
  cp -p actions.local.json.rollback actions.local.json.restore
  mv actions.local.json.restore actions.local.json
  ./restart-display.sh 3600 live || echo 'Previous album map needs manual restart.' >&2
 fi
}
trap rollback EXIT
trap 'exit 130' HUP INT TERM
changed=1
mv actions.local.json.new actions.local.json
./restart-display.sh 3600 live
sleep 2
if ! systemctl is-active --quiet transit-display.service && ! systemctl is-active --quiet transit-prototype.service; then exit 1; fi
mv actions.local.json.rollback actions.local.json.previous
finished=1
trap - EXIT HUP INT TERM
echo 'Album map published; previous map retained.'
