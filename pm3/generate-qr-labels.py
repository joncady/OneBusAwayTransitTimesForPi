#!/usr/bin/env python3
"""Create private album QR tokens, PNGs and an offline printable HTML label sheet."""
import argparse
import html
import json
import io
import os
from pathlib import Path
import secrets
import tempfile

from qr_config import PREFIX, defaults, load, valid_label, validate

ROOT = Path(__file__).resolve().parent


def private_write(path, data):
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix='.qr-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=Path.home() / 'Desktop' / 'Transit Albums',
                        help='album folders to register when --album is omitted')
    parser.add_argument('--album', action='append', help='register a label; repeat for multiple albums')
    parser.add_argument('--config', type=Path, default=ROOT / 'qr.local.json')
    parser.add_argument('--output', type=Path, default=ROOT / 'qr.local')
    parser.add_argument('--size-mm', type=int, default=30, help='QR square including quiet zone, 20–60 mm')
    parser.add_argument('--show-labels', action='store_true', help='also print private album names below codes')
    args = parser.parse_args()
    if not 20 <= args.size_mm <= 60:
        parser.error('--size-mm must be between 20 and 60')
    try:
        import qrcode
    except ImportError:
        raise ValueError('Install pm3/requirements-qr.txt in the host media virtual environment.') from None
    config_path = args.config.expanduser().resolve()
    output = args.output.expanduser().resolve()
    if config_path == output:
        raise ValueError('Configuration path must be a file, not the output directory.')
    if config_path.parent == output and (
            config_path.name in {'labels.html', 'manifest.local.json'}
            or config_path.suffix.lower() == '.png'):
        raise ValueError('Configuration path conflicts with generated label assets.')
    if args.album:
        labels = sorted(set(args.album))
    else:
        source = args.source.expanduser().resolve(strict=True)
        labels = sorted(p.name for p in source.iterdir()
                        if p.is_dir() and not p.is_symlink() and not p.name.startswith('.'))
    if not labels or any(not valid_label(label) for label in labels):
        raise ValueError('Choose album folders/labels with 1–32 printable characters.')
    config = load(config_path) if config_path.exists() else defaults()
    existing = set(config['albums'].values())
    for label in labels:
        if label not in existing:
            token = secrets.token_hex(16)
            while token in config['albums']:
                token = secrets.token_hex(16)
            config['albums'][token] = label
            existing.add(label)
    validate(config)
    output.mkdir(mode=0o700, parents=True, exist_ok=True)
    os.chmod(output, 0o700)
    rendered = []
    manifest = []
    for index, (token, label) in enumerate(config['albums'].items(), 1):
        qr = qrcode.QRCode(error_correction=qrcode.constants.ERROR_CORRECT_M, box_size=12, border=4)
        qr.add_data(PREFIX + token)
        qr.make(fit=True)
        name = token + '.png'
        # A generic opaque filename and number avoid personal names in image metadata.
        image = io.BytesIO()
        qr.make_image(fill_color='black', back_color='white').save(image, format='PNG')
        private_write(output / name, image.getvalue())
        caption = html.escape(label) if args.show_labels else f'Album {index}'
        rendered.append(f'<figure><img src="{name}" alt="Album QR {index}"><figcaption>{caption}</figcaption></figure>')
        manifest.append({'label': label, 'image': name})
    sheet = '''<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>Private transit album labels</title>
<style>
@page { margin: 10mm; }
body { font-family: sans-serif; margin: 10mm; color: black; background: white; }
main { display: flex; flex-wrap: wrap; gap: 5mm; align-items: flex-start; }
figure { margin: 0; padding: 1mm; text-align: center; break-inside: avoid; page-break-inside: avoid; }
img { display: block; width: SIZE_mm; height: SIZE_mm; image-rendering: pixelated; }
figcaption { max-width: SIZE_mm; font-size: 9pt; overflow-wrap: anywhere; margin-top: 1mm; }
@media print { .instructions { display: none; } body { margin: 0; } }
</style>
<div class="instructions"><h1>Private album labels</h1>
<p>Print at 100% / actual size, with browser headers and footers off. The QR square
includes its white quiet zone. Start with plain paper; verify scanning before
buying a printer or adhesive labels. This sheet stays local and loads no remote assets.</p>
<p>Do not cover the card number, important markings or the terminal's scanner window.
These codes select local photo albums; they are not fare or ticket credentials.</p></div>
<main>LABELS</main></html>
'''.replace('SIZE_mm', f'{args.size_mm}mm').replace('LABELS', '\n'.join(rendered))
    private_write(output / 'labels.html', sheet.encode())
    private_write(output / 'manifest.local.json', (json.dumps(manifest, indent=2) + '\n').encode())
    # Save stable tokens only after all label assets have been generated successfully.
    private_write(config_path, (json.dumps(config, indent=2) + '\n').encode())
    print(f'Prepared {len(manifest)} private QR labels. Existing tokens preserved.')
    print(f'Configuration: {config_path}\nPrint sheet: {output / "labels.html"}')
    print('No device connection, upload or restart performed.')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError) as error:
        raise SystemExit(f'QR label generation stopped: {error}')
