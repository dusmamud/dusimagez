/**
 * dusimagez JS tests — pure modules only (no WASM).
 *
 * These run without the wasm build: Zod schemas, option
 * translation + validation, error-JSON parsing, and report
 * parsing / maths. The op tests live in `index.test.ts` and run
 * once the wasm package is built (`npm run build` via `pretest`).
 */
import { describe, expect, it } from "vitest";
import {
  DusimagezError,
  toDusimagezError,
} from "./errors";
import {
  assertImageBytes,
  assertTargetBytes,
  mapFormat,
  mapPreset,
  mapStripMetadata,
  mapSubsampling,
  toCompressOptions,
} from "./options";
import {
  checkReportMaths,
  formatLower,
  parseBulk,
  parseImageInfo,
  parseReport,
  type CompressReport,
} from "./parse";
import {
  bulkResultSchema,
  compressOptionsSchema,
  compressReportSchema,
  imageBytesSchema,
  imageInfoSchema,
} from "./zod";

// ---------------------------------------------------------------------------
// Zod schemas
// ---------------------------------------------------------------------------

describe("zod schemas", () => {
  it("accepts a valid compress options object", () => {
    const parsed = compressOptionsSchema.parse({
      quality: 80,
      format: "jpeg",
      subsampling: "s420",
      preset: "web",
      stripMetadata: "keep_exif",
      effort: 6,
      pngCompressionLevel: 6,
      targetBytes: 200000,
      maxWidth: 1200,
      dpi: [300, 300],
      maxPixels: 120000000,
      maxDimension: 16000,
    });
    expect(parsed.quality).toBe(80);
    expect(parsed.format).toBe("jpeg");
  });

  it("accepts an empty (all-default) options object", () => {
    expect(compressOptionsSchema.parse({})).toEqual({});
  });

  it("rejects quality outside 1-100", () => {
    expect(() => compressOptionsSchema.parse({ quality: 0 })).toThrow();
    expect(() => compressOptionsSchema.parse({ quality: 101 })).toThrow();
    expect(() => compressOptionsSchema.parse({ quality: 80.5 })).toThrow();
  });

  it("rejects effort outside 0-10 and PNG level outside 0-9", () => {
    expect(() => compressOptionsSchema.parse({ effort: 11 })).toThrow();
    expect(() => compressOptionsSchema.parse({ pngCompressionLevel: 10 })).toThrow();
  });

  it("rejects unknown enum values", () => {
    expect(() => compressOptionsSchema.parse({ format: "heic" })).toThrow();
    expect(() => compressOptionsSchema.parse({ subsampling: "s111" })).toThrow();
    expect(() => compressOptionsSchema.parse({ preset: "tiny" })).toThrow();
    expect(() => compressOptionsSchema.parse({ stripMetadata: "keep_all_exif" })).toThrow();
  });

  it("accepts tiff as a format (core falls back to PNG output)", () => {
    expect(compressOptionsSchema.parse({ format: "tiff" }).format).toBe("tiff");
  });

  it("validates image bytes / info / report shapes", () => {
    expect(imageBytesSchema.parse(new Uint8Array([1, 2, 3]))).toBeInstanceOf(Uint8Array);
    expect(() => imageBytesSchema.parse("nope")).toThrow();
    expect(
      imageInfoSchema.parse({
        format: "png",
        width: 64,
        height: 48,
        hasAlpha: false,
        colourType: "rgb8",
        fileSize: 1234,
      }),
    ).toBeTruthy();
    expect(
      compressReportSchema.parse({
        output: new Uint8Array([1]),
        originalSize: 1000,
        outputSize: 800,
        savedBytes: 200,
        savedPercent: 20,
        format: "jpeg",
        width: 64,
        height: 48,
        qualityUsed: 80,
        resized: false,
      }),
    ).toBeTruthy();
    expect(
      bulkResultSchema.parse({
        index: 0,
        ok: false,
        error: { code: "INVALID_IMAGE", messageEn: "x", messageHi: "y" },
      }),
    ).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Options translation
// ---------------------------------------------------------------------------

describe("toCompressOptions", () => {
  it("fills Rust defaults for an empty options object", () => {
    const out = toCompressOptions({});
    expect(out["quality"]).toBe(80);
    expect(out["format"]).toBeNull();
    expect(out["progressive"]).toBe(false);
    expect(out["lossless"]).toBe(false);
    expect(out["effort"]).toBe(6);
    expect(out["subsampling"]).toBeNull(); // unset: the core applies the preset / quality rule
    expect(out["png_compression_level"]).toBe(6);
    expect(out["palette"]).toBe(false);
    expect(out["preset"]).toBeNull();
    expect(out["target_bytes"]).toBeNull();
    expect(out["max_width"]).toBeNull();
    expect(out["max_height"]).toBeNull();
    expect(out["strip_metadata"]).toBe("All");
    expect(out["auto_orient"]).toBe(true);
    expect(out["dpi"]).toBeNull();
    expect(out["max_pixels"]).toBe(120_000_000);
    expect(out["max_dimension"]).toBe(16000);
  });

  it("leaves subsampling unset so the core applies its rule (preset, else S420 < 90 / S444 >= 90)", () => {
    // The quality-dependent default lives in the core's
    // CompressOptions::resolved(), not here: deriving it in JS
    // would silently override a preset (e.g. preset Web + quality
    // 95 must stay S420, not become S444).
    expect(toCompressOptions({ quality: 90 })["subsampling"]).toBeNull();
    expect(toCompressOptions({ quality: 95 })["subsampling"]).toBeNull();
    expect(toCompressOptions({ quality: 89 })["subsampling"]).toBeNull();
    expect(toCompressOptions({ subsampling: "s444" })["subsampling"]).toBe("S444");
    expect(toCompressOptions({ subsampling: "s420" })["subsampling"]).toBe("S420");
  });

  it("maps friendly enum spellings to canonical serde names", () => {
    expect(mapFormat("jpeg")).toBe("Jpeg");
    expect(mapFormat("jpg")).toBe("Jpeg");
    expect(mapFormat("webp")).toBe("WebP");
    expect(mapFormat("Jpeg")).toBe("Jpeg");
    expect(mapFormat("tiff")).toBe("Tiff"); // core falls back to PNG output
    expect(mapSubsampling("s444")).toBe("S444");
    expect(mapSubsampling("4:2:0")).toBe("S420");
    expect(mapPreset("web")).toBe("Web");
    expect(mapStripMetadata("keep_exif")).toBe("KeepExif");
    expect(mapStripMetadata("keep-icc")).toBe("KeepIcc");
  });

  it("rejects unknown enum values with INVALID_ARGUMENT", () => {
    expect(() => mapFormat("heic")).toThrow(DusimagezError);
    try {
      mapFormat("heic");
    } catch (e) {
      expect((e as DusimagezError).code).toBe("INVALID_ARGUMENT");
    }
  });

  it("translates camelCase keys to snake_case", () => {
    const out = toCompressOptions({
      quality: 70,
      format: "jpeg",
      pngCompressionLevel: 9,
      targetBytes: 200000,
      maxWidth: 1200,
      maxHeight: 800,
      stripMetadata: "keep_all",
      dpi: [300, 300],
    });
    expect(out["png_compression_level"]).toBe(9);
    expect(out["target_bytes"]).toBe(200000);
    expect(out["max_width"]).toBe(1200);
    expect(out["max_height"]).toBe(800);
    expect(out["strip_metadata"]).toBe("KeepAll");
    expect(out["dpi"]).toEqual([300, 300]);
  });

  it("throws INVALID_QUALITY for quality outside 1-100", () => {
    for (const bad of [0, 101, -5, 80.5, Number.NaN]) {
      try {
        toCompressOptions({ quality: bad });
        expect.unreachable();
      } catch (e) {
        expect((e as DusimagezError).code).toBe("INVALID_QUALITY");
      }
    }
  });

  it("throws INVALID_ARGUMENT for effort / PNG level out of range", () => {
    expect(() => toCompressOptions({ effort: 11 })).toThrow(DusimagezError);
    expect(() => toCompressOptions({ pngCompressionLevel: 10 })).toThrow(DusimagezError);
  });

  it("throws INVALID_DIMENSIONS for non-positive maxWidth/maxHeight", () => {
    try {
      toCompressOptions({ maxWidth: 0 });
      expect.unreachable();
    } catch (e) {
      expect((e as DusimagezError).code).toBe("INVALID_DIMENSIONS");
    }
  });
});

// ---------------------------------------------------------------------------
// Input validation + error parsing
// ---------------------------------------------------------------------------

describe("input validation and error parsing", () => {
  it("accepts Uint8Array (and Buffer) and rejects everything else", () => {
    expect(assertImageBytes(new Uint8Array([1]))).toBeInstanceOf(Uint8Array);
    expect(assertImageBytes(Buffer.from([1, 2]))).toBeInstanceOf(Uint8Array);
    for (const bad of ["x", 42, null, {}, [1, 2]]) {
      try {
        assertImageBytes(bad);
        expect.unreachable();
      } catch (e) {
        expect((e as DusimagezError).code).toBe("INVALID_TYPE");
      }
    }
  });

  it("empty bytes are EMPTY_INPUT", () => {
    try {
      assertImageBytes(new Uint8Array(0));
      expect.unreachable();
    } catch (e) {
      expect((e as DusimagezError).code).toBe("EMPTY_INPUT");
    }
  });

  it("parses the wasm error JSON into a DusimagezError", () => {
    const err = toDusimagezError(
      JSON.stringify({
        ok: false,
        code: "INVALID_IMAGE",
        message_en: "Not an image.",
        message_hi: "Image nahi hai.",
      }),
    );
    expect(err).toBeInstanceOf(DusimagezError);
    expect(err.code).toBe("INVALID_IMAGE");
    expect(err.messageEn).toBe("Not an image.");
    expect(err.messageHi).toBe("Image nahi hai.");
    expect(err.reasonEn).toBe("Not an image.");
  });

  it("passes DusimagezError through and wraps plain strings / Errors", () => {
    const original = new DusimagezError("EMPTY_INPUT", "Empty.", "Khaali hai.");
    expect(toDusimagezError(original)).toBe(original);
    expect(toDusimagezError("boom").code).toBe("OPERATION_FAILED");
    expect(toDusimagezError(new Error("kaboom")).messageEn).toBe("kaboom");
  });

  it("assertTargetBytes rejects negatives", () => {
    expect(assertTargetBytes(0)).toBe(0);
    expect(() => assertTargetBytes(-1)).toThrow(DusimagezError);
  });
});

// ---------------------------------------------------------------------------
// Report parsing + maths
// ---------------------------------------------------------------------------

function fakeReportJson(overrides: Record<string, unknown> = {}): string {
  const output = Buffer.from("x".repeat(800)).toString("base64");
  return JSON.stringify({
    output,
    original_size: 1000,
    output_size: 800,
    saved_bytes: 200,
    saved_percent: 20,
    format: "Jpeg",
    width: 64,
    height: 48,
    quality_used: 80,
    resized: false,
    ...overrides,
  });
}

describe("report parsing and maths", () => {
  it("parses a report JSON string (base64 output, lowercased format)", () => {
    const report = parseReport(fakeReportJson());
    expect(report.output).toBeInstanceOf(Uint8Array);
    expect(report.output.byteLength).toBe(800);
    expect(report.originalSize).toBe(1000);
    expect(report.outputSize).toBe(800);
    expect(report.savedBytes).toBe(200);
    expect(report.savedPercent).toBe(20);
    expect(report.format).toBe("jpeg");
    expect(report.width).toBe(64);
    expect(report.qualityUsed).toBe(80);
    expect(report.resized).toBe(false);
    expect(checkReportMaths(report)).toEqual([]);
  });

  it("lowercases format names", () => {
    expect(formatLower("Jpeg")).toBe("jpeg");
    expect(formatLower("WebP")).toBe("webp");
    expect(formatLower("jpeg")).toBe("jpeg");
    expect(formatLower("JPG")).toBe("jpeg");
  });

  it("parses image info JSON", () => {
    const info = parseImageInfo(
      JSON.stringify({
        format: "Png",
        width: 64,
        height: 48,
        has_alpha: false,
        colour_type: "rgb8",
        file_size: 1234,
      }),
    );
    expect(info.format).toBe("png");
    expect(info.width).toBe(64);
    expect(info.hasAlpha).toBe(false);
    expect(info.colourType).toBe("rgb8");
    expect(info.fileSize).toBe(1234);
  });

  it("checkReportMaths catches bad savedBytes / savedPercent / outputSize", () => {
    const base: CompressReport = parseReport(fakeReportJson());
    expect(
      checkReportMaths({ ...base, savedBytes: 150 }).length,
    ).toBeGreaterThan(0);
    expect(
      checkReportMaths({ ...base, savedPercent: 90 }).length,
    ).toBeGreaterThan(0);
    expect(
      checkReportMaths({ ...base, outputSize: 799 }).length,
    ).toBeGreaterThan(0);
    expect(checkReportMaths(base)).toEqual([]);
  });

  it("parses a bulk JSON string (report + per-item error)", () => {
    const reportJson = JSON.parse(fakeReportJson()) as Record<string, unknown>;
    const bulk = parseBulk(
      JSON.stringify([
        { index: 0, ok: true, report: reportJson },
        {
          index: 1,
          ok: false,
          error: {
            code: "INVALID_IMAGE",
            message_en: "Not an image.",
            message_hi: "Image nahi hai.",
          },
        },
      ]),
    );
    expect(bulk).toHaveLength(2);
    expect(bulk[0].ok).toBe(true);
    expect(bulk[0].report?.format).toBe("jpeg");
    expect(bulk[0].report?.output.byteLength).toBe(800);
    expect(bulk[1].ok).toBe(false);
    expect(bulk[1].error?.code).toBe("INVALID_IMAGE");
    expect(bulk[1].report).toBeUndefined();
  });
});
