#!/usr/bin/env bash
# Regenerate the Kotlin bindings from the UDL and copy them into the
# Android library module. Run from the repo root.
#
# Requires: the in-workspace `uniffi-bindgen` binary (UniFFI 0.29 —
# same version the core's `uniffi` dependency pins — see
# crates/dusimagez-uniffi/Cargo.toml). There is no `uniffi-bindgen`
# binary crate on crates.io in 0.29; do NOT `cargo install` it.
# The binary is built on demand by the command below.
# Output: library/src/main/kotlin/uniffi/dusimagez/dusimagez.kt
# (package uniffi.dusimagez — the path matches the package).
# Commit the regenerated file; the library
# must build without uniffi-bindgen at consumer build time.
set -euo pipefail
# Script lives three levels below the repo root
# (packages/kotlin/scripts/) — go up three, not two.
cd "$(dirname "$0")/../../.."

UDL="crates/dusimagez-uniffi/src/dusimagez.udl"
OUT="packages/kotlin/library/src/main/kotlin/uniffi/dusimagez"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if ! cargo run -q -p dusimagez-uniffi --bin uniffi-bindgen -- --version >/dev/null 2>&1; then
  echo "uniffi-bindgen binary failed to build — see crates/dusimagez-uniffi/Cargo.toml" >&2
  exit 1
fi

cargo run -q -p dusimagez-uniffi --bin uniffi-bindgen -- generate --language kotlin --out-dir "$TMP" "$UDL"
mkdir -p "$OUT"
# Generated file declares `package uniffi.dusimagez`; keep the path
# in step with the package (do not move it under com/dusimagez/).
cp "$TMP/uniffi/dusimagez/dusimagez.kt" "$OUT/dusimagez.kt"
echo "Regenerated $OUT/dusimagez.kt from $UDL"
