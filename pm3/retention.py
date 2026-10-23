#!/usr/bin/env python3
"""Preview or apply bounded media/deployment retention. Runs on the computer."""
import argparse
import io
import json
from pathlib import PurePosixPath
import re
import shlex
import subprocess
import tarfile

from device_tools import HOME, ssh, maintenance_lock

USB = '/init/efad/mp_sd/transit-media/albums'
RELEASE = re.compile(r'[a-f0-9]{32}\Z')
SNAPSHOT = re.compile(r'\d{8}T\d{6}Z-\d+\Z')

INVENTORY = r'''
set -eu
cd /init/transit-prototype
test ! -L deployments
test ! -L /init/efad/mp_sd/transit-media
test ! -L /init/efad/mp_sd/transit-media/albums
grep -q ' /init/efad/mp_sd ' /proc/mounts
for group in media deployment; do
 if [ "$group" = media ]; then root=/init/efad/mp_sd/transit-media/albums; else root=deployments; fi
 for path in "$root"/*; do
  if [ -d "$path" ] && [ ! -L "$path" ]; then
   if [ "$group" = deployment ]; then
    test -f "$path/format" && test ! -L "$path/format" || continue
    grep -qx 'PM3-deployment-1' "$path/format" || continue
   fi
   printf '%s|%s|' "$group" "${path##*/}"
   stat -c %Y "$path"
  fi
 done
done
'''

def select_expired(entries, keep, protected=()):
    if not 1 <= keep <= 20:
        raise ValueError('Retention must be 1..20')
    ordered = sorted(entries, key=lambda name: (entries[name], name), reverse=True)
    retained = set(ordered[:keep]) | set(protected)
    return sorted(set(entries) - retained)


def release_refs(action):
    if not isinstance(action, dict) or not isinstance(action.get('albums', {}), dict):
        raise ValueError('Invalid action configuration; refusing cleanup')
    paths = list(action.get('albums', {}).values())
    if action.get('photoDirectory'):
        paths.append(action['photoDirectory'])
    refs = set()
    for path in paths:
        if not isinstance(path, str):
            raise ValueError('Invalid album path in rollback configuration')
        parts = PurePosixPath(path).parts
        root = PurePosixPath(USB).parts
        if '..' in parts:
            raise ValueError('Ambiguous album path; refusing cleanup')
        if parts[:len(root)] == root and len(parts) > len(root):
            name = parts[len(root)]
            if RELEASE.fullmatch(name):
                refs.add(name)
    return refs


def cleanup(target, keep=3, log_bytes=1024*1024, apply=False):
    if not 65536 <= log_bytes <= 16*1024*1024:
        raise ValueError('Log cap must be 64 KiB..16 MiB')
    with maintenance_lock(target):
        raw = ssh(target, INVENTORY, stdout=subprocess.PIPE).stdout.decode()
        media, snapshots = {}, {}
        for line in raw.splitlines():
            group, name, stamp = line.split('|')
            pattern, entries = (RELEASE, media) if group == 'media' else (SNAPSHOT, snapshots)
            if pattern.fullmatch(name):
                entries[name] = int(stamp)
        pointer = ssh(target, f'cat {HOME}/last-good-deployment.local.txt 2>/dev/null || true', stdout=subprocess.PIPE).stdout.decode().strip()
        if pointer and (not pointer.startswith('deployments/') or not SNAPSHOT.fullmatch(pointer.removeprefix('deployments/')) or PurePosixPath(pointer).name not in snapshots):
            raise ValueError('Invalid last-good snapshot pointer; refusing cleanup')
        protected_snapshot = {PurePosixPath(pointer).name} if pointer else set()
        old_snapshots = select_expired(snapshots, keep, protected_snapshot)
        retained_snapshots = set(snapshots) - set(old_snapshots)
        # Read configs without extracting private files onto the computer.
        config_paths = ['actions.local.json', 'actions.local.json.previous']
        config_paths += [f'deployments/{name}/actions.local.json' for name in sorted(retained_snapshots)]
        script = f'cd {HOME}\ntest -f actions.local.json && test ! -L actions.local.json || exit 1\nset --\n'
        for path in config_paths:
            q = shlex.quote(path)
            script += f'if [ -L {q} ]; then exit 1; fi\nif [ -f {q} ]; then set -- "$@" {q}; fi\n'
        script += 'test "$#" -gt 0\ntar -cf - "$@"\n'
        data = ssh(target, script, stdout=subprocess.PIPE).stdout
        if len(data) > 4*1024*1024:
            raise ValueError('Configuration archive too large')
        protected = set()
        with tarfile.open(fileobj=io.BytesIO(data)) as archive:
            for member in archive:
                if not member.isfile() or member.size > 16384 or member.name not in config_paths:
                    raise ValueError('Unexpected configuration archive entry')
                protected |= release_refs(json.load(archive.extractfile(member)))
        old_media = select_expired(media, keep, protected)
        print(f'Retention preview: {len(old_media)} unused media releases, {len(old_snapshots)} old deployment snapshots; keep newest {keep} plus referenced versions.')
        print(f'Legacy log cap: {log_bytes} bytes; recovery images/bootstrap/card registry are excluded.')
        if not apply:
            return old_media, old_snapshots
        # Recheck mount/path type immediately before deleting validated child names.
        script = f'''set -eu
cd {HOME}
grep -q ' /init/efad/mp_sd ' /proc/mounts
test ! -L deployments
test ! -L /init/efad/mp_sd/transit-media
test ! -L {USB}
'''
        for base, names in [(USB, old_media), (f'{HOME}/deployments', old_snapshots)]:
            for name in names:
                path = shlex.quote(base + '/' + name)
                script += f'test -d {path} && test ! -L {path}\nrm -rf -- {path}\n'
        script += f'''for path in prototype.log check-qr.log; do
 if [ -f "$path" ] && [ ! -L "$path" ]; then
  size=$(stat -c %s "$path")
  if [ "$size" -gt {log_bytes} ]; then
   test ! -L "$path.trim"
   umask 077
   tail -c {log_bytes} "$path" > "$path.trim"
   cat "$path.trim" > "$path"
   rm -f "$path.trim"
  fi
 fi
done
'''
        ssh(target, script)
        print('Retention applied.')
        return old_media, old_snapshots


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('target')
    parser.add_argument('--keep-releases', type=int, default=3)
    parser.add_argument('--log-max-bytes', type=int, default=1024*1024)
    parser.add_argument('--apply', action='store_true', help='default is preview only')
    args = parser.parse_args()
    cleanup(args.target, args.keep_releases, args.log_max_bytes, args.apply)

if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError):
        raise SystemExit('Maintenance stopped; inspect connectivity, mount and configuration. No private values are printed.')
