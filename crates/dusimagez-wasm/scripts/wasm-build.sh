#!/bin/sh
# Build the dusimagez WASM package with wasm-pack.
#
# Why this script exists: the `webp` crate links C libwebp, whose
# libc symbols (malloc/free/calloc/realloc/qsort/bsearch) are NOT
# provided by Rust std for `wasm32-unknown-unknown`. A plain
# `wasm-pack build` therefore fails at the final link. This script:
#   1. Finds or downloads wasi-sdk (its clang + wasi-libc sysroot
#      compile the C code). Set WASI_SDK to an existing install to
#      skip the download (e.g. WASI_SDK=$HOME/workspace/tools/wasi-sdk-25.0-x86_64-linux).
#   2. Builds `wasi-shim` (a tiny Rust rlib providing the missing
#      libc symbols) for wasm32-unknown-unknown.
#   3. Runs wasm-pack with the wasi clang as CC and the shim rlib
#      as an extra link argument.
#
# Usage (run from the repo root):
#   crates/dusimagez-wasm/scripts/wasm-build.sh [nodejs|web] [out-dir]
# Defaults: target nodejs, out-dir packages/js/wasm-pkg.
# The website playground uses: wasm-build.sh web website/public/wasm
set -eu

TARGET="${1:-nodejs}"
OUT_DIR="${2:-packages/js/wasm-pkg}"

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
WASI_SDK="${WASI_SDK:-${WASI_SDK_DIR:-$HOME/.cache/dusimagez-wasi-sdk}}"
WASI_VERSION="25.0"

if [ ! -x "$WASI_SDK/bin/clang" ]; then
  echo "wasi-sdk not found at $WASI_SDK — downloading $WASI_VERSION ..."
  mkdir -p "$(dirname "$WASI_SDK")"
  TMP_TAR="$(mktemp).tar.gz"
  curl -fsSL -o "$TMP_TAR" \
    "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_VERSION%%.*}/wasi-sdk-${WASI_VERSION}-x86_64-linux.tar.gz"
  mkdir -p "$WASI_SDK"
  tar -xzf "$TMP_TAR" -C "$WASI_SDK" --strip-components=1
  rm -f "$TMP_TAR"
fi
echo "wasi-sdk: $WASI_SDK"

# cc-rs compiles C deps for --target=wasm32-unknown-unknown, but the
# wasi-libc headers refuse that triple ("<wasi/api.h> is only
# supported on WASI platforms"). Rewrite it to wasm32-wasip1 so
# wasi-sdk's clang applies its sysroot; the resulting objects link
# into the wasm32-unknown-unknown crate.
WRAPPER="$(mktemp -d)/wasi-clang"
cat > "$WRAPPER" <<EOF
#!/bin/sh
WASI_CLANG="$WASI_SDK/bin/clang"
args=""
for a in "\$@"; do
  case "\$a" in
    --target=wasm32-unknown-unknown) a="--target=wasm32-wasip1" ;;
    wasm32-unknown-unknown) a="wasm32-wasip1" ;;
  esac
  args="\$args \"\$a\""
done
eval "exec \"\$WASI_CLANG\" \$args"
EOF
chmod +x "$WRAPPER"

# 1. Build the libc shim rlib for wasm32-unknown-unknown.
cargo build --release --target wasm32-unknown-unknown \
  --manifest-path "$REPO_ROOT/crates/dusimagez-wasm/wasi-shim/Cargo.toml"
SHIM_RLIB="$REPO_ROOT/crates/dusimagez-wasm/wasi-shim/target/wasm32-unknown-unknown/release/libwasi_shim.rlib"

# 2. Build the WASM package.
export CC_wasm32_unknown_unknown="$WRAPPER"
export AR_wasm32_unknown_unknown="$WASI_SDK/bin/llvm-ar"
export RUSTFLAGS="-C link-arg=$SHIM_RLIB"
wasm-pack build --target "$TARGET" \
  "$REPO_ROOT/crates/dusimagez-wasm" --out-dir "$REPO_ROOT/$OUT_DIR"

# wasm-pack writes a `.gitignore` containing `*` into the out-dir.
# It is redundant (the parent .gitignore already ignores the whole
# out-dir) and harmful: npm-packlist reads nested .gitignore files,
# so it would silently drop the entire WASM bundle from `npm pack`
# even though package.json `files` lists the out-dir. Remove it.
rm -f "$REPO_ROOT/$OUT_DIR/.gitignore"

echo "WASM package built: $OUT_DIR (target $TARGET)"
