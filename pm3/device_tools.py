"""Host-side SSH and a shared lock for deployment, publication and cleanup."""
from contextlib import contextmanager
import subprocess

HOME = '/init/transit-prototype'

def ssh(target, command, **kwargs):
    return subprocess.run(['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=8', target, command], check=True, **kwargs)

@contextmanager
def maintenance_lock(target):
    ssh(target, f'umask 077; mkdir {HOME}/maintenance.lock')
    try:
        yield
    finally:
        ssh(target, f'rmdir {HOME}/maintenance.lock')
