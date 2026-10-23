# PM3 maintenance and recovery

These tools run on the development computer; the terminal still needs only its
native app and installed Linux utilities. No Python or Node runtime is installed
on it. Use a private SSH destination, for example:

```sh
PM3_TARGET='root@device-host'
```

## Compatible deployments and rollback

Build with `./pm3/build.sh --test`, then update the app while preserving all
on-device transit/API/card/album/QR configuration:

```sh
./pm3/deploy-update.sh "$PM3_TARGET"
```

`deploy-live.sh` still explicitly copies local transit/display/API settings;
`deploy-qr.sh` explicitly replaces the QR map. Both use the shared activation
helper. Album sync validates and publishes its action map with rollback too.

Transfers finish and configuration validates before the working app stops.
Activation snapshots the previous executable, launchers and managed configuration
under private `deployments/` on the data card. A trap restores that compatible set
on a failed swap, failed startup or catchable interruption. The card registry is
never swapped, recreated or pruned. A failed transfer leaves the live app alone.
A successful update retains a pointer to its prior compatible snapshot.

```sh
./pm3/rollback-update.sh "$PM3_TARGET"
```

Manual rollback stages the pointed-to snapshot and uses the same activation
checks. It takes a new snapshot first, so the replaced version is recoverable.
Neither command is a filesystem transaction across sudden power loss or SIGKILL.
Full private images and physical recovery access remain necessary. The older
`run-prototype.sh` is a low-level, bounded initial trial launcher, not the normal
transactional update path.

Deployment, publication, backups and retention cooperate through the app-owned
`maintenance.lock` directory. If the computer is killed or loses connectivity,
a lock may remain. Confirm no maintenance command is running and inspect service
and staging state before manually removing the empty lock directory; never
remove it simply to bypass an active operation.

## Storage retention

Preview first, then apply:

```sh
python3 pm3/maintain-device.py "$PM3_TARGET"
python3 pm3/maintain-device.py "$PM3_TARGET" --apply --keep-releases 3
```

The policy keeps the newest three media releases and managed deployment
snapshots, **plus** the last-good snapshot and every media release referenced by
the current action map, previous published map or retained snapshot. The protected
set can therefore exceed three. It operates only inside the dedicated albums and
managed deployments directories. Unrecognized names and symlinked entries are
not pruned. Missing/corrupt configuration aborts cleanup. Recovery images, USB
bootstrap, personal source photos and the private card registry are excluded.

Successful album sync and normal deployments run retention automatically. For
sync use `--keep-releases N`; for deployment use `PM3_KEEP_RELEASES=N`. Both allow
1–20. Cleanup failure reports a warning without reverting a successful update.

Persistent and transient app sessions now use journald instead of an unbounded
`prototype.log`. The audited terminal already has `RuntimeMaxUse=1M`, and measured
journal usage is 1 MiB in `/run`, avoiding persistent flash writes for these logs.
Check this policy on other devices rather than assuming they share it:

```sh
ssh "$PM3_TARGET" 'journalctl --disk-usage; cat /etc/systemd/journald.conf'
ssh "$PM3_TARGET" 'journalctl -u transit-display.service -n 50 --no-pager'
```

Maintenance also caps the two known legacy files (`prototype.log`, `check-qr.log`)
to their last 1 MiB. Override with `--log-max-bytes` (64 KiB–16 MiB). Keep old
legacy sessions stopped before trimming logs; current launchers use the journal.

## Private recovery images

```sh
./pm3/backup-device.sh "$PM3_TARGET"
python3 pm3/verify-recovery.py pm3/recovery.local/<backup-directory>
python3 pm3/verify-recovery.py pm3/recovery.local/<backup-directory> \
  --rehearse-dir /tmp/new-pm3-restoration-directory
```

Backup checks the audited storage mapping, stops the app, makes the data
filesystem read-only, reads both internal cards/SPI and archives the USB. Cleanup
restores the writable mount and resumes a previously running app. It coordinates
with deployment/retention through the shared lock. USB files are not frozen at
block level, so keep other writers stopped. The updated helper was run end to
end on October 22: both cards/SPI and USB were backed up, the writable data mount
was restored, the maintenance lock released and the live service resumed. Local
compressed SHA-256, gzip integrity, raw-size and USB archive checks also passed.

Verification checks every compressed SHA-256, gzip integrity and exact image
size. The two card images each expand to 1,024,983,040 bytes; SPI is 16,777,216
bytes. File rehearsal restores only into a **new directory of regular files**,
then independently reads those outputs and checks their raw hashes. It refuses
to overwrite existing outputs and never writes a block device. Allow about 2 GiB
of temporary space. USB tar paths and gzip integrity are checked without
extracting its private files.

These archives contain keys, card identity state and personal media. Keep them
private; a checksum is an integrity check, not encryption or proof of bootability.

### Physical restore procedure and remaining checkpoint

1. Verify the backup before touching hardware. Preserve the original cards and
   label which is root (`/dev/sda` in the audited unit) and data (`/dev/sdb`).
2. Shut down the terminal and remove power. Use separate replacement cards and a
   reader on another computer. Each replacement must hold the full raw image.
3. Identify the replacement device by its actual capacity/model, unmount its
   partitions, and use an image writer to write the corresponding **whole-disk**
   decompressed root/data image. Do not write a whole-disk image into one partition
   or choose the computer's own system disk.
4. Read back the replacement's first expected image-size bytes and compare their
   SHA-256 with the verifier's raw-image report. Restore the USB archive onto
   separately prepared compatible media, preserving bootstrap scripts and keys.
5. Fit the replacements, retain the originals untouched, handle ignition, and
   verify boot, SSH, private card identities, arrivals and albums. A data image
   captured during backup may contain the empty `maintenance.lock` directory;
   after confirming no maintenance job is running, remove that stale lock before
   using maintenance commands.
6. The SPI file is a recovery artifact, not an invitation to write live flash.
   Establish a device-specific, validated recovery/programmer procedure before
   any SPI restoration. No SPI write tooling is supplied or tested here.

No physical spare-card boot or SPI restoration has been performed. File
restoration verification does not close that hardware checkpoint in issue #6.

## Missing media and network recovery

Configured album labels now remain available even when their folders are absent
at startup. A requested tap/scan re-indexes that album; after media returns, the
next request can work without an app restart. Missing/empty media rejects the
request without a confirmation beep. If a selected photo is unreadable or
invalid, the display returns to arrivals. A held QR still does not reset an
already active session; remove/re-present it to request a recovered album.

Network failures preserve the prior snapshot as cached data until its configured
expiry; expired arrivals are hidden. A partial stop failure does not publish a
misleading partial replacement snapshot. Startup retries back off separately
from normal polling and reset after a successful refresh.

Optional on-device startup-outage check:

```sh
python3 pm3/check-api-recovery.py "$PM3_TARGET"
```

This briefly substitutes an unreachable loopback HTTPS endpoint, keeping the NIC
and SSH connected. A device-owned timer restores the original display settings
if the host disconnects; normal completion restores them byte-for-byte, cancels
the timer and checks that arrivals refresh. It exercises initial failure and
recovery, not physical cable loss, cached mid-session outage or power loss.

## Repeat automated checks

```sh
./pm3/build.sh --test
python3 -m unittest discover -s pm3/tests -v
```

The host tests use synthetic private-free fixtures and new temporary directories.
They do not contact the terminal. The separately named
`pm3/tests/check-retention-device.py` is an opt-in on-device rehearsal; it creates
empty test releases, temporarily substitutes a synthetic legacy log, exercises
cleanup, and restores the original log. Keep other maintenance operations stopped
while running that diagnostic.

## Recorded maintenance verification — 2026-10-22

- 13 Rust tests passed, including missing-at-boot/media-return, invalid photo,
  shared feedback cooldown, stable synthetic card aliases, partial-stop failure,
  cached/expired arrivals, recovery and bounded retry behavior.
- Fourteen host tests passed: successful activation, incomplete transfer,
  mid-swap failure, failed startup rollback, retention protection and recovery
  checksum/regular-file restoration behavior, and album-map publication/rollback.
- The updated full-backup helper completed imaging on the terminal and resumed
  the app. Root remained read-only, data returned to read/write, nine albums and
  live arrivals loaded, amplifier stayed off and all six private file hashes
  matched the pre-maintenance baseline. Local copies matched remote compressed
  hashes, passed gzip integrity/expected sizes and verified the USB archive.
- Real saved root/data/SPI images restored into new regular files; raw checksums
  and expected sizes matched. USB archive integrity passed (210 entries).
- A deliberately failing executable was deployed on the terminal. The helper
  restored the working app, and all six private configuration/registry hashes
  matched the baseline.
- The on-device outage trial logged four failed requests while the service/SSH
  stayed available. Original settings restored and live arrivals refreshed.
- The retention rehearsal pruned three disposable empty releases and capped a
  synthetic 2 MiB legacy log to 1 MiB. The original log was restored; fixture
  directories were removed. Active photo release/configuration were preserved.

Remaining physical checks: distinct cards/experimental fingerprints across power
cycles, actual USB removal/reinsertion, printed QR/reconnect behavior, spare-card
restore/boot and SPI recovery. Do not infer experimental fingerprint uniqueness
from repeat matching alone.
