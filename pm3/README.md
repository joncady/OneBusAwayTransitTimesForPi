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

Defaults: train mode; before the first successful fetch, retry after 5 seconds and double up to a 60-second cap (`startupRetrySeconds`, optional, 1–60 seconds). After the first success, use one refresh cycle per 60 seconds after completion, one request per configured stop; three visible rows, paging every 10 seconds; stale after 180 seconds; cached arrivals expire after 900 seconds; failures back off up to 300 seconds. Fetches are sequential within one background worker with a 12-second request timeout and bounded responses. A failed stop makes the whole refresh fail so the previous complete snapshot is retained. Network failures never block NFC/rendering. Refreshes continue at the same bounded cadence during slideshows, so returning to departures uses the latest cache. Only the selected mode is fetched.

Countdowns and the Pacific clock use the API's `currentTime` anchored to monotonic elapsed time, refreshed each successful cycle. This avoids the device's incorrect wall clock without changing global time/RTC settings. The clock stays unavailable until the first successful fetch and disappears when the cache expires. System-wide time synchronization remains separate work; TLS still checks certificate validity against the OS clock, so a severely wrong OS date can prevent initial HTTPS access. This integration uses the new arrivals-while-idle requirement and does not enable the old Pi's scheduled screen blanking.

Request paths and fields follow the [OneBusAway arrivals-and-departures-for-stop API](https://developer.onebusaway.org/api/where/methods/arrivals-and-departures-for-stop). API keys are passed to curl through stdin, never command-line arguments or logs. Private config/API files are mode 0600 on-device. The card-triggered sound and slideshow run independently of live fetching, and the action returns to live departures when its configured session ends.

## Run from internal storage

SSH key access must already work. Leave the working USB bootstrap attached initially, because it suppresses the stock fare UI and handles the PIC watchdog; the binary and log run from internal `/init` storage.

```sh
./pm3/run-prototype.sh root@<device-host> 600
ssh root@<device-host> 'cat /init/transit-prototype/prototype.log'
```

The script refuses to launch while stock fare services or a previous prototype process are present. Persistent files go only into `/init/transit-prototype`, including the private `cards.local.json` alias registry. By default it launches a temporary systemd service for the specified number of seconds, with an additional runtime limit. This survives SSH session cleanup and installs no boot service. If automatic startup is already installed, live mode uses the continuous persistent service instead. SSH host-key checking remains enabled. Use the private host address outside this repository.

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

## QR scanner: decode confirmed, album integration awaiting device verification

On 2026-10-22 the built-in scanner decoded the exact `PM3-QR-CHECK` payload
from a phone through `/dev/ttyUSB0` at 115200 8N1. The user heard no beep.
SSH stayed responsive, uptime continued, and the transit app remained active.
This trial listened while the scanner light was already on; it sent no power,
trigger or programming commands. The `barcode` control remained 0, so that value
does not establish that the scanner is unpowered.

QR album support is implemented locally behind an opt-in configuration; it has
not yet been deployed or physically verified. See [local preparation and deployment](#prepare-qr-album-triggers-locally).
Per-album QR tokens and printable stickers are tracked as a [stretch goal](../BACKLOG.md#stretch-goal-qr-triggered-transit-photo-albums-and-printable-card-labels).
No label printer is required for the first prototype.

The earlier trial lost connectivity and required a reset; its cause remains
unknown. `check-qr.sh` is that older power-toggle trial, not the passive procedure
that succeeded. Do not use it as the normal scanner integration. Restoring UART
settings after the successful passive trial produced ioctl warnings, so future
integration must configure the port explicitly and handle reconnect/cleanup.
Full observations: [QR trial history and successful follow-up](../docs/pm3-qr-and-albums-2026-10-21.md).

Local sample assets, source notes and personalized configuration live in ignored `pm3/media.local/`. The user-selected README demo is a separate, intentionally shared asset; see [demo media notes](../docs/media/README.md). The stock success sound is copied into app-owned storage for local use; its redistribution license is unconfirmed, so it is not tracked. Audio remains disabled by default. On October 22, a manual trial produced an owner-confirmed clean chime by opening a continuous audio stream with the amplifier off, waiting three seconds before enabling it, then disabling it before closing playback. This sequence is not yet implemented in the application; playback startup while amplified remains the leading explanation for the pop. See issue [#8](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/8).

## Hardware references and limits

The [RIPTA design document](https://kevin.wallace.seattle.wa.us/pm3/ripta-system-design-document.pdf), page 9, marks the touch panel/controller as optional. Page 12 specifies an 800 × 480 display with a smaller 754 × 480 visible area. Pages 13–14 describe the serial cVEND reader and ISO 14443 support. This document describes a product variant, not a physical inspection of this unit.

The PM3 uses transit cards and QR codes as its interface. Touchscreen support
and touch wake are out of scope by the owner's decision. Earlier hardware
findings are retained only as [archived notes](../docs/pm3-touchscreen-2026-10-22.md).

Transport framing and card-event message types follow the community [cVEND protocol notes](https://wiki.pm3.dev/cvend). Broader card recognition validation, screen power/schedule, system time synchronization, automatic startup, and full recovery backups remain subsequent work. Full stock OS images have not been backed up by this prototype; boot/firmware changes should wait for those backups.

Actual measurements and checks are recorded in [the prototype results](../docs/pm3-prototype-results-2026-10-21.md).

### Card-target illumination

The normal launcher enables `--reader-lights`: the first documented LED is on
while polling is ready; recognizing a card selects the second LED for one second,
then returns to the ready indication. Feedback also works during a slideshow.
The user verified left-to-right tap feedback, the correct album opening and
the ready light returning. Slight pulsation was observed; colors are not assumed. Audio remains controlled separately and disabled by default.

Direct launches can omit `--reader-lights` to leave lighting unmanaged. Optional
CLI settings are `--ready-light-mask 64`, `--tap-light-mask 128`, and
`--tap-light-ms 1000`. Allowed masks are 0 (off), 64, 128, or 192 (both); tap
duration is 100–5000 ms. These values can also be changed in `restart-display.sh`.
LED commands are sent only on state changes, with no sleeps in the render loop.
Graceful shutdown switches managed LEDs off; abrupt power loss/SIGKILL cannot
run cleanup. Reader startup events reapply the indication after polling setup.
Protocol reference: [cVEND LED command 0x20](https://wiki.pm3.dev/cvend).


## Overnight shutdown, LCD timeout and automatic startup

LCD timeout and scheduled wake are not implemented on PM3. The Pi schedule does
not control this device. The planned behavior is configurable awake windows,
backlight timeout, paused API polling while asleep and card wake; see the
[LCD backlog](../BACKLOG.md#lcd-timeout-overnight-schedule-and-wake).

It is fine to disconnect PoE overnight after a graceful shutdown:

```sh
ssh root@<device-host> 'systemctl poweroff'
```

Allow shutdown to complete before unplugging. An illuminated LCD does not by
itself establish that Linux is still running. Keep the bootstrap USB attached.
After reconnecting power and waiting for SSH, start a new session:

```sh
ssh root@<device-host> '/init/transit-prototype/restart-display.sh 3600 live'
```

Transient session duration is 1–86400 seconds (24 hours). Without the opt-in
boot service, rebooting or reconnecting PoE does not automatically launch the app.
With automatic startup installed, live mode runs continuously after boot.
[Automatic startup and restart recovery](../BACKLOG.md#automatic-startup-and-restart-recovery)
tracks persistent systemd setup, graceful dependency handling, crash recovery,
preserved private settings and verification after a power cycle.

## Prepare QR album triggers locally

**Deployment status:** the QR-enabled app and private album map are deployed.
The serial port connected at 115200 8N1, nine albums loaded and live arrivals
continued under the persistent service. The first physical QR-to-album check is
pending. The earlier successful scan proved transport, not this new integration.
[Issue #3](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/3) stays open
until the on-device and printed-label checks below pass.

### Generate private codes and printable labels

Use the same album folder names as your existing Desktop albums. A QR-only album
may also use a new folder name; it does not require a working NFC card.

```sh
python3 -m venv .venv-media
.venv-media/bin/python -m pip install -r pm3/requirements-qr.txt
.venv-media/bin/python pm3/generate-qr-labels.py
```

This reads folder names under `~/Desktop/Transit Albums/` and creates:

- `pm3/qr.local.json`: private token-to-album configuration, mode 0600.
- `pm3/qr.local/*.png`: one code per registered album.
- `pm3/qr.local/labels.html`: an offline print sheet, 30 mm QR squares by default.
- `pm3/qr.local/manifest.local.json`: the private album-to-image index.

These paths are ignored by Git. The tool does not connect to the PM3, upload
photos, or change originals. Existing tokens are reused on subsequent runs so
printed stickers continue working. Back up the private configuration; deleting
it and regenerating creates different codes. Registered mappings are retained
when a folder disappears; remove obsolete mappings explicitly from private JSON.
Changing a token revokes its old code after the next QR deployment. Remove old
PNGs/sheets separately if no longer needed.

To choose individual album labels or another physical label size:

```sh
.venv-media/bin/python pm3/generate-qr-labels.py \
  --album '<album name>' --size-mm 30
```

`--album` can be repeated. `--source`, `--config` and `--output` override default
locations; `--show-labels` opts into printing personal album names below codes.
The default captions are generic numbers. PNG filenames use opaque tokens;
use the private manifest to find a particular album. Avoid publishing codes,
sheets or the manifest. Codes select albums; they are not authentication,
transit tickets, payment credentials, or a way to read fare balances.

Open `labels.html` in a browser and print at **100% / actual size**, with browser
headers/footers disabled. Start with plain paper or show a PNG on a phone. The
white quiet zone is part of the printed square; do not crop it. A small printer
is optional after physical size/distance checks. Keep important card markings
visible if applying a sticker.

### Configuration and runtime behavior

See [qr.example.json](qr.example.json). The example uses a dummy token and is
disabled; generate real private tokens with the host tool instead of reusing it.
The generator enables new configurations and preserves existing settings.

| Setting | Default / allowed values |
| --- | --- |
| `version` | `1` |
| `enabled` | `true` for generated config; set false to disable scanning |
| `device` | `/dev/ttyUSB0`; allowlisted `/dev/ttyUSB0`–`/dev/ttyUSB31` |
| `cooldownMs` | `2000`; 250–10000 |
| `frameGapMs` | `150`; 50–1000 |
| `reconnectSeconds` | `5`; 2–60 |
| `albums` | 1–32 opaque 32-character lowercase hex tokens mapped to configured album labels |

Payload format is exactly `PM3-ALBUM:1:<token>`; no URL, name or path is encoded.
Unknown codes, binary/malformed input and oversized records are discarded without
logging contents. CR/LF delimiters are accepted. For the installed delimiter-free
scanner, a quiet gap completes a scan; fragmented bytes within that gap are
combined. Concatenated scans without a delimiter or sufficient gap are rejected
as a whole. `frameGapMs` may need physical tuning; do not claim repeat-scan
reliability until it has been checked on the actual reader.

The app opens the serial port read-only/nonblocking, explicitly sets 115200 8N1,
and reads at most 2048 bytes per UI iteration with a 256-byte record cap. It sends
no scanner power, trigger or programming commands. A missing/disconnected port is
retried without stopping NFC or arrivals. An advisory lock prevents cooperating
app instances sharing the port; unrelated tools can ignore it, so do not run
`check-qr.sh` or other scanner tools alongside the app. Saved UART restoration is
attempted on close; driver failures are reported without stopping the display.

A mapped QR calls the existing album action, using the same two-second cooldown
across NFC and QR, slide/session duration, return to arrivals and optional audio
setting. Repeated scans of the already-active album do not restart its session;
NFC re-taps retain their existing restart behavior. A continuously presented code
can open another session after the current one ends, so remove it after scanning.
QR actions also request tap light feedback if reader lights are enabled.
Unavailable media mappings are skipped at startup; restart after restoring USB
media or changing configuration. Amplified WAV audio stays disabled unless you explicitly
change the existing action settings. Scanning an unknown QR never opens a URL or
executes a command.

### Deploy and verify when the device is available

The following commands **contact the device**. Wait until it is powered, ignition
has been handled if required, and SSH is available. The camera does not provide
remote ignition. No automatic boot service is installed by this QR feature.

For existing albums already synced to USB, no photo transfer is necessary. For
new QR-only albums, put photos in matching Desktop folders, start the existing
app if necessary, then sync before enabling the new QR mappings:

```sh
.venv-media/bin/python pm3/sync-albums.py "$PM3_TARGET" --qr-file pm3/qr.local.json
```

`--qr-file` permits explicitly mapped QR-only folder names in addition to enrolled
card labels and requires each mapped album to have photos. Without it, the
original enrolled-card-only sync behavior remains. Photo sync does not install
QR configuration; the next command deploys the app and map together.

```sh
./pm3/deploy-qr.sh "$PM3_TARGET" 3600
```

The opt-in helper builds, stages the app/map/launcher, validates QR album
references against the existing action configuration before stopping the app,
and restarts the continuous service if installed, otherwise starting a bounded live session. It preserves on-device transit stops, API
settings, card registry and action/photo files. Unlike `deploy-live.sh`, it does
not copy local transit settings over a device-only configuration. Use
`PM3_QR_CONFIG=/path/to/private.json` for a different map or append `--skip-build`
to use the already compiled binary.

The launcher includes `--qr-file` only when `/init/transit-prototype/qr.local.json`
exists. Direct launches opt in with `--qr-file PATH`. Configuration can be checked
without opening serial ports or playing audio using `--check-config --qr-file PATH
--actions-file PATH`; add the usual transit/display/env flags to validate those
as well. The existing action file must contain every QR-mapped label, even if
scanning is disabled.

The helper saves `prototype.before-qr`, `restart-display.before-qr.sh`,
`start-display.before-qr.sh` (if present) and, if
present, `qr.before-qr.local.json`. If immediate startup fails, it attempts to
restore them and restart the previous app. It is not a power-failure-atomic
upgrade or a full recovery backup. For manual rollback, stop the service, restore
the compatible app/launcher/map set (remove `qr.local.json` if previously absent),
then use the restored `restart-display.sh`. Never overwrite a running binary.

Physical acceptance checklist (two distinct phone-to-album selections have passed):

1. Present one generated PNG on a phone: correct album opens, tap light changes,
   slideshow returns to live arrivals, and amplified WAV audio stays muted. With
   reader feedback enabled, expect one confirmation beep for an accepted selection.
2. Present a second album's code; verify it selects a different album. Remove the
   phone and wait at least two seconds before repeating. Check that a held code
   does not keep resetting the active session; remove it before the session ends.
3. Present an unrelated QR; it should leave the current view unchanged. Tap a
   familiar NFC card to confirm the original album flow still works.
4. Try a plain-paper code at actual label size, then choose adhesive/printer size
   based on readable distance and placement.
5. Confirm scanner reconnect and configuration rollback while preserving SSH.
   Reboot/ignition testing waits for the owner's physical presence.

### Reader confirmation beep

The standard launchers enable `--reader-buzzer`: an accepted NFC or QR album
selection sends one documented cVEND `0x22` command through the existing reader
connection. It uses the reader's own buzzer, not ALSA or the Linux amplifier.
Unknown cards/codes, cooldown rejections and repeated QR scans of the active album
do not request a beep. Reader write failures disable buzzer feedback for that run
without cancelling the album action. A missing NFC reader leaves QR albums usable
without sound.

Direct launches are silent unless `--reader-buzzer` is supplied. Tune the tone
with `--buzzer-frequency` (default 1536 Hz, allowed 100–5000) and
`--buzzer-duration-ms` (default 256, allowed 20–1000). To mute the standard
launchers, create `/init/transit-prototype/buzzer.disabled` and restart the display;
remove the marker and restart to restore beeps. `soundEnabled: false` continues
to disable WAV playback independently. The amplifier remains off.

Two phone QR codes were physically verified selecting their distinct albums.
Printed stickers, scanner reconnect and a reboot with QR enabled remain to be
verified. The owner physically confirmed that the integrated card and QR selections each
give one clean beep and open the correct album. Amplified WAV playback stays off.

## Automatic startup (opt-in)

`transit-display.service` runs the live app continuously after the existing
`init-rc`/USB bootstrap completes. It does not replace the bootstrap or provide
ignition. Leave the USB attached. Network availability does not block launch;
the app retries promptly while DHCP/DNS settles and displays its existing
offline/cache status. After the first successful API fetch, regular polling and
error backoff apply.

Before the first install, save private recovery images of both internal cards,
SPI flash and the USB contents. The helper below applies only to the audited
storage layout and refuses a different mapping. It temporarily stops the app
and remounts `/init` read-only while imaging, then restores the writable mount
and prior app mode. Do not disconnect power during imaging.

```sh
./pm3/backup-device.sh "$PM3_TARGET"
./pm3/install-autostart.sh "$PM3_TARGET"
```

Recovery images are stored under USB `transit-recovery/` and copied to ignored
`pm3/recovery.local/` on the development computer. Local compressed checksums and
gzip integrity are checked. These images contain private credentials/settings;
keep them out of Git and shared media. Imaging is not a physical restoration test.
The USB file archive is not frozen; internal disk images are taken read-only.

The installer builds by default, validates private configuration, saves the
previous app/launcher and stock proxy enabled state, then installs a regular unit
file in `/etc/systemd/system/`. It enables the display and disables the stock
proxy's boot entry so it cannot compete for the reader. The stock NX program,
USB bootstrap, PIC watchdog setup, API settings, card registry and albums are
preserved. Root is returned to read-only if originally read-only. Install failure
attempts to restore the previous bounded app. No reboot is performed automatically.

For an already compiled binary, append `--skip-build`. `PM3_BINARY` can select
an explicitly prepared executable; it must support `--continuous`. A startup-only
trial can be built from the last working app plus that flag, leaving QR support
out until its separate acceptance test. The installer refuses to overwrite an
existing installed display unit; use normal app deployment helpers for updates.

The service uses `Restart=on-failure`, a ten-second delay, five starts per five
minutes, graceful SIGTERM shutdown, and journal logging. It has no poweroff
failure action. `--continuous` removes the app's prototype expiry; it is not a
hardware watchdog. Bad configuration can exhaust the start limit while SSH
remains available for recovery.

```sh
ssh "$PM3_TARGET" 'systemctl status transit-display --no-pager'
ssh "$PM3_TARGET" 'journalctl -u transit-display -b --no-pager -n 40'
ssh "$PM3_TARGET" 'systemctl restart transit-display'
# After fixing a repeated startup failure:
ssh "$PM3_TARGET" 'systemctl reset-failed transit-display; systemctl start transit-display'
```

Once installed, `restart-display.sh ... live` and album sync restart the
persistent live service; the duration argument applies only to transient
sessions. A sample-mode launch temporarily stops the persistent service and
runs the bounded sample instead. Resume live with `restart-display.sh ... live`.
Deployment scripts stop either service before replacing its executable. The
persistent and transient services cannot own the display/reader simultaneously.

For a cold-boot acceptance check, gracefully power off, disconnect/reconnect PoE,
and handle ignition physically if required. Wait for the app to appear before
issuing any manual start command. Read-only SSH checks can verify the boot ID,
service startup timestamp, live-arrival logs and whether the unit is enabled;
SSH must not be used to launch it during that check. Confirm an existing card
still selects its album. Record delayed-network, missing-media and crash-recovery
checks separately rather than assuming they passed from one successful boot.

To remove only automatic startup and restore the stock proxy's previous enabled
state (without starting fare software immediately):

```sh
./pm3/uninstall-autostart.sh "$PM3_TARGET"
```

The app and private files remain. `restart-display.sh 3600 live` then creates a
bounded session again. For emergency recovery without deleting files, stop and
disable `transit-display.service` and remove `/init/transit-prototype/autostart.enabled`.
Power management/LCD timeout is a separate feature; this service does not turn
the backlight off on a schedule.
