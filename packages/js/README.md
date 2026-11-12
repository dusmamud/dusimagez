# dusimagez (JavaScript / TypeScript)

Offline image compressor — reduce image file size with maximum
control. One Rust core, wrapped as WASM; no network, no uploads.
MIT.

## Quickstart

```bash
npm install dusimagez
```

```ts
import {
  compressImage,
  compressToSize,
  compressWithReport,
  getImageInfo,
  bulkCompress,
} from "dusimagez";

const png = new Uint8Array(/* ... PNG/JPEG/WebP bytes ... */);

// Re-encode at a fixed quality (keeps the input format by default).
const out = compressImage(png, { quality: 80 });

// Convert to JPEG and squeeze under 200 KB.
const small = compressToSize(png, 200_000, { format: "jpeg" });

// Full before/after report.
const report = compressWithReport(png, { format: "jpeg", quality: 80 });
console.log(report.originalSize, report.outputSize, report.savedPercent);

// Summary without modifying anything.
const info = getImageInfo(png);
console.log(info.format, info.width, info.height);

// Batch: one result per input, same order; one bad input fails
// only its own item.
const results = bulkCompress([png, badBytes, png], { format: "jpeg" });
```

All functions are synchronous and throw `DusimagezError` on
failure — a stable machine-readable `code` plus `messageEn` /
`messageHi`:

```ts
import { DusimagezError } from "dusimagez";

try {
  compressImage(bytes, { quality: 80 });
} catch (e) {
  if (e instanceof DusimagezError) {
    console.log(e.code); // e.g. "INVALID_IMAGE"
  }
}
```

## Options

| Option | Default | Notes |
| --- | --- | --- |
| `quality` | `80` | 1–100. `INVALID_QUALITY` outside. |
| `format` | keep input | `"jpeg"`, `"png"`, `"webp"`, `"avif"`, `"gif"`, `"bmp"` (`"tiff"` falls back to PNG) |
| `progressive` | `false` | Progressive JPEG / interlaced PNG |
| `lossless` | `false` | WebP/PNG only |
| `effort` | `6` | WebP 0–6, AVIF speed 0–10 |
| `subsampling` | `s420` < q90, else `s444` | `"s444"`, `"s422"`, `"s420"` |
| `pngCompressionLevel` | `6` | 0–9 |
| `palette` | `false` | PNG quantise to palette |
| `preset` | unset | `"web"`, `"social"`, `"print"`, `"archive"` |
| `targetBytes` | unset | Same as `compressToSize` via one call |
| `maxWidth` / `maxHeight` | unset | Downscale-only, never upscale |
| `stripMetadata` | `"all"` | `"all"`, `"keep_exif"`, `"keep_icc"`, `"keep_all"` |
| `autoOrient` | `true` | Apply EXIF orientation on load |
| `dpi` | unset | `[x, y]` pair, JPEG/PNG output |
| `maxPixels` | `120_000_000` | Refuse larger (`IMAGE_TOO_LARGE`) |
| `maxDimension` | `16000` | Refuse longer sides |

Enum values are friendly lowercase on the TS side; the wrapper
maps them to the Rust serde names before calling WASM.

## Zod schemas

```ts
import { compressOptionsSchema } from "dusimagez/zod";

compressOptionsSchema.parse({ quality: 80, format: "jpeg" });
```

## Error codes

`INVALID_IMAGE`, `UNSUPPORTED_FORMAT`, `IMAGE_TOO_LARGE`,
`TARGET_SIZE_UNREACHABLE`, `INVALID_QUALITY`,
`INVALID_DIMENSIONS`, `INVALID_ARGUMENT`, `EMPTY_INPUT`,
`INVALID_TYPE` (non-`Uint8Array` input), `OPERATION_FAILED`.

## Limits (v0.1.0, same as the Rust core)

NO HEIC. AVIF encode is lossy-only / 8-bit. GIF encode is
first-frame only for animated input. Whole images live in memory,
no streaming.

## Develop

```bash
npm run build    # wasm-pack (nodejs target) + tsc
npm run typecheck
npm test         # vitest; op tests need the wasm build
```
