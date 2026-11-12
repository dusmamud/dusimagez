#!/usr/bin/env bash
# Build the dusimagez C FFI library for the PHP binding.
#
#   bash packages/php/scripts/build-ffi.sh
#
# Runs `cargo build -p dusimagez-ffi --release` from the repo root
# (or builds the FFI crate manifest directly when no workspace
# Cargo.toml exists yet) and leaves the shared library at
#   target/release/libdusimagez_ffi.so   (Linux)
#   target/release/libdusimagez_ffi.dylib (macOS)
#   target/release/dusimagez_ffi.dll     (Windows)
# which the PHP wrapper loads with FFI::cdef (see src/Dusimagez.php).
#
# No prebuilt binary is ever downloaded: the library is always built
# from the Rust sources in this repo. Requires a Rust toolchain
# (cargo on PATH, or ~/.cargo/bin/cargo).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo "build-ffi.sh: cargo not found (install Rust: https://rustup.rs)" >&2
        exit 1
    fi
fi

# The FFI crate manifest builds standalone (it has literal
# version/edition, no workspace inheritance), so this works whether
# or not the repo root has a workspace Cargo.toml yet — as long as
# the crate is not sitting under a workspace that does NOT list it.
# If the root workspace exists and lists the crate, build via the
# workspace (`-p`); if there is no workspace at all, build the crate
# manifest directly; if a workspace exists but does not list the
# crate yet, fail with a clear message (the coordinator owns the
# root Cargo.toml — add "crates/dusimagez-ffi" to its members).
if [ -f "$ROOT/Cargo.toml" ]; then
    if grep -q "dusimagez-ffi" "$ROOT/Cargo.toml"; then
        cargo build -p dusimagez-ffi --release --manifest-path "$ROOT/Cargo.toml"
    else
        echo "build-ffi.sh: $ROOT/Cargo.toml is a workspace that does not list crates/dusimagez-ffi." >&2
        echo "build-ffi.sh: add \"crates/dusimagez-ffi\" to its workspace.members, then re-run." >&2
        exit 1
    fi
else
    cargo build --release --manifest-path "$ROOT/crates/dusimagez-ffi/Cargo.toml"
fi

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        LIB="$ROOT/target/release/dusimagez_ffi.dll"
        ;;
    Darwin*)
        LIB="$ROOT/target/release/libdusimagez_ffi.dylib"
        ;;
    *)
        LIB="$ROOT/target/release/libdusimagez_ffi.so"
        ;;
esac

if [ ! -f "$LIB" ]; then
    echo "build-ffi.sh: expected library not found: $LIB" >&2
    exit 1
fi

echo "build-ffi.sh: built $LIB"
