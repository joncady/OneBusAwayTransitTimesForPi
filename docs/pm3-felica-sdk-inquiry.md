# Legacy cVEND inquiry draft — not sent

Use FEIG's [payment-terminal contact route](https://www.feig.de/en/payment-terminals/)
or ask the PM3 community whether an approved discovery-only demo is available.
The generic identification-reader SDK is not automatically the cVEND SDK.

**Subject:** Legacy cVEND plug: discovery-only FeliCa app / SDK and signing support

I own a retired INIT PROXmobil3 and am repurposing it as a personal photo/transit
display. Its cVEND module reports platform `cS01.05.45-49.00-2-2` and INIT reader
application `02.35`. The module runs ARMv5TE soft-float Linux, separate from the
PM3's ARMv7 Linux host.

I need only NFC-F/FeliCa discovery, the eight-byte IDm and card-removal events for
Suica/Hayakaken cards. I do not need payment processing, authentication, fare data,
keystore access or card writes.

The original application handles my ISO/DESFire cards. Offline inspection found a
FeliCa driver and service branch, but the INIT app's processor mask excludes it.
A standalone app executes, but direct RF open returns EPERM. The PKCS proxy
initializes but reports no present tokens; the stock app continues to operate.
An adapter preloads into my diagnostic program but is rejected when preloaded
into the stock application. The module reports PCI-LSM active. Original software
and launch files are backed up; I have not altered firmware/security settings.

Could you help with:

1. The compatible legacy cVEND SDK, GNU toolchain, `libfetpf`/`libfeclr` headers,
   FeliCa discovery example and discovery-data layout.
2. Whether a non-payment discovery app on this secure platform still requires
   vendor/customer signing or another explicit application authorization step.
3. The supported service/token initialization sequence and dynamic-library rules.
4. Whether an existing approved discovery-only demo or host protocol can return
   IDm, avoiding a custom privileged application.
5. A documented deployment and recovery procedure that preserves the current
   working application, including any supported development-device option.
6. Whether the legacy **CLRDemo / Contactless Demo Application** can be supplied
   as an authorized discovery-only example for this secure configuration.
7. Whether a **Transparent Channel Application (TCA)** for this generation
   supports NFC-F/FeliCa IDm, and can coexist with INIT app 02.35. The public TCA
   description mentions ISO14443, so I am not assuming FeliCa support.

Please advise which details you need privately. I can provide version information
without publishing a device serial number, keys or card identifiers.
