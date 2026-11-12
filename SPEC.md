# dusimagez — SPEC (contract, v0.1.0)

Offline image compressor. One Rust core, every language binding calls the
same core. No upload, no network at runtime. MIT.

## 1. Core API (Rust crate `dusimagez`)

```rust
compress_image(input: &[u8], options: &CompressOptions) -> Result<Vec<u8>, DusimagezError>
compress_to_size(input: &[u8], target_bytes: u64, options: &CompressOptions) -> Result<Vec<u8>, DusimagezError>
compress_with_report(input: &[u8], options: &CompressOptions) -> Result<CompressReport, DusimagezError>
get_image_info(input: &[u8]) -> Result<ImageInfo, DusimagezError>
bulk_compress(inputs: &[Vec<u8>], options: &CompressOptions) -> Vec<Result<CompressReport, DusimagezError>>
```

- `compress_to_size`: iterative quality search (binary search on quality,
  max 10 steps); if even lowest quality exceeds target, downscale in 10%
  steps (never below 10% of original dimensions, never upscale); if still
  over target, fail with `TARGET_SIZE_UNREACHABLE`.
- `CompressReport { output: Vec<u8>, original_size: u64, output_size: u64,
  saved_bytes: u64, saved_percent: f32, format: ImageFormat, width: u32,
  height: u32, quality_used: u8, resized: bool }`
- `ImageInfo { format, width, height, has_alpha, colour_type, file_size }`
- `bulk_compress`: one result per input, same order, partial failures
  reported per item, never aborting the batch.

## 2. Options (pro control layer)

- `CompressOptions`:
  - `quality: u8` (1-100, default 80 JPEG/WebP; PNG uses
    `png_compression_level` 0-9 + `palette` quantise flag)
  - `format: Option<ImageFormat>` (default: keep input format)
  - `progressive: bool` (JPEG progressive / PNG interlaced)
  - `lossless: bool` (WebP/PNG only)
  - `effort: u8` (WebP 0-6; AVIF speed 0-10, default 6)
  - `subsampling: Subsampling` (`S444 | S422 | S420`, default S420 for
    JPEG quality < 90, S444 otherwise)
  - `png_compression_level: u8` (0-9, default 6)
  - `palette: bool` (PNG quantise to palette, default false)
  - `preset: Option<Preset>` (`Web | Social | Print | Archive` — preset
    sets quality/effort/subsampling defaults; explicit fields override it)
  - `target_bytes: Option<u64>` (same as `compress_to_size` via one call)
  - `max_width: Option<u32>`, `max_height: Option<u32>` (downscale-only,
    never upscale; aspect preserved)
  - `strip_metadata: StripMetadata` (`All | KeepExif | KeepIcc | KeepAll`,
    default All)
  - `auto_orient: bool` (default true — apply EXIF orientation on load)
  - `dpi: Option<(u32, u32)>` (set on JPEG/PNG output where supported)
  - `max_pixels: u64` (default 120_000_000 — refuse larger with
    `IMAGE_TOO_LARGE`), `max_dimension: u32` (default 16000)
- `Subsampling`, `Preset`, `StripMetadata`, `ImageFormat` enums are part of
  the public API in every binding with identical names/values.

## 3. Formats

- In: JPEG, PNG, WebP, GIF, BMP, TIFF, AVIF (decode via `image` crate).
- Out: JPEG, PNG, WebP, AVIF, GIF, BMP.
- v0.1.0 limits (document in README + docs, do not silently claim more):
  NO HEIC (libheif + licence); AVIF encode lossy-only / 8-bit (underlying
  crate limit); GIF encode is first-frame only for animated input; whole
  image in memory, no streaming.

## 4. Errors — stable codes, English + Hindi

`INVALID_IMAGE`, `UNSUPPORTED_FORMAT`, `IMAGE_TOO_LARGE`,
`TARGET_SIZE_UNREACHABLE`, `INVALID_QUALITY`, `INVALID_DIMENSIONS`,
`INVALID_ARGUMENT`, `EMPTY_INPUT`.
Every binding returns the same code + EN message + HI message.

## 5. Layout (v0.1.0 — ALL bindings in v0.1.0, per user order)

- `crates/dusimagez` — Rust core (publish = true, name `dusimagez`).
- `crates/dusimagez-ffi` — C API (`extern "C"`, bytes in/out, error codes,
  C header `dusimagez.h`): `dusimagez_compress`,
  `dusimagez_compress_to_size`, `dusimagez_compress_with_report`,
  `dusimagez_get_info`, `dusimagez_free`, `dusimagez_last_error`.
  Go + PHP both build on this crate. No business logic duplicated in FFI.
- `crates/dusimagez-cli` — binary `dusimagez`: `compress`, `info`, `mock`
  (+ `--version`, `--help`). `mock` generates test images (solid /
  gradient / photo-like noise). Flags mirror CompressOptions 1:1.
- `crates/dusimagez-py` — PyO3 + maturin (publish = false);
  `packages/python` thin wrapper + Pydantic v2 models (optional import).
- `crates/dusimagez-wasm` — wasm-bindgen (publish = false);
  `packages/js` TS wrapper + Zod schemas, `vitest` tests.
- `packages/go` — cgo wrapper over the FFI crate, Go-idiomatic API
  (`Compress`, `CompressToSize`, `CompressWithReport`, `GetInfo`,
  `BulkCompress`), `go.mod` module `github.com/dusmamud/dusimagez/go`,
  `go test` on shared vectors.
- `packages/php` — PHP FFI wrapper over the compiled C lib
  (`src/Dusimagez.php`, Composer `dusmamud/dusimagez`), PHPUnit tests on
  shared vectors. Composer-direct install is NOT possible for a Rust .so —
  document the `FFI::cdef` + prebuilt-lib step honestly in README/docs.
- `packages/kotlin` — UniFFI Kotlin (Android): `uniffi-bindgen` from the
  core UDL, `CompressOptions`/`CompressReport` types, JUnit tests on
  shared vectors, published as Android library module (AAR via Gradle).
- `packages/swift` — UniFFI Swift (iOS/macOS): same UDL, SwiftPM package,
  XCTest on shared vectors. iOS build needs macOS CI runner — use
  `macos-latest` in mobile.yml, do not claim Linux iOS builds.
- `website/` — Astro 5 + Starlight docs (compressor guide, options
  reference, presets, formats, error-codes, per-language quickstarts,
  comparison) + WASM playground page + landing.
  `astro.config.mjs` site: `https://dusimagez.pages.dev`.
- `shared/` — `test-vectors.json` (the contract EVERY language must pass:
  input fixture name + op + options + expected output hash / dimensions /
  size bounds / report fields) + `spec.md` (this file's API section).
  Fixtures generated by `dusimagez mock` (deterministic seed), never
  committed as binary blobs except 2 tiny PNGs (<50 KB total).

## 6. Tests — everything verified by GitHub Actions

- Rust: `cargo test --workspace` — unit + integration (round-trip:
  decode → compress → decode; dimensions, pixel spot-checks, error codes,
  target-size guarantee, report maths).
- Python: `pytest` on the maturin wheel (all ops + pydantic + report).
- JS: `vitest` on the WASM wrapper (all ops + zod + report).
- Go: `go test ./...` on shared vectors.
- PHP: `phpunit` on shared vectors (skip gracefully if FFI .so missing,
  with a clear message — never fake-pass).
- Kotlin/Swift: UniFFI-generated bindings compile + run shared vectors
  (JUnit / XCTest).
- Shared vectors: Rust, Python, JS, Go, PHP, Kotlin, Swift all run
  `shared/test-vectors.json` and must agree.
- `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings),
  `cargo fmt --all -- --check`.

## 7. CI workflows (`.github/workflows/`)

`ci.yml` (Rust), `wasm.yml`, `python.yml`, `go.yml`, `php.yml`,
`mobile.yml` (Kotlin + Swift), `website.yml` (build + Cloudflare Pages
deploy step), `npm.yml` + `pypi.yml` (Trusted Publishing OIDC, manual
dispatch), `release.yml` (release-plz). Website deploy needs Cloudflare
secrets (`CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID`) — dashboard
step, not code.
