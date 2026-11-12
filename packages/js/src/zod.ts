/**
 * dusimagez — Zod schemas.
 *
 * Optional integration: `zod` is a dependency of this package.
 * Importing this module requires zod to be installed; the main entry
 * point (`index.ts`) never imports zod.
 *
 * The schemas mirror the TS option interfaces in `options.ts`
 * (camelCase keys; the wrapper translates them to the Rust
 * snake_case JSON keys). They check shape only — image validity
 * and value ranges like quality 1–100 are enforced by the core and
 * reported as `DusimagezError`s, not Zod errors — except quality,
 * which the wrapper validates up front with `INVALID_QUALITY`.
 * This module validates quality 1–100 too, so a schema check and
 * the wrapper agree.
 */
import { z } from "zod";

export const imageBytesSchema = z.instanceof(Uint8Array, {
  message: "Input must be a Uint8Array. / Input Uint8Array hona chahiye.",
});

export const imageArraySchema = z.array(imageBytesSchema);

export const imageFormatSchema = z.enum([
  "jpeg",
  "jpg",
  "png",
  "webp",
  "avif",
  "gif",
  "bmp",
  "tiff",
  "Jpeg",
  "Png",
  "WebP",
  "Avif",
  "Gif",
  "Bmp",
  "Tiff",
]);

export const subsamplingSchema = z.enum([
  "s444",
  "s422",
  "s420",
  "444",
  "422",
  "420",
  "4:4:4",
  "4:2:2",
  "4:2:0",
  "S444",
  "S422",
  "S420",
]);

export const presetSchema = z.enum([
  "web",
  "social",
  "print",
  "archive",
  "Web",
  "Social",
  "Print",
  "Archive",
]);

export const stripMetadataSchema = z.enum([
  "all",
  "keep_exif",
  "keep_icc",
  "keep_all",
  "keep-exif",
  "keep-icc",
  "keep-all",
  "All",
  "KeepExif",
  "KeepIcc",
  "KeepAll",
]);

export const imageInfoSchema = z.object({
  format: z.string(),
  width: z.number().int().nonnegative(),
  height: z.number().int().nonnegative(),
  hasAlpha: z.boolean(),
  colourType: z.string(),
  fileSize: z.number().int().nonnegative(),
});

export const compressReportSchema = z.object({
  output: imageBytesSchema,
  originalSize: z.number().int().nonnegative(),
  outputSize: z.number().int().nonnegative(),
  savedBytes: z.number().int().nonnegative(),
  savedPercent: z.number().min(0).max(100),
  format: z.string(),
  width: z.number().int().nonnegative(),
  height: z.number().int().nonnegative(),
  qualityUsed: z.number().int().min(1).max(100),
  resized: z.boolean(),
});

export const commonOptionsSchema = z
  .object({
    autoOrient: z.boolean(),
    dpi: z.tuple([z.number().int().positive(), z.number().int().positive()]).nullable(),
    maxPixels: z.number().int().nonnegative(),
    maxDimension: z.number().int().positive(),
  })
  .partial();

export const compressOptionsSchema = commonOptionsSchema
  .extend({
    quality: z.number().int().min(1).max(100),
    format: imageFormatSchema.nullable(),
    progressive: z.boolean(),
    lossless: z.boolean(),
    effort: z.number().int().min(0).max(10),
    subsampling: subsamplingSchema,
    pngCompressionLevel: z.number().int().min(0).max(9),
    palette: z.boolean(),
    preset: presetSchema.nullable(),
    targetBytes: z.number().int().nonnegative().nullable(),
    maxWidth: z.number().int().positive().nullable(),
    maxHeight: z.number().int().positive().nullable(),
    stripMetadata: stripMetadataSchema,
  })
  .partial();

export const bulkResultSchema = z.object({
  index: z.number().int().nonnegative(),
  ok: z.boolean(),
  report: compressReportSchema.optional(),
  error: z
    .object({
      code: z.string(),
      messageEn: z.string(),
      messageHi: z.string(),
    })
    .optional(),
});
