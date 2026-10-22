# PM3 prototype results

Date: 2026-10-21 Pacific time. See [source and instructions](../pm3/README.md) and the [hardware/software audit](pm3-audit-2026-10-21.md).

## Outcome

A native Rust/Slint software-rendered display runs on the actual reader, from internal `/init` storage, with its existing OS. The cVEND module accepts generic ISO and DESFire detection configuration. At least one physical-card event (`ISORead`, type `0xb1`) reached the prototype during the live trial.

This establishes display rendering, NFC communication, and a card-event path. The user then reported that only the first tap registered. A follow-up patch releases the reader session after each event and before enabling polling, and adds a private card-alias registry. In the subsequent physical test, two distinct cards each retained their assigned label across repeated taps. The user confirmed that distinct cards were recognized correctly. Broader card compatibility, stability across reader power cycles, chimes, live arrivals, touch support, and unattended deployment remain unverified.

## Checks performed

| Check | Observed result |
| --- | --- |
| Protocol unit tests | All 3 passed: known CRC-8/MAXIM and JAMCRC vectors; fragmented/noisy input; rejection of corrupt payload with recovery to the next valid frame. |
| ARM executable | Initial renderer build: 2,623,964 bytes (~2.50 MiB). Card-alias build: 2,679,716 bytes (~2.56 MiB). ELF32 ARM EABI5, statically linked and stripped; both executed successfully on the reader. |
| Reader-only probe | Ten seconds; 7 decoded NFC messages, zero rejected frames; version/status query response received; no framebuffer changes requested. |
| Polling configuration | Reader returned accepted replies `[0, 6, 0, 0]` (ISO) and `[0, 7, 0, 0]` (DESFire). |
| Native display | Actual framebuffer screenshot inspected: installed DejaVu font renders, three full departure rows fit, explicit sample-times label, 36-pixel side margins, reader status visible. |
| Resource use | Renderer build: 20,420 KiB RSS (~19.9 MiB), one thread. Ten-second sample: 21 process CPU ticks / 1,001 total CPU ticks, about 2.1% of the single core. Follow-up card-alias build: 20,468 KiB RSS (~20.0 MiB), one thread; its CPU usage was not separately benchmarked. |
| Rendering sample | Earlier 40-second run with the same UI/font renderer: 40 draws, 1,177.5 ms total rendering/conversion/write time (~29.4 ms per draw). Not an FPS or animation benchmark. |
| Automatic expiry | The 40-second trial exited normally after 40.0 seconds with 26 NFC messages and zero rejected frames. |
| Service stop / cleanup | Final build stopped through systemd/SIGTERM after a brief trial; immediate framebuffer readback matched the saved original pixels exactly. Console cursor blink was 0 during the prototype and returned to its original value of 1. |
| Card event | The initial trial recorded one ISO event but failed to re-arm. After adding CardRelease, the observed label sequence was Card 1, Card 2, Card 1, Card 2, with two recorded taps for each card; the user confirmed distinct cards. Raw UID/card data was neither printed nor persisted. |

The follow-up card-registry build has five passing tests: the original three plus normalization of a synthetic UID across supported ISO/DESFire layouts, and private registry persistence/label retention with mode-0600 files and no raw UID in saved JSON. This does not establish that every real card layout or mode switch is normalized correctly. The real test events were ISORead messages with 12-byte payloads. New cards receive generic Card N labels; agency attribution comes from the user. QuickTime's live webcam preview was opened and the physical display was observed without starting a recording.

Further scanning enrolled seven distinct identifiers. At that snapshot Card 1 had three recorded taps, Card 2 had two, and Cards 3-7 had one each. The user's scan order supplied seven physical card labels, now saved only in the private on-device registry. One other event was `UnhandledCard` (`0xbe`) with a 21-byte payload; a separate tap and user confirmation linked the unknown-format response to the Taiwan keychain. Seven identifiers are not proof of seven unique physical cards if any card randomizes its identifier; repeat tests are needed to validate attribution.

The follow-up experimental fallback hashes the entire `UnhandledCard` payload and message type in a separate namespace, without guessing a UID layout. Registry entries and the UI distinguish this method from decoded UIDs. All six unit tests passed; the added test covers repeat matching, changed-payload separation, separation from UID fingerprints, persistence, and absence of raw payload text in the saved registry. The update was deployed for another bounded one-hour trial; all seven existing aliases loaded successfully and a known card retained its name after restart. Two separate keychain taps produced the same new experimental label (21-byte messages), confirmed both by the user and metadata-only logs. That eighth alias was named privately on the device. Stability across a reader power cycle and separation from other unhandled cards remain untested.

Resource figures are short samples of a mostly static demo refreshing its elapsed label once per second. They exclude live API requests, scrolling/animation, audio, and any future settings server. They support further work on this architecture; they are not production performance or power-consumption guarantees.

## Device changes and launch behavior

The executable, PID, and small metadata-only log live under `/init/transit-prototype`. A transient systemd service runs the prototype independently of SSH session cleanup. The live trial was launched for 3,600 seconds, with a second service runtime limit; it should stop and restore the prior display when the application duration expires.

No boot service, package, kernel, firmware, system-clock adjustment, or permanent display-power setting was installed. Console blink is temporarily paused and restored on graceful exit. Reader polling functions enabled by the process are disabled on graceful exit, and the prior UART settings are restored. Abrupt power loss/SIGKILL cannot run cleanup.

The USB bootstrap remains attached for the existing startup/PIC-watchdog/recovery path. The prototype executable is not on that USB drive. Removing this startup dependency and installing a boot service are separate deployment work.

Initial trials found missing runtime font support and SSH-session background-process cleanup. The build now explicitly enables the software renderer's font support; the launcher uses a transient systemd service. The static build uses musl's normal empty `libdl` compatibility archive while loading the installed font directly, rather than depending on dynamically loading old OS font libraries.

Raw logs, screenshot, build output, and hardware evidence are kept in ignored `pm3-audit.local/`. They are not intended for Git.

## Touch and input decision

The provided [RIPTA system design document](https://kevin.wallace.seattle.wa.us/pm3/ripta-system-design-document.pdf), page 9, marks the touch panel/controller as optional. This reader enumerates no touch controller. USB HID multitouch support is compiled in; dedicated touchscreen drivers are not. No existing driver/module or GUI flag was found that makes touch available on this unit.

Leave touch out of the first version. If desired later, inspect the physical display/controller before considering kernel work. A confirmed card-event path could instead support wake or mode selection, independently of departures polling.

## Next work

### Sound/photo action follow-up

The device exposes ALSA playback and `aplay`. Three stock WAV assets were found under `/init/efad/tmp/wav`; the success sound (`ProxOk.wav`, 0.2 seconds, mono PCM16 at 44.1 kHz) was copied into app-owned storage. Existing PCM mixer volume was 49% (-21 dB). The first physical trial showed working photos but no audible sound despite successful playback. `NxExe prox2 portinfo` reported amplifier state 0; the documented `NxExe prox2 amp` toggle changed it to 1, followed by successful WAV playback. The user confirmed audible sound with a slight pop; no mixer setting was changed. The follow-up adds optional state-aware amplifier setup/restoration and a locally padded WAV with short fades; whether these reduce the physical pop remains unverified.

An optional validated action file now selects a private card alias, USB photo directory, sound path, slide/session durations, and audio timeout. The provisional trial uses a configured enrolled card, two licensed Seattle photos, five-second slides, and a 60-second session. Photos and attribution are on USB in a separate `transit-media` directory; development copies and private settings are in ignored `pm3/media.local/`. Stock audio redistribution rights have not been established, so it remains untracked.

The prototype preloads up to 16 bounded display-sized JPEG/PNG images before starting reader polling, plays audio asynchronously, and returns to the sample departure board after the slideshow. Other cards leave the session unchanged; the selected card restarts it after a two-second cooldown. macOS AppleDouble metadata files initially caused decoding to fail; the loader now skips hidden filenames. The rebuilt ARM executable was deployed for a one-hour trial, logged two loaded photos, and accepted reader polling configuration. The existing six protocol/registry tests pass; they do not exercise slideshow/audio behavior. This is not yet a live-arrivals implementation or a streaming media library.

The user confirmed photos work, and the live webcam preview showed a Seattle photo on the physical display. Runtime logs showed the configured card triggering the action and alternating photo indices.

The action returned to departures after its session, as recorded in the log. The amplifier update was subsequently deployed after restoring the manual trial's amplifier state to 0. Startup logged `Audio amplifier ready` and a port-state read confirmed 1. The faded/padded WAV is installed; reduction of the audible pop and amplifier restoration on exit still need physical validation.

1. Validate the configured sound/photo action across repeated sessions and test card label stability across a reader power cycle. Eight cards have private aliases; the keychain's whole-event identity remains experimental.
2. Capture full recovery backups before permanent startup, firmware, or OS changes.
3. Establish correct UTC, Pacific scheduling, and time synchronization before using live countdowns.
4. Add live OneBusAway requests, validated private configuration, stale/offline/error states, and configurable refresh/schedule to the native app.
5. Test LCD/backlight off/on and choose a wake policy. Ethernet is accepted; Wi-Fi is not required.
6. Install a persistent service and repeatable deployment/rollback once the prototype is validated. Keep a recovery route and account for both watchdog mechanisms.
