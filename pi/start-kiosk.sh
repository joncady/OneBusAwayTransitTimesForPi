#!/bin/sh

while ! curl --silent --fail http://127.0.0.1:4173/ >/dev/null; do
  sleep 2
done

exec chromium-browser --kiosk --noerrdialogs --disable-infobars --disable-session-crashed-bubble --no-first-run http://localhost:4173
