#!/bin/sh
set -eu
umask 077
cd /home/app0
cp pm3-original-run pm3-run.restore
chmod 700 pm3-run.restore
mv pm3-run.restore run
sync
trap 'exec /bin/sh /home/app0/run' EXIT
printf 'restored-before-rf-trial\n' > pm3-trial-status
chmod 700 /home/app0/pm3-rf-probe
/home/app0/pm3-rf-probe > pm3-trial-output 2>&1 &
trial_pid=$!
(
  sleep 900
  kill "$trial_pid" 2>/dev/null || true
  sleep 2
  kill -9 "$trial_pid" 2>/dev/null || true
) &
watchdog_pid=$!
printf 'rf-trial-running\n' >> pm3-trial-status
wait "$trial_pid" || true
kill "$watchdog_pid" 2>/dev/null || true
printf 'rf-trial-ended\n' >> pm3-trial-status
exec /bin/sh /home/app0/run
