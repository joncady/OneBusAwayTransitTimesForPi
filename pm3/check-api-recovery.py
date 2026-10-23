#!/usr/bin/env python3
"""Exercise startup API failure/recovery without changing Ethernet or SSH."""
import argparse
import json
from pathlib import Path
import subprocess
import time
from device_tools import HOME, ssh, maintenance_lock


def run(target):
    with maintenance_lock(target):
        before = ssh(target, f'cat {HOME}/display.local.json', stdout=subprocess.PIPE).stdout
        settings = json.loads(before)
        settings['baseUrl'] = 'https://127.0.0.1:1'
        settings['startupRetrySeconds'] = 1
        # A device-owned timer restores the original even if this computer disconnects.
        ssh(target, f'''set -eu
cd {HOME}
test ! -e display.recovery-test.local.json
umask 077
cp -p display.local.json display.recovery-test.local.json
cat > restore-api-test.sh <<'RESTORE'
#!/bin/sh
set -eu
cd /init/transit-prototype
cp -p display.recovery-test.local.json display.local.json.restore
mv display.local.json.restore display.local.json
./restart-display.sh 3600 live
RESTORE
chmod 700 restore-api-test.sh
systemd-run --unit=transit-api-recovery-restore --on-active=60s /bin/sh {HOME}/restore-api-test.sh
''')
        try:
            ssh(target, f'umask 077; cat > {HOME}/display.local.json.test', input=(json.dumps(settings)+'\n').encode())
            ssh(target, f'''set -eu
cd {HOME}
./prototype --check-config --transit-file transit.local.json --display-file display.local.json.test --env-file api.env
mv display.local.json.test display.local.json
./restart-display.sh 3600 live
''')
            time.sleep(7)
            report = ssh(target, f'''set -eu
systemctl is-active --quiet transit-display.service
pid=$(systemctl show transit-display.service -p MainPID --value)
journalctl _PID="$pid" --no-pager -o cat | grep 'Arrivals refresh:'
''', stdout=subprocess.PIPE).stdout.decode()
            count = report.count('Arrivals refresh:')
            if count < 2:raise ValueError('Expected repeated failed requests while service stays active')
            print(f'Startup outage: {count} failed requests; service stayed active and SSH remained available.')
        finally:
            ssh(target, f'/bin/sh {HOME}/restore-api-test.sh')
            ssh(target, 'systemctl stop transit-api-recovery-restore.timer transit-api-recovery-restore.service')
            restored = ssh(target, f'cat {HOME}/display.local.json', stdout=subprocess.PIPE).stdout
            if restored != before:raise ValueError('Original display configuration did not restore')
            ssh(target, f'rm -f {HOME}/display.recovery-test.local.json {HOME}/restore-api-test.sh {HOME}/display.local.json.test')
        for _ in range(15):
            result = ssh(target, '''pid=$(systemctl show transit-display.service -p MainPID --value)
journalctl _PID="$pid" --no-pager -o cat
''', stdout=subprocess.PIPE).stdout.decode()
            if 'Arrivals refreshed:' in result:
                print('Original configuration restored byte-for-byte; live arrivals refreshed successfully.')
                return
            time.sleep(1)
        raise ValueError('Live arrivals have not refreshed; original configuration is restored')

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('target')
    args = parser.parse_args()
    try:run(args.target)
    except (OSError, ValueError, subprocess.CalledProcessError):
        raise SystemExit('Recovery test did not complete; inspect service/timer state. No private configuration is printed.')
