#!/bin/sh
# Install first persistent boot service. Full recovery images are a prerequisite.
set -eu
PM3_SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PM3_TARGET=${1:?Usage: install-autostart.sh root@DEVICE_HOST [--skip-build]}
case "${2:-}" in
  '') "$PM3_SOURCE_DIR/build.sh";;
  --skip-build) ;;
  *) echo 'Unknown option.' >&2; exit 1;;
esac
PM3_BINARY=${PM3_BINARY:-"$PM3_SOURCE_DIR/target/armv7-unknown-linux-musleabihf/release/pm3-transit-prototype"}
ssh -o BatchMode=yes -o ConnectTimeout=8 "$PM3_TARGET" '
  set -eu
  test ! -e /etc/systemd/system/transit-display.service || { echo "Already installed; use normal app deployment helpers." >&2; exit 1; }
  if systemctl is-active --quiet nx || systemctl is-active --quiet init-abtproxy; then
    echo "Stock fare software is active; stop and review first." >&2; exit 1
  fi
  test -s /init/efad/mp_sd/autorun.sh
  test -d /init/transit-prototype
  umask 077
  mkdir -p /init/transit-prototype/autostart-incoming
'
for PM3_FILE in start-display.sh restart-display.sh transit-display.service; do
  ssh -o BatchMode=yes "$PM3_TARGET" "umask 077; cat > /init/transit-prototype/autostart-incoming/$PM3_FILE" < "$PM3_SOURCE_DIR/$PM3_FILE"
done
ssh -o BatchMode=yes "$PM3_TARGET" 'umask 077; cat > /init/transit-prototype/autostart-incoming/prototype' < "$PM3_BINARY"
ssh -o BatchMode=yes "$PM3_TARGET" 'bash -s' <<'REMOTE'
set -euo pipefail
cd /init/transit-prototype
chmod 700 autostart-incoming/prototype autostart-incoming/start-display.sh autostart-incoming/restart-display.sh
# Validate the incoming executable against existing private settings before stopping anything.
autostart-incoming/prototype --help | grep -q -- '--continuous'
set -- --check-config --transit-file transit.local.json --display-file display.local.json --env-file api.env
if [ -f qr.local.json ]; then set -- "$@" --qr-file qr.local.json --actions-file actions.local.json; fi
autostart-incoming/prototype "$@"
umask 077
cp -p prototype prototype.before-autostart
cp -p restart-display.sh restart-display.before-autostart.sh
systemctl is-enabled nx.service > nx.before-autostart.local.txt || true
systemctl is-enabled init-abtproxy.service > abtproxy.before-autostart.local.txt || true
root_ro=0
if grep -q ' / ext4 ro,' /proc/mounts; then root_ro=1; fi
installed=0
cleanup() {
 trap - EXIT HUP INT TERM
 if [ "$installed" != 1 ]; then
   echo 'Installation did not complete; restoring the previous bounded app.' >&2
   systemctl stop transit-display.service 2>/dev/null || true
   systemctl disable transit-display.service 2>/dev/null || true
   rm -f /etc/systemd/system/transit-display.service autostart.enabled
   cp -p prototype.before-autostart prototype.restore
   mv prototype.restore prototype
   cp -p restart-display.before-autostart.sh restart-display.sh
   if grep -qx enabled abtproxy.before-autostart.local.txt; then systemctl enable init-abtproxy.service || true; fi
   systemctl daemon-reload
   ./restart-display.sh 3600 live || true
 fi
 sync
 if [ "$root_ro" = 1 ]; then mount -o remount,ro / || true; fi
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
if [ "$root_ro" = 1 ]; then mount -o remount,rw /; fi
# Stop the bounded session; stock software and USB bootstrap files are unchanged.
if systemctl show transit-prototype.service --property=LoadState --value 2>/dev/null | grep -q '^loaded$'; then
  systemctl stop transit-prototype.service
fi
mv autostart-incoming/prototype prototype
mv autostart-incoming/start-display.sh start-display.sh
mv autostart-incoming/restart-display.sh restart-display.sh
install -m 644 autostart-incoming/transit-display.service /etc/systemd/system/transit-display.service
# Preserve the former enabled state for uninstall. Avoid the proxy competing for boot.
systemctl disable init-abtproxy.service
touch autostart.enabled
systemctl daemon-reload
systemd-analyze verify /etc/systemd/system/transit-display.service
systemctl enable --now transit-display.service
sleep 2
systemctl is-active --quiet transit-display.service
systemctl show transit-display.service --property=MainPID --value > prototype.pid
installed=1
sync
echo 'Automatic startup installed and running; boot/ignition still need physical verification.'
REMOTE
