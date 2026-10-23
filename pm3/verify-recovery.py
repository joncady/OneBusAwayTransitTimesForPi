#!/usr/bin/env python3
"""Verify private recovery images; optionally restore into NEW regular files only."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import tarfile

EXPECTED = {'root-card.img.gz': 1024983040, 'data-card.img.gz': 1024983040, 'spi.img.gz': 16777216}
NAMES = set(EXPECTED) | {'usb-files.tar.gz'}


def verify(root, rehearsal=None, expected=None):
    expected = EXPECTED if expected is None else expected
    names = set(expected) | {'usb-files.tar.gz'}
    manifest = {}
    for line in (root / 'images.sha256').read_text().splitlines():
        digest, name = line.split(None, 1)
        name = PurePosixPath(name.strip()).name
        if name not in names or name in manifest or not re.fullmatch('[a-f0-9]{64}', digest):
            raise ValueError('Invalid checksum manifest')
        manifest[name] = digest
    if set(manifest) != names:
        raise ValueError('Missing recovery file in checksum manifest')
    # Validate every compressed file before creating any restored output.
    for name, digest in manifest.items():
        p = root / name
        if not p.is_file() or p.is_symlink():
            raise ValueError('Recovery input must be a regular file')
        h = hashlib.sha256()
        with p.open('rb') as stream:
            for chunk in iter(lambda: stream.read(1024*1024), b''):h.update(chunk)
        if h.hexdigest() != digest:
            raise ValueError('Compressed recovery checksum mismatch')
    if rehearsal:
        # Refuse existing directories and all raw/block device destinations.
        rehearsal.mkdir(mode=0o700, parents=False, exist_ok=False)
    report = {}
    for name, size in expected.items():
        total = 0
        h = hashlib.sha256()
        output = None
        try:
            if rehearsal:
                destination = rehearsal / name.removesuffix('.gz')
                output = destination.open('xb')
                os.chmod(destination, 0o600)
            with gzip.open(root / name, 'rb') as stream:
                for chunk in iter(lambda: stream.read(1024*1024), b''):
                    total += len(chunk)
                    if total > size:raise ValueError('Recovery image exceeds expected size')
                    h.update(chunk)
                    if output:output.write(chunk)
            if total != size:raise ValueError('Recovery image size mismatch')
        finally:
            if output:output.close()
        if rehearsal:
            restored = hashlib.sha256()
            with destination.open('rb') as stream:
                for chunk in iter(lambda: stream.read(1024*1024), b''):restored.update(chunk)
            if restored.hexdigest() != h.hexdigest():raise ValueError('Restored file checksum mismatch')
        report[name] = {'rawBytes': total, 'rawSha256': h.hexdigest()}
        print(f'{name}: compressed checksum, gzip integrity and raw size verified' + ('; restored regular-file checksum matches' if rehearsal else ''))
    with tarfile.open(root / 'usb-files.tar.gz', 'r:gz') as archive:
        count = 0
        for member in archive:
            if member.name.startswith('/') or '..' in PurePosixPath(member.name).parts:
                raise ValueError('Unsafe USB archive path')
            count += 1
    # Consume to EOF as well, validating the gzip trailer beyond the tar end marker.
    with gzip.open(root / 'usb-files.tar.gz', 'rb') as stream:
        while stream.read(1024*1024):pass
    print(f'USB archive verified ({count} entries); no files extracted.')
    if rehearsal:
        (rehearsal / 'verification.local.json').write_text(json.dumps(report, indent=2)+'\n')
        os.chmod(rehearsal / 'verification.local.json', 0o600)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('backup', type=Path)
    parser.add_argument('--rehearse-dir', type=Path, help='new directory for ~2 GiB restored files; NEVER writes hardware')
    args = parser.parse_args()
    verify(args.backup, args.rehearse_dir)
    print('This is integrity/file restoration verification, not a physical boot or SPI restore test.')

if __name__ == '__main__':
    main()
