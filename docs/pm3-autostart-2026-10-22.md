# PM3 automatic startup trial — 2026-10-22

## Current checkpoint

The owner completed a physical cold boot. The app launched automatically about
14.2 seconds after Linux boot, with no manual SSH start. Nine albums loaded and
both NFC polling modes were accepted. The first API request preceded DHCP; live
arrivals recovered on the existing automatic retry after two minutes, with zero
service restarts. QR integration was not deployed for this first boot checkpoint; it was deployed later and two phone-to-album actions were physically verified.

A startup-only fix now retries initial failures after five seconds, doubling to
a one-minute cap until the first successful refresh. `startupRetrySeconds` is
optional (default 5, range 1–60); original private config is unchanged. Normal
polling/backoff resumes after the first success. The fix compiled and is deployed. A second physical cold boot passed: the
service launched at about 14.4 seconds, the first API fetch succeeded immediately,
and refreshes continued for more than seven minutes with zero restarts. The owner
confirmed the display looked good. Since networking was ready on the second
boot, the new five-second failure retry timing was not exercised in that run.

## Recovery preparation

- Imaged both internal cards and SPI flash, and archived USB contents before
  installation. Both internal filesystems were read-only during disk imaging;
  the app was paused, then resumed after restoring the writable data mount.
- Each card image expands to 1,024,983,040 bytes; SPI expands to 16,777,216 bytes.
- Saved private copies on USB and in ignored `pm3/recovery.local/` on the computer.
  The computer copies match remote SHA-256 values, pass gzip integrity checks,
  and have expected raw image sizes. The USB archive passes checksum/gzip checks.
- No physical restoration has been performed. These images include credentials,
  device data and personal media; do not add them to Git.

## Installed changes and observed checks

- Startup-only app built from the last verified commit `fe14629` plus the
  `--continuous` flag. The separate full local source includes the prepared QR
  feature, but that build was not used for this device trial.
- Installed `/etc/systemd/system/transit-display.service` and app-owned launcher
  `/init/transit-prototype/start-display.sh`. Enabled the service for multi-user
  boot. It waits for `init-rc.service` and the existing USB bootstrap; it does not
  require SSH to launch or wait for network-online before rendering.
- Added explicit conditions for the app enable marker and bootstrap file. Kept
  the bootstrap and SSH setup unchanged. Disabled the stock ABT proxy's boot
  entry to avoid reader contention, retaining its former state for uninstall.
  NX was already disabled. No firmware, kernel, ignition or watchdog changes.
- Configured continuous live mode, failure restart with ten-second delay, five
  starts per five minutes, SIGTERM cleanup and journal logging. No poweroff
  failure action. The persistent service conflicts with the bounded prototype.
- Unit verification succeeded. It emitted an existing systemd-udevd/BPF firewall
  warning from this legacy OS; no firewall configuration was changed.
- Before shutdown: service enabled/active, zero restarts, nine usable albums,
  accepted ISO/DESFire polling and successful live arrival refreshes. Original
  transit and display settings matched the saved pre-demo settings by hash.
  Root filesystem was read-only and `/init` writable. QR config remained absent.

## Remaining verification

- [x] Owner powers on/handles ignition; display starts without an SSH start.
- [x] Read-only inspection confirms a new boot ID, enabled/active service,
      early boot start time, healthy reader and live arrival refreshes.
- [x] Updated app verified after another physical cold boot; first API fetch succeeded.
- [ ] New startup retry timing measured under a controlled initial network failure.
- [x] Existing card opens its correct album and ready/tap lights work; amplified audio stays off.
- [ ] Controlled crash/restart, delayed network and missing album media recovery.
- [ ] Full backup restoration procedure exercised on appropriate spare media.

Installation and uninstall helpers are in `pm3/`; see
[automatic startup operation](../pm3/README.md#automatic-startup-opt-in).
[Issue #5](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/5)
tracks remaining startup acceptance;
[issue #6](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/6)
tracks recovery/rollback work. LCD timeout remains separate.
