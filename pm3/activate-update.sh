#!/bin/sh
# On-device activation; caller holds maintenance.lock while staging and activation run.
set -eu
cd /init/transit-prototype
PM3_DURATION=${1:-3600}
case "$PM3_DURATION" in ''|*[!0-9]*) exit 1;; esac
[ "$PM3_DURATION" -ge 1 ] && [ "$PM3_DURATION" -le 86400 ]
test -d maintenance.lock
test -d update-incoming
managed='prototype restart-display.sh start-display.sh transit.local.json display.local.json api.env qr.local.json actions.local.json'
for name in prototype restart-display.sh start-display.sh; do
  test -f "update-incoming/$name" && test ! -L "update-incoming/$name"
  chmod 700 "update-incoming/$name"
done
if [ -e update-incoming/remove ]; then
  test -f update-incoming/remove && test ! -L update-incoming/remove
  while IFS= read -r name; do
    case "$name" in qr.local.json|actions.local.json) ;;
      *) echo 'Invalid optional-removal manifest.' >&2; exit 1;;
    esac
    test ! -e "update-incoming/$name"
  done < update-incoming/remove
fi
# Fill absent configuration from the working version, without changing live files.
for name in transit.local.json display.local.json api.env qr.local.json actions.local.json; do
  test ! -L "$name"
  if [ -f update-incoming/remove ] && grep -qx "$name" update-incoming/remove; then continue; fi
  if [ ! -e "update-incoming/$name" ] && [ -f "$name" ]; then
    cp -p "$name" "update-incoming/$name"
  fi
  test ! -L "update-incoming/$name"
done
set -- --check-config --transit-file update-incoming/transit.local.json \
  --display-file update-incoming/display.local.json --env-file update-incoming/api.env
if [ -f update-incoming/actions.local.json ]; then set -- "$@" --actions-file update-incoming/actions.local.json; fi
if [ -f update-incoming/qr.local.json ]; then set -- "$@" --qr-file update-incoming/qr.local.json; fi
update-incoming/prototype "$@"
umask 077
mkdir -p deployments
snapshot="deployments/$(date -u +%Y%m%dT%H%M%SZ)-$$"
mkdir "$snapshot"
printf "PM3-deployment-1\n" > "$snapshot/format"
: > "$snapshot/present"
: > "$snapshot/absent"
# Finish the rollback copy BEFORE stopping the app or swapping any files.
for name in $managed; do
  test ! -L "$name"
  if [ -f "$name" ]; then
    cp -p "$name" "$snapshot/$name"
    printf '%s\n' "$name" >> "$snapshot/present"
  else
    printf '%s\n' "$name" >> "$snapshot/absent"
  fi
done
committed=0
stop_units() {
  for unit in transit-display.service transit-prototype.service; do
    if systemctl show "$unit" --property=LoadState --value 2>/dev/null | grep -q '^loaded$'; then
      systemctl stop "$unit"
    fi
  done
}
rollback() {
  trap - EXIT HUP INT TERM
  if [ "$committed" != 1 ]; then
    echo 'Activation failed; restoring the prior compatible app/configuration.' >&2
    stop_units || true
    while IFS= read -r name; do
      cp -p "$snapshot/$name" "$name.restore" && mv "$name.restore" "$name" || return 1
    done < "$snapshot/present"
    while IFS= read -r name; do rm -f "$name"; done < "$snapshot/absent"
    systemctl reset-failed transit-display.service transit-prototype.service 2>/dev/null || true
    ./restart-display.sh "$PM3_DURATION" live || { echo 'Previous app needs manual recovery.' >&2; return 1; }
    echo 'Previous app restored and running.' >&2
  fi
}
trap rollback EXIT
trap 'exit 130' HUP INT TERM
stop_units
for name in $managed; do
  if [ -f "update-incoming/$name" ]; then mv "update-incoming/$name" "$name";
  elif [ -f update-incoming/remove ] && grep -qx "$name" update-incoming/remove; then rm -f "$name"; fi
done
./restart-display.sh "$PM3_DURATION" live
sleep 2
if ! systemctl is-active --quiet transit-display.service && ! systemctl is-active --quiet transit-prototype.service; then
  echo 'Updated service did not remain active.' >&2
  exit 1
fi
printf '%s\n' "$snapshot" > last-good-deployment.local.txt.new
mv last-good-deployment.local.txt.new last-good-deployment.local.txt
committed=1
trap - EXIT HUP INT TERM
sync
echo 'Update active; previous compatible app/configuration retained for rollback.'
