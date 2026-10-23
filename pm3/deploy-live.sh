#!/bin/sh
# Bounded live integration trial; preserves USB bootstrap and card/photo settings.
set -eu
PM3_SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PM3_TARGET=${1:?Usage: deploy-live.sh root@DEVICE_HOST [seconds] [--skip-build]}
PM3_DURATION=${2:-3600}
case "$PM3_DURATION" in ''|*[!0-9]*) echo 'Duration must be numeric.' >&2; exit 1;; esac
[ "$PM3_DURATION" -ge 1 ] && [ "$PM3_DURATION" -le 86400 ] || exit 1
case "${3:-}" in
  '') "$PM3_SOURCE_DIR/build.sh" --skip-tests;;
  --skip-build) ;;
  *) echo 'Unknown option.' >&2; exit 1;;
esac
python3 - "$PM3_SOURCE_DIR" <<'PY'
import json, os, pathlib, shutil, sys
source = pathlib.Path(sys.argv[1])
runtime = source / 'runtime.local'
runtime.mkdir(mode=0o700, exist_ok=True)
os.chmod(runtime, 0o700)
shutil.copyfile(source.parent / 'config/transit.json', runtime / 'transit.json')
if not (runtime / 'display.json').exists():
    shutil.copyfile(source / 'display.example.json', runtime / 'display.json')
# Only copy API_KEY; other .env credentials never go to the PM3.
key = os.environ.get('API_KEY', '')
if not key:
    for line in (source.parent / '.env').read_text().splitlines():
        line = line.strip()
        if line.startswith('export '): line = line[7:].lstrip()
        name, separator, value = line.partition('=')
        if separator and name.strip() == 'API_KEY':
            key = value.strip().strip('"\'')
            break
if not key or len(key) > 512 or not all(33 <= ord(c) <= 126 for c in key):
    raise SystemExit('Missing or invalid API_KEY in local .env')
(runtime / 'api.env').write_text('API_KEY=' + key + '\n')
for path in runtime.iterdir():
    if path.is_file(): os.chmod(path, 0o600)
json.loads((runtime / 'transit.json').read_text())
json.loads((runtime / 'display.json').read_text())
PY
ssh -o BatchMode=yes "$PM3_TARGET" '
  if systemctl is-active --quiet nx || systemctl is-active --quiet init-abtproxy; then
    echo "Stock fare software is running; stop and review first." >&2; exit 1
  fi
  umask 077
  mkdir -p /init/transit-prototype/incoming
'
for PM3_FILE in transit.json display.json api.env; do
  ssh -o BatchMode=yes "$PM3_TARGET" "umask 077; cat > /init/transit-prototype/incoming/$PM3_FILE" < "$PM3_SOURCE_DIR/runtime.local/$PM3_FILE"
done
ssh -o BatchMode=yes "$PM3_TARGET" 'umask 077; cat > /init/transit-prototype/incoming/validator' < "$PM3_SOURCE_DIR/target/armv7-unknown-linux-musleabihf/release/pm3-transit-prototype"
ssh -o BatchMode=yes "$PM3_TARGET" '
  chmod 700 /init/transit-prototype/incoming/validator
  /init/transit-prototype/incoming/validator --check-config \
    --transit-file /init/transit-prototype/incoming/transit.json \
    --display-file /init/transit-prototype/incoming/display.json \
    --env-file /init/transit-prototype/incoming/api.env
'
ssh -o BatchMode=yes "$PM3_TARGET" '
  set -eu
  for unit in transit-display.service transit-prototype.service; do
    if systemctl show "$unit" --property=LoadState --value 2>/dev/null | grep -q "^loaded$"; then
      systemctl stop "$unit"
    fi
  done
  cd /init/transit-prototype
  umask 077
  for name in transit.local.json display.local.json api.env; do
    if [ -f "$name" ]; then cp -p "$name" "$name.previous"; fi
  done
  mv incoming/transit.json transit.local.json
  mv incoming/display.json display.local.json
  mv incoming/api.env api.env
  chmod 600 transit.local.json display.local.json api.env
'
"$PM3_SOURCE_DIR/run-prototype.sh" "$PM3_TARGET" "$PM3_DURATION" live
