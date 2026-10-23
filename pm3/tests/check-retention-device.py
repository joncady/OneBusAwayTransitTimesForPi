#!/usr/bin/env python3
"""Optional on-device retention rehearsal using empty releases and a temporary log."""
from pathlib import Path
import subprocess
import sys
import uuid
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from device_tools import ssh, HOME
from retention import cleanup, USB

def run(target):
    releases = [uuid.uuid4().hex for _ in range(5)]
    work = '/tmp/pm3-retention-test-' + uuid.uuid4().hex
    try:
        script=f'''set -eu
umask 077
mkdir {work}
cd {HOME}
test ! -L prototype.log
if [ -f prototype.log ]; then cp -p prototype.log {work}/original.log; fi
'''
        for name in releases:
            script+=f'mkdir {USB}/{name}\ntouch -t 200001010000 {USB}/{name}\n'
        script+=f'touch {work}/log-replaced\n'
        script+='dd if=/dev/zero of=prototype.log bs=1048576 count=2 2>/dev/null\n'
        ssh(target, script)
        expired, _ = cleanup(target, apply=True)
        if len(set(expired)&set(releases)) < 2:raise ValueError('Expected unused fixtures to be pruned')
        ssh(target, f'test "$(stat -c %s {HOME}/prototype.log)" -eq 1048576')
        print('On-device retention: unused fixtures pruned and 2 MiB legacy log capped to 1 MiB.')
    finally:
        script=f'cd {HOME}\n'
        script+=f'if [ -f {work}/log-replaced ]; then if [ -f {work}/original.log ]; then cp -p {work}/original.log prototype.log; else rm -f prototype.log; fi; fi\n'
        for name in releases:script+=f'rmdir {USB}/{name} 2>/dev/null || true\n'
        script+=f'rm -rf {work}\n'
        ssh(target, script)

if __name__=='__main__':run(sys.argv[1])
