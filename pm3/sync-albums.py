#!/usr/bin/env python3
"""Prepare Desktop card albums and publish a new private USB media release."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import uuid
import warnings

try:
    from PIL import Image, ImageOps
except ImportError:
    raise SystemExit('Pillow is required: install it in your Python environment, then rerun.')

ROOT = Path(__file__).resolve().parent
REMOTE = '/init/transit-prototype'
USB = '/init/efad/mp_sd/transit-media'
SUPPORTED = {'.jpg', '.jpeg', '.png', '.heic', '.heif', '.webp'}


def ssh(target, command, **kwargs):
    return subprocess.run(['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=8', target, command], check=True, **kwargs)


def prepare_photo(source, output, work):
    if source.suffix.lower() in {'.heic', '.heif'}:
        converted = work / 'converted.jpg'
        subprocess.run(['sips', '-s', 'format', 'jpeg', str(source), '--out', str(converted)],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        source = converted
    with warnings.catch_warnings():
        warnings.simplefilter('error', Image.DecompressionBombWarning)
        with Image.open(source) as image:
            image = ImageOps.exif_transpose(image)
            image.thumbnail((754, 480), Image.Resampling.LANCZOS)
            if image.mode in ('RGBA', 'LA') or 'transparency' in image.info:
                rgba = image.convert('RGBA')
                rgb = Image.new('RGB', rgba.size, 'black')
                rgb.paste(rgba, mask=rgba.getchannel('A'))
            else:
                rgb = image.convert('RGB')
            # No EXIF/location metadata is copied to the prepared slideshow.
            rgb.save(output, 'JPEG', quality=85, optimize=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('target', nargs='?', help='SSH destination, e.g. root@device-host')
    parser.add_argument('--source', type=Path, default=Path.home() / 'Desktop' / 'Transit Albums')
    parser.add_argument('--prepare-only', action='store_true', help='prepare locally; do not connect or upload')
    args = parser.parse_args()
    if not args.prepare_only and not args.target:
        parser.error('target is required unless --prepare-only is used')
    source = args.source.expanduser().resolve(strict=True)
    folders = sorted((p for p in source.iterdir() if p.is_dir() and not p.is_symlink() and not p.name.startswith('.')), key=lambda p: p.name.casefold())
    if len(folders) > 32:
        raise ValueError('Use at most 32 album folders (one per saved card label).')
    local = ROOT / 'albums.local'
    local.mkdir(mode=0o700, exist_ok=True)
    os.chmod(local, 0o700)
    release = uuid.uuid4().hex
    destination = f'{USB}/albums/{release}'
    manifest = {}
    counts = {}
    with tempfile.TemporaryDirectory(prefix='prepare-', dir=local) as temporary:
        work = Path(temporary)
        staged = work / 'photos'
        staged.mkdir()
        for folder in folders:
            label = folder.name
            if not label.strip() or len(label) > 32 or any(ord(c) < 32 or ord(c) == 127 for c in label):
                raise ValueError('Each album folder must use a valid saved card label, at most 32 characters.')
            photos = sorted((p for p in folder.rglob('*') if p.is_file() and not p.is_symlink()
                             and p.suffix.lower() in SUPPORTED
                             and not any(part.startswith('.') for part in p.relative_to(folder).parts)),
                            key=lambda p: str(p.relative_to(folder)).casefold())
            if not photos:
                continue
            if len(photos) > 512:
                raise ValueError('Use at most 512 photos per album for this first version.')
            album_id = hashlib.sha256(label.encode()).hexdigest()[:16]
            output = staged / album_id
            output.mkdir()
            for number, photo in enumerate(photos, 1):
                photo.resolve(strict=True).relative_to(source)
                prepare_photo(photo, output / f'{number:04d}.jpg', work)
            manifest[label] = f'{destination}/{album_id}'
            counts[label] = len(photos)
        if not manifest:
            raise ValueError('No photos found. Add photos inside folders named after saved card labels.')
        total = sum(counts.values())
        print(f'Prepared {total} photos in {len(manifest)} albums; originals untouched.', flush=True)
        if args.prepare_only:
            preview = local / f'prepared-{release}'
            shutil.move(str(staged), preview)
            (preview / 'albums.json').write_text(json.dumps(counts, indent=2) + '\n')
            print(f'Prepared copies: {preview}')
            return
        # Confirm every folder is enrolled before transferring private photos.
        labels = ssh(args.target, f'{REMOTE}/prototype --cards-file {REMOTE}/cards.local.json --list-cards',
                     stdout=subprocess.PIPE).stdout.decode()
        enrolled = {line.rsplit(': ', 1)[0] for line in labels.splitlines() if ': ' in line}
        if set(manifest) - enrolled:
            raise ValueError('An album folder does not match an enrolled card label; rename it before syncing.')
        ssh(args.target, f'test -x {REMOTE}/restart-display.sh && '
            f'systemctl is-active --quiet transit-prototype.service && grep -q " /init/efad/mp_sd " /proc/mounts')
        action = json.loads(ssh(args.target, f'cat {REMOTE}/actions.local.json', stdout=subprocess.PIPE).stdout)
        action.pop('triggerCard', None)
        action.pop('photoDirectory', None)
        action['albums'] = manifest
        archive_path = work / 'photos.tar'
        with tarfile.open(archive_path, 'w') as archive:
            for path in sorted(staged.rglob('*.jpg')):
                archive.add(path, arcname=str(path.relative_to(staged)), recursive=False)
        # Refuse uploads that would leave less than 8 MiB free on the USB.
        available = ssh(args.target, 'df -Pk /init/efad/mp_sd', stdout=subprocess.PIPE).stdout.decode().splitlines()[-1].split()[3]
        if int(available) * 1024 < archive_path.stat().st_size + 8 * 1024 * 1024:
            raise ValueError('Not enough USB space; previous releases were retained for recovery.')
        with archive_path.open('rb') as archive:
            ssh(args.target, f'umask 077; mkdir -p {destination} && tar -xf - -C {destination}', stdin=archive)
        ssh(args.target, f'umask 077; cat > {REMOTE}/actions.local.json.new',
            input=(json.dumps(action, indent=2) + '\n').encode())
        ssh(args.target, f'cp -p {REMOTE}/actions.local.json {REMOTE}/actions.local.json.previous && '
            f'mv {REMOTE}/actions.local.json.new {REMOTE}/actions.local.json && '
            f'{REMOTE}/restart-display.sh 3600 live')
        ssh(args.target, 'systemctl is-active --quiet transit-prototype.service')
        report = local / 'last-sync.json'
        report.write_text(json.dumps({'source': str(source), 'release': release, 'counts': counts}, indent=2) + '\n')
        os.chmod(report, 0o600)
        print('Albums synced to USB; display restarted. Previous USB releases retained.')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f'Album sync stopped: {error}')
