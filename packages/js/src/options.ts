/**
 * dusimagez — options (pure module, no WASM import).
 *
 * The TS API uses camelCase option keys (`pngCompressionLevel`,
 * `targetBytes`, …) and this module translates them to the complete
 * snake_case Rust objects the wasm layer expects. Every field is
 * optional: omitted fields use the Rust defaults (SPEC §2).
 *
 * Enum values are friendly lowercase on the TS side (`"jpeg"`,
 * `"s420"`, `"web"`, `"keep_exif"`); the translator maps them to
 * the canonical serde names (`"Jpeg"`, `"S420"`, `"Web"`,
 * `"KeepExif"`). Canonical names are also accepted at runtime.
 */

import { DusimagezError, invalidArgument } from "./errors";

/** Output image format (lowercase friendly names). */
/**
 * Output formats. `"tiff"` is accepted too: the core decodes TIFF
 * but cannot encode it, so it falls back to PNG output (see
 * `mapFormat`); it is in the union because the runtime and the
 * Zod schema both accept it.
 */
export type ImageFormat = "jpeg" | "png" | "webp" | "avif" | "gif" | "bmp" | "tiff";

/** Chroma subsampling (lowercase friendly names). */
export type Subsampling = "s444" | "s422" | "s420";

/** Preset (lowercase friendly names). */
export type Preset = "web" | "social" | "print" | "archive";

/** Metadata handling (lowercase friendly names). */
export type StripMetadata = "all" | "keep_exif" | "keep_icc" | "keep_all";

/** Options for {@link compressImage} / {@link compressToSize} /
 *  {@link compressWithReport} / {@link bulkCompress} (SPEC §2). */
export interface CompressOptions {
  /** Quality 1-100 (default 80). */
  quality?: number;
  /** Output format; omitted = keep the input format. */
  format?: ImageFormat;
  /** Progressive JPEG / interlaced PNG (default false). */
  progressive?: boolean;
  /** Lossless output (WebP/PNG only; default false). */
  lossless?: boolean;
  /** Encoding effort: WebP 0-6, AVIF speed 0-10 (default 6). */
  effort?: number;
  /** Chroma subsampling (default S420 for JPEG quality < 90, S444 otherwise). */
  subsampling?: Subsampling;
  /** PNG compression level 0-9 (default 6). */
  pngCompressionLevel?: number;
  /** PNG quantise to palette (default false). */
  palette?: boolean;
  /** Preset: sets quality/effort/subsampling defaults; explicit fields override it. */
  preset?: Preset;
  /** Target size in bytes (same as `compressToSize` via one call). */
  targetBytes?: number;
  /** Downscale-only max width in px (never upscale; aspect preserved). */
  maxWidth?: number;
  /** Downscale-only max height in px (never upscale; aspect preserved). */
  maxHeight?: number;
  /** Metadata handling (default "all" = strip everything). */
  stripMetadata?: StripMetadata;
  /** Apply EXIF orientation on load (default true). */
  autoOrient?: boolean;
  /** DPI to write on JPEG/PNG output (default unset). */
  dpi?: [number, number] | null;
  /** Refuse images with more pixels than this (default 120,000,000). */
  maxPixels?: number;
  /** Refuse images with a side longer than this (default 16000). */
  maxDimension?: number;
}

/** Extra options for {@link bulkCompress} — same as
 *  {@link CompressOptions} (bulk uses one options object for all
 *  inputs). */
export type BulkOptions = CompressOptions;

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function mapEnum(
  value: string,
  pairs: Array<[string, string]>,
  what: string,
): string {
  const lower = value.trim().toLowerCase();
  for (const [friendly, canonical] of pairs) {
    if (lower === friendly) return canonical;
  }
  for (const [, canonical] of pairs) {
    if (value === canonical) return canonical;
  }
  throw invalidArgument(
    `Unknown ${what} "${value}".`,
    `Unknown ${what} "${value}" hai.`,
  );
}

export function mapFormat(value: string): string {
  return mapEnum(
    value,
    [
      ["jpeg", "Jpeg"],
      ["jpg", "Jpeg"],
      ["png", "Png"],
      ["webp", "WebP"],
      ["avif", "Avif"],
      ["gif", "Gif"],
      ["bmp", "Bmp"],
      ["tiff", "Tiff"],
      ["tif", "Tiff"],
    ],
    "format",
  );
}

export function mapSubsampling(value: string): string {
  return mapEnum(
    value,
    [
      ["s444", "S444"],
      ["444", "S444"],
      ["4:4:4", "S444"],
      ["s422", "S422"],
      ["422", "S422"],
      ["4:2:2", "S422"],
      ["s420", "S420"],
      ["420", "S420"],
      ["4:2:0", "S420"],
    ],
    "subsampling",
  );
}

export function mapPreset(value: string): string {
  return mapEnum(
    value,
    [
      ["web", "Web"],
      ["social", "Social"],
      ["print", "Print"],
      ["archive", "Archive"],
    ],
    "preset",
  );
}

export function mapStripMetadata(value: string): string {
  return mapEnum(
    value,
    [
      ["all", "All"],
      ["keep_exif", "KeepExif"],
      ["keep-exif", "KeepExif"],
      ["keep_icc", "KeepIcc"],
      ["keep-icc", "KeepIcc"],
      ["keep_all", "KeepAll"],
      ["keep-all", "KeepAll"],
    ],
    "strip_metadata",
  );
}

/** Validate quality 1-100, throwing `INVALID_QUALITY` (the same
 *  code the core reports). */
export function assertQuality(quality: unknown): number {
  if (
    typeof quality !== "number" ||
    !Number.isInteger(quality) ||
    quality < 1 ||
    quality > 100
  ) {
    throw new DusimagezError(
      "INVALID_QUALITY",
      `Quality ${String(quality)} is invalid. Quality must be between 1 and 100.`,
      `Quality ${String(quality)} galat hai. Quality 1 se 100 ke beech honi chahiye.`,
    );
  }
  return quality;
}

function assertNonNegativeInt(value: unknown, what: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0) {
    throw invalidArgument(
      `${what} must be a non-negative integer.`,
      `${what} non-negative integer hona chahiye.`,
    );
  }
  return value;
}

function assertPositiveInt(value: unknown, what: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value <= 0) {
    throw new DusimagezError(
      "INVALID_DIMENSIONS",
      `${what} must be a positive integer.`,
      `${what} positive integer hona chahiye.`,
    );
  }
  return value;
}

/** Translate camelCase TS options to a complete snake_case Rust
 *  object. Every Rust-required field is filled with the Rust
 *  default, because serde rejects partial objects for non-`Option`
 *  fields. Undefined optional fields are sent as `null`. */
export function toCompressOptions(
  options?: CompressOptions,
): Record<string, unknown> {
  const quality = assertQuality(options?.quality ?? 80);
  const effort = options?.effort ?? 6;
  if (!Number.isInteger(effort) || effort < 0 || effort > 10) {
    throw invalidArgument(
      `Effort ${effort} is invalid. Effort must be between 0 and 10.`,
      `Effort ${effort} galat hai. Effort 0 se 10 ke beech hona chahiye.`,
    );
  }
  const pngCompressionLevel = options?.pngCompressionLevel ?? 6;
  if (
    !Number.isInteger(pngCompressionLevel) ||
    pngCompressionLevel < 0 ||
    pngCompressionLevel > 9
  ) {
    throw invalidArgument(
      `PNG compression level ${pngCompressionLevel} is invalid. It must be between 0 and 9.`,
      `PNG compression level ${pngCompressionLevel} galat hai. Ye 0 se 9 ke beech hona chahiye.`,
    );
  }
  // Subsampling: send null when not set explicitly. The core then
  // applies its own rule (preset value, else S420 for quality < 90
  // and S444 otherwise). Deriving it here would silently override
  // the preset, e.g. preset Web + quality 95 must stay S420.
  const subsampling =
    options?.subsampling !== undefined
      ? mapSubsampling(options.subsampling)
      : null;
  let dpi: [number, number] | null = null;
  if (options?.dpi !== undefined && options.dpi !== null) {
    const [x, y] = options.dpi;
    if (
      !isFiniteNumber(x) ||
      !isFiniteNumber(y) ||
      !Number.isInteger(x) ||
      !Number.isInteger(y) ||
      x <= 0 ||
      y <= 0
    ) {
      throw invalidArgument(
        "dpi must be a [x, y] pair of positive integers.",
        "dpi [x, y] positive integers ka pair hona chahiye.",
      );
    }
    dpi = [x, y];
  }
  return {
    quality,
    format:
      options?.format !== undefined && options.format !== null
        ? mapFormat(options.format)
        : null,
    progressive: options?.progressive ?? false,
    lossless: options?.lossless ?? false,
    effort,
    subsampling,
    png_compression_level: pngCompressionLevel,
    palette: options?.palette ?? false,
    preset:
      options?.preset !== undefined && options.preset !== null
        ? mapPreset(options.preset)
        : null,
    target_bytes:
      options?.targetBytes !== undefined && options.targetBytes !== null
        ? assertNonNegativeInt(options.targetBytes, "Target size")
        : null,
    max_width:
      options?.maxWidth !== undefined && options.maxWidth !== null
        ? assertPositiveInt(options.maxWidth, "Max width")
        : null,
    max_height:
      options?.maxHeight !== undefined && options.maxHeight !== null
        ? assertPositiveInt(options.maxHeight, "Max height")
        : null,
    strip_metadata:
      options?.stripMetadata !== undefined
        ? mapStripMetadata(options.stripMetadata)
        : "All",
    auto_orient: options?.autoOrient ?? true,
    dpi,
    max_pixels:
      options?.maxPixels !== undefined
        ? assertNonNegativeInt(options.maxPixels, "Max pixels")
        : 120_000_000,
    max_dimension:
      options?.maxDimension !== undefined
        ? assertPositiveInt(options.maxDimension, "Max dimension")
        : 16000,
  };
}

/** Assert a value is a Uint8Array (Buffers pass: Buffer extends
 *  Uint8Array). Empty bytes are EMPTY_INPUT, like the Rust core. */
export function assertImageBytes(value: unknown, what = "Image bytes"): Uint8Array {
  if (!(value instanceof Uint8Array)) {
    throw new DusimagezError(
      "INVALID_TYPE",
      `${what} must be a Uint8Array.`,
      `${what} Uint8Array hona chahiye.`,
    );
  }
  if (value.byteLength === 0) {
    throw new DusimagezError(
      "EMPTY_INPUT",
      `${what} is empty.`,
      `${what} khaali hai.`,
    );
  }
  return value;
}

/** Assert an array of Uint8Arrays (bulk inputs). */
export function assertImageArray(
  value: unknown,
  what: string,
  emptyWhat: string,
): Uint8Array[] {
  if (!Array.isArray(value)) {
    throw new DusimagezError(
      "INVALID_TYPE",
      `${what} must be an array of Uint8Array.`,
      `${what} Uint8Array ki array honi chahiye.`,
    );
  }
  if (value.length === 0) {
    throw new DusimagezError(
      "EMPTY_INPUT",
      `${emptyWhat} is empty.`,
      `${emptyWhat} khaali hai.`,
    );
  }
  for (const item of value) {
    if (!(item instanceof Uint8Array)) {
      throw new DusimagezError(
        "INVALID_TYPE",
        `${what} must be an array of Uint8Array.`,
        `${what} Uint8Array ki array honi chahiye.`,
      );
    }
  }
  return value as Uint8Array[];
}

export function assertTargetBytes(value: unknown): number {
  return assertNonNegativeInt(value, "Target size");
}
