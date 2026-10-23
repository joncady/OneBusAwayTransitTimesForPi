# PM3 QR trials and Desktop albums — 2026-10-21–22

**Current result:** the follow-up on 2026-10-22 successfully decoded the exact
test QR from a phone without a beep, power toggle, reboot or loss of SSH. QR album
integration has since been deployed; the owner confirmed two phone codes select their distinct albums. Printed labels and scanner reconnect remain unverified. See [the successful follow-up](#follow-up-validation--2026-10-22-successful-decode).

## First QR trial: unconfirmed after lost connectivity (historical)

The user authorized a QR trial. The scanner's FTDI port enumerated at `/dev/ttyUSB0`, and no existing process held it. Community notes describe 115200 8N1 and QR bytes without a CRLF delimiter. A harmless generated test QR contained `PM3-QR-CHECK`.

The bounded `pm3/check-qr.sh` helper saved UART settings, changed the reported scanner control bit from 0 to 1 via `NxExe prox2 barcode`, and listened for the expected QR for up to 180 seconds. It did not send scanner programming commands. It was designed to omit arbitrary scanned content from logs, remove its temporary capture, and restore UART/control settings on normal exit.

The user saw no light or response. No matching QR or received-byte report was observed. SSH and ping subsequently stopped responding; a webcam preview still showed the departures display. The user reset/rebooted the board, after which SSH returned. These observations do not establish whether the whole system crashed, network access failed, or power was interrupted, nor whether the scanner control change caused the incident.

After reset, uptime indicated a recent boot, the journal listed only the current boot, and no retained pstore crash record was found. Therefore the previous failure cannot be diagnosed from available logs. The scanner control reported its original value of 0. No further scanner commands were sent, and QR scanning remains outside the main application. The trial's normal-exit restoration cannot be assumed after a reset.

The known live-arrivals/card/photo application was restored and completed successful arrival refreshes. Further QR work should identify this unit's scanner model and trigger/control semantics and check the power path before another power/trigger trial. A scanner-related crash or hardware defect has not been proven.

## Desktop albums

The user chose `~/Desktop/Transit Albums`. Folders matching the existing private card aliases and arrangement instructions were created without overwriting existing files. Only the empty initial example-card folder was seeded with the two Seattle demo photos and their attribution. Personal photos have not been synced by this work yet.

`pm3/sync-albums.py` prepares copies on the development computer using Pillow, with macOS `sips` for HEIC conversion. It preserves originals, applies EXIF orientation, resizes to fit 754 × 480, strips EXIF metadata from the generated copies, and encodes ordered JPEGs. Folder names must match enrolled private card labels. Empty folders are skipped; up to 32 mappings and 512 photos per album are supported.

The sync flow stages a new release under the USB's dedicated `transit-media/albums` directory, verifies USB mount/free space and enrolled labels, and updates the private card-to-album action map. Previous USB releases and the prior action file are retained. It restarts the existing transient service; this still does not install automatic startup. The end-to-end Desktop album transfer remains pending until the user arranges the desired photos. Generated staging/reports remain ignored by Git.

The native media loader now supports the per-card `albums` map while retaining the legacy single-card configuration. It indexes paths at startup and decodes one bounded, display-sized image per slide transition rather than loading every album into RAM. An unreadable image returns to live departures. Image decode/USB reads at that transition are currently synchronous; background prefetch is a later optimization if needed.

The ARM build succeeded with unrelated unit tests skipped. The backward-compatible album-capable app was deployed after recovery, with the prior single-card configuration preserved. The service is active and authenticated live arrivals refreshed successfully. Scanner control stayed at 0.

One new generic alias was recorded after reboot. The user confirmed it was an additional ORCA card and supplied its private name; this does not demonstrate identity drift in an earlier card. The new alias was named privately and its Desktop album folder added. Card identity stability across a reader power cycle still must not be assumed from earlier repeat-tap results alone.

## Follow-up validation — 2026-10-22: successful decode

With the user reporting that the scanner light was already on, a passive UART trial at 115200 8N1 decoded the exact generated payload `PM3-QR-CHECK` presented on the user's phone. The user heard no beep. This confirms the built-in scanner can read a QR code and deliver its contents to the host; audible feedback is not required for a successful scan.

No scanner power/trigger commands were sent. The reported `barcode` control value remained 0 throughout, so that value should not be treated as proof that the scanner is unpowered. The temporary serial reader closed after the match. SSH remained responsive, uptime continued without a reboot, and `transit-prototype` remained active. Arbitrary scan contents were not printed and the temporary capture was deleted.

Restoring the saved UART settings reported that not all requested operations could be performed. A subsequent status read showed 9600 baud and canonical/echo settings, with another ioctl warning. Exact restoration is therefore unconfirmed; the successful scan itself used 115200 raw mode. The new local scanner integration explicitly configures the port on open and reports restore failures; those driver paths still need physical verification. At the transport-trial checkpoint, QR events were not yet integrated into the slideshow application.


## Local album integration preparation — 2026-10-22

With the device powered off, added an opt-in native scanner module, private
validated token-to-album configuration, shared NFC/QR slideshow cooldown, and
bounded input/reconnect behavior. Input opens read-only at 115200 8N1; no scanner
power, trigger or programming commands are sent. Idle-gap framing handles the
observed delimiter-free transport, pending physical timing confirmation.

The host label tool generated nine private PNG codes and an offline HTML print
sheet from existing Desktop album folder names. Tokens remain stable across
subsequent generation. QR-only folders are permitted explicitly through the sync
tool's `--qr-file` setting. The deployment helper preserves existing on-device
transit/card/media settings and validates the app/map before stopping the app.

The ARM release compiled. No new tests were added/run, no device was contacted,
and this does not establish end-to-end QR album or printed-label operation.
See [setup and physical acceptance](../pm3/README.md#prepare-qr-album-triggers-locally).


## QR deployment prepared for the owner — 2026-10-22

The owner confirmed the original NFC album/light flow still works after the
second boot and requested QR preparation for their return. Deployed the full
QR-enabled native app and private nine-album token map using `deploy-qr.sh`.
Incoming configuration validated before stopping the prior app. The persistent
live service restarted successfully, with zero automatic restarts. Its scanner
port connected read-only at 115200 8N1, all nine albums loaded, both NFC polling
modes were accepted and live arrivals refreshed. Amplifier state remained 0.
Original transit/display settings still matched their pre-demo copies by hash.

Two private phone-test PNGs and an offline switchable preview were prepared in
ignored `pm3/qr.local/`; the first PNG was queued to open in the app. The owner
will present the codes when back. No QR-to-album physical scan, printed-label,
scanner reconnect or QR-enabled reboot acceptance is claimed yet. Tokens,
manifest, personal photos and private paths are not published in the repository.

## Physical album acceptance — 2026-10-22

The owner presented two generated phone QR codes and confirmed that both opened
their respective USB albums as expected. These are private codes and mappings,
not shared documentation assets. This supersedes the earlier transport-only and
local-preparation checkpoints. Printed-label size/reliability, unrelated-code
physical checks, scanner reconnect and reboot with QR enabled remain open in
issue #3. Reader-buzzer feedback is separate from amplified WAV audio; the latter
remains disabled.

After deploying reader-buzzer integration, the owner confirmed that a card tap
and a QR for another album each produced one clean beep and selected the correct
album. Linux amplified WAV playback remained disabled. All nine Rust unit tests
passed, including accepted/rejected action and shared cooldown checks.
