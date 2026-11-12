# dusimagez-ffi — C ABI bindings

Thin C wrappers over the `dusimagez` Rust core. This crate is the
foundation the Go (cgo) and PHP (FFI) bindings build on. **No business
logic lives here** — every function delegates to the core crate.

## API

See `dusimagez.h` (the authoritative C header — it matches the
exported symbols exactly). The six SPEC §5 functions:

| Function | What it does |
| --- | --- |
| `dusimagez_compress` | Re-encode at a fixed quality; honours `options.target_bytes` (behaves like `compress_to_size` when set) |
| `dusimagez_compress_to_size` | Re-encode to at most `target_bytes` bytes |
| `dusimagez_compress_with_report` | Re-encode and fill a caller-allocated `DusimagezReport` |
| `dusimagez_get_info` | Fill a caller-allocated `DusimagezInfo` |
| `dusimagez_free` | Free a buffer returned by `*out` / `report.output`, exactly once |
| `dusimagez_last_error` | Last error on this thread as JSON `{"code","message_en","message_hi"}`, or `NULL` |

Extra exports (beyond the six SPEC names, needed by the Go/PHP
bindings): `dusimagez_version` and `dusimagez_bulk_compress`
(one JSON array, per-item results, image outputs base64-encoded).

Status codes: `0` OK, `1` INVALID_IMAGE, `2` UNSUPPORTED_FORMAT,
`3` IMAGE_TOO_LARGE, `4` TARGET_SIZE_UNREACHABLE, `5` INVALID_QUALITY,
`6` INVALID_DIMENSIONS, `7` INVALID_ARGUMENT, `8` EMPTY_INPUT,
`99` UNKNOWN.

## Options struct

Options arrive as a `DusimagezOptions` C struct, **not** JSON.
Numeric fields use `-1` for "unset / use the core default"; a `NULL`
options pointer means "all defaults". Enum fields are `int32_t`
codes — see `dusimagez.h` for the tables (format 0=keep/1=jpeg/2=png/
3=webp/4=avif/5=gif/6=bmp/7=tiff; subsampling 0=default/1=S444/2=S422/
3=S420; preset 0=none/1=Web/2=Social/3=Print/4=Archive;
strip_metadata 0=All/1=KeepExif/2=KeepIcc/3=KeepAll). Bool fields are
`-1` = default, `0` = false, `1` = true (except `auto_orient`, whose
default is true).

The FFI layer starts from `CompressOptions::default()` and overrides
only the fields the caller set, so core-side default rules (e.g. the
subsampling default, which depends on quality) stay in the core.

## Memory ownership

Every `*out` buffer and every `report.output` buffer is heap
allocated by Rust and owned by the caller after a successful call.
Free it exactly once with `dusimagez_free(ptr, len)`.
`dusimagez_last_error()` / `dusimagez_version()` return NUL-terminated
C strings owned by the library — do NOT free them.

## Build

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo build -p dusimagez-ffi --release   # from the repo root (workspace)
# or standalone:
cargo build --release --manifest-path crates/dusimagez-ffi/Cargo.toml
```

Artifacts: `target/release/libdusimagez_ffi.so` (Linux),
`libdusimagez_ffi.dylib` (macOS), `dusimagez_ffi.dll` (Windows),
plus the static `libdusimagez_ffi.a` / `dusimagez_ffi.lib` that the
Go binding links.

## Tests

```bash
cargo test -p dusimagez-ffi
```

## Core API surface (verified against the landed core)

This crate was first written against the SPEC contract before the
core landed, then re-checked against `crates/dusimagez` as landed
(uncommitted, W1). The following are **verified**, not assumed:

- `DusimagezError` has private fields and the methods `code()`
  (NOT `code_str()`), `message_en()`, `message_hi()`; `ErrorCode`
  variants `InvalidImage`, `UnsupportedFormat`, `ImageTooLarge`,
  `TargetSizeUnreachable`, `InvalidQuality`, `InvalidDimensions`,
  `InvalidArgument`, `EmptyInput`. This crate only matches on the
  `code()` string, so a renamed enum still compiles.
- `compress_image(&[u8], &CompressOptions) -> Result<Vec<u8>, _>`,
  `compress_to_size(&[u8], u64, &CompressOptions)`,
  `compress_with_report(&[u8], &CompressOptions) ->
  Result<CompressReport, _>`, `get_image_info(&[u8]) ->
  Result<ImageInfo, _>`, `bulk_compress(&[Vec<u8>],
  &CompressOptions) -> Vec<Result<CompressReport, _>>`.
- `CompressOptions` fields: `quality: u8`, `format:
  Option<ImageFormat>`, `progressive: bool`, `lossless: bool`,
  `effort: u8`, `subsampling: Subsampling`, `png_compression_level:
  u8`, `palette: bool`, `preset: Option<Preset>`, `target_bytes:
  Option<u64>`, `max_width: Option<u32>`, `max_height: Option<u32>`,
  `strip_metadata: StripMetadata`, `auto_orient: bool`, `dpi:
  Option<(u32, u32)>`, `max_pixels: u64`, `max_dimension: u32` —
  all `pub`, with a `Default` impl (quality 80, effort 6,
  png_compression_level 6, max_pixels 120_000_000, max_dimension
  16000, auto_orient true, subsampling S420, strip All).
- `ImageFormat::{Jpeg, Png, WebP, Avif, Gif, Bmp, Tiff}` — `Tiff`
  is decode-only (input / `ImageInfo` only, never an output
  format), which is why code `7` exists in the C enum.
- `Subsampling::{S444, S422, S420}` (default S420),
  `Preset::{Web, Social, Print, Archive}`,
  `StripMetadata::{All, KeepExif, KeepIcc, KeepAll}` (default All).
- `CompressReport { output: Vec<u8>, original_size: u64,
  output_size: u64, saved_bytes: u64, saved_percent: f32, format:
  ImageFormat, width: u32, height: u32, quality_used: u8, resized:
  bool }`.
- `ImageInfo { format: ImageFormat, width: u32, height: u32,
  has_alpha: bool, colour_type: String, file_size: u64 }` —
  `colour_type` is the `image`-crate `ColorType` **Debug** name
  (`"Rgb8"`, `"Rgba8"`, `"La8"`, `"L8"`, ...), NOT lowercase.
- The core's `compress_image` already honours
  `options.target_bytes` natively (via its shared `compress_inner`
  path); this crate's `dusimagez_compress` additionally routes to
  `compress_to_size` when `target_bytes` is set, so the behaviour is
  identical either way.

If any of these names change in the core, this crate — and only
this crate — needs adapting; the Go and PHP bindings talk to the C
API only, so they are unaffected.

## Workspace membership (known gap, not this crate's file)

The repo root `Cargo.toml` workspace currently lists only
`crates/dusimagez` and `crates/dusimagez-cli` — `dusimagez-ffi` is
not a member yet, so `cargo build -p dusimagez-ffi` from the repo
root fails until the coordinator adds `"crates/dusimagez-ffi"` to
`workspace.members`. The Go/PHP `scripts/build-ffi.sh` detect this
and print that instruction instead of a cryptic cargo error. This
crate's `Cargo.toml` uses literal `version`/`edition` (no
`workspace = true` inheritance) precisely so it builds standalone
today and joins the workspace unchanged later.
