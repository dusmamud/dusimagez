/* dusimagez.h — C API for dusimagez v0.1.0 (offline image compressor).
 *
 * This header matches the exported symbols of `libdusimagez_ffi`
 * (`crates/dusimagez-ffi`, crate-type cdylib/staticlib) exactly.
 * Every function is `dusimagez_`-prefixed; all image work is done by
 * the same Rust core the CLI / Python / WASM bindings use, so results
 * agree across languages.
 *
 * Status codes (return value of every function except
 * `dusimagez_last_error` and `dusimagez_version`, which return
 * pointers, and `dusimagez_free`, which returns nothing):
 *   0  OK
 *   1  INVALID_IMAGE            input bytes are not a recognisable image
 *   2  UNSUPPORTED_FORMAT       format known but not supported by this build
 *   3  IMAGE_TOO_LARGE          exceeds max_pixels / max_dimension
 *   4  TARGET_SIZE_UNREACHABLE  even the lowest quality exceeds the target
 *   5  INVALID_QUALITY          quality outside 1-100
 *   6  INVALID_DIMENSIONS       a dimension is zero, missing or unusable
 *   7  INVALID_ARGUMENT         bad value, NULL pointer, ...
 *   8  EMPTY_INPUT              empty input where input is required
 *   99 UNKNOWN                   anything else (including an internal error)
 *
 * On failure, call `dusimagez_last_error()` for the JSON error:
 *   {"code": "...", "message_en": "...", "message_hi": "..."}
 *
 * Memory ownership: on success, `*out` / `report.output` points at a
 * heap buffer of `*out_len` / `report.output_len` bytes owned by the
 * caller. Free it exactly once with `dusimagez_free(ptr, len)`.
 * `dusimagez_last_error()` and `dusimagez_version()` return
 * NUL-terminated C strings owned by the library; do NOT free them.
 *
 * Options: every `options` parameter is a pointer to a
 * `DusimagezOptions` struct, or NULL for all defaults. Numeric fields
 * use -1 for "unset / use the core default" (see the struct comments).
 * The FFI layer starts from `CompressOptions::default()` and overrides
 * only the fields the caller set, so core-side default rules (e.g. the
 * subsampling default, S420 for quality < 90 and S444 otherwise) stay
 * in the core. `dusimagez_compress` honours `options.target_bytes`
 * when set: it behaves exactly like `dusimagez_compress_to_size`.
 *
 * Threading: the last error is thread-local; a call on one thread
 * never overwrites another thread's error. The image functions are
 * safe to call concurrently from multiple threads.
 */

#ifndef DUSIMAGEZ_H
#define DUSIMAGEZ_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Status codes */
#define DUSIMAGEZ_OK 0
#define DUSIMAGEZ_INVALID_IMAGE 1
#define DUSIMAGEZ_UNSUPPORTED_FORMAT 2
#define DUSIMAGEZ_IMAGE_TOO_LARGE 3
#define DUSIMAGEZ_TARGET_SIZE_UNREACHABLE 4
#define DUSIMAGEZ_INVALID_QUALITY 5
#define DUSIMAGEZ_INVALID_DIMENSIONS 6
#define DUSIMAGEZ_INVALID_ARGUMENT 7
#define DUSIMAGEZ_EMPTY_INPUT 8
#define DUSIMAGEZ_UNKNOWN 99

/* Format codes (DusimagezOptions.format, DusimagezReport.format,
 * DusimagezInfo.format) */
#define DUSIMAGEZ_FORMAT_KEEP 0
#define DUSIMAGEZ_FORMAT_JPEG 1
#define DUSIMAGEZ_FORMAT_PNG 2
#define DUSIMAGEZ_FORMAT_WEBP 3
#define DUSIMAGEZ_FORMAT_AVIF 4
#define DUSIMAGEZ_FORMAT_GIF 5
#define DUSIMAGEZ_FORMAT_BMP 6
#define DUSIMAGEZ_FORMAT_TIFF 7

/* Subsampling codes (DusimagezOptions.subsampling) */
#define DUSIMAGEZ_SUBSAMPLING_DEFAULT 0
#define DUSIMAGEZ_SUBSAMPLING_S444 1
#define DUSIMAGEZ_SUBSAMPLING_S422 2
#define DUSIMAGEZ_SUBSAMPLING_S420 3

/* Preset codes (DusimagezOptions.preset) */
#define DUSIMAGEZ_PRESET_NONE 0
#define DUSIMAGEZ_PRESET_WEB 1
#define DUSIMAGEZ_PRESET_SOCIAL 2
#define DUSIMAGEZ_PRESET_PRINT 3
#define DUSIMAGEZ_PRESET_ARCHIVE 4

/* Metadata strip codes (DusimagezOptions.strip_metadata) */
#define DUSIMAGEZ_STRIP_ALL 0
#define DUSIMAGEZ_STRIP_KEEP_EXIF 1
#define DUSIMAGEZ_STRIP_KEEP_ICC 2
#define DUSIMAGEZ_STRIP_KEEP_ALL 3

/* C mirror of CompressOptions (SPEC section 2). Numeric fields are -1
 * for "unset / use the core default", except the enum/bool fields
 * where 0 already means the default (see below). A NULL options
 * pointer means "all defaults". */
typedef struct {
    /* Quality 1-100. -1 = default (80). 0 or >100 is INVALID_QUALITY. */
    int32_t quality;
    /* Format code: 0 = keep input format, 1 = jpeg, 2 = png,
     * 3 = webp, 4 = avif, 5 = gif, 6 = bmp, 7 = tiff. -1 = keep. */
    int32_t format;
    /* -1 = default (false), 0 = false, 1 = true. */
    int32_t progressive;
    /* -1 = default (false), 0 = false, 1 = true. */
    int32_t lossless;
    /* Encoding effort 0-10. -1 = default (6). >10 is INVALID_ARGUMENT. */
    int32_t effort;
    /* Subsampling code: 0 = core default, 1 = S444, 2 = S422, 3 = S420.
     * -1 = core default (same as 0). */
    int32_t subsampling;
    /* PNG compression level 0-9. -1 = default (6). >9 is
     * INVALID_ARGUMENT. */
    int32_t png_compression_level;
    /* -1 = default (false), 0 = false, 1 = true. */
    int32_t palette;
    /* Preset code: 0 = none, 1 = Web, 2 = Social, 3 = Print,
     * 4 = Archive. -1 = none (same as 0). */
    int32_t preset;
    /* Target output size in bytes. -1 = unset. 0 is INVALID_ARGUMENT.
     * When set, dusimagez_compress behaves like
     * dusimagez_compress_to_size. */
    int64_t target_bytes;
    /* Downscale-only max width in px. -1 = unset. 0 is
     * INVALID_DIMENSIONS. */
    int64_t max_width;
    /* Downscale-only max height in px. -1 = unset. 0 is
     * INVALID_DIMENSIONS. */
    int64_t max_height;
    /* Metadata strip code: 0 = All (default), 1 = KeepExif,
     * 2 = KeepIcc, 3 = KeepAll. -1 = All (same as 0). */
    int32_t strip_metadata;
    /* -1 = default (true), 0 = false, 1 = true. */
    int32_t auto_orient;
    /* Horizontal DPI. -1 = unset. Must be set together with dpi_y
     * (both -1, or both >0). */
    int64_t dpi_x;
    /* Vertical DPI. -1 = unset. Must be set together with dpi_x. */
    int64_t dpi_y;
    /* Refuse images with more pixels than this. -1 = default
     * (120000000). 0 is INVALID_ARGUMENT. */
    int64_t max_pixels;
    /* Refuse images with either side longer than this. -1 = default
     * (16000). 0 is INVALID_ARGUMENT. */
    int64_t max_dimension;
} DusimagezOptions;

/* C mirror of CompressReport (SPEC section 1). The caller allocates
 * this struct; dusimagez_compress_with_report fills it in. On success
 * `output` points at a heap buffer of `output_len` bytes owned by the
 * caller — free it exactly once with dusimagez_free(output,
 * output_len). On failure `output` is NULL and `output_len` is 0. */
typedef struct {
    uint8_t *output;
    size_t output_len;
    uint64_t original_size;
    uint64_t output_size;
    uint64_t saved_bytes;
    float saved_percent;
    /* Format code (same table as DusimagezOptions.format). 0 never
     * occurs here — the report always names a format. */
    int32_t format;
    uint32_t width;
    uint32_t height;
    uint8_t quality_used;
    /* 0 = not resized, 1 = resized. */
    int32_t resized;
} DusimagezReport;

/* C mirror of ImageInfo (SPEC section 1). The caller allocates this
 * struct; dusimagez_get_info fills it in. */
typedef struct {
    /* Format code (same table as DusimagezOptions.format). */
    int32_t format;
    uint32_t width;
    uint32_t height;
    /* 0 = no alpha, 1 = has alpha. */
    int32_t has_alpha;
    /* Lowercase image-crate colour type ("rgb8", "rgba8", "la8",
     * "l8", ...), NUL-terminated, truncated to 23 chars. */
    char colour_type[24];
    uint64_t file_size;
} DusimagezInfo;

/* Re-encode at a fixed quality (and `format`, or the input format when
 * unset). `options` is a DusimagezOptions pointer, or NULL for all
 * defaults. When `options.target_bytes` is set, this behaves exactly
 * like dusimagez_compress_to_size (SPEC section 2). */
int32_t dusimagez_compress(const uint8_t *input, size_t input_len,
                           const DusimagezOptions *options,
                           uint8_t **out, size_t *out_len);

/* Re-encode so the result is at most `target_bytes` bytes (iterative
 * quality search; status 4 when unreachable). Same options as
 * dusimagez_compress. */
int32_t dusimagez_compress_to_size(const uint8_t *input, size_t input_len,
                                   uint64_t target_bytes,
                                   const DusimagezOptions *options,
                                   uint8_t **out, size_t *out_len);

/* Re-encode and report the before/after numbers. `report` is a
 * caller-allocated DusimagezReport that this function fills in.
 * `options` is a DusimagezOptions pointer, or NULL for all defaults. */
int32_t dusimagez_compress_with_report(const uint8_t *input, size_t input_len,
                                       const DusimagezOptions *options,
                                       DusimagezReport *report);

/* Image summary: format, dimensions, alpha, colour type, file size.
 * `info` is a caller-allocated DusimagezInfo that this function
 * fills in. */
int32_t dusimagez_get_info(const uint8_t *input, size_t input_len,
                           DusimagezInfo *info);

/* Free a buffer returned by any `*out` parameter or `report.output`:
 * exactly the pointer and length that call returned, exactly once.
 * A NULL pointer or a zero length is a no-op. */
void dusimagez_free(uint8_t *ptr, size_t len);

/* The last error on this thread, as JSON:
 * {"code": "...", "message_en": "...", "message_hi": "..."}.
 * Returns NULL when the last call on this thread succeeded (or no FFI
 * call has failed yet). Do NOT free the pointer; it stays valid until
 * the next FFI call on the same thread. */
const char *dusimagez_last_error(void);

/* --- Extra exports (beyond the six SPEC section 5 names) --- */

/* Library version, e.g. "0.1.0" (lockstep with the core crate's
 * CARGO_PKG_VERSION). Do NOT free the pointer. */
const char *dusimagez_version(void);

/* Run compress_with_report over many inputs; one result per input,
 * same order, partial failures reported per item, never aborting the
 * batch. `options` is a DusimagezOptions pointer, or NULL for all
 * defaults. A zero `count` returns "[]" with status 0.
 *
 * Returns a JSON array of per-item results (UTF-8, `*out_len` bytes,
 * NOT NUL-terminated — free it with dusimagez_free):
 * {"index": 0, "ok": true, "output": "<base64>", "original_size": …,
 *  "output_size": …, "saved_bytes": …, "saved_percent": …,
 *  "format": "jpeg", "width": …, "height": …, "quality_used": …,
 *  "resized": false} or {"index": 0, "ok": false, "error": {"code": …,
 *  "message_en": …, "message_hi": …}}. The function itself returns
 * status 0 whenever the JSON was produced — even when every item
 * failed; only a bad pointer / bad options argument returns a
 * non-zero status. */
int32_t dusimagez_bulk_compress(const uint8_t *const *inputs,
                                const size_t *input_lens, size_t count,
                                const DusimagezOptions *options,
                                uint8_t **out_json, size_t *out_len);

#ifdef __cplusplus
}
#endif

#endif /* DUSIMAGEZ_H */
