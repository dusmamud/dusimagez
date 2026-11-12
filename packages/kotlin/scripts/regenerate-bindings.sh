#!/usr/bin/env bash
# Regenerate the Kotlin bindings from the UDL and copy them into the
# Android library module. Run from the repo root.
#
# Requires: uniffi-bindgen 0.29 (same version the core's `uniffi`
# dependency pins — see crates/dusimagez/Cargo.toml).
#   cargo install uniffi-bindgen --version 0.29
#
# Output: library/src/main/kotlin/uniffi/dusimagez/dusimagez.kt
# (package uniffi.dusimagez — the path matches the package).
# Commit the regenerated file; the library
# must build without uniffi-bindgen at consumer build time.
set -euo pipefail
cd "$(dirname "$0")/../.."

UDL="crates/dusimagez/src/dusimagez.udl"
OUT="packages/kotlin/library/src/main/kotlin/uniffi/dusimagez"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if ! command -v uniffi-bindgen >/dev/null 2>&1; then
  echo "uniffi-bindgen not found — install it first:" >&2
  echo "  cargo install uniffi-bindgen --version 0.29" >&2
  exit 1
fi

uniffi-bindgen generate --language kotlin --out-dir "$TMP" "$UDL"
mkdir -p "$OUT"
# Generated file declares `package uniffi.dusimagez`; keep the path
# in step with the package (do not move it under com/dusimagez/).
cp "$TMP/uniffi/dusimagez/dusimagez.kt" "$OUT/dusimagez.kt"
echo "Regenerated $OUT/dusimagez.kt from $UDL"
