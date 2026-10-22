# Temporary FeliCa discovery trial

## Status: SDK/service access required

Built-in FeliCa research is paused after a three-agent review. See
[findings and resume criteria](../../docs/pm3-felica-parallel-research.md).

The reversible experiments ended without FeliCa processor readiness. Normal
transit and card operation is restored; physical taps are not useful yet.

- Direct `/dev/feclr0` access returned `EPERM`.
- The proxy provider initialized, but slot 1 returned `0xe0` (token absent).
  Enumeration succeeded with **zero present slots**, so no public slot was
  available. Earlier bounded FETPF session attempts returned `0xb1`.
- The explicit `libcvend-pkcs11.so` provider returned `0x30` (device error) on
  initialization. The cause is not established.
- The shared-library observer was rejected by the stock app's loader. It
  loads and preloads successfully in the standalone diagnostic executable;
  general shared-object validity is no longer the leading explanation.

Compatible legacy FEIG SDK documentation and a documented application/service
initialization or deployment route are prerequisites. No PIN was attempted,
token reset, platform security setting changed or firmware flashed. The final
retrieved 909-byte `run` script matches the original backup byte-for-byte. Host
trial capture is disabled. See the investigation notes for an unsent SDK inquiry.

## Investigated approaches

The service trial builds **`rf-probe.rs`** with `--cfg fetpf_client`, includes
`service-client.rs` and `provider.rs`, and uses `observer-rf-once.sh`. It
temporarily runs instead of the reader application, registers with the existing
FETPF service, and forwards discovery through the private diagnostic IPP message
described below. It polls
FeliCa only; other card handling returns when the original application resumes.
Readiness is emitted only after the service accepts card-processor registration.
The program and an independent shell watchdog both bound the trial to 900 seconds.
The host capture mode also has a one-time cleanup timer.

The shared-library approach below is retained as investigation code, **not a
working deployment route**. The installed loader rejected both LLVM- and
GNU-linked libraries with `failed to map segment from shared object`, including
from RAM. A separate 4 KiB static mapping probe executed successfully and mapped
the trial file with both read and read/execute permissions. This did not establish
the cause of the loader failure. The module reports `armv5tejl`; it does not share
the PM3 host's ARMv7 target. No platform security setting was changed.

The service client owns `/dev/ttyS0` only for this trial,
restores UART settings and releases its service context on normal exit, and never sends an
APDU or accesses a card's fare data. The shell launcher restores the original boot
script before running it and starts the original application afterward. After
physical taps, the private discovery structure must still be decoded and checked
for a stable, distinct IDm before adding album identities.

The earlier static direct-driver build could execute, but opening `/dev/feclr0`
returned `EPERM`. That route was abandoned without changing permissions. The
first service build linked `libfepkcs11.so`; its slot 1 reported token-not-present
and opening a session returned `0xb1`. Process metadata showed no competing
`feig` instance. One build explicitly binds the standard v2 function
table from `libcvend-pkcs11.so` and forwards only the service's required functions.
It logs session counts only, never token labels, serials, PINs or keys.

Build the explicit-provider client with `build-service.sh` and private matching module
libraries. `build-probes.sh` builds the earlier diagnostic-only static programs.
No vendor binary is distributed. Source and firmware versions must be matched
before using the allowlisted staging tool; it is not a general deployment tool.

## Earlier process-local adapter

Experimental, specific to the backed-up INIT reader application **02.35**, SHA-256
`429b5c8da8fe62f2cf667a8f8a3a65d5cd19fedc37ba4929f1cf552e93903625`.
This runs inside the separate cVEND module, not the PM3 host. It is not a portable
reader driver or a firmware image. Do not deploy it to another application build.

## Operation

- The module's normal executable is preserved. A one-time shell launcher restores
  the original `run` script atomically before starting an application with this
  library in its process-local `LD_PRELOAD` environment.
- The library wraps the imported `fetpf_register_card_processor` and
  `fetpf_process_card` interfaces. It adds the traced FeliCa category to the
  existing mask and wraps the card-found callback, preserving the stock callback
  and release/ownership logic. Runtime instruction guards supplement the required
  full software hash comparison performed before staging.
- FeliCa discovery is forwarded through the existing application's IPP send
  function. Diagnostic message `0xf1` carries `[1, 0]` for adapter readiness or
  `[1, 4]` followed by the 512-byte discovery union. This is an experimental local
  extension, not a documented FEIG message or decoded card identifier.
- The host must explicitly opt in with `--felica-trial-file NEW_PRIVATE_PATH`.
  Captures are exclusive-create, mode 0600, capped at 64 records. Logs contain
  readiness and scan counts only. After readiness, trial fallback `0xbe` packets are released
  without enrollment; the established registry and album mappings are preserved.
- The trial launcher limits the observer to 900 seconds, then starts the normal
  application. Reboot also selects the restored original launch script. The
  first, observer-free launcher check was read back and compared byte-for-byte.

No authenticated card operations, payment operations, keys, balances, travel
history or card writes are involved. Discovery data is private and is not yet
treated as a validated IDm. The ABI was derived from the exact backed-up binaries;
compatible vendor headers remain preferable for a maintainable implementation.

## Local tooling and evidence

`reader-files` is an allowlisted, bounded software backup utility using the
installed application's existing GetFile operation. `reader-trial` stages only
trial files and the original-backed-up launch script, verifies all writes by
readback, and exposes explicit restore/fetch/module-reboot actions. Neither is
part of normal transit polling. Stop the transit service before using either;
both acquire the NFC serial lock.

`Dockerfile` adds the module's **ARMv5TE soft-float** Rust target to `pm3-transit-build`.
`adapter.rs` builds as a `no_std` shared library against the private, matching
`libdl.so.2`; the normal host application uses a different ARM hard-float target.
No vendor library or firmware binary is distributed in this repository.

Launch assets, original software, module backups and captures are held in ignored
`*.local` directories or private device storage. The native host accepts the
diagnostic messages only while explicitly in trial mode. A permanent service,
IDm parser, stable identity enrollment and Japanese-card album mapping are later
checkpoints, following physical card validation.


## Reproducible build variants

First build the PM3 image using `../build.sh`. Research builds additionally need
private, version-matched module libraries in ignored `pm3-audit.local/` at the
repository root. Never download arbitrary ABI substitutes or commit those files.

```sh
./pm3/felica-trial/build-service.sh cvend  # explicit provider, fetpf-probe
./pm3/felica-trial/build-service.sh proxy  # proxy provider, fetpf-proxy-probe
./pm3/felica-trial/build-service.sh loader # read-only dlopen diagnostic
./pm3/felica-trial/build-adapter.sh       # process-local observer, not operational
./pm3/felica-trial/build-probes.sh        # mapping and raw-driver diagnostics
```

Run commands from the repository root. The loader variant needs `libdl.so.2`;
service variants need `libfetpf.so` plus their selected provider. All require the
matching module libc/libgcc runtime as specified by the build script. The legacy
ARMv5 module's libraries must not be linked into the ARMv7 host app.

`loader-probe.rs` performs no RF/session/card operation. It only loads the
trial-owned shared library and prints bounded loader diagnostics. Its abort-only
unwind personality exists for legacy linking; unwinding is not supported.
The launcher source restores the original script first and falls back to it on
shell errors. Production deployments never stage these programs or launchers.

[Updated investigation and primary sources](../../docs/pm3-japanese-cards-2026-10-21.md)
explain the active PCI-LSM and signed-app hypothesis. No SDK inquiry has been sent.
