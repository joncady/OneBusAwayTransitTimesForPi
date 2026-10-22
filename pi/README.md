# Raspberry Pi kiosk

The original 480 × 320 browser display runs on the Pi Zero W with the UCTRONICS
3.5-inch HDMI LCD. It uses React/Vite for the frontend and a Python OneBusAway
server. For the newer native reader/photo integration, see the
[project README](../README.md) and [PM3 guide](../pm3/README.md).

Run the commands below from the repository root. Create private `.env` and
`config/transit.json` as described in the [shared configuration guide](../README.md#private-configuration).

## Develop on a computer

1. Put your OneBusAway key in `.env` as `API_KEY=...`.
2. Start the Python API in one terminal with `npm run dev:api` (Python dependencies from `requirements.txt` must be installed).
3. Start the frontend in another terminal with `npm run dev`.

Vite forwards `/api` requests to the Python server, so development and production use the same OneBusAway SDK implementation.

## Private transit configuration

The real `config/transit.json` is ignored by Git because it contains location-specific station and stop information. Create it from `config/transit.example.json`, then replace placeholders such as `<station name>`, `<service label>`, `<bus stop name>`, `<bus route label>`, and the stop ID placeholders with your own values. Keep this private config and `.env` on the development computer and Pi only.

The Python server provides display settings from this private config to the frontend. The JSON structure is validated when the server starts. Add modes with stop IDs, then add matching frontend presentation if the new mode needs different treatment.

## Display schedule and idle behavior

Adjust the tracked `config/display.json` to change the timezone, weekday active hours, idle timeout, default refresh interval, or screen policy check interval. The default keeps the display and arrivals polling active Monday through Friday from 7:00 to 8:00 AM. Outside those hours, a touchscreen tap wakes the display and resumes polling for five minutes after the last interaction. The refresh-rate control remains available on screen.

## Run the production build

1. Run `npm run build` on the development computer.
2. Make sure the Pi has Python 3.7 or later and a `.env` file containing `API_KEY=...`.
3. Copy this folder's `dist/`, `server.py`, `requirements.txt`, and your private `config/transit.json` to the Pi. The deploy script includes the private config automatically.
4. Install the server dependency with `python3 -m pip install --user -r requirements.txt`.
5. From that folder on the Pi, run `python3 server.py`.
6. Open `http://localhost:4173` in Chromium.

For Raspberry Pi OS (Legacy) with the LXDE desktop, `pi/lxsession-autostart` preserves the stock panel and starts the server and kiosk browser after desktop login. The server applies the configured X11 display power policy. The deploy script installs the autostart settings automatically. For manual setup, copy the autostart file to `/home/pi/.config/lxsession/LXDE-pi/autostart` and `pi/start-kiosk.sh` to `/home/pi/sound-transit-display/`, then make the kiosk script executable. This setup starts after desktop login; enable desktop auto-login in Raspberry Pi Configuration if it is not already enabled. The kiosk launcher adds `?kiosk=1` to hide the pointer over the app.

The Pi does not need Node.js or npm to serve the production app. The OneBusAway Python SDK is pinned to version 1.2.4, which supports the Pi's current Python 3.7 runtime. The API key is read from `.env` and stop IDs from the private transit config; neither is sent to the browser. Display settings are returned by the server's config endpoint.

## Deploy to the Raspberry Pi

Run `./pi/deploy.sh` from this project folder. The script builds the frontend, looks for `raspberrypi.local` or a Raspberry Pi address in the local ARP table, uses `PI_PASSWORD` from the local `.env` for non-interactive SSH, installs the Python SDK, copies the app without replacing the Pi's `.env`, updates desktop auto-start, and restarts the server and kiosk. The Pi and computer need to be on the same network, and SSH must be enabled on the Pi. Add `PI_PASSWORD=your_pi_ssh_password` to your local `.env`; do not commit `.env`.

If discovery cannot find it, enter the Pi's hostname or IP when prompted. You can also set `PI_HOST`, `PI_USER`, or `PI_DIR` before running the script, for example `PI_HOST=raspberrypi.local ./pi/deploy.sh`. The defaults are user `pi` and `/home/pi/sound-transit-display`.
