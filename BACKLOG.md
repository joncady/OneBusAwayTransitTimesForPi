# Project backlog

## GitHub issue tracker

Open work is tracked in GitHub issues. The sections below retain historical
checkpoints and implementation context; use the issues for current task tracking.

- [Japanese FeliCa card discovery — #2](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/2)
- [QR-triggered albums and printable card labels — #3](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/3)
- [LCD idle timeout, scheduled wake and card wake — #4](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/4)
- [Automatic startup and restart recovery — #5](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/5)
- [Full device recovery backups and deployment rollback — #6](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/6)
- [Reliable system time, timezone and clock recovery — #7](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/7)
- [Investigate speaker pop before re-enabling audio — #8](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/8)
- [Validate card identity and offline/media failure recovery — #9](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/9)
- [Bound log growth and retained media releases — #10, completed](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/10)
- [Prefetch slideshow images without blocking the UI — #11](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/11)
- [Optional LAN album hosting and remote photo management — #13](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/13)

### Maintenance checkpoint — 2026-10-22

- #6: compatible deployment rollback implemented and tested on-device; saved
  images verified and restored to regular files. Physical restore/boot remains open.
- #9: missing-at-boot media can recover on the next request; synthetic cache,
  partial-stop and media-failure tests passed. On-device startup API outage/recovery
  passed with private configuration preserved. Physical media/power-cycle cases remain.
- #10: bounded journal logs, protected release/snapshot retention and automatic
  cleanup after deployment/sync implemented. Disposable on-device pruning/log-cap
  tests passed. See [maintenance evidence and commands](docs/pm3-maintenance.md).

### Out of scope

- Touchscreen support and touch wake: [#12](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/12), closed as not planned. The owner chose transit cards and QR codes as the PM3 interface. [Archived findings](docs/pm3-touchscreen-2026-10-22.md).

## Port transit display to PROXmobil3

**Status:** native Rust/Slint integration runs from internal storage with authenticated live OneBusAway arrivals, automatic paging, and the existing private card-triggered USB slideshow/sound. Nine card aliases are saved privately. Live fetching and paging were observed on-device on 2026-10-21. A slight audio pop is parked at the user's request. Production startup, recovery backups, broader failure/reboot validation, and optional controls remain pending. See [the audit](docs/pm3-audit-2026-10-21.md) and [prototype results](docs/pm3-prototype-results-2026-10-21.md).

The intended host is a PROXmobil3, replacing the Raspberry Pi. The actual unit has one ARM Cortex-A9 core at up to 792 MHz, 512 MiB physical RAM, an 800 × 480 framebuffer, custom Linux 4.9.166, and working 100 Mbps Ethernet/HTTPS. No Python, Node, or browser was found. Touch is not operational and must not be assumed from vendor configuration. Although the device tree declares TI WL1837 Wi-Fi and drivers are installed, no radio enumerates; the board documentation identifies an unpopulated Wi-Fi footprint. Physical presence still requires inspection of this unit.

Reference: [PROXmobil3 reverse engineering and boot notes](https://kevin.wallace.seattle.wa.us/pm3/).
Additional references: [board overview](https://wiki.pm3.dev/main_board_overview), [existing native OneBusAway board](https://github.com/haylinmoore/board).

### Checkpoints

1. **Inventory and connectivity — done:** Ethernet and authenticated, certificate-verified OneBusAway requests work on this device. Quiet pre-application baseline: ~337 MiB estimated available memory, ~99.9% CPU idle. These are not application benchmarks.
2. **Recovery and physical requirements — pending:** back up both internal cards and SPI flash. Preserve SSH/bootstrap access.
3. **Time and power — pending:** correct the nearly five-hour UTC offset, choose Pacific timezone and time synchronization, and test vendor PIC LCD/backlight controls. Existing Pi DPMS behavior cannot be reused. Wake will use transit cards and QR codes.
4. **Native rendering experiment — passed:** static ARMv7 Rust/Slint prototype renders directly to the framebuffer with the installed font. Initial short sample: ~19.9 MiB RSS, ~2.1% CPU, one thread. NFC module replies and polling configuration are verified. Adding CardRelease fixed repeat taps; two physical cards each kept their private alias across repeated scans. Additional card types and stability across device power cycles remain to test. See the results for measurement limits; the current Node/Python implementation is not a constraint.
5. **Integration — live trial deployed:** train/bus modes are selected through validated private settings; refresh, paging, cache lifetime and backoff are configurable. One background worker refreshes the selected mode during arrivals and slideshow views. Arrival countdowns and the displayed clock use API server time plus monotonic elapsed time. `pm3/deploy-live.sh` builds/transfers private configuration and launches a transient service; permanent startup and recovery remain pending. The latest idle behavior is arrivals on-screen, so the old scheduled-off policy is not enabled.

**Do not reflash as the first step.** Keep the working OS for the first rendering experiment; treat replacing the board-specific OS/kernel as a separate decision after backups. Rust/Slint is the leading candidate based on the audit and an existing project for this device, not a committed architecture or measured application result.

### Follow-up requirements and prototype

- Ethernet is acceptable; the reader will live near the router. Onboard Wi-Fi is not required.
- Preserve cVEND NFC support for tapping physical ORCA cards. First prove generic card detection, then test stable identification for an enrolled card and add configurable visual/chime actions. Card recognition does not require fare operations; keep raw card identifiers and enrollment data private.
- **Card trial update:** Nine aliases are named in the private on-device registry. Eight use the current ISO UID extraction; the Taiwan keychain uses an experimental hash of its complete `UnhandledCard` message. Two separate keychain taps matched. Test stability across power cycles and separation from other unhandled cards before relying on that experimental identity.
- Touch is optional. The RIPTA design diagram marks touch hardware as optional; USB HID multitouch is compiled in but no controller enumerates. No working touch enable switch is established. Further touch work is out of scope; use transit cards and QR codes as the interface.
- Native Rust/Slint integration lives in `pm3/`: live or sample departures, optional NFC detection, and a configurable sound/photo action. Speaker audio is confirmed; a slight pop remains parked. See [instructions](pm3/README.md) and [results](docs/pm3-prototype-results-2026-10-21.md).
- Prefer internal `/init` storage for the prototype. Leave the USB bootstrap attached initially for recovery/startup access; remove that dependency only after a persistent startup plan and backups are ready.

### Next feature: arrivals and card-triggered photo slideshow

Requested behavior: show live arrivals by default. A configured enrolled card plays a terminal sound, then opens a photo slideshow; when the slideshow ends, return to arrivals. Start with USB media; an optional Raspberry Pi media/API server can follow later.

1. **Audio checkpoint — audible playback confirmed:** stock `ProxOk.wav` was copied into app-owned storage. The first trial was silent until `NxExe prox2 amp` changed amplifier state from 0 to 1; the user then heard it with a slight pop. Add scoped amplifier setup/restore, keep it enabled between taps, and try a locally faded/padded WAV. The existing mixer is unchanged. Playback cleanup and restart behavior remain to validate.
2. **Photo/action checkpoint — implemented for trial:** ORCA 1 is the provisional configurable trigger. Two licensed Seattle example photos and credits are in ignored `pm3/media.local/`; copies are on USB. Validated private action settings cover card alias, media directory, sound, slide duration, session duration, and audio timeout. Fit is currently contain; volume uses existing mixer settings. Trial: 5 seconds per photo, one 60-second session, immediate cuts. Same-card re-taps restart the session with a two-second cooldown; other cards leave it unchanged. The small prototype preloads at most 16 display-sized photos; streaming/refresh and volume/fit configuration remain follow-up work.
3. **USB photo prototype:** use a dedicated `/init/efad/mp_sd/transit-media/photos` directory without changing bootstrap files. Prepare orientation-corrected, display-sized JPEGs on the development machine; decode only current/next images with bounded dimensions and memory. Missing/unreadable USB media returns to arrivals with a recoverable status. Run sound asynchronously; missing/failed playback must not block the slideshow or NFC reader. Recheck the USB path when each session starts.
4. **Live arrivals checkpoint — deployed:** existing private stops and API key are used by the native app through the installed HTTPS client. Defaults: one refresh cycle per 60 seconds after completion, one request per configured stop, one worker, failure backoff to 300 seconds. Predicted/scheduled states, stale/offline messages and cache expiry are implemented; fault injection remains unperformed. Refresh continues during slideshows. API server time avoids incorrect countdowns without modifying the system clock; OS time sync is still needed for system-wide correctness and robust TLS startup after clock loss. Arrivals stay visible while idle.
5. **Deployment checkpoint:** perform recovery backups, install a persistent service with repeatable updates/rollback, and confirm reboot, card re-taps, missing USB, network loss, audio failure, and slideshow-to-arrivals recovery. No permanent startup changes until this checkpoint.

**Integration confirmation:** the user confirmed the full card → slideshow → live-arrivals return. Logs recorded successful arrival refreshes during and after the slideshow. See [live integration results](docs/pm3-live-integration-2026-10-21.md). Remaining deployment/failure checkpoints above are still pending.

### Desktop albums and QR follow-up

- Per-card USB album mappings are implemented and the backward-compatible native build is deployed. `pm3/sync-albums.py` prepares Desktop originals into resized, oriented JPEG copies and publishes a new USB release. The user-approved Desktop album directory and card folders are ready; syncing personal photos awaits the user's arrangement. Path indexing and one-photo-at-a-time decoding replace the previous 16-photo preload limit. Up to 512 photos per album and 32 mappings are supported.
- The first authorized QR trial produced no illumination or decoded data. SSH/ping became unavailable during the trial, and the user reset the reader. No prior boot journal/crash evidence survived; cause is unknown. Live arrivals were restored, scanner control is back at its original 0, and further power/trigger changes were deferred. A later passive trial on 2026-10-22 successfully decoded a known QR without a power toggle; see the stretch goal below. The original connectivity loss remains unexplained.
- The new post-reboot alias was confirmed to be an additional ORCA card, named privately, with a matching Desktop album folder. This was not evidence of identifier drift in an existing card; cross-reboot stability still needs its own confirmation.
- Details: [QR and album results](docs/pm3-qr-and-albums-2026-10-21.md).

### Japanese cards and completed photo sync

- Synced the supplied Desktop photos: 73 resized/oriented JPEG copies across nine card albums. The native app loaded all nine mappings and continued authenticated live arrival refreshes; originals and previous USB releases were preserved.
- Suica/Hayakaken support remains unconfirmed. FEIG lists Sony FeliCa support in the cVEND plug family, so hardware capability is plausible; the current documented PM3 IPP card functions do not establish an NFC-F/FeliCa polling path. A FeliCa setting found in the host binary belongs to a different LEGIC driver and was not applied.
- Next: identify the installed cVEND module/firmware and obtain matching protocol/SDK or inspect an offline copy for a documented NFC-F identifier path. Do not guess function numbers or flash firmware as a first step. Repeated-tap and reboot validation follow only after a supported path is found.
- Details and sources: [Japanese-card feasibility](docs/pm3-japanese-cards-2026-10-21.md). QR scanner controls were not touched.

### Muted playback and firmware investigation update

- The user confirmed the per-card albums work correctly. Sound is now disabled explicitly in the private action configuration; `soundEnabled` defaults to false in code, prevents starting `aplay`, and prevents enabling the amplifier. The deployed service retained all nine albums and live arrivals; the amplifier was observed at 0. Audio improvements remain parked.
- Captured the normal NFC version reply privately: platform `cS01.05.45-49.00-2-2`, reader application `02.35`. Found the matching-version existing `V0200.C72` host archive and inspected it offline. No reader firmware or module files were changed.
- The packaged application has ISO-oriented identification and no named FeliCa handler; the underlying rootfs includes FEIG polling/technology-selection libraries. A module-side bridge may be needed, but the technology mask and host exposure are not yet established. Continue static tracing/SDK matching before any live module change. See [the investigation notes](docs/pm3-japanese-cards-2026-10-21.md).

### FeliCa follow-up: application limitation traced

- The matching package contains `felica.ko`, startup loading of technology drivers, and a `fetpfd` FeliCa detection branch. Static tracing maps FeliCa RF technology `4` to processor category `0x40`; the reader application's registered mask `0x2e` excludes that category. This supersedes the earlier unknown-mask observation, subject to confirming that the running module matches the package.
- A mask change alone is insufficient: the application's `Picc` identification and fallback IPP serialization do not extract a FeliCa IDm. Do not enroll constant/empty fallback packets as unique Japanese cards.
- Next prerequisite: matching legacy cVEND SDK/ABI plus verified reader-module backup/access/recovery. Then build an IDm-only adapter and validate the two physical cards, existing card handling and album identities. An SDK inquiry draft and concrete evidence addresses are in [the Japanese-card notes](docs/pm3-japanese-cards-2026-10-21.md). No inquiry was sent and no reader firmware/application or scanner controls were changed.

Keep rendering, reader polling, network work, and audio nonblocking. Prefer direct PM3 OneBusAway access and USB photos initially. If remote photo management is useful later, a Pi can serve a manifest and resized photos over the LAN; the PM3 should cache them and continue working if that server is unavailable. A Pi API proxy is optional rather than required for the native port.


### FeliCa prototype checkpoint: waiting for SDK/service access

- Verified running reader application/libraries against the package and saved private backups. Reversible launch trials restore the original script before execution; the final retrieved script matches byte-for-byte.
- No trial reached FeliCa readiness: direct RF open was denied, the proxy provider exposed zero present slots, and the explicit provider failed initialization. Shared-library loading also failed. Root cause remains unconfirmed; no physical tap requested at this checkpoint.
- Normal transit/card albums restored, experimental capture stopped, audio muted. No security settings or vendor executable/firmware changed.
- Next: obtain matching legacy FEIG SDK and the documented custom-application/service initialization procedure. An unsent inquiry draft and exact versions are in the Japanese-card investigation notes. Then validate Suica/Hayakaken repeated taps and stable IDm before album enrollment.


### Repository preparation and further FeliCa research

- Adapter loads/preloads in a standalone diagnostic but fails in the stock application's context. PCI-LSM activation is directly observed. Primary FEIG security documentation supports a signed-application explanation; the exact failing check remains unproven. Updated SDK/signing inquiry is drafted, not sent. No Japanese-card readiness or IDm is claimed.
- Ordinary transit and private card albums restored after diagnostics, with audio still off. No firmware/security changes. Physical taps follow authorized discovery readiness.
- Gated maintenance binaries behind an explicit Cargo feature, made builds compile-only by default, added a shared transient-service restart helper and separated working setup from experimental research in docs. Added a reference index, handoff/privacy/recovery instructions and stronger private-file ignore patterns.
- Pending before permanent deployment: full recovery images, boot-service/watchdog integration, bounded log/media release retention, background photo prefetch if transition latency is visible, and repeated-tap/reboot identity confirmation. QR decoding was subsequently confirmed; album integration is now prepared locally, pending physical verification. Audio remains parked.

### Japanese cards: paused after parallel research

Tracked on GitHub: [issue #2](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/2).

- Three research agents independently reviewed public PM3 code, FEIG authorization/demo options and alternative NFC interfaces. No verified built-in FeliCa route was found for INIT app02.35 on cS01.05. A matching firmware-family security policy strengthens the signing hypothesis without proving the exact RF denial.
- Concrete vendor leads: compatible authorized CLRDemo discovery sample or a FeliCa-capable TCA. Inquiry updated, not sent. Resume on a verified compatible host interface or authorized deployment path; no more physical taps requested yet.
- External fallback: supported Sony USB reader on the spare Pi using nfcpy, or a separately validated Rust/libusb helper on PM3. No purchase or integration performed.
- Details, source files and resume criteria: [parallel research](docs/pm3-felica-parallel-research.md). No device changes or tests in this research pass.

### Stretch goal: QR-triggered transit photo albums and printable card labels

**Status:** deployed with scanner connected; two distinct phone-to-album selections verified; printed-label and reconnect acceptance pending in [issue #3](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/3). The built-in scanner decoded a known QR from a phone on 2026-10-22 without a beep; live arrivals remained active. See [scanner validation](docs/pm3-qr-and-albums-2026-10-21.md). This supersedes the earlier deferred/unconfirmed QR status above. The new opt-in QR module is deployed and maps tokens to the existing album action; two distinct phone-to-album selections are physically verified. Private PNGs and an offline print sheet have been generated. See [the setup/checklist](pm3/README.md#prepare-qr-album-triggers-locally).

- Allow each album to have a configurable QR trigger alongside its NFC card mapping. Scanning opens the same slideshow and returns to arrivals under the existing duration/cooldown rules. This can provide an album trigger for cards whose NFC technology is not supported.
- Generate QR images and printable label sheets on the development computer. Use opaque album tokens rather than personal names, card identifiers, local paths or credentials in the QR payload. Resolve tokens through private configuration; never execute commands or open arbitrary scanned URLs.
- Support phone-displayed codes first, then adhesive labels small enough to attach to transit cards. Include a short optional human-readable label and preserve the QR quiet zone and contrast. Validate physical label size, scanning distance and placement before selecting a small label printer or buying supplies; no printer purchase is required for the prototype.
- Integrate bounded serial reads, repeated-scan cooldown, safe handling of unknown codes, and port reconnect/recovery while keeping NFC, arrivals and photo playback responsive. Configure UART settings explicitly and address the restoration/ioctl warnings observed during the successful trial.

**Checkpoints:** (1) one configured QR opens an existing USB album; (2) multiple QR codes select distinct albums and unknown codes are ignored safely; (3) a printed sticker scans reliably on a card; (4) document generation, optional printer workflow and reboot/reconnect behavior. Use the reader buzzer for accepted album actions; amplified WAV playback remains disabled.

### LCD timeout, overnight schedule and wake

**Status:** requested; not implemented on PM3. The LCD currently stays illuminated.

- Add configurable idle timeout and scheduled awake windows, initially weekdays 07:00–08:00 in America/Los_Angeles. Keep settings in private configuration and permit disabling the timeout.
- Verify actual LCD/backlight off/on controls with a reversible experiment and state readback. A black framebuffer alone does not turn off the backlight. The vendor `prox2 backlight` command is a toggle; establish its semantics before using it automatically.
- Keep Linux and the card reader available while dark. Wake on a recognized card, start its album, then return to the appropriate arrivals/idle state. Add QR wake after scanner integration; touch wake is out of scope. Coordinate the ready light with the asleep state.
- Pause OneBusAway polling while asleep outside the awake schedule; refresh promptly on wake and preserve sensible stale/offline behavior. Resume scheduled arrivals automatically during the commute window.
- Establish reliable system time/timezone before wall-clock scheduling. Coordinate with permanent boot startup so power cycling restores the app and its intended display state.

**Checkpoints:** safely switch the backlight off/on without losing SSH; demonstrate a short idle timeout and card wake; verify scheduled wake/polling; verify reboot and recovery. Until then, use a graceful OS shutdown before disconnecting PoE for the night. The default transient service does not automatically return after power loss; the opt-in continuous boot service is now installed and its first cold boot verified.

### Automatic startup and restart recovery

**Status:** continuous app/service installed and enabled; automatic launch and live-arrival recovery verified after a physical cold boot. The updated app also passed a second cold boot with immediate arrivals and zero restarts. Controlled retry timing and broader recovery acceptance remain pending. The original transient mode remains available. See [startup trial](docs/pm3-autostart-2026-10-22.md).

- Install a persistent systemd service that launches the native transit app automatically after boot, preserving existing USB bootstrap and SSH recovery access.
- Remove the prototype's bounded runtime for permanent operation; retain a separate bounded diagnostic launch option. Restart on unexpected failure with a delay and bounded retries, without invoking the stock service's poweroff behavior.
- Handle network/API unavailability at startup and missing USB albums gracefully. Show arrivals/offline status while retrying dependencies, and keep card polling and reader illumination working.
- Preserve private transit settings, card registry, album mappings and muted audio across updates and reboots. Coordinate boot-time LCD state with the timeout/schedule feature.
- Document installation, service status/logs, manual restart, disable/uninstall, and rollback. Complete recovery backups before enabling permanent startup.

**Acceptance:** after a graceful shutdown and PoE reconnect, the device starts the app without SSH intervention, shows the configured arrivals once networking is ready, and recognizes existing cards/albums. Confirm controlled reboot, delayed network availability, missing USB media, and app-crash recovery without a reboot loop. Confirm remote recovery remains available.
