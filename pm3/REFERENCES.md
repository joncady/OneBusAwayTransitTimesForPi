# PM3 references

Reference research checked 2026-10-21; local evidence updated 2026-10-22. Hardware versions and direct observations take precedence
over examples for another product variant. External pages are references, not
deployment scripts to execute unchanged.

| Reference | Used for | Limitation |
| --- | --- | --- |
| [Kevin Wallace's PM3 overview](https://kevin.wallace.seattle.wa.us/pm3/) | Hardware, dongle and USB bootstrap context | Individual units and dongle revisions differ. |
| [PM3 SSH setup](https://wiki.pm3.dev/ssh) | Establishing access before deployment | Preserve the working bootstrap and your private SSH keys. |
| [RIPTA system design](https://kevin.wallace.seattle.wa.us/pm3/ripta-system-design-document.pdf) | 800 × 480 panel, visible area, optional touch, reader interfaces | Product documentation; not proof optional hardware is fitted. |
| [Community cVEND notes](https://wiki.pm3.dev/cvend) | IPP framing, checksums and reader message types | Reverse-engineered protocol, not a complete FEIG SDK. |
| [OneBusAway stop-arrivals API](https://developer.onebusaway.org/api/where/methods/arrivals-and-departures-for-stop) | Live arrivals and server-time fields | API key and actual stop configuration stay private. |
| [Slint software renderer](https://docs.slint.dev/latest/docs/rust/slint/platform/software_renderer/struct.MinimalSoftwareWindow) | Native rendering architecture | This app pins Slint 1.14.1; latest docs may describe later APIs. |
| [FEIG cVEND plug flex datasheet](https://www.feig.de/wp-content/uploads/Data_sheet_cVEND_plug_flex_EN.pdf) | Reader-family FeliCa capability and SDK context | Does not establish the installed INIT app exposes FeliCa. |
| [FEIG cVEND generic](https://www.feig-electronics.com/cvend-generic/) | Vendor-supported custom app/SDK route | Current product information; request compatibility with the legacy unit. |
| [FEIG cVEND security policy](https://listings.pcisecuritystandards.org/ptsdocs/Security_Policy-1439934808.11289.pdf) | Older platform's signed-application model, §7 and §9 | Describes an older firmware revision, not an exact policy dump for this unit. |
| [FEIG cS01.05-family security policy](https://listings.pcisecuritystandards.org/ptsdocs/Security_Policy-1467754052.59556.pdf) | Matching firmware-family signed-app model | Does not prove RF discovery itself is signature-gated or identify our exact failing check. |
| [FEIG engineer's 2019 presentation](https://events.nuttx.apache.org/wp-content/uploads/2019/11/MJung_nx2019.pdf) | cVEND signed-code and SDK background, slide 3 | Later slides discuss NuttX proposals, not this Linux implementation. |
| [PKCS #11 v2.40 base specification](https://docs.oasis-open.org/pkcs11/pkcs11-base/v2.40/pkcs11-base-v2.40.html) | Standard return-code names during service diagnostics | Codes alone do not establish why this vendor service refused a session. |

## Related public implementations

- [haylinmoore/board](https://github.com/haylinmoore/board): OneBusAway Puget
  Sound arrivals on the PROXmobil3 using Rust/Slint, with desktop/framebuffer
  modes. A close match for the arrivals portion; its README does not claim
  card-triggered albums or FeliCa support.
- [bettse/PROXmobil3_trimet](https://github.com/bettse/PROXmobil3_trimet): native
  TriMet board; the author describes watchdog handling and QR stop selection.
  Our unit separately decoded a known QR in the 2026-10-22 passive trial;
  QR album integration is prepared locally, pending physical verification.
- [checkraisefold/rust-feig-app](https://github.com/checkraisefold/rust-feig-app):
  community host/cVEND IPP analysis. The inspected Rust sources have no FeliCa
  implementation or NFC-F references.
- [bettse/pmreader](https://github.com/bettse/pmreader): built-in cVEND plus
  internal SAM for HID Seos over ISO14443A/4. Its stated scope is Seos, not FeliCa.
- [nfcpy](https://github.com/nfcpy/nfcpy): generic NFC library including
  [Type 3/FeliCa handling](https://nfcpy.readthedocs.io/en/latest/modules/tag.html).
  Useful protocol reference; it does not by itself establish a usable transport
  or authorized polling route through this installed cVEND application.

These are source/research leads, not unreviewed deployment dependencies. No
third-party deployment script was executed and no source was copied into the app.
The search did not identify a verified public FeliCa implementation for the
installed PM3/cVEND combination; this is not a claim that none exists.

## Project evidence

- [Hardware/software audit](../docs/pm3-audit-2026-10-21.md).
- [Touchscreen findings (archived, not planned)](../docs/pm3-touchscreen-2026-10-22.md).
- [Native prototype measurements](../docs/pm3-prototype-results-2026-10-21.md).
- [Live-arrivals and slideshow integration](../docs/pm3-live-integration-2026-10-21.md).
- [Albums, successful QR validation and local album integration](../docs/pm3-qr-and-albums-2026-10-21.md).
- [Japanese-card investigation](../docs/pm3-japanese-cards-2026-10-21.md).
- [Parallel research and pause/resume criteria](../docs/pm3-felica-parallel-research.md).
- [Unsent legacy SDK inquiry](../docs/pm3-felica-sdk-inquiry.md).

Vendor executables, firmware images, extracted proprietary libraries and original
vendor scripts are private research inputs, not repository assets. Personal
photos, stock audio with an unconfirmed redistribution license, card registries,
device identifiers and raw captures are excluded. Example configs use placeholders.
