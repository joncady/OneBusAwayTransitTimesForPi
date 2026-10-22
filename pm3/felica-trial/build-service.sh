#!/bin/sh
set -eu
PM3_VARIANT=${1:-cvend}
case "$PM3_VARIANT" in
  cvend) PM3_PROVIDER=libcvend-pkcs11.so;;
  proxy) PM3_PROVIDER=libfepkcs11.so;;
  loader) PM3_PROVIDER=libdl.so.2;;
  *) echo 'Usage: build-service.sh [cvend | proxy | loader]' >&2; exit 1;;
esac
PM3_TRIAL_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PM3_PROJECT_DIR=$(CDPATH= cd -- "$PM3_TRIAL_DIR/../.." && pwd)
for PM3_LIBRARY in libfetpf.so "$PM3_PROVIDER" libc.so.6 libgcc_s.so.1; do
  test -f "$PM3_PROJECT_DIR/pm3-audit.local/$PM3_LIBRARY" || {
    echo "Private matching module library required: $PM3_LIBRARY" >&2
    exit 1
  }
done
[ "$PM3_VARIANT" != loader ] || ln -sf libdl.so.2 "$PM3_PROJECT_DIR/pm3-audit.local/libdl.so"
ln -sf libc.so.6 "$PM3_PROJECT_DIR/pm3-audit.local/libc.so"
ln -sf libgcc_s.so.1 "$PM3_PROJECT_DIR/pm3-audit.local/libgcc_s.so"
docker build -t pm3-felica-build "$PM3_TRIAL_DIR"
docker build -t pm3-felica-gnu-build -f "$PM3_TRIAL_DIR/Dockerfile.service" "$PM3_TRIAL_DIR"
docker run --rm -e PM3_VARIANT="$PM3_VARIANT" -v "$PM3_PROJECT_DIR:/work/project" -w /work/project pm3-felica-gnu-build sh -ec '
  mkdir -p pm3/target
  case "$PM3_VARIANT" in
    cvend) set -- -l dylib=fetpf -l dylib=cvend-pkcs11 -C "link-arg=--export-dynamic-symbol=C_*"; PM3_OUTPUT=fetpf-probe;;
    proxy) set -- --cfg service_proxy -l dylib=fetpf -l dylib=fepkcs11; PM3_OUTPUT=fetpf-proxy-probe;;
    loader) set -- --cfg loader_probe -l dylib=dl -C "link-args=--no-as-needed -lgcc_s --allow-shlib-undefined"; PM3_OUTPUT=loader-probe;;
  esac
  rustc --edition 2024 --cfg fetpf_client --target armv5te-unknown-linux-gnueabi \
    -C panic=abort -C opt-level=s -C strip=symbols -C linker=arm-linux-gnueabi-ld \
    -C relocation-model=static -C link-arg=--dynamic-linker=/lib/ld-linux.so.3 \
    -C link-arg=--allow-shlib-undefined \
    -C link-arg=-z -C link-arg=max-page-size=32768 \
    -C link-arg=-e -C link-arg=_start -L native=/work/project/pm3-audit.local \
    "$@" -l dylib=c -l dylib=gcc_s \
    pm3/felica-trial/rf-probe.rs -o "pm3/target/$PM3_OUTPUT"
'
