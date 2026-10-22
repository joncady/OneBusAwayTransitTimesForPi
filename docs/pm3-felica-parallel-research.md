# Japanese-card parallel research

Three independent research agents reviewed public PM3 code, FEIG documentation,
and alternative FeliCa interfaces. This pass made no device changes and ran no
tests. Findings are research evidence, not successful physical-card validation.

## Decision

**Pause built-in Japanese-card integration pending a concrete new interface or
authorized deployment path.** Existing arrivals and enrolled-card albums remain
the working integration. No new physical taps are useful at this checkpoint.

This search did not establish that FeliCa is impossible on the hardware. It found
no verified public implementation for the installed INIT application `02.35` on
platform `cS01.05.45-49.00-2-2` that resolves the observed access failures.

## 1. Public code: what is actually implemented

| Source | Inspected behavior | What remains missing |
| --- | --- | --- |
| [rust-feig-app](https://github.com/checkraisefold/rust-feig-app) | UART/IPP parsing and analysis | No FeliCa/NFC-F polling implementation found in its Rust sources. |
| [pmreader reader.go](https://github.com/bettse/pmreader/blob/main/internal/ipp/reader.go) and [card.go](https://github.com/bettse/pmreader/blob/main/internal/ipp/card.go) | Enables ISO polling; waits for B1/B9 events; ISO14443-4 APDU transport | No NFC-F discovery or IDm route. Seos support does not establish FeliCa support. |
| [zk_seos_reader](https://github.com/bettse/zk_seos_reader) and [PROXmobil3_eMRTD](https://github.com/bettse/PROXmobil3_eMRTD) | Related ISO card/APDU implementations | No verified built-in Japanese-card path found. |
| [pm3.dev cVEND code](https://git.doof.net/pm3.dev/tree/cvend/cvend.go) and [ORCA example](https://git.doof.net/pm3.dev/tree/cmd/orca/main.go) | ISO/DESFire/unhandled events; ORCA example selects DESFire | No NFC-F processor registration/IDm export found. |
| [PROXMobil3-HOP](https://github.com/Jarynnnn/PROXMobil3-HOP) | Serial-event-triggered UI | Does not establish distinct Japanese-card identity or FeliCa polling. |

The existing [OneBusAway board](https://github.com/haylinmoore/board) and
[TriMet board](https://github.com/bettse/PROXmobil3_trimet) are useful application
references, but do not supply the missing FeliCa reader interface.

## 2. Stronger firmware-family evidence and vendor leads

The [2015–2016 FEIG security policy](https://listings.pcisecuritystandards.org/ptsdocs/Security_Policy-1467754052.59556.pdf)
explicitly covers the **cS01.05** firmware family. Sections 7 and 9 describe secure
versus development variants and signed-app access to sensitive services. It
supports an authorization explanation, but does not prove that RF discovery
itself requires signing or identify the exact check behind our failures.

Two named vendor questions are now available:

- **CLRDemo / Contactless Demo Application:** a FEIG-authored application note
  located on a third-party document host describes a FeliCa-capable SDK example
  for older development firmware. This is a discovery lead, not an official
  distribution or a compatible binary. Request an authorized legacy example
  capable of polling and exporting IDm on this secure INIT configuration.
- **Transparent Channel Application (TCA):** FEIG's
  [public-transport documentation](https://www.feig-electronics.com/public-transport/)
  describes a separate closed-loop application channel. Its public description
  mentions ISO14443, so FeliCa and this legacy configuration remain unconfirmed.
  Ask specifically about NFC-F discovery and authorized coexistence with INIT.

The [inquiry draft](pm3-felica-sdk-inquiry.md) includes these requests. No message
was sent, application purchased, binary installed or security setting changed.

## 3. Concrete fallback with another reader

An external **Sony RC-S380/S** has a documented Linux/Pi software path:

- [Sony specifications](https://www.sony.co.jp/en/Products/felica/business/products/RC-S380.html)
  establish USB and FeliCa support.
- [nfcpy's support matrix](https://nfcpy.readthedocs.io/en/latest/overview.html)
  lists RC-S380/S as a tested device and includes Linux/Raspbian among its test
  platforms. This is compatibility evidence, not a stock/price recommendation.
- [Type 3 implementation](https://github.com/nfcpy/nfcpy/blob/master/src/nfc/tag/tt3.py)
  extracts the eight-byte IDm from the polling response. Our use needs only
  discovery, not fare data, authentication or card writes.
- [felica-rs](https://github.com/soltia48/felica-rs) provides Rust PaSoRi drivers,
  polling/IDm APIs and a TCP transport. Its
  [driver list](https://github.com/soltia48/felica-rs/blob/main/src/driver/mod.rs)
  contains no cVEND driver; it is an external-reader implementation lead.

| Option | Additional work | Tradeoff |
| --- | --- | --- |
| USB reader on spare Pi | Identifier-only event bridge to the PM3, removal/re-arm handling, private enrollment | Strongest documented OS/library combination; extra reader/computer and tap location. |
| USB reader directly on PM3 | libusb/native cross-build, available port/power check, physical trial | One computer; unverified on this PM3. |
| Vendor-authorized cVEND app | Compatible demo/TCA, deployment and recovery instructions, then physical validation | Preserves built-in tap target; external dependency remains unresolved. |

No extra reader was ordered or assumed to be available. These options are not
integrated. Multiple nearby NFC antennas would need a placement check.

## Resume criteria

Resume built-in work when any one of these is available:

1. A verified public implementation exposing NFC-F on this INIT app/version.
2. A compatible authorized discovery sample plus deployment/recovery instructions.
3. Confirmation and access to a FeliCa-capable TCA for this unit.

An external-reader prototype can be a separate next task once suitable hardware
is available. Then verify repeated scans and removal, distinct Suica/Hayakaken
IDm values, persistence across restart and album mapping before normal use.
