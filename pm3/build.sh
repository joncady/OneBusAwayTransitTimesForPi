#!/bin/sh
set -eu
PM3_SKIP_TESTS=1
PM3_MAINTENANCE=0
case "${1:-}" in
  --skip-tests) PM3_SKIP_TESTS=1;;
  --test) PM3_SKIP_TESTS=0;;
  --maintenance) PM3_MAINTENANCE=1;;
  '') ;;
  *) echo 'Usage: build.sh [--test | --skip-tests | --maintenance]' >&2; exit 1;;
esac
PM3_SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
docker build -t pm3-transit-build "$PM3_SOURCE_DIR"
docker run --rm \
  -e PM3_SKIP_TESTS="$PM3_SKIP_TESTS" \
  -e PM3_MAINTENANCE="$PM3_MAINTENANCE" \
  -v "$PM3_SOURCE_DIR:/work" \
  -v pm3-transit-cargo-cache:/usr/local/cargo/registry \
  pm3-transit-build sh -ec '
    if [ "$PM3_SKIP_TESTS" != 1 ]; then cargo test --locked; fi
    export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER="$(find /usr/local/rustup/toolchains -path "*/bin/rust-lld" | head -n 1)"
    # musl provides dlopen/dlsym stubs in libc. The Rust target bundles libc
    # but omits the otherwise-empty libdl archive requested by libloading.
    # Supply the standard empty compatibility archive; no functions are replaced.
    mkdir -p target/musl-compat
    ar crs target/musl-compat/libdl.a
    cargo rustc --locked --release --bin pm3-transit-prototype --target armv7-unknown-linux-musleabihf -- \
      -L native=/work/target/musl-compat
    if [ "$PM3_MAINTENANCE" = 1 ]; then
      for PM3_BIN in reader-files reader-trial; do
        cargo rustc --locked --release --features reader-maintenance --bin "$PM3_BIN" \
          --target armv7-unknown-linux-musleabihf -- -L native=/work/target/musl-compat
      done
    fi
  '
