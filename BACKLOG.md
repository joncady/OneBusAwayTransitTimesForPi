# Project backlog

## Port transit display to PROXmobil3

**Status:** native Rust/Slint integration runs from internal storage with authenticated live OneBusAway arrivals, automatic paging, and the existing private card-triggered USB slideshow/sound. Nine card aliases are saved privately. Live fetching and paging were observed on-device on 2026-10-21. A slight audio pop is parked at the user's request. Production startup, recovery backups, broader failure/reboot validation, and optional controls remain pending. See [the audit](docs/pm3-audit-2026-10-21.md) and [prototype results](docs/pm3-prototype-results-2026-10-21.md).

The intended host is a PROXmobil3, replacing the Raspberry Pi. The actual unit has one ARM Cortex-A9 core at up to 792 MHz, 512 MiB physical RAM, an 800 × 480 framebuffer, custom Linux 4.9.166, and working 100 Mbps Ethernet/HTTPS. No Python, Node, or browser was found. Touch is not operational and must not be assumed from vendor configuration. Although the device tree declares TI WL1837 Wi-Fi and drivers are installed, no radio enumerates; the board documentation identifies an unpopulated Wi-Fi footprint. Physical presence still requires inspection of this unit.

Reference: [PROXmobil3 reverse engineering and boot notes](https://kevin.wallace.seattle.wa.us/pm3/).
Additional references: [board overview](https://wiki.pm3.dev/main_board_overview), [existing native OneBusAway board](https://github.com/haylinmoore/board).

### Checkpoints

1. **Inventory and connectivity — done:** Ethernet and authenticated, certificate-verified OneBusAway requests work on this device. Quiet pre-application baseline: ~337 MiB estimated available memory, ~99.9% CPU idle. These are not application benchmarks.
2. **Recovery and physical requirements — pending:** back up both internal cards and SPI flash. Confirm input/touch hardware and decide on Ethernet or an external wireless solution. Preserve SSH/bootstrap access.
3. **Time and power — pending:** correct the nearly five-hour UTC offset, choose Pacific timezone and time synchronization, and test vendor PIC LCD/backlight controls. Existing Pi DPMS behavior cannot be reused. Touch wake remains unproven.
4. **Native rendering experiment — passed:** static ARMv7 Rust/Slint prototype renders directly to the framebuffer with the installed font. Initial short sample: ~19.9 MiB RSS, ~2.1% CPU, one thread. NFC module replies and polling configuration are verified. Adding CardRelease fixed repeat taps; two physical cards each kept their private alias across repeated scans. Additional card types and stability across device power cycles remain to test. See the results for measurement limits; the current Node/Python implementation is not a constraint.
5. **Integration — live trial deployed:** train/bus modes are selected through validated private settings; refresh, paging, cache lifetime and backoff are configurable. One background worker refreshes the selected mode during arrivals and slideshow views. Arrival countdowns and the displayed clock use API server time plus monotonic elapsed time. `pm3/deploy-live.sh` builds/transfers private configuration and launches a transient service; permanent startup and recovery remain pending. The latest idle behavior is arrivals on-screen, so the old scheduled-off policy is not enabled.

**Do not reflash as the first step.** Keep the working OS for the first rendering experiment; treat replacing the board-specific OS/kernel as a separate decision after backups. Rust/Slint is the leading candidate based on the audit and an existing project for this device, not a committed architecture or measured application result.

### Follow-up requirements and prototype

- Ethernet is acceptable; the reader will live near the router. Onboard Wi-Fi is not required.
- Preserve cVEND NFC support for tapping physical ORCA cards. First prove generic card detection, then test stable identification for an enrolled card and add configurable visual/chime actions. Card recognition does not require fare operations; keep raw card identifiers and enrollment data private.
- **Card trial update:** Nine aliases are named in the private on-device registry. Eight use the current ISO UID extraction; the Taiwan keychain uses an experimental hash of its complete `UnhandledCard` message. Two separate keychain taps matched. Test stability across power cycles and separation from other unhandled cards before relying on that experimental identity.
- Touch is optional. The RIPTA design diagram marks touch hardware as optional; USB HID multitouch is compiled in but no controller enumerates. No working touch enable switch is established. Physical inspection is the next useful check if touch becomes important.
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
- The first authorized QR trial produced no illumination or decoded data. SSH/ping became unavailable during the trial, and the user reset the reader. No prior boot journal/crash evidence survived; cause is unknown. Live arrivals were restored, scanner control is back at its original 0, and further QR power/trigger trials are deferred pending model/control/power investigation. Do not label this as confirmed scanner support or a proven scanner-caused crash.
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
- Pending before permanent deployment: full recovery images, boot-service/watchdog integration, bounded log/media release retention, background photo prefetch if transition latency is visible, and repeated-tap/reboot identity confirmation. QR and audio remain parked.

### Japanese cards: paused after parallel research

Tracked on GitHub: [issue #2](https://github.com/joncady/OneBusAwayTransitTimesForPi/issues/2).

- Three research agents independently reviewed public PM3 code, FEIG authorization/demo options and alternative NFC interfaces. No verified built-in FeliCa route was found for INIT app02.35 on cS01.05. A matching firmware-family security policy strengthens the signing hypothesis without proving the exact RF denial.
- Concrete vendor leads: compatible authorized CLRDemo discovery sample or a FeliCa-capable TCA. Inquiry updated, not sent. Resume on a verified compatible host interface or authorized deployment path; no more physical taps requested yet.
- External fallback: supported Sony USB reader on the spare Pi using nfcpy, or a separately validated Rust/libusb helper on PM3. No purchase or integration performed.
- Details, source files and resume criteria: [parallel research](docs/pm3-felica-parallel-research.md). No device changes or tests in this research pass.
