/**
 * dusimagez — offline image compressor (TypeScript wrapper over the Rust core).
 *
 * The binary work lives in the Rust core and this package wraps it
 * via WASM (`crates/dusimagez-wasm`, `--target nodejs`). Every
 * operation takes bytes in and returns bytes out, synchronously —
 * the WASM calls are synchronous, so there is nothing to `await`
 * (awaiting a plain value still works, it just does nothing).
 *
 * WASM contract (see crates/dusimagez-wasm/src/lib.rs):
 * - Binary results are `Uint8Array`s.
 * - On failure a wasm function THROWS a JSON string:
 *   `{"ok":false,"code":"INVALID_IMAGE","message_en":"…",
 *   "message_hi":"…"}`. Every failure surfaces here as a
 *   `DusimagezError` with the same `code` / `messageEn` /
 *   `messageHi` — never a raw TypeError, never a raw JSON string.
 * - `get_image_info` / `compress_with_report` / `bulk_compress`
 *   return JSON strings; this wrapper parses them into camelCase
 *   objects (see `parse.ts`). Report `output` bytes arrive base64
 *   and are decoded back to `Uint8Array`s.
 * - Options are plain JS objects matching the Rust `CompressOptions`
 *   struct, except this wrapper ALWAYS sends complete objects:
 *   every field is filled with the Rust default, because serde
 *   rejects partial objects for non-`Option` fields. (The wasm
 *   layer ALSO deep-merges partial objects over the defaults, so
 *   the playground's `{ quality: 70 }` works — but this wrapper
 *   does not rely on that.)
 *
 * Naming: the TS API uses camelCase option keys (`targetBytes`,
 * `pngCompressionLevel`, …) and the wrapper translates them to the
 * Rust snake_case JSON keys (`target_bytes`,
 * `png_compression_level`, …) before calling wasm. Enum values are
 * friendly lowercase (`"jpeg"`, `"s420"`, `"web"`, `"keep_exif"`).
 *
 * Input rule: functions accept `Uint8Array` only. ANY non-Uint8Array
 * input (string, number, null, plain object) throws a
 * `DusimagezError` with code `INVALID_TYPE` — never a raw TypeError.
 * (A Node `Buffer` IS a `Uint8Array` subclass, so Buffers are
 * accepted as-is.)
 *
 * Honest limits (same as the Rust core, v0.1.0): NO HEIC. AVIF
 * encode is lossy-only / 8-bit. GIF encode is first-frame only for
 * animated input. Whole images live in memory, no streaming.
 */

// The wasm-pack `--target nodejs` output. Built by `npm run build`
// (wasm-pack first, then tsc); gitignored — see packages/js/.gitignore.
import * as wasm from "../wasm-pkg/dusimagez_wasm";
import { DusimagezError, call } from "./errors";
import {
  assertImageArray,
  assertImageBytes,
  assertTargetBytes,
  toCompressOptions,
  type BulkOptions,
  type CompressOptions,
} from "./options";
import {
  parseBulk,
  parseImageInfo,
  parseReport,
  type BulkResult,
  type CompressReport,
  type ImageInfo,
} from "./parse";

export { DusimagezError, type BulkResult, type CompressReport, type ImageInfo };
export type { BulkOptions, CompressOptions };
export type {
  ImageFormat,
  Preset,
  StripMetadata,
  Subsampling,
} from "./options";
export type { DusimagezErrorCode } from "./errors";

// ---------------------------------------------------------------------------
// Wasm result helpers
// ---------------------------------------------------------------------------

function asBytes(value: unknown): Uint8Array {
  if (value instanceof Uint8Array) return value;
  // wasm-bindgen returns Uint8Array for Vec<u8>; anything else is a bug.
  throw new DusimagezError(
    "OPERATION_FAILED",
    "WASM returned a non-bytes result.",
    "WASM ne bytes result nahi diya.",
  );
}

// ---------------------------------------------------------------------------
// Public API — all synchronous, all throw DusimagezError on failure.
// ---------------------------------------------------------------------------

/** Crate version, e.g. `"0.1.0"` (lockstep with npm / PyPI / crates.io). */
export function version(): string {
  return call(() => wasm.version());
}

/** Re-encode an image at a fixed quality (and `format`, or the input format when unset). */
export function compressImage(
  input: unknown,
  options?: CompressOptions,
): Uint8Array {
  const bytes = assertImageBytes(input);
  return call(() =>
    asBytes(wasm.compress_image(bytes, toCompressOptions(options))),
  );
}

/**
 * Re-encode an image so the result is at most `targetBytes` bytes,
 * at the highest quality that fits. Fails with
 * `TARGET_SIZE_UNREACHABLE` when even the lowest quality is too big.
 */
export function compressToSize(
  input: unknown,
  targetBytes: unknown,
  options?: CompressOptions,
): Uint8Array {
  const bytes = assertImageBytes(input);
  const target = assertTargetBytes(targetBytes);
  return call(() =>
    asBytes(wasm.compress_to_size(bytes, target, toCompressOptions(options))),
  );
}

/** Compress an image and return the full before/after report. */
export function compressWithReport(
  input: unknown,
  options?: CompressOptions,
): CompressReport {
  const bytes = assertImageBytes(input);
  const json = call(() =>
    wasm.compress_with_report(bytes, toCompressOptions(options)),
  );
  return parseReport(json);
}

/** Read an image's summary without modifying it. */
export function getImageInfo(input: unknown): ImageInfo {
  const bytes = assertImageBytes(input);
  const json = call(() => wasm.get_image_info(bytes));
  return parseImageInfo(json);
}

/**
 * Compress many images in a single call. One bad input fails only
 * its own item (with `error.code` / `error.messageEn`), never the
 * whole batch. An empty input list returns `[]` (core semantics).
 */
export function bulkCompress(
  inputs: unknown,
  options?: BulkOptions,
): BulkResult[] {
  if (Array.isArray(inputs) && inputs.length === 0) return [];
  const bufs = assertImageArray(inputs, "Bulk inputs", "bulk inputs");
  const json = call(() =>
    wasm.bulk_compress(bufs, toCompressOptions(options)),
  );
  return parseBulk(json);
}
