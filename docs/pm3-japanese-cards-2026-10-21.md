# Japanese transit card feasibility — 2026-10-21

## Conclusion

The matching-version cVEND firmware package contains a FeliCa driver and a FeliCa detection path in its polling service. The packaged reader application excludes the FeliCa category from its registered card mask and has no FeliCa identifier serialization path. This is substantially stronger evidence of a reader-application limitation than the hardware-family datasheet alone. It remains offline evidence: the package has not been proven byte-for-byte identical to the running module, and the physical Japanese cards have not been detected successfully.

The previously pictured Japanese cards were Suica and Hayakaken. Suica uses FeliCa; Sony describes FeliCa as NFC-F. These are not the ISO/Type A and DESFire paths currently enabled by the prototype. For card-triggered albums we only need detection and a stable identifier, not fare balance/history or authenticated card operations.

## Evidence and limits

- The PM3 hardware description identifies its NFC module as a FEIG cVEND plug. [PM3 hardware overview](https://kevin.wallace.seattle.wa.us/pm3/)
- FEIG's cVEND plug flex datasheet explicitly includes Sony FeliCa in its supported contactless cards. This is evidence of hardware-family capability, not proof of matching firmware/application support in this installed unit. It also states that development tools and SDK are available on request. [FEIG datasheet](https://www.feig.de/wp-content/uploads/Data_sheet_cVEND_plug_flex_EN.pdf)
- Sony's documentation identifies NFC-F as the technology used by FeliCa and provides card/protocol documentation. [Sony technical information](https://www.sony.co.jp/en/Products/felica/business/tech-support/), [Sony NFC definitions](https://www.sony.co.jp/Products/felica/NFC/)
- The current community IPP reference documents card functions 4–10 (VDV-KA, MIFARE Classic, ISO, DESFire, Girogo, ITSO, Ultralight C), but no FeliCa polling/identifier command. The app enables only ISO and DESFire. Absence from community notes is not proof of firmware impossibility. [cVEND protocol notes](https://wiki.pm3.dev/cvend)
- Read-only inspection of the host stock application found a FeliCa RF-mask description specifically for a LEGIC 4200 driver. The audited unit uses cVEND, so that string is not a valid basis for applying a LEGIC configuration to this hardware. No relevant FEIG/FeliCa interface was established from that inspection.

## Next implementation checkpoint

1. Identify the installed cVEND module/application versions, then inspect matching SDK/protocol or an offline firmware copy for a documented NFC-F discovery/transparent interface. No guessed function-number sweep, card writes, firmware flash, or power toggles were performed.
2. If a supported path exists, add a bounded NFC-F polling/identifier adapter and validate repeat taps/removal and stability across reader reboot with the two physical cards. Keep identifiers in the existing private registry.
3. Add the resulting aliases to Desktop albums and sync those folders using the existing workflow. If the installed firmware exposes no usable path, an additional FeliCa-capable reader remains a fallback; no accessory has been selected or purchased.

The QR scanner remained untouched during this investigation. No changes to NFC polling, module firmware, keys, or card data were made.

## Installed version and offline package inspection

A private capture of the already-issued Version query identified:

- Platform firmware: **cS01.05.45-49.00-2-2**.
- Reader application: **02.35**.
- An existing host archive, **V0200.C72**, contains matching platform/application version files. Matching version strings do not establish a byte-for-byte match to the currently running module.

The archive was copied into ignored local evidence storage for offline inspection. Its ZIP contains a reader application update and a compressed UBIFS root filesystem. Only the application binary and relevant polling libraries were extracted for analysis; no bundled executable or installer was run. Firmware was not flashed, module files were not uploaded, and new NFC polling commands were not sent.

Concrete findings:

- The packaged `feig` reader application retains C++ symbols. It contains `ProxCardCtrl`, `Picc::_identify`, `RFIDReaderMultiApp`, and the existing ISO/DESFire/other known handlers. No named FeliCa handler was found.
- `Picc::_identify` at address `0x0000dad8` explicitly checks technology values 1 and 2, with an ISO/Type-A-style UID/ATQA/SAK path for the first value. Other values return without that identification path. This is static evidence of a frontend limitation, not proof that the underlying RF hardware cannot discover FeliCa.
- The constructor `RFIDReaderMultiApp::RFIDReaderMultiApp` at `0x0002eb4c` registers a card processor with constant `0x2e`. The meaning of its bits has not been established; it must not be labeled a FeliCa enable/disable mask or changed by guessing.
- The matching rootfs contains `libfeclr.so` with exported `feclr_start_polling`, `feclr_wait_for_card`, `feclr_select_protocol`, `feclr_set_poll_parms`, and `feclr_transceive`. These wrappers call the reader module's kernel interface; they are not equivalent to an existing documented host IPP command.
- The generic IPP command handler inspected offline involves SAM activation/deactivation, rather than providing an established FeliCa discovery interface. It was not invoked.

The next useful step is tracing the module's technology mapping and existing IPP serialization against the SDK/driver interfaces. A reader-side FeliCa identifier bridge may be necessary; this remains an inference. Any module application experiment needs a recovery path and comparison with the unchanged archive first. A guessed host function number or a PM3 UI setting is not a demonstrated solution.

Raw version data (which includes device identity), archive/binaries, library copies, and analysis output remain private/ignored. A temporary local UBI inspection tool was used; no package was installed on the PM3.

## Follow-up: driver, polling mask and identifier tracing

Further offline inspection of the same package established:

| Layer | Evidence | Implication |
| --- | --- | --- |
| Module startup | `/etc/init.d/S03feclr` loads drivers from `extra/devs`, `protos`, `techs` and `loops` | The packaged startup intends to load all supplied technology drivers; this does not confirm runtime load status. |
| FeliCa driver | `extra/techs/felica.ko` exports polling, collision and removal routines. Its `init_module` passes technology value `4` to `feclr_tech_init` before registering the technology. | The package contains actual FeliCa implementation, not merely a marketing string. |
| Polling service | `fetpfd` compares the returned technology with `4` at `0xc6b4`, branches to `0xc82c`, and logs `Felica detected`. That path checks the saved request bit `0x40` and submits category `0x40`. Earlier mask translation at `0xc1d0`–`0xc1f8` maps request bit `0x40` to RF technology `4`. | This establishes the FeliCa category/technology relationship for this package. These are internal module values, **not host IPP function numbers**. |
| Application registration | `RFIDReaderMultiApp` stores `0x2e` at `0x2eba0` and passes it to `fetpf_register_card_processor` at `0x2ebe0`. The library sends the 64-bit argument as an eight-byte registration parameter. | This application's registered mask has no `0x40` bit. The earlier unknown-mask finding can now be narrowed using the matching daemon. |
| Identifier handling | `Picc::_identify` handles technologies `1` and `2`, not `4`. `_createUnhandledCardTelegram` at `0xe8e4` serializes the identifier vector populated by `Picc`; its additional technology-data path is guarded by technology `1`. | Merely adding the FeliCa registration bit would not add IDm extraction. Hashing an empty/constant fallback packet could merge distinct cards. |
| Launch configuration | Packaged application's help exposes only version/help options. Its `run` script launches `feig` without card-technology options. | No supported command-line FeliCa switch was found. Absence of a discovered option is not proof that no undocumented interface exists. |

All addresses above refer to the extracted application, service or driver, not the PM3 host executable. Disassembly and the four additional package artifacts remain ignored locally. Neither executable was run.

### Recommended implementation route

Build a small **reader-module application/adapter** using the matching cVEND SDK. First establish module access and recovery, then register FeliCa alongside the current card types, extract the eight-byte IDm from validated technology data, and emit an explicit technology-tagged event to the host. Keep card removal/re-arm behavior bounded and preserve existing ISO/DESFire support. The host should salt/hash `technology + identifier` into the private registry and use the existing album actions. Raw identifiers should never appear in normal logs or tracked configuration.

Required prerequisites before an on-device reader application experiment:

1. Obtain the legacy cVEND SDK headers/toolchain or equivalent documented ABI for `libfeclr`, `libfetpf`, `tech_data`, FeliCa IDm layout, event ownership and release semantics. The current public SDK search did not locate matching headers. FEIG's datasheet says SDK/development tools are available on request; the current SDK page alone does not establish compatibility with this older cVEND OS. [FEIG cVEND datasheet](https://www.feig.de/wp-content/uploads/Data_sheet_cVEND_plug_flex_EN.pdf)
2. Establish read-only access to the running module's application and loaded-driver metadata. Compare actual application hashes with the archive, back up its launch configuration and confirm a recovery method that survives a failed application. Host SSH access does not establish SSH/shell access to the separate reader module. The packaged startup includes a fallback loader, but its presence is not a validated rollback procedure.
3. Prepare an isolated diagnostic application and reversible launch procedure. First milestone: one IDm-only detection for Suica and Hayakaken; then removal/re-tap and distinctness checks, followed by reboot stability and album integration.

There is no justified one-byte host configuration change to deploy at this point. No mask patch, reader application replacement, reset, unknown command probe or scanner control was applied. Existing arrivals, nine album mappings and muted audio were left unchanged.

### SDK inquiry draft

This text is prepared for the user to send; no message has been sent:

> I have a retired INIT PROXmobil3 with a first-generation FEIG cVEND plug module. Its reported platform is cS01.05.45-49.00-2-2 and its INIT reader application is 02.35. For a personal display project I only need Sony FeliCa/NFC-F discovery, an eight-byte IDm and card-removal events; no payment processing or fare-data access. Can you provide the compatible legacy cVEND SDK/toolchain and libfeclr/libfetpf headers or documentation, including the tech_data layout and FeliCa polling example? Is there a supported host interface for this operation, and a documented development/application recovery procedure for this platform?

## Album sync completed in parallel

The user's Desktop photos were prepared and published to the existing USB through `pm3/sync-albums.py`: **73 photos across nine card albums**. Originals were preserved; the new private action map loaded nine albums, and live arrival refreshes continued after restart. Previous USB media releases remain available. Source paths, private mappings and media are excluded from Git.

## Continued implementation: exact running software and reversible trial

The allowlisted `reader-files` utility retrieved the running application's binary,
fallback loader, launch script, startup selection and two polling libraries over
the existing GetFile interface. The application, loader and both libraries match
their extracted package copies **byte-for-byte**. Private backups are stored on
the host and development machine. The earlier package-match caveat is resolved
for these four files; this does not independently hash the entire running OS.

A first one-time launcher trial changed only app0 launch files, rebooted the NFC
module and restored its original launch script before running the unchanged
application. The original 909-byte script was subsequently retrieved and matched
byte-for-byte. An initial post-reboot read timed out during startup; a later read
succeeded. The PM3 host remained available and resumed live arrivals.

An experimental process-local adapter is now prepared using the traced ABI,
without waiting for vendor headers. It wraps the library imports rather than
patching the installed application binary. It forwards bounded discovery data
to an opt-in private host capture; it does not enroll guessed identifiers. The
one-time launcher restores the original boot script first and limits the observer
to 15 minutes. See [adapter design and limitations](../pm3/felica-trial/README.md).
This is a research trial, not confirmed Suica/Hayakaken support. Physical taps are
the next required evidence once adapter readiness is confirmed.

### Loader investigation and standalone discovery

The first shared-library builds did not load: the cVEND loader reported
`failed to map segment from shared object` and continued running the unchanged
application. The initial ARMv7 library was never established as executing; static
attributes and a later `uname -m` confirmed this module needs ARMv5TE. Rebuilding
for ARMv5TE, executable file permissions, legacy ELF layouts, GNU linking, and a
temporary RAM copy did not resolve the loader error. No security setting was
changed and no firmware image was installed.

A compact static ARMv5TE syscall probe executed on the module. Opening the trial
file and mapping it read-only and read/execute all succeeded. This establishes a
usable static application route, but does not explain the dynamic-loader error.

`rf-probe.rs` now implements a bounded standalone FeliCa-only discovery trial
using the exact start/wait/removal/stop ioctl structures recovered from the
matching installed `libfeclr`. The original application is paused during this
trial. The original launch script is restored first; two independent limits
return to the original application after 15 minutes, and host capture cleanup
is scheduled separately. Discovery bytes remain private, are capped, and are
not enrolled as guessed identifiers. Hardware readiness and physical taps remain
separate checkpoints. See [the trial README](../pm3/felica-trial/README.md).

The direct-driver executable returned `EPERM` opening `/dev/feclr0` and exited;
the original application resumed. Device permissions were not changed. The next
client uses the existing FETPF service. Its first provider (`libfepkcs11.so`)
reported token-not-present for the stock app0 slot and `0xb1` opening a session.
That code normally denotes a session-count limit, but process metadata showed
no competing `feig` instance and token metadata did not support assuming a stale
session. [PKCS #11 base specification](https://docs.oasis-open.org/pkcs11/pkcs11-base/v2.40/pkcs11-base-v2.40.html)
The current client explicitly obtains the v2 function table from the separate
`libcvend-pkcs11.so` provider. Only counts and return codes are logged; token
identity fields and discovery data remain private. No session belonging to
another application was forcibly closed.


### Final checkpoint: SDK/service access needed before taps

The explicit `libcvend-pkcs11.so` provider returned `0x30` (device error) on
initialization. A final proxy-provider client found slot 1 absent; enumeration
of present slots succeeded with **zero slots**. It exited without registering
an RF processor. No readiness event or Japanese-card discovery was obtained.
The cause of these access failures is not established, and these results do
not establish that the hardware lacks FeliCa support.

Trials are stopped. The retrieved original 909-byte reader launch script matches
its private backup byte-for-byte. Normal host operation was recreated without
trial capture; amplifier and barcode controls were both observed at 0. Existing
card identities and USB albums were preserved.

Next action is the legacy SDK/support inquiry drafted above, specifically the
supported app0 service/token initialization and custom-application deployment
procedure. No inquiry was sent. Physical taps become useful after successful
FeliCa processor readiness. No token was reset, PIN attempted, security setting
changed or firmware flashed.


### Further research: loader comparison and primary security documentation

A read-only ARMv5 diagnostic loaded the same trial adapter successfully using
`dlopen`, reporting a 4096-byte page size and errno 0. A follow-up also preloaded
it into the diagnostic executable successfully. In contrast, the stock `feig`
application rejected the same adapter before initialization. Its file is owned
by app0 with ordinary executable permissions, not a setuid executable. These
results narrow the issue to the stock application's execution context rather
than a generally invalid shared object. They do not identify the exact failing
kernel check.

Bounded module kernel diagnostics confirm **PCI-LSM activated**. They did not
provide a specific logged shared-library refusal reason. No policy was disabled,
no token reset and no vendor binary was modified.

New primary sources provide a stronger explanation:

- FEIG's [2015 cVEND security policy](https://listings.pcisecuritystandards.org/ptsdocs/Security_Policy-1439934808.11289.pdf),
  §7 and §9, distinguishes secure/development firmware and describes signed-app
  authorization for sensitive services. This is an older revision, so its exact
  enforcement details cannot simply be assumed for cS01.05.
- A [2019 FEIG engineer presentation](https://events.nuttx.apache.org/wp-content/uploads/2019/11/MJung_nx2019.pdf),
  slide 3, independently identifies cVEND signed application code and its SDK.
  Subsequent NuttX slides describe a different architecture and are not evidence
  for this module's Linux implementation.
- [FEIG's cVEND generic information](https://www.feig-electronics.com/cvend-generic/)
  confirms a vendor-supported custom-application route, but does not supply a
  publicly downloadable SDK for this legacy unit.

Application authorization is now a well-supported hypothesis for the combination
of working stock app, refused RF access and absent service tokens. It is not proof
that every observed error has the same cause. The next vendor/community questions
are concrete: compatible SDK, signing/authorization for discovery-only apps,
service initialization and approved FeliCa demo. See the updated
[unsent inquiry](pm3-felica-sdk-inquiry.md). No useful physical-card trial is ready.

### Parallel research decision

Three independent agents completed a public-code, vendor-documentation and
alternative-interface review. The newer cS01.05-family security policy resolves
the earlier policy-family mismatch; it still does not prove the exact RF denial.
No verified public built-in FeliCa implementation was found. Named vendor leads
and a documented external-reader/Pi fallback are recorded in the
[parallel research report](pm3-felica-parallel-research.md). Built-in work is
paused pending the report's concrete resume criteria. This pass changed no device
settings and requested no physical taps.
