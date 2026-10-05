#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PI_USER="${PI_USER:-pi}"
PI_DIR="${PI_DIR:-/home/pi/sound-transit-display}"
PI_HOST="${PI_HOST:-}"
ENV_FILE="$ROOT_DIR/.env"

die() {
  printf 'Deploy failed: %s\n' "$*" >&2
  exit 1
}

for command_name in npm ssh tar python3; do
  command -v "$command_name" >/dev/null 2>&1 || die "Required command not found: $command_name"
done

[[ "$PI_USER" =~ ^[a-zA-Z_][a-zA-Z0-9_-]*$ ]] || die "PI_USER must be a simple SSH username."
[[ "$PI_DIR" =~ ^/[a-zA-Z0-9_./-]+$ ]] || die "PI_DIR must be an absolute path without spaces or shell punctuation."
[[ -f "$ENV_FILE" ]] || die "Missing $ENV_FILE. Add PI_PASSWORD to it for non-interactive SSH."
[[ -f "$ROOT_DIR/config/transit.json" ]] || die "Missing private config/transit.json. Copy config/transit.example.json and fill in your locations first."
PI_PASSWORD="$(python3 - "$ENV_FILE" <<'PY'
import sys
from pathlib import Path

for raw_line in Path(sys.argv[1]).read_text().splitlines():
    line = raw_line.strip()
    if line.startswith("export "):
        line = line[7:].lstrip()
    if not line.startswith("PI_PASSWORD="):
        continue
    value = line.split("=", 1)[1].strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        value = value[1:-1]
    sys.stdout.write(value)
    break
PY
)"
[[ -n "$PI_PASSWORD" ]] || die "Add a non-empty PI_PASSWORD to $ENV_FILE."

discover_pi() {
  local candidate line mac oui
  local -a candidates=()

  candidates+=("raspberrypi.local")
  if command -v arp >/dev/null 2>&1; then
    while IFS= read -r line; do
      mac="$(python3 -c 'import re,sys; m=re.search(r"\(([^)]+)\)\s+at\s+([0-9a-f:]+)", sys.stdin.read(), re.I); print(m.group(1)+" "+m.group(2).lower() if m else "")' <<<"$line")"
      [[ -n "$mac" ]] || continue
      read -r candidate mac <<<"$mac"
      oui="${mac//:/}"
      oui="${oui:0:6}"
      case "$oui" in
        b827eb|dca632|e45f01|d83add|2ccf67|28cdc1|9c5322|4cedfb)
          candidates+=("$candidate")
          ;;
      esac
    done < <(arp -an 2>/dev/null || true)
  fi

  for candidate in "${candidates[@]}"; do
    if nc -z -w 2 "$candidate" 22 >/dev/null 2>&1; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

if [[ -z "$PI_HOST" ]]; then
  if command -v nc >/dev/null 2>&1; then
    PI_HOST="$(discover_pi || true)"
  fi
  if [[ -z "$PI_HOST" ]]; then
    printf 'Could not auto-find raspberrypi.local on this Wi-Fi.\n'
    read -r -p 'Enter the Pi hostname or IP (for example raspberrypi.local): ' PI_HOST
  fi
fi
[[ -n "$PI_HOST" ]] || die "No Pi hostname or IP was provided."
[[ "$PI_HOST" =~ ^[a-zA-Z0-9._:-]+$ ]] || die "PI_HOST must be a hostname or IP address."

TARGET="${PI_USER}@${PI_HOST}"
CONTROL_DIR="$(mktemp -d "${TMPDIR:-/tmp}/train-times-ssh.XXXXXX")"
CONTROL_PATH="$CONTROL_DIR/control"
ASKPASS_HELPER="$CONTROL_DIR/askpass"
cat >"$ASKPASS_HELPER" <<'ASKPASS'
#!/bin/sh
printf '%s\n' "$PI_DEPLOY_SSH_PASSWORD"
ASKPASS
chmod 700 "$ASKPASS_HELPER"
export SSH_ASKPASS="$ASKPASS_HELPER"
export SSH_ASKPASS_REQUIRE=force
export DISPLAY="${DISPLAY:-:0}"
export PI_DEPLOY_SSH_PASSWORD="$PI_PASSWORD"
unset PI_PASSWORD
cleanup() {
  ssh -S "$CONTROL_PATH" -O exit "$TARGET" >/dev/null 2>&1 || true
  rm -rf "$CONTROL_DIR"
}
trap cleanup EXIT

printf 'Building the transit display…\n'
cd "$ROOT_DIR"
if [[ ! -d node_modules ]]; then
  npm ci
fi
npm run build

printf 'Connecting to %s…\n' "$TARGET"
ssh -M -N -f \
  -o ControlPath="$CONTROL_PATH" \
  -o ControlPersist=5m \
  -o ServerAliveInterval=15 \
  -o ServerAliveCountMax=3 \
  -o PreferredAuthentications=password \
  -o PubkeyAuthentication=no \
  -o NumberOfPasswordPrompts=1 \
  -o StrictHostKeyChecking=accept-new \
  "$TARGET" || die "Could not authenticate to $TARGET. Check the username, password, or SSH key."

printf 'Copying the build and server files…\n'
tar -czf - -C "$ROOT_DIR" dist server.py requirements.txt config -C "$ROOT_DIR/pi" start-kiosk.sh |
  ssh -S "$CONTROL_PATH" "$TARGET" \
    "mkdir -p '$PI_DIR' && rm -rf '$PI_DIR/.deploy-staging' && mkdir -p '$PI_DIR/.deploy-staging' && tar -xzf - -C '$PI_DIR/.deploy-staging'" \
    || die "Could not copy deployment files to the Pi."

printf 'Installing the OneBusAway SDK and restarting the kiosk…\n'
ssh -S "$CONTROL_PATH" "$TARGET" "APP_DIR='$PI_DIR' bash -s" <<'REMOTE_DEPLOY'
set -Eeuo pipefail
APP_DIR="$APP_DIR"
STAGING="$APP_DIR/.deploy-staging"

python3 -m pip install --user --disable-pip-version-check -r "$STAGING/requirements.txt"
python3 -c 'import onebusaway, pydantic; print("OneBusAway SDK dependency is ready.")'

rm -rf "$APP_DIR/dist"
mv "$STAGING/dist" "$APP_DIR/dist"
cp "$STAGING/server.py" "$APP_DIR/server.py"
cp "$STAGING/requirements.txt" "$APP_DIR/requirements.txt"
rm -rf "$APP_DIR/config"
mv "$STAGING/config" "$APP_DIR/config"
cp "$STAGING/start-kiosk.sh" "$APP_DIR/start-kiosk.sh"
chmod +x "$APP_DIR/start-kiosk.sh"
rm -rf "$STAGING"

APP_DIR="$APP_DIR" python3 - <<'PY'
import os
from pathlib import Path

app_dir = os.environ["APP_DIR"]
autostart = Path.home() / ".config/lxsession/LXDE-pi/autostart"
autostart.parent.mkdir(parents=True, exist_ok=True)
lines = autostart.read_text().splitlines() if autostart.exists() else [
    "@lxpanel --profile LXDE-pi",
    "@pcmanfm --desktop --profile LXDE-pi",
]
managed = (
    "xset s off -dpms",
    "/disable-screen-blanking.sh",
    "/server.py",
    "/start-kiosk.sh",
)
lines = [line for line in lines if not any(item in line for item in managed)]
lines.extend((
    "@python3 {}/server.py".format(app_dir),
    "@{}/start-kiosk.sh".format(app_dir),
))
autostart.write_text("\n".join(lines) + "\n")
PY

pkill -f "^python3 $APP_DIR/server.py" || true
cd "$APP_DIR"
nohup python3 "$APP_DIR/server.py" >>"$APP_DIR/server.log" 2>&1 </dev/null &

ready=0
for attempt in $(seq 1 20); do
  if curl --silent --fail http://127.0.0.1:4173/ >/dev/null; then
    ready=1
    break
  fi
  sleep 1
done
if [[ "$ready" != 1 ]]; then
  echo "The web server did not start. Recent server log:" >&2
  tail -40 "$APP_DIR/server.log" >&2 || true
  exit 1
fi

pkill -f '^/usr/lib/chromium-browser/chromium-browser .*--kiosk' || true
DISPLAY=:0 XAUTHORITY="$HOME/.Xauthority" nohup "$APP_DIR/start-kiosk.sh" \
  >>/tmp/sound-transit-kiosk.log 2>&1 </dev/null &

echo "Deployment complete. The kiosk will also start automatically after reboot."
echo "API key file was left unchanged: $APP_DIR/.env"
REMOTE_DEPLOY

printf 'Deploy complete.\n'
