"""Private QR configuration shared by host-side album tools (stdlib only)."""
import json
from pathlib import Path
import re

PREFIX = 'PM3-ALBUM:1:'
TOKEN = re.compile(r'[0-9a-f]{32}\Z')
FIELDS = {'version', 'enabled', 'device', 'cooldownMs', 'frameGapMs', 'reconnectSeconds', 'albums'}


def valid_label(label):
    return (isinstance(label, str) and bool(label.strip()) and len(label) <= 32
            and not any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in label))


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('Duplicate field in private QR configuration.')
        result[key] = value
    return result


def validate(config):
    if not isinstance(config, dict) or set(config) != FIELDS:
        raise ValueError('QR configuration has missing or unknown fields.')
    if type(config['version']) is not int or config['version'] != 1 or type(config['enabled']) is not bool:
        raise ValueError('Invalid QR version/enabled setting.')
    if not isinstance(config['device'], str) or not re.fullmatch(r'/dev/ttyUSB(?:[0-9]|[12][0-9]|3[01])', config['device']):
        raise ValueError('QR device must be /dev/ttyUSB0 through /dev/ttyUSB31.')
    for name, low, high in [('cooldownMs', 250, 10000), ('frameGapMs', 50, 1000), ('reconnectSeconds', 2, 60)]:
        if type(config[name]) is not int or not low <= config[name] <= high:
            raise ValueError('QR timing setting is outside its allowed bounds.')
    albums = config['albums']
    if not isinstance(albums, dict) or not 1 <= len(albums) <= 32:
        raise ValueError('Configure 1–32 QR token mappings.')
    if any(not TOKEN.fullmatch(token) or not valid_label(label) for token, label in albums.items()):
        raise ValueError('Invalid QR token or album label.')
    return config


def load(path):
    path = Path(path)
    if path.stat().st_size > 16 * 1024:
        raise ValueError('QR configuration exceeds 16 KiB.')
    try:
        config = json.loads(path.read_text(), object_pairs_hook=unique_object)
    except (json.JSONDecodeError, UnicodeError):
        raise ValueError('Invalid private QR JSON.') from None
    return validate(config)


def defaults():
    return {'version': 1, 'enabled': True, 'device': '/dev/ttyUSB0', 'cooldownMs': 2000,
            'frameGapMs': 150, 'reconnectSeconds': 5, 'albums': {}}
