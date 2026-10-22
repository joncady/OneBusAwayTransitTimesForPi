# PROXmobil3 setup and operation

This is a bounded integration experiment for the PROXmobil3, separate from the existing Pi app. It uses Rust and Slint's software renderer directly on the audited 800 × 480 framebuffer. Live mode fetches OneBusAway arrivals; launching without transit configuration shows explicitly labeled samples. No fare transactions are performed.

## What it exercises

- Native framebuffer rendering, installed DejaVu font, and large departure rows with 36-pixel side margins.
- cVEND version/status queries over `/dev/ttymxc3` at 115200 baud.
- Optional generic ISO/DESFire polling, session release after a tap, and a visual response to a card event.
- Optional private card aliases, automatically labeled Card 1, Card 2, and so on.
- Monotonic timing, bounded execution, SIGINT/SIGTERM handling, and restoration of saved screen pixels, console cursor blink, and UART settings on graceful exit.

The normal display program never prints or persists raw card payloads or card UIDs. It has no fare/card write, payment, firmware update, reset, or security-key command. CardRelease finishes an NFC reader session; it does not change fare data. Card polling is runtime reader configuration; polling functions enabled by the prototype are disabled on graceful exit. That is not a snapshot of the reader's prior polling configuration. A card event does not prove the card is an ORCA card or identify its owner. Repeated taps are needed to establish whether a particular physical card exposes a stable identifier.

## Build

Prerequisite: a running Docker engine. Build tools are confined to the build container.

```sh
./pm3/build.sh
```

The build compiles a static ARMv7 hard-float musl executable. Cargo.lock pins resolved dependencies. No system libraries or Python/Node runtime need to be installed on the PM3. The first build downloads the Rust image, cross-target standard library, and crates; later builds use caches.

Output: `pm3/target/armv7-unknown-linux-musleabihf/release/pm3-transit-prototype`.

Compilation is the default. Use `./pm3/build.sh --test` only when you want to run the existing protocol tests as well; `--skip-tests` remains a compatible alias for the default.

## Quick setup checklist

1. Enable SSH using the [PM3 community instructions](https://wiki.pm3.dev/ssh)
   and verify key-based access with `ssh -o BatchMode=yes root@<device-host>`.
   Keep the working USB bootstrap attached; these scripts do not install or
   replace it, or a permanent boot service.
2. Create private `config/transit.json` from the repository's example and add
   `API_KEY` to local `.env`. No device address or password is tracked.
3. Run `./pm3/deploy-live.sh root@<device-host> 3600`. This is a one-hour trial.
4. Enroll cards one at a time, remove each for two seconds, then repeat the tap.
   Use the commands below to name the saved labels. Back up the private registry.
5. Create the private action file from `pm3/albums.example.json` (stream it over
   SSH into `/init/transit-prototype/actions.local.json`, mode 0600). Put photos
   in Desktop folders named after enrolled labels, install the host media-tool
   dependency, then run `python3 pm3/sync-albums.py root@<device-host>`.
6. For another bounded session, run
   `ssh root@<device-host> '/init/transit-prototype/restart-display.sh 3600 live'`.
   This recreates the unit even if its previous transient instance was collected.

The [source/reference index](REFERENCES.md) distinguishes observed hardware,
working integration and experimental reader work. For review and rollback steps,
see [repository handoff](../docs/pm3-repository-handoff.md).

## Deploy live arrivals with the existing card/photo action

On the development machine, retain the existing private `config/transit.json` and `.env` with `API_KEY`. SSH key access and the working USB bootstrap must be available. The installed OS provides `/usr/bin/curl`; the native app uses it for certificate-verified HTTPS in a background worker. It does not require a Python/Node runtime or a separate server.

```sh
./pm3/deploy-live.sh root@<device-host> 3600
```

This compiles without running tests, prepares ignored `pm3/runtime.local/` settings, transfers only API_KEY from `.env`, validates the incoming configuration before stopping the current prototype, and launches a one-hour transient service. Existing card aliases, photo action, USB bootstrap and boot settings are preserved. The previous binary and existing transit/display/API files get `.previous` copies on-device. These are limited deployment rollback copies, not full recovery backups. Failed validation leaves the current prototype running. A transfer/startup failure after stopping it still requires manual recovery; this is not a transactional deployment system.

To reuse an already compiled binary, append `--skip-build`. To change the interval, selected train/bus mode, paging, timezone, cache lifetime or error backoff, edit ignored `pm3/runtime.local/display.json` and redeploy. It is created from `display.example.json` on first deployment and preserved thereafter. Stops continue to come from `config/transit.json`. A station changed only on the device is replaced by that local config on the next deployment. Both JSON contracts reject unknown fields and validate bounds on load. Settings currently apply at restart; there is no settings web server or working touchscreen control.

Defaults: train mode; one refresh cycle per 60 seconds after completion, one request per configured stop; three visible rows, paging every 10 seconds; stale after 180 seconds; cached arrivals expire after 900 seconds; failures back off up to 300 seconds. Fetches are sequential within one background worker with a 12-second request timeout and bounded responses. A failed stop makes the whole refresh fail so the previous complete snapshot is retained. Network failures never block NFC/rendering. Refreshes continue at the same bounded cadence during slideshows, so returning to departures uses the latest cache. Only the selected mode is fetched.

Countdowns and the Pacific clock use the API's `currentTime` anchored to monotonic elapsed time, refreshed each successful cycle. This avoids the device's incorrect wall clock without changing global time/RTC settings. The clock stays unavailable until the first successful fetch and disappears when the cache expires. System-wide time synchronization remains separate work; TLS still checks certificate validity against the OS clock, so a severely wrong OS date can prevent initial HTTPS access. This integration uses the new arrivals-while-idle requirement and does not enable the old Pi's scheduled screen blanking.

Request paths and fields follow the [OneBusAway arrivals-and-departures-for-stop API](https://developer.onebusaway.org/api/where/methods/arrivals-and-departures-for-stop). API keys are passed to curl through stdin, never command-line arguments or logs. Private config/API files are mode 0600 on-device. The card-triggered sound and slideshow run independently of live fetching, and the action returns to live departures when its configured session ends.

## Run from internal storage

SSH key access must already work. Leave the working USB bootstrap attached initially, because it suppresses the stock fare UI and handles the PIC watchdog; the binary and log run from internal `/init` storage.

```sh
./pm3/run-prototype.sh root@<device-host> 600
ssh root@<device-host> 'cat /init/transit-prototype/prototype.log'
```

The script refuses to launch while stock fare services or a previous prototype process are present. Persistent files go only into `/init/transit-prototype`, including the private `cards.local.json` alias registry. It launches a temporary systemd service for the specified number of seconds, with an additional service runtime limit. This survives SSH session cleanup and installs no boot service. SSH host-key checking remains enabled. Use the private host address outside this repository.

For a bounded reader-only test (no framebuffer changes):

```sh
ssh root@<device-host> '/init/transit-prototype/prototype --probe-reader --seconds 10'
```

Add `--enable-card-detection` to the reader-only test to enable polling. Do not run two programs against the NFC UART at once. The advisory lock prevents two instances of this prototype; it cannot stop unrelated programs that ignore the lock.

To stop the launched prototype early:

```sh
ssh root@<device-host> 'systemctl stop transit-prototype.service'
```

Avoid `kill -9`: cleanup cannot run after SIGKILL, sudden power loss, or a process abort. The prototype temporarily pauses console cursor blink, then restores its original value; screen pixel restoration is verified by immediate readback. It does not change display modes, power state, system time, boot settings, or watchdog settings. The elapsed timer does not depend on the reader's incorrect wall clock.

## Attribute transit cards

With `--cards-file PATH`, the prototype gives a newly seen UID a label such as `Card 1`. Tap the same card again after removing it: the label should remain the same. Then present the next card and record which physical card matches each displayed label. An issuing agency is not inferred automatically.

The registry contains a random local salt, SHA-256 fingerprints, identification methods, user labels, and tap counts. It is created with mode 0600 and updated through an atomic rename. Raw UIDs are hashed only in memory. The parser extracts UIDs only from known ISO/DESFire event layouts. The same UID arriving through ISO or DESFire maps to the same label. Existing registries remain compatible and retain their labels.

For `UnhandledCard` (`0xbe`) events with 4–512 payload bytes, an experimental fallback hashes the entire detection payload and message type under a separate namespace. The screen and label listing mark this method as experimental. It does not decode a UID or establish a unique physical identity: metadata may change between taps, or different cards may produce identical messages. Repeated taps, comparisons with other cards, and a reader restart are needed before relying on it. Raw detection payloads are never saved or logged. Other unknown event layouts remain unassigned.

List labels or name a card without touching the display/UART:

```sh
ssh root@<device-host> '/init/transit-prototype/prototype --cards-file /init/transit-prototype/cards.local.json --list-cards'
ssh root@<device-host> '/init/transit-prototype/prototype --cards-file /init/transit-prototype/cards.local.json --name-card "Card 1" "ORCA"'
```

The running display picks up a changed label on the next tap. Keep this registry private and back it up separately; deleting it resets aliases and fingerprints. A local `pm3/cards.local.json` copy is ignored by Git. Some cards may expose changing or non-unique identifiers, so do not assume labels are dependable until repeat taps and a restart have been tested.

## Card-triggered slideshow experiment

Copy `actions.example.json` to a private location and set `triggerCard` to an enrolled label. Launch with `--actions-file PATH`; the deployment launcher looks for `/init/transit-prototype/actions.local.json`. A missing/invalid file disables media while leaving the departure/card prototype usable. The configuration rejects unknown fields and bounds slide/session/audio timeouts.

Use a separate media directory on the existing USB stick and preserve all bootstrap files. Supply JPEG/PNG images already oriented and resized to fit within 800 × 480, with lowercase extensions. Up to 32 card/album mappings and 512 images per album are supported, with at most 2 MiB per prepared image. The app indexes paths at startup and decodes one display-sized photo at a slide transition rather than preloading every album into RAM. Decoding/USB reads currently happen on the UI thread at that transition; background prefetch can be added if this becomes noticeable. Images are sorted by filename and shown within the display's central 754 pixels without cropping. Missing/unreadable media returns to arrivals. Restart after replacing photos.

### Arrange albums on your development computer

Use `~/Desktop/Transit Albums/`, with a folder named exactly after each enrolled card label. Put that card's photos inside its folder; nested folders are included. Prefix filenames with `01-`, `02-`, etc. to control ordering. Empty card folders are skipped. You can export Google Photos albums here manually; no Google credentials are required.

`sync-albums.py` prepares resized, orientation-corrected JPEG copies (up to 754 × 480), strips EXIF/location metadata from those copies, verifies folder names against enrolled aliases, and uploads a new release into the USB's dedicated `transit-media/albums` directory. Originals are untouched. The private action file is updated to the per-card album map, retaining the chosen sound and durations; restarting the transient service loads the new map. Existing USB releases and the prior action file remain available for recovery; no unrelated USB files are removed. Old releases can eventually fill the drive, so the tool checks free space before uploading.

On the development machine, use an isolated Python environment for Pillow. macOS
`sips` handles HEIC/HEIF conversion; supported inputs are JPEG, PNG, HEIC/HEIF and
WebP. Run these commands from the repository root with the live app deployed:

```sh
PM3_TARGET='root@device-host'
python3 -m venv .venv-media
.venv-media/bin/python -m pip install -r pm3/requirements-media.txt

# First-time album setup only: keep an existing private action file.
ssh "$PM3_TARGET" 'umask 077; if [ ! -f /init/transit-prototype/actions.local.json ]; then cat > /init/transit-prototype/actions.local.json; fi' < pm3/albums.example.json

.venv-media/bin/python pm3/sync-albums.py "$PM3_TARGET"
```

The template's placeholder album is replaced by the sync tool's map of enrolled
folder names. No sound file is needed while `soundEnabled` is false. The tool
starts another one-hour live session after publishing the new map.

Use `--source /path/to/albums` for another source directory, or `--prepare-only` without a device argument to prepare local copies without connecting. Generated staging and the last-sync report live under ignored `pm3/albums.local/`. The tool preserves previous USB releases but does not automatically roll back a failed restart. A private action file can also use the `albums` map in `albums.example.json`; the original single `triggerCard`/`photoDirectory` form remains compatible.

`soundEnabled` defaults to **false**. A matching card immediately opens its slideshow without launching a player or enabling the amplifier. To opt into audio later, set `soundEnabled` to true; the configured WAV is then played by `/usr/bin/aplay` as a child process before the slideshow. Completion or the bounded timeout advances to photos, and failed audio does not block them. Taps have a two-second cooldown; a later same-card tap restarts the action. An unconfigured card does not interrupt it; a different configured card selects its own album. The session then returns to the departure board (live or sample, depending on launch arguments). Graceful exit stops any outstanding audio child. Mixer settings are not changed automatically.

The optional `enableAmplifier` setting is only consulted when `soundEnabled` is true. It uses the audited device's `/usr/bin/NxExe prox2 portinfo` and `prox2 amp` commands to enable the amplifier only if its reported state is 0. It stays enabled throughout the process to avoid toggling on every tap; graceful exit restores an initially disabled amplifier. An unknown state is not blindly toggled. This setting defaults to false when omitted and is specific to this PM3. Abrupt power loss cannot run restoration. A transient click may still occur as the amplifier/codec powers up. The local demo WAV has 10 ms fades and 100 ms of silence at each end; the original is retained locally.

## Experimental Japanese-card work

Suica and Hayakaken are **not supported yet**. The installed stock application
excludes FeliCa polling, and the experimental module-side routes have not reached
confirmed readiness. Built-in FeliCa work is paused following the
[parallel research review](../docs/pm3-felica-parallel-research.md), with explicit
resume criteria and an external-reader fallback recorded there.
See [research and references](../docs/pm3-japanese-cards-2026-10-21.md)
and [trial tooling](felica-trial/README.md). This is separate from the working
ISO/DESFire card-to-album integration.

Reader file/launch maintenance binaries require the explicit Cargo feature
`reader-maintenance`; normal builds do not include them. Build them explicitly
with `./pm3/build.sh --maintenance`. Experimental
`--felica-trial-file` mode can save bounded raw discovery data to a private file.
Do not enable it for normal playback or publish its captures. Vendor firmware,
libraries, device identifiers, card registries and personal media are not
redistributed with this source.

## Private NFC version diagnostics

`--reader-info-file PATH` saves the first NFC version reply to a private mode-0600 file. It does not send any additional command: the startup version query is already part of normal operation. The reply includes a device identifier, so it is not printed in logs or intended for Git. The launcher uses `/init/transit-prototype/reader-info.local.bin`; only platform/application version strings are extracted for public investigation notes. Card-event payloads are not captured by this option.

## QR scanner investigation

The FTDI serial interface `/dev/ttyUSB0` enumerates. Community notes specify 115200 8N1, with no CRLF on decoded output. The bounded `check-qr.sh` trial only changed the scanner power bit and UART parameters while listening for a generated harmless QR; it did not send scanner programming commands.

The first trial produced no illumination or decoded data, and SSH/ping became unavailable during the trial. The user reset the reader. The prior boot journal was not retained and no persistent crash record was found, so the cause is unknown; a kernel crash or scanner-caused power fault is not established. The scanner was observed at state 0 after reboot and was left disabled. Live arrivals were restored. Do not rerun the power trial until the scanner model, trigger behavior and power path have been investigated. The helper attempts UART/power restoration on normal exit, but reboot/power loss cannot run cleanup. QR scanning is not integrated into the app.

Local sample assets, source notes and personalized configuration live in ignored `pm3/media.local/`. The user-selected README demo is a separate, intentionally shared asset; see [demo media notes](../docs/media/README.md). The stock success sound is copied into app-owned storage for local use; its redistribution license is unconfirmed, so it is not tracked. A slight playback pop remains; further audio work is parked at the user's request.

## Hardware references and limits

The [RIPTA design document](https://kevin.wallace.seattle.wa.us/pm3/ripta-system-design-document.pdf), page 9, marks the touch panel/controller as optional. Page 12 specifies an 800 × 480 display with a smaller 754 × 480 visible area. Pages 13–14 describe the serial cVEND reader and ISO 14443 support. This document describes a product variant, not a physical inspection of this unit.

The actual reader enumerates no touch controller. Dedicated touchscreen drivers are disabled, but USB HID multitouch support is compiled in. With no touch device on the USB bus, changing a GUI option or loading an existing touch module cannot currently enable it. Physical inspection is needed to distinguish absent hardware from a disconnected or unpowered controller.

Transport framing and card-event message types follow the community [cVEND protocol notes](https://wiki.pm3.dev/cvend). Broader card recognition validation, screen power/schedule, system time synchronization, automatic startup, and full recovery backups remain subsequent work. Full stock OS images have not been backed up by this prototype; boot/firmware changes should wait for those backups.

Actual measurements and checks are recorded in [the prototype results](../docs/pm3-prototype-results-2026-10-21.md).

### Card-target illumination

The normal launcher enables `--reader-lights`: the first documented LED is on
while polling is ready; recognizing a card selects the second LED for one second,
then returns to the ready indication. Feedback also works during a slideshow.
The user observed slight pulsation in the physical LED trial; colors are not
assumed. Audio remains controlled separately and disabled by default.

Direct launches can omit `--reader-lights` to leave lighting unmanaged. Optional
CLI settings are `--ready-light-mask 64`, `--tap-light-mask 128`, and
`--tap-light-ms 1000`. Allowed masks are 0 (off), 64, 128, or 192 (both); tap
duration is 100–5000 ms. These values can also be changed in `restart-display.sh`.
LED commands are sent only on state changes, with no sleeps in the render loop.
Graceful shutdown switches managed LEDs off; abrupt power loss/SIGKILL cannot
run cleanup. Reader startup events reapply the indication after polling setup.
Protocol reference: [cVEND LED command 0x20](https://wiki.pm3.dev/cvend).
