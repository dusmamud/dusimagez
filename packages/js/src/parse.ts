/**
 * dusimagez — result parsing (pure module, no WASM import).
 *
 * The wasm layer returns `get_image_info` / `compress_with_report`
 * / `bulk_compress` as JSON strings; this module parses them into
 * camelCase objects. Report `output` bytes arrive base64 and are
 * decoded back to `Uint8Array`s. `format` is lowercased
 * (`"jpeg"`, `"png"`, …) whatever the core's representation is.
 */

/** Summary of an image, from {@link getImageInfo}. */
export interface ImageInfo {
  /** Lowercase short name: "jpeg" | "png" | "webp" | "avif" | "gif" | "bmp" | "tiff". */
  format: string;
  width: number;
  height: number;
  hasAlpha: boolean;
  /** Lowercase `image`-crate colour type, e.g. "rgb8", "rgba8". */
  colourType: string;
  /** Input byte length. */
  fileSize: number;
}

/** Before/after report, from {@link compressWithReport}. */
export interface CompressReport {
  /** Output bytes. */
  output: Uint8Array;
  /** Input byte length. */
  originalSize: number;
  /** Output byte length. */
  outputSize: number;
  /** `originalSize - outputSize`. */
  savedBytes: number;
  /** `100 * savedBytes / originalSize`. */
  savedPercent: number;
  /** Lowercase output format, e.g. "jpeg". */
  format: string;
  /** Output width in px. */
  width: number;
  /** Output height in px. */
  height: number;
  /** Quality the encoder actually used. */
  qualityUsed: number;
  /** True when the output was downscaled (`maxWidth` / `maxHeight` / target size). */
  resized: boolean;
}

/** One item of a {@link bulkCompress} result, in input order. */
export interface BulkResult {
  /** Input index (0-based). */
  index: number;
  /** True when this item succeeded. */
  ok: boolean;
  /** Report on success; absent on failure. */
  report?: CompressReport;
  /** Error on failure; absent on success. */
  error?: { code: string; messageEn: string; messageHi: string };
}

/** Lowercase a format name whatever the core's representation is:
 *  `"Jpeg"` -> `"jpeg"`, `"WebP"` -> `"webp"`, `"jpeg"` stays. */
export function formatLower(value: string): string {
  const lower = value.trim().toLowerCase();
  return lower === "jpg" ? "jpeg" : lower;
}

function base64ToBytes(b64: string): Uint8Array {
  // Node-only package (wasm --target nodejs); Buffer is available.
  return new Uint8Array(Buffer.from(b64, "base64"));
}

export function parseImageInfo(json: string): ImageInfo {
  const parsed = JSON.parse(json) as {
    format: string;
    width: number;
    height: number;
    has_alpha: boolean;
    colour_type: string;
    file_size: number;
  };
  return {
    format: formatLower(parsed.format),
    width: parsed.width,
    height: parsed.height,
    hasAlpha: parsed.has_alpha,
    colourType: parsed.colour_type,
    fileSize: parsed.file_size,
  };
}

export function parseReport(json: string): CompressReport {
  const parsed = JSON.parse(json) as {
    output: string;
    original_size: number;
    output_size: number;
    saved_bytes: number;
    saved_percent: number;
    format: string;
    width: number;
    height: number;
    quality_used: number;
    resized: boolean;
  };
  return {
    output: base64ToBytes(parsed.output),
    originalSize: parsed.original_size,
    outputSize: parsed.output_size,
    savedBytes: parsed.saved_bytes,
    savedPercent: parsed.saved_percent,
    format: formatLower(parsed.format),
    width: parsed.width,
    height: parsed.height,
    qualityUsed: parsed.quality_used,
    resized: parsed.resized,
  };
}

/** Parse one report object (already JSON-parsed, as inside a bulk
 *  result) into a {@link CompressReport}. */
export function parseReportObject(parsed: {
  output: string;
  original_size: number;
  output_size: number;
  saved_bytes: number;
  saved_percent: number;
  format: string;
  width: number;
  height: number;
  quality_used: number;
  resized: boolean;
}): CompressReport {
  return {
    output: base64ToBytes(parsed.output),
    originalSize: parsed.original_size,
    outputSize: parsed.output_size,
    savedBytes: parsed.saved_bytes,
    savedPercent: parsed.saved_percent,
    format: formatLower(parsed.format),
    width: parsed.width,
    height: parsed.height,
    qualityUsed: parsed.quality_used,
    resized: parsed.resized,
  };
}

export function parseBulk(json: string): BulkResult[] {
  const items = JSON.parse(json) as Array<{
    index: number;
    ok: boolean;
    report?: {
      output: string;
      original_size: number;
      output_size: number;
      saved_bytes: number;
      saved_percent: number;
      format: string;
      width: number;
      height: number;
      quality_used: number;
      resized: boolean;
    } | null;
    error?: { code: string; message_en: string; message_hi: string } | null;
  }>;
  return items.map((item) => {
    const out: BulkResult = { index: item.index, ok: item.ok };
    if (item.report) out.report = parseReportObject(item.report);
    if (item.error) {
      out.error = {
        code: item.error.code,
        messageEn: item.error.message_en,
        messageHi: item.error.message_hi,
      };
    }
    return out;
  });
}

/**
 * Pure check: does a report's maths add up? Returns a list of
 * human-readable mismatches (empty when the maths holds).
 * `savedPercent` is compared with a 0.5-point tolerance (the core
 * stores it as f32).
 */
export function checkReportMaths(report: CompressReport): string[] {
  const problems: string[] = [];
  if (report.outputSize !== report.output.byteLength) {
    problems.push("outputSize does not match len(output)");
  }
  if (report.savedBytes !== report.originalSize - report.outputSize) {
    problems.push(
      `savedBytes ${report.savedBytes} != originalSize - outputSize ` +
        `(${report.originalSize} - ${report.outputSize} = ${report.originalSize - report.outputSize})`,
    );
  }
  const expectedPercent =
    report.originalSize === 0
      ? 0
      : (100 * report.savedBytes) / report.originalSize;
  if (Math.abs(report.savedPercent - expectedPercent) > 0.5) {
    problems.push(
      `savedPercent ${report.savedPercent} != 100*saved/original (${expectedPercent.toFixed(2)})`,
    );
  }
  return problems;
}
