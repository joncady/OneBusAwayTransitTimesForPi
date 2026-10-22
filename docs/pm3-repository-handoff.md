# PM3 repository handoff

## What is ready

The native app combines live OneBusAway arrivals, private card aliases and USB
photo albums. The user confirmed card → slideshow → live-arrivals return and
distinct labels for the currently enrolled cards. Sound is muted by default;
the amplifier pop remains parked. Photos are arranged on the development
computer and resized before USB publication.

The original Pi app remains available. PM3 uses Rust/Slint plus the installed
`curl`; Python/Pillow is a development-machine photo tool, not a PM3 runtime.
The PM3 and its embedded cVEND reader use different ARM targets.

| Area | Current state |
| --- | --- |
| Native board, live arrivals, card albums | Working integration, transient sessions |
| Audio | Disabled; pop unresolved |
| Japanese FeliCa cards | Experimental; no confirmed readiness or decoded IDm |
| QR | Deferred after a prior loss of connectivity; cause unconfirmed |
| Touch | No controller/input device exposed on this unit |
| Permanent startup, power schedule, full OS recovery | Pending |

## Source layout

- `pm3/src/main.rs`: Slint board and event loop.
- `transit.rs`: config validation, background API requests, time/cache/paging.
- `reader.rs`, `ipp.rs`: UART ownership and framed transport.
- `cards.rs`: salted local fingerprints and aliases.
- `media.rs`: USB albums and optional bounded audio playback.
- `felica_trial.rs`: explicit private diagnostic capture, requiring readiness.
- `pm3/src/bin/`: allowlisted reader-maintenance utilities, gated by the
  `reader-maintenance` feature.
- `pm3/felica-trial/`: separate, version-specific research. Not a working reader
  replacement and not installed by normal deployment.
- `pm3/REFERENCES.md`: sources and evidence index.

## Build and deploy

Follow [PM3 setup](../pm3/README.md). `./pm3/build.sh` compiles only; tests are
opt-in with `--test`. `--maintenance` also builds the separate maintenance tools.
Normal deployment does not run or install those tools.

`run-prototype.sh` installs `restart-display.sh` alongside the executable. The
latter runs on the PM3, validates live config before stopping the current app,
and recreates the transient systemd unit. Album sync uses this same helper;
it no longer relies on restarting a unit systemd may already have collected.
Restarting via this helper never enables experimental capture.

The source cleanup was compiled for ARMv7; the research variants were checked
separately against private matching libraries. Syntax/format checks are recorded
in the work summary. This is not a new physical-card, reboot or recovery test.

Latest cleanup checkpoint: the normal app and both maintenance binaries compiled;
all five documented research builds compiled against private matching libraries.
Shell/Python syntax, example JSON, local Markdown links and whitespace were checked.
No tests were run for this cleanup. The cleaned normal binary was deployed after
validating the existing private settings. Its one-hour service reported nine
albums, ten live departures and accepted ordinary card-polling configuration;
amplifier and barcode remained 0. No trial cleanup timers remained active.

The candidate source review found no matches for the current private stop values,
credentials, known personal labels or device/network identifiers. At that checkpoint, the only binary
asset in the candidate list was the previously approved U District screenshot.
This scoped review is not a guarantee against every possible sensitive value.

## Preserve private state

Before deployment or reader research, keep separate private backups of:

- `.env`, `config/transit.json` and `pm3/runtime.local/` on the computer.
- `/init/transit-prototype/cards.local.json` on the PM3. Its salt/fingerprints
  are necessary to keep existing album identities; do not recreate it casually.
- The PM3's actions, transit, display and API files, plus USB `transit-media/`.
- The working USB bootstrap. Media tools do not replace it.

Original photos, card labels/fingerprints, stop names/IDs, device/network
identifiers, extracted firmware/libraries, logs and discovery captures must stay
out of the GitHub change. They belong in ignored private directories, not in
example JSON. Generated photo copies strip EXIF/location metadata; originals
remain private and untouched. Do not force-add ignored files.

Review the actual candidate file list before staging:

```sh
git status --short
git ls-files --cached --others --exclude-standard
git diff --check
```

No commit or push is performed by these instructions. Review staged content
before creating the GitHub commit. No vendor binaries, firmware images, stock
audio or individual personal photos are needed in the repository. The approved
README demo is the intentional media exception; see [media notes](media/README.md).

## Stop, resume and rollback

```sh
ssh root@<device-host> 'systemctl stop transit-prototype.service'
ssh root@<device-host> '/init/transit-prototype/restart-display.sh 3600 live'
```

Graceful stop releases UART/card polling, stops audio and restores saved screen
pixels. It cannot recover after power loss or SIGKILL.

Deployment keeps `.previous` copies of the binary and existing transit/display/API
files; album sync retains its previous action file and older USB releases. These
are limited rollback copies. Stop the service before restoring a known compatible
set, preserve the card registry, then use `restart-display.sh`. Do not overwrite a
binary currently being executed or delete the active USB media release.

Reader research has its own private original launch-script backup and restores
that script before each one-time program. A module reboot normally selects that
original again. Full device images and out-of-band recovery are still pending;
this research tooling is not a substitute for them.

## Next FeliCa checkpoint

The latest diagnostics show that the adapter loads/preloads into an unprivileged
diagnostic program, but is rejected when preloaded into the stock reader app.
PCI-LSM is active. FEIG's older policy describes signed-app access controls;
app authorization is a strong explanation, not a proven complete root cause for
this exact firmware. See [investigation](pm3-japanese-cards-2026-10-21.md) and
the [unsent SDK/signing inquiry](pm3-felica-sdk-inquiry.md).

First obtain a documented authorized discovery route or compatible signed demo.
Then establish readiness, tap Suica twice with removal between taps, validate
the IDm layout, repeat with Hayakaken and confirm distinct identities. Only then
enroll those cards for albums. Further taps cannot fix service authorization.


## README/demo preparation — 2026-10-22

The root README now presents the PM3 integration first, with an inline animated
preview and a linked MP4. The original Pi workflow lives in `pi/README.md`. PM3
instructions cover first setup, key-based SSH, private stops/API settings, bounded
sessions, enrollment, initial action config, Desktop albums, isolated host media
dependencies, sync and restart. Japanese-card work links to GitHub issue #2.

The owner requested full framing and selective ORCA-number redaction. The
approved derivatives keep the full shot, remove audio/recording metadata and
mask only number-field regions. Original recordings and processing artifacts
remain outside the repository. The demo station's device-only configuration
was not copied into the development computer's private transit settings.

A small deployment correction handles a missing/collected transient unit before
config replacement and makes that remote deployment block fail on errors. It
was reviewed with shell syntax checks; this documentation/media pass does not
claim a new end-to-end deployment or physical-card test. Existing compile results
above predate this shell-only correction. No commit or push is performed here.
