# dusimagez — shared API contract (v0.1.0)

This file is the shared copy of the API section of `SPEC.md` (repo root).
If this file and `SPEC.md` disagree, `SPEC.md` wins — fix this file, not the
core. Every language binding implements exactly this surface with identical
names, values and error codes.

## 1. Core operations

```rust
compress_image(input: &[u8], options: &CompressOptions) -> Result<Vec<u8>, DusimagezError>
compress_to_size(input: &[u8], target_bytes: u64, options: &CompressOptions) -> Result<Vec<u8>, DusimagezError>
compress_with_report(input: &[u8], options: &CompressOptions) -> Result<CompressReport, DusimagezError>
get_image_info(input: &[u8]) -> Result<ImageInfo, DusimagezError>
bulk_compress(inputs: &[Vec<u8>], options: &CompressOptions) -> Vec<Result<CompressReport, DusimagezError>>
```

- `compress_to_size`: iterative quality search (binary search on quality,
  max 10 steps); if even the lowest quality exceeds the target, downscale
  in 10% steps (never below 10% of original dimensions, never upscale);
  if still over target, fail with `TARGET_SIZE_UNREACHABLE`.
- `CompressReport { output: Vec<u8>, original_size: u64, output_size: u64,
  saved_bytes: u64, saved_percent: f32, format: ImageFormat, width: u32,
  height: u32, quality_used: u8, resized: bool }`
  - `saved_bytes = original_size - output_size`;
    `saved_percent = 100.0 * saved_bytes / original_size`.
  - `resized` is true only when the output dimensions differ from the
    input dimensions (max_width / max_height downscale, or target-size
    downscale).
- `ImageInfo { format, width, height, has_alpha, colour_type, file_size }`
- `bulk_compress`: one result per input, same order, partial failures
  reported per item, never aborting the batch.

## 2. CompressOptions

| Field | Type | Default | Notes |
|---|---|---|---|
| `quality` | u8, 1–100 | `80` | JPEG/WebP quality. PNG uses `png_compression_level` + `palette` instead. Outside 1–100 → `INVALID_QUALITY`. |
| `format` | `ImageFormat` or null | keep input format | Explicit output format overrides the input format. |
| `progressive` | bool | `false` | JPEG progressive / PNG interlaced. |
| `lossless` | bool | `false` | WebP/PNG only; ignored (not an error) for other formats. |
| `effort` | u8 | `6` | WebP 0–6; AVIF speed 0–10. |
| `subsampling` | `S444 \| S422 \| S420` | `S420` for JPEG quality < 90, `S444` otherwise | Chroma subsampling (JPEG). |
| `png_compression_level` | u8, 0–9 | `6` | PNG compression level. |
| `palette` | bool | `false` | PNG quantise to palette. |
| `preset` | `Web \| Social \| Print \| Archive` or null | none | Sets quality/effort/subsampling defaults; explicit fields override it. See §3. |
| `target_bytes` | u64 or null | none | Same target-size search as `compress_to_size`, via the one-call path. |
| `max_width` | u32 or null | none | Downscale-only, never upscale; aspect preserved. |
| `max_height` | u32 or null | none | Downscale-only, never upscale; aspect preserved. |
| `strip_metadata` | `All \| KeepExif \| KeepIcc \| KeepAll` | `All` | Which metadata survives compression. |
| `auto_orient` | bool | `true` | Apply EXIF orientation on load. |
| `dpi` | `(u32, u32)` or null | none | Set on JPEG/PNG output where supported. |
| `max_pixels` | u64 | `120_000_000` | Larger images are refused with `IMAGE_TOO_LARGE`. |
| `max_dimension` | u32 | `16000` | Larger width/height refused with `IMAGE_TOO_LARGE`. |

Enum names and values are identical in every binding: `ImageFormat`
(`jpeg | png | webp | avif | gif | bmp`), `Subsampling`
(`S444 | S422 | S420`), `Preset` (`Web | Social | Print | Archive`),
`StripMetadata` (`All | KeepExif | KeepIcc | KeepAll`).

## 3. Presets (v0.1.0 defaults — defined by this contract)

SPEC.md says a preset "sets quality/effort/subsampling defaults; explicit
fields override it" but does not fix the numbers. The numbers below ARE the
v0.1.0 contract: the core implements exactly these values, and the docs
describe exactly these values.

| Preset | Quality | Subsampling | Effort | Best for |
|---|---|---|---|---|
| `Web` | 80 | S420 | 4 | Small files for web pages |
| `Social` | 85 | S420 | 4 | Balanced quality for sharing |
| `Print` | 95 | S444 | 6 | High quality for print |
| `Archive` | 100 | S444 | 6 | Near-lossless keeping (lossless where the format supports it) |

Presets set only quality / effort / subsampling — they never touch
`progressive` (default `false`) or any other field.

Explicit option fields always override the preset's value for that field.

## 4. Formats

- In: JPEG, PNG, WebP, GIF, BMP, TIFF, AVIF (decode via the `image` crate).
- Out: JPEG, PNG, WebP, AVIF, GIF, BMP.
- v0.1.0 limits (document everywhere, never silently claim more):
  - NO HEIC (libheif + licence).
  - AVIF encode is lossy-only / 8-bit (underlying crate limit).
  - GIF encode is first-frame only for animated input.
  - Whole image in memory, no streaming.

## 5. Errors — stable codes, English + Hindi

Every binding returns the same code plus the same English and Hindi
messages. The message strings below are the v0.1.0 contract — do not
reword them in one binding and not the others.

| Code | message_en (template) | message_hi (template) |
|---|---|---|
| `EMPTY_INPUT` | No input: {what} is empty. | कोई इनपुट नहीं है: {what} खाली है। |
| `INVALID_IMAGE` | The input is not a valid image: {detail}. | इनपुट एक मान्य इमेज नहीं है: {detail}। |
| `UNSUPPORTED_FORMAT` | Format '{format}' is not supported in v0.1.0. Supported inputs: JPEG, PNG, WebP, GIF, BMP, TIFF. AVIF input needs a dav1d decoder, which is not bundled. | '{format}' फ़ॉर्मैट v0.1.0 में समर्थित नहीं है। समर्थित इनपुट: JPEG, PNG, WebP, GIF, BMP, TIFF। AVIF इनपुट के लिए dav1d डिकोडर चाहिए, जो शामिल नहीं है। |
| `IMAGE_TOO_LARGE` | Image is {width}x{height}, which exceeds the limits (max {max_pixels} pixels, max {max_dimension}px per side). | इमेज {width}x{height} है, जो सीमा से बड़ी है (अधिकतम {max_pixels} पिक्सेल, प्रति तरफ़ अधिकतम {max_dimension}px)। |
| `TARGET_SIZE_UNREACHABLE` | Cannot reach {target_bytes} bytes: even the lowest quality at 10% of the original dimensions is larger. | {target_bytes} बाइट तक नहीं पहुँचा जा सकता: सबसे कम क्वालिटी पर मूल आकार के 10% पर भी फ़ाइल बड़ी है। |
| `INVALID_QUALITY` | Quality must be 1-100, got {quality}. | क्वालिटी 1-100 के बीच होनी चाहिए, मिली {quality}। |
| `INVALID_DIMENSIONS` | Invalid dimensions: {detail}. | अमान्य आयाम: {detail}। |
| `INVALID_ARGUMENT` | Invalid argument: {detail}. | अमान्य आर्ग्युमेंट: {detail}। |
| `INVALID_TYPE` | (binding-layer only) The input is not bytes, or an option has the wrong type. Messages are binding-specific. | (सिर्फ़ बाइंडिंग में) इनपुट बाइट्स नहीं है, या विकल्प का टाइप गलत है। |

The **code** is the stable contract — identical in every binding. Messages
follow the core's templates above and carry the failing detail, so exact
message text may differ run to run; code against the code, not the message.

## 6. Shared test vectors

`shared/test-vectors.json` is the contract every language runs: fixture
name + op + options + expected output (format / dimensions / size bounds /
report fields / error code). Rust, Python, JS, Go, PHP, Kotlin and Swift
all run it and must agree. See the `_header` field of that file for the
rules — most importantly, cases assert size BOUNDS, not exact output
hashes, because exact bytes are encoder-dependent.
