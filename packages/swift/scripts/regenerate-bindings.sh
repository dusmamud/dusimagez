#!/usr/bin/env bash
# Regenerate the Swift bindings from the UDL and copy them into the
# SwiftPM package. Run from the repo root.
#
# Requires: uniffi-bindgen 0.29 (same version the core's `uniffi`
# dependency pins — see crates/dusimagez/Cargo.toml).
#   cargo install uniffi-bindgen --version 0.29
#
# Output:
#   Sources/Dusimagez/dusimagez.swift   (generated Swift API)
#   Sources/DusimagezFFI/dusimagezFFI.h (generated C header)
#   Sources/DusimagezFFI/module.modulemap
# Commit the regenerated files; the package must build without
# uniffi-bindgen at consumer build time (the Rust static library is
# still supplied separately — see README.md).
set -euo pipefail
cd "$(dirname "$0")/../.."

UDL="crates/dusimagez/src/dusimagez.udl"
SWIFT_OUT="packages/swift/Sources/Dusimagez"
FFI_OUT="packages/swift/Sources/DusimagezFFI"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if ! command -v uniffi-bindgen >/dev/null 2>&1; then
  echo "uniffi-bindgen not found — install it first:" >&2
  echo "  cargo install uniffi-bindgen --version 0.29" >&2
  exit 1
fi

uniffi-bindgen generate --language swift --out-dir "$TMP" "$UDL"
mkdir -p "$SWIFT_OUT" "$FFI_OUT"
cp "$TMP/dusimagez.swift" "$SWIFT_OUT/dusimagez.swift"
# uniffi-bindgen 0.29 emits the C header + modulemap next to the Swift
# file (names: dusimagezFFI.h / dusimagezFFI.modulemap).
cp "$TMP/dusimagezFFI.h" "$FFI_OUT/dusimagezFFI.h"
cp "$TMP/dusimagezFFI.modulemap" "$FFI_OUT/module.modulemap"
echo "Regenerated $SWIFT_OUT/dusimagez.swift and $FFI_OUT/ from $UDL"
