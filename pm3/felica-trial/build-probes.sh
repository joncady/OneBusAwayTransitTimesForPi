#!/bin/sh
set -eu
PM3_TRIAL_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PM3_PROJECT_DIR=$(CDPATH= cd -- "$PM3_TRIAL_DIR/../.." && pwd)
docker build -t pm3-felica-build "$PM3_TRIAL_DIR"
docker run --rm -v "$PM3_PROJECT_DIR:/work/project" -w /work/project pm3-felica-build sh -ec '
  PM3_PROBE_LINKER=$(find /usr/local/rustup/toolchains -path "*/bin/rust-lld" | head -n 1)
  mkdir -p pm3/target
  for PM3_PROBE_NAME in mapping-probe rf-probe; do
    rustc --edition 2024 --target armv5te-unknown-linux-gnueabi \
      -C panic=abort -C opt-level=s -C strip=symbols \
      -C linker="$PM3_PROBE_LINKER" -C relocation-model=static \
      -C link-arg=--no-dynamic-linker -C link-arg=-e -C link-arg=_start \
      "pm3/felica-trial/$PM3_PROBE_NAME.rs" -o "pm3/target/$PM3_PROBE_NAME"
  done
'
