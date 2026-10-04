#!/usr/bin/env python3
"""Serve the transit display and proxy OneBusAway arrivals."""

from concurrent.futures import ThreadPoolExecutor
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import time
from urllib.parse import parse_qs, unquote, urlparse

from onebusaway import APIError, OnebusawaySDK


ROOT = Path(__file__).resolve().parent
DIST = (ROOT / "dist").resolve()


def validate_config(config):
    """Validate the small config contract without adding a runtime dependency."""
    if not isinstance(config, dict):
        raise ValueError("transit config must be a JSON object")
    for key, minimum, maximum in (("arrivalWindowMinutes", 1, 360), ("maxArrivals", 1, 50)):
        value = config.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or not minimum <= value <= maximum:
            raise ValueError("transit config {} must be an integer from {} to {}".format(key, minimum, maximum))
    modes = config.get("modes")
    if not isinstance(modes, dict) or not modes:
        raise ValueError("transit config modes must be a non-empty object")
    for mode_name, mode in modes.items():
        if not isinstance(mode_name, str) or not re.match(r"^[a-z][a-z0-9_-]*$", mode_name):
            raise ValueError("transit config contains an invalid mode name")
        if not isinstance(mode, dict):
            raise ValueError("transit mode {} must be an object".format(mode_name))
        for field in ("name", "serviceLabel", "plural"):
            if not isinstance(mode.get(field), str) or not mode[field].strip():
                raise ValueError("transit mode {} requires a non-empty {}".format(mode_name, field))
        stops = mode.get("stops")
        if not isinstance(stops, list) or not stops:
            raise ValueError("transit mode {} requires at least one stop".format(mode_name))
        for stop in stops:
            if not isinstance(stop, dict):
                raise ValueError("stops in mode {} must be objects".format(mode_name))
            if not isinstance(stop.get("id"), str) or not stop["id"].strip():
                raise ValueError("stops in mode {} require a non-empty id".format(mode_name))
            if stop["id"].strip().startswith("<") and stop["id"].strip().endswith(">"):
                raise ValueError("replace the stop ID placeholder in mode {}".format(mode_name))
            if not isinstance(stop.get("direction", ""), str):
                raise ValueError("stop direction in mode {} must be a string".format(mode_name))
    return config


try:
    CONFIG = validate_config(json.loads((ROOT / "config" / "transit.json").read_text(encoding="utf-8")))
except (OSError, json.JSONDecodeError, ValueError) as error:
    raise SystemExit("Invalid config/transit.json: {}".format(error))


def read_env_values():
    values = dict(os.environ)
    env_file = ROOT / ".env"
    if env_file.exists():
        for raw_line in env_file.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if line.startswith("export "):
                line = line[7:].lstrip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            key, value = line.split("=", 1)
            if key.strip() not in values:
                values[key.strip()] = value.strip().strip("\"'")
    return values


def read_api_key():
    return read_env_values().get("API_KEY", "")


def get_stop_arrivals(client, stop, now):
    window = CONFIG["arrivalWindowMinutes"]
    payload = client.arrival_and_departure.list(
        stop["id"], minutes_before=0, minutes_after=window, timeout=12
    )
    if payload.code != 200:
        raise RuntimeError(payload.text or "OneBusAway could not load arrivals.")
    results = []
    for arrival in payload.data.entry.arrivals_and_departures:
        timestamp = arrival.predicted_arrival_time or arrival.scheduled_arrival_time or 0
        if timestamp < now - 60000 or timestamp > now + window * 60000:
            continue
        results.append({
            "id": "{}-{}-{}".format(arrival.trip_id or "trip", stop["id"], arrival.stop_sequence or 0),
            "route": arrival.route_short_name or "",
            "destination": arrival.trip_headsign or "Transit arrival",
            "direction": stop.get("direction", ""),
            "minutes": max(0, (timestamp - now + 59999) // 60000),
            "predicted": bool(arrival.predicted and arrival.predicted_arrival_time),
            "timestamp": timestamp,
        })
    return results


class DisplayHandler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(DIST), **kwargs)

    def log_message(self, format_string, *args):
        print("{} - {}".format(self.address_string(), format_string % args))

    def do_GET(self):
        parsed = urlparse(self.path)
        if parsed.path == "/api/config":
            public_modes = {
                name: {key: mode[key] for key in ("name", "serviceLabel", "plural")}
                for name, mode in CONFIG["modes"].items()
            }
            self.send_json(200, {
                "arrivalWindowMinutes": CONFIG["arrivalWindowMinutes"],
                "modes": public_modes,
            })
            return
        if parsed.path == "/api/arrivals":
            self.serve_arrivals(parse_qs(parsed.query).get("mode", ["train"])[0])
            return

        path = unquote(self.path.lstrip("/").split("?", 1)[0])
        requested = (DIST / path).resolve()
        try:
            requested.relative_to(DIST)
        except ValueError:
            self.path = "/index.html"
        super().do_GET()

    def serve_arrivals(self, mode):
        mode_config = CONFIG["modes"].get(mode)
        if mode_config is None:
            self.send_json(400, {"error": "Unknown transit mode."})
            return
        api_key = read_api_key()
        if not api_key:
            self.send_json(503, {"error": "Add API_KEY to the local .env file to load live arrivals."})
            return
        stops = mode_config["stops"]
        now = int(time.time() * 1000)
        try:
            with OnebusawaySDK(api_key=api_key, timeout=12, max_retries=1) as client:
                with ThreadPoolExecutor(max_workers=len(stops)) as pool:
                    results = list(pool.map(lambda stop: get_stop_arrivals(client, stop, now), stops))
            unique = {item["id"]: item for result in results for item in result}
            arrivals = sorted(unique.values(), key=lambda item: item["timestamp"])
            arrivals = arrivals[:CONFIG["maxArrivals"]]
            self.send_json(200, {"arrivals": arrivals, "updatedAt": int(time.time() * 1000)})
        except (APIError, TimeoutError, ValueError, RuntimeError) as error:
            self.send_json(502, {"error": str(error) or "Unable to load OneBusAway arrivals."})

    def send_json(self, status, payload):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


if __name__ == "__main__":
    port = int(os.environ.get("PORT", "4173"))
    server = ThreadingHTTPServer(("0.0.0.0", port), DisplayHandler)
    print("Train times available on port {}".format(port))
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
