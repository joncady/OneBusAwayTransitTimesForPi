# PM3 QR trial and Desktop albums — 2026-10-21

## QR result: unconfirmed; investigation stopped after lost connectivity

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

Restoring the saved UART settings reported that not all requested operations could be performed. A subsequent status read showed 9600 baud and canonical/echo settings, with another ioctl warning. Exact restoration is therefore unconfirmed; the successful scan itself used 115200 raw mode. A future scanner integration should explicitly configure the port on open and handle these driver quirks. QR events are not yet integrated into the slideshow application.
