#!/bin/sh
set -eu
PM3_TRIAL_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PM3_PROJECT_DIR=$(CDPATH= cd -- "$PM3_TRIAL_DIR/../.." && pwd)
test -f "$PM3_PROJECT_DIR/pm3-audit.local/libdl.so.2" || {
  echo 'A private matching cVEND libdl.so.2 backup is required.' >&2
  exit 1
}
# The versioned library comes from the private backup, never a downloaded SDK.
ln -sf libdl.so.2 "$PM3_PROJECT_DIR/pm3-audit.local/libdl.so"
docker build -t pm3-felica-build "$PM3_TRIAL_DIR"
docker run --rm -v "$PM3_PROJECT_DIR:/work/project" -w /work/project pm3-felica-build sh -ec '
  PM3_ADAPTER_LINKER=$(find /usr/local/rustup/toolchains -path "*/bin/rust-lld" | head -n 1)
  mkdir -p pm3/target
  rustc --edition 2024 --target armv5te-unknown-linux-gnueabi --crate-type cdylib \
    -C panic=abort -C opt-level=s -C linker="$PM3_ADAPTER_LINKER" \
    -C link-arg=--hash-style=sysv \
    -C link-arg=--no-rosegment -C link-arg=-z -C link-arg=norelro \
    -C link-arg=-z -C link-arg=max-page-size=32768 \
    -L native=/work/project/pm3-audit.local -l dylib=dl \
    pm3/felica-trial/adapter.rs -o pm3/target/felica-adapter.so
'
