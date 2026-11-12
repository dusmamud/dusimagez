# dusimagez-wasm

WASM bindings for dusimagez — thin `wasm-bindgen` wrappers over the
Rust core (`crates/dusimagez`). No image logic lives here, only
JS ↔ Rust conversion.

## Build

Use the repo script — a plain `wasm-pack build` fails at the final
link step (C libwebp needs the wasi-shim libc symbols; see
`wasi-shim/` and `scripts/wasm-build.sh`):

```bash
# Node.js target (npm package + vitest), from the repo root:
crates/dusimagez-wasm/scripts/wasm-build.sh nodejs packages/js/wasm-pkg

# Web target (website playground):
crates/dusimagez-wasm/scripts/wasm-build.sh web website/public/wasm
```

`wasm-opt` is disabled in `Cargo.toml` (`wasm-pack` profile):
binaryen cannot be downloaded in offline/sandboxed builds.

## Contract

- Binary results are `Uint8Array`s.
- On failure a function throws a JSON string:
  `{"ok":false,"code":"INVALID_IMAGE","message_en":"…","message_hi":"…"}`.
- Options are plain JS objects; partial objects work (deep-merged
  over the Rust `Default`). Enum fields accept friendly lowercase
  spellings (`"jpeg"`, `"s420"`, `"web"`, `"keep_exif"`) or the
  canonical serde names (`"Jpeg"`, `"S420"`, `"Web"`, `"KeepExif"`).
- `compress_with_report` / `bulk_compress` return JSON strings;
  report `output` bytes are base64, `format` is lowercase.
- `target_bytes` is a plain JS number (`f64`), not a `BigInt`.

## Assumed core surface

Written against the real core in `crates/dusimagez/src/` (read,
not assumed): `dusimagez::{CompressOptions, CompressReport,
DusimagezError, ImageFormat, ImageInfo, Preset, StripMetadata,
Subsampling, bulk_compress, compress_image, compress_to_size,
compress_with_report, get_image_info}` with
`DusimagezError::{code, message_en, message_hi}` and
`ImageFormat::from_name`. The core types carry NO serde derives,
so options are built field by field in `src/lib.rs` and reports
are serialised to JSON by hand. If W1 changes the core API, this
crate is the one place to reconcile — the TS/Python layers only
see the JSON contract above.
