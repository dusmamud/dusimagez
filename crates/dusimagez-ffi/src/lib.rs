//! C ABI bindings for dusimagez — the Go / PHP FFI foundation.
//!
//! Thin wrappers over the `dusimagez` core crate: every function takes
//! bytes in and returns bytes (or a filled struct) out, with an `int32_t`
//! status code (0 = OK, see `STATUS_*` / `dusimagez.h`) and a
//! thread-local last-error JSON string (`dusimagez_last_error`).
//!
//! Contract (SPEC section 5):
//! * Options arrive as a `DusimagezOptions` C struct (see
//!   `dusimagez.h`), NOT as JSON. Every numeric field uses `-1` for
//!   "unset / use the core default"; `NULL` options also mean "all
//!   defaults". Enum fields are plain `int32_t` codes (0 = keep/none/
//!   default — see the header for the code tables). The FFI layer
//!   starts from `CompressOptions::default()` and overrides only the
//!   fields the caller set, so core-side default rules (e.g. the
//!   subsampling default, which depends on quality) stay in the core.
//! * `dusimagez_compress` honours `options.target_bytes` when set:
//!   it delegates to `compress_to_size`, per SPEC section 2
//!   ("`target_bytes` — same as `compress_to_size` via one call").
//! * No business logic is duplicated here: every function delegates
//!   to the core crate. The only checks done in this layer are
//!   pointer/range checks that keep a caller bug from becoming a
//!   crash or an out-of-range value reaching the core (quality
//!   outside 1-100, PNG level outside 0-9, effort outside 0-10,
//!   zero dimensions, a `dpi` with only one axis set). Those return
//!   the same stable codes the core returns for the same bad values.
//! * `NULL` input / output pointers are `INVALID_ARGUMENT`, never a
//!   crash. Empty input (`len == 0`) is passed to the core, which
//!   reports `EMPTY_INPUT`. Nothing here panics on bad input: every
//!   export is wrapped in `catch_unwind`, and a panic (which should
//!   not happen) surfaces as status 99 `UNKNOWN`.
//!
//! Memory ownership: every `*out` buffer and every `report.output`
//! buffer is heap allocated by Rust and owned by the caller after a
//! successful call. Free it exactly once with
//! `dusimagez_free(ptr, len)`. `dusimagez_last_error` /
//! `dusimagez_version` return NUL-terminated C strings owned by this
//! library — do NOT free them; the last-error pointer stays valid
//! until the next FFI call on the same thread.
//!
//! Extra exports beyond the six SPEC section 5 names (marked as such
//! in the header): `dusimagez_version` and
//! `dusimagez_bulk_compress`. Bulk exists because the Go and PHP
//! bindings must offer `BulkCompress` / `bulkCompress` with
//! per-item results, and that needs a bulk entry point in the C API;
//! it returns one JSON array (image outputs base64-encoded) so the
//! bindings never re-implement batching.
//!
//! ASSUMPTIONS about the core crate (W1 owns `crates/dusimagez`;
//! this crate was written against the SPEC contract before the core
//! landed — see `README.md` for the full list, and re-check this
//! file against the core the moment it lands):
//! * error type `DusimagezError { code: ErrorCode, message_en,
//!   message_hi }` with `code()`, `message_en()`, `message_hi()`;
//! * `ErrorCode` variants `InvalidImage`, `UnsupportedFormat`,
//!   `ImageTooLarge`, `TargetSizeUnreachable`, `InvalidQuality`,
//!   `InvalidDimensions`, `InvalidArgument`, `EmptyInput`;
//! * `CompressOptions` fields exactly as in SPEC section 2, with
//!   `ImageFormat::{Jpeg, Png, WebP, Avif, Gif, Bmp, Tiff}`,
//!   `Subsampling::{S444, S422, S420}`,
//!   `Preset::{Web, Social, Print, Archive}`,
//!   `StripMetadata::{All, KeepExif, KeepIcc, KeepAll}`,
//!   `dpi: Option<(u32, u32)>`, `target_bytes: Option<u64>`;
//! * `CompressReport { output, original_size, output_size,
//!   saved_bytes, saved_percent, format: ImageFormat, width, height,
//!   quality_used, resized }`;
//! * `ImageInfo { format: ImageFormat, width, height, has_alpha,
//!   colour_type: String, file_size }` — `colour_type` is the
//!   lowercase `image`-crate colour type (`"rgb8"`, `"rgba8"`,
//!   `"la8"`, `"l8"`, ...); the core's `Debug` name is lowercased
//!   in `copy_colour_type`.
//! * `bulk_compress(&[Vec<u8>], &CompressOptions) ->
//!   Vec<Result<CompressReport, DusimagezError>>`.

use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;

use dusimagez::{
    CompressOptions, DusimagezError, ImageFormat, Preset, StripMetadata, Subsampling,
    bulk_compress as core_bulk_compress, compress_image as core_compress_image,
    compress_to_size as core_compress_to_size, compress_with_report as core_compress_with_report,
    get_image_info as core_get_image_info,
};

// ---------------------------------------------------------------------------
// Status codes (must match `dusimagez.h`)
// ---------------------------------------------------------------------------

/// Success.
pub const STATUS_OK: i32 = 0;
/// Input bytes are not a recognisable image.
pub const STATUS_INVALID_IMAGE: i32 = 1;
/// The image format is known but not supported by this build.
pub const STATUS_UNSUPPORTED_FORMAT: i32 = 2;
/// The image exceeds `max_pixels` / `max_dimension`.
pub const STATUS_IMAGE_TOO_LARGE: i32 = 3;
/// Even the lowest quality still exceeds the requested target size.
pub const STATUS_TARGET_SIZE_UNREACHABLE: i32 = 4;
/// A quality value is outside 1-100.
pub const STATUS_INVALID_QUALITY: i32 = 5;
/// A dimension is zero, missing, or otherwise unusable.
pub const STATUS_INVALID_DIMENSIONS: i32 = 6;
/// An argument is wrong (bad value, NULL pointer, ...).
pub const STATUS_INVALID_ARGUMENT: i32 = 7;
/// An input list / byte buffer is empty where input is required.
pub const STATUS_EMPTY_INPUT: i32 = 8;
/// Anything else (including a Rust panic, which should not happen).
pub const STATUS_UNKNOWN: i32 = 99;

fn status_of(code: &str) -> i32 {
    match code {
        "INVALID_IMAGE" => STATUS_INVALID_IMAGE,
        "UNSUPPORTED_FORMAT" => STATUS_UNSUPPORTED_FORMAT,
        "IMAGE_TOO_LARGE" => STATUS_IMAGE_TOO_LARGE,
        "TARGET_SIZE_UNREACHABLE" => STATUS_TARGET_SIZE_UNREACHABLE,
        "INVALID_QUALITY" => STATUS_INVALID_QUALITY,
        "INVALID_DIMENSIONS" => STATUS_INVALID_DIMENSIONS,
        "INVALID_ARGUMENT" => STATUS_INVALID_ARGUMENT,
        "EMPTY_INPUT" => STATUS_EMPTY_INPUT,
        _ => STATUS_UNKNOWN,
    }
}

// ---------------------------------------------------------------------------
// C structs (must match `dusimagez.h` field-for-field, in this order)
// ---------------------------------------------------------------------------

/// C mirror of `CompressOptions` (SPEC section 2).
///
/// Every field is `-1` for "unset / use the core default", except the
/// enum/bool fields documented below (where `0` already means the
/// default). A `NULL` options pointer means "all defaults" — the same
/// as a struct full of `-1`s (and `0`s for the enum/bool fields).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DusimagezOptions {
    /// Quality 1-100. `-1` = default (80). `0` or `>100` is
    /// `INVALID_QUALITY`.
    pub quality: i32,
    /// Output format code: `0` = keep input format, `1` = jpeg,
    /// `2` = png, `3` = webp, `4` = avif, `5` = gif, `6` = bmp,
    /// `7` = tiff. `-1` = keep input format (same as `0`).
    pub format: i32,
    /// `-1` = default (false), `0` = false, `1` = true.
    pub progressive: i32,
    /// `-1` = default (false), `0` = false, `1` = true.
    pub lossless: i32,
    /// Encoding effort 0-10. `-1` = default (6). `>10` is
    /// `INVALID_ARGUMENT`.
    pub effort: i32,
    /// Subsampling code: `0` = core default (S420 for quality < 90,
    /// S444 otherwise), `1` = S444, `2` = S422, `3` = S420.
    /// `-1` = core default (same as `0`).
    pub subsampling: i32,
    /// PNG compression level 0-9. `-1` = default (6). `>9` is
    /// `INVALID_ARGUMENT`.
    pub png_compression_level: i32,
    /// `-1` = default (false), `0` = false, `1` = true.
    pub palette: i32,
    /// Preset code: `0` = none, `1` = Web, `2` = Social,
    /// `3` = Print, `4` = Archive. `-1` = none (same as `0`).
    pub preset: i32,
    /// Target output size in bytes. `-1` / `0` = unset. When set,
    /// `dusimagez_compress` behaves like `dusimagez_compress_to_size`.
    pub target_bytes: i64,
    /// Downscale-only max width in px. `-1` = unset. `0` is
    /// `INVALID_DIMENSIONS`.
    pub max_width: i64,
    /// Downscale-only max height in px. `-1` = unset. `0` is
    /// `INVALID_DIMENSIONS`.
    pub max_height: i64,
    /// Metadata strip code: `0` = All (default), `1` = KeepExif,
    /// `2` = KeepIcc, `3` = KeepAll. `-1` = All (same as `0`).
    pub strip_metadata: i32,
    /// `-1` = default (true), `0` = false, `1` = true.
    pub auto_orient: i32,
    /// Horizontal DPI. `-1` = unset. Must be set together with
    /// `dpi_y` (both `-1`, or both `>0`).
    pub dpi_x: i64,
    /// Vertical DPI. `-1` = unset. Must be set together with `dpi_x`.
    pub dpi_y: i64,
    /// Refuse images with more pixels than this. `-1` = default
    /// (120_000_000). `0` is `INVALID_ARGUMENT`.
    pub max_pixels: i64,
    /// Refuse images with either side longer than this. `-1` =
    /// default (16000). `0` is `INVALID_ARGUMENT`.
    pub max_dimension: i64,
}

impl Default for DusimagezOptions {
    fn default() -> Self {
        Self {
            quality: -1,
            format: 0,
            progressive: -1,
            lossless: -1,
            effort: -1,
            subsampling: 0,
            png_compression_level: -1,
            palette: -1,
            preset: 0,
            target_bytes: -1,
            max_width: -1,
            max_height: -1,
            strip_metadata: 0,
            auto_orient: -1,
            dpi_x: -1,
            dpi_y: -1,
            max_pixels: -1,
            max_dimension: -1,
        }
    }
}

/// C mirror of `CompressReport` (SPEC section 1), filled in by
/// `dusimagez_compress_with_report` / `dusimagez_bulk_compress`.
///
/// The caller allocates this struct (on the stack is fine) and passes
/// a pointer to it. On success `output` points at a heap buffer of
/// `output_len` bytes owned by the caller — free it exactly once with
/// `dusimagez_free(output, output_len)`. On failure `output` is NULL
/// and `output_len` is 0.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DusimagezReport {
    pub output: *mut u8,
    pub output_len: usize,
    pub original_size: u64,
    pub output_size: u64,
    pub saved_bytes: u64,
    pub saved_percent: f32,
    /// Format code, same table as `DusimagezOptions.format`
    /// (`0` never occurs here — the report always names a format).
    pub format: i32,
    pub width: u32,
    pub height: u32,
    pub quality_used: u8,
    /// `0` = not resized, `1` = resized.
    pub resized: i32,
}

/// C mirror of `ImageInfo` (SPEC section 1), filled in by
/// `dusimagez_get_info`. The caller allocates this struct.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DusimagezInfo {
    /// Format code, same table as `DusimagezOptions.format`.
    pub format: i32,
    pub width: u32,
    pub height: u32,
    /// `0` = no alpha, `1` = has alpha.
    pub has_alpha: i32,
    /// Lowercase `image`-crate colour type (`"rgb8"`, `"rgba8"`,
    /// `"la8"`, `"l8"`, ...), NUL-terminated, truncated to 23 chars.
    pub colour_type: [c_char; 24],
    pub file_size: u64,
}

// ---------------------------------------------------------------------------
// Thread-local last error
// ---------------------------------------------------------------------------

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error_json(json: String) {
    LAST_ERROR.with(|cell| *cell.borrow_mut() = CString::new(json).ok());
}

fn set_last_error(e: &DusimagezError) {
    set_last_error_json(
        serde_json::json!({
            "code": e.code(),
            "message_en": e.message_en(),
            "message_hi": e.message_hi(),
        })
        .to_string(),
    );
}

fn clear_last_error() {
    LAST_ERROR.with(|cell| *cell.borrow_mut() = None);
}

/// Record a core error as the thread-local last error and return its status.
fn fail(e: &DusimagezError) -> i32 {
    let status = status_of(e.code());
    set_last_error(e);
    status
}

/// Record a validation error raised by this layer itself (no core
/// error object exists for it) and return its status.
fn fail_local(status: i32, code: &str, message_en: &str, message_hi: &str) -> i32 {
    set_last_error_json(
        serde_json::json!({
            "code": code,
            "message_en": message_en,
            "message_hi": message_hi,
        })
        .to_string(),
    );
    status
}

/// Record an `UNKNOWN` error (no stable code exists for it) and return
/// status 99.
fn fail_unknown() -> i32 {
    fail_local(
        STATUS_UNKNOWN,
        "UNKNOWN",
        "An unexpected internal error occurred.",
        "Ek anpekkshit internal error hua.",
    )
}

// ---------------------------------------------------------------------------
// Format / enum mapping
// ---------------------------------------------------------------------------

fn format_code(format: &ImageFormat) -> i32 {
    match format {
        ImageFormat::Jpeg => 1,
        ImageFormat::Png => 2,
        ImageFormat::WebP => 3,
        ImageFormat::Avif => 4,
        ImageFormat::Gif => 5,
        ImageFormat::Bmp => 6,
        ImageFormat::Tiff => 7,
    }
}

fn format_str(format: &ImageFormat) -> &'static str {
    match format {
        ImageFormat::Jpeg => "jpeg",
        ImageFormat::Png => "png",
        ImageFormat::WebP => "webp",
        ImageFormat::Avif => "avif",
        ImageFormat::Gif => "gif",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Tiff => "tiff",
    }
}

fn format_from_code(code: i32) -> Result<Option<ImageFormat>, i32> {
    match code {
        -1 | 0 => Ok(None),
        1 => Ok(Some(ImageFormat::Jpeg)),
        2 => Ok(Some(ImageFormat::Png)),
        3 => Ok(Some(ImageFormat::WebP)),
        4 => Ok(Some(ImageFormat::Avif)),
        5 => Ok(Some(ImageFormat::Gif)),
        6 => Ok(Some(ImageFormat::Bmp)),
        7 => Ok(Some(ImageFormat::Tiff)),
        _ => Err(fail_local(
            STATUS_INVALID_ARGUMENT,
            "INVALID_ARGUMENT",
            &format!(
                "Unknown image format code {code}. Use 0 (keep), 1 (jpeg), 2 (png), 3 (webp), 4 (avif), 5 (gif), 6 (bmp) or 7 (tiff)."
            ),
            "Galat image format code hai.",
        )),
    }
}

fn subsampling_from_code(code: i32) -> Result<Option<Subsampling>, i32> {
    match code {
        -1 | 0 => Ok(None),
        1 => Ok(Some(Subsampling::S444)),
        2 => Ok(Some(Subsampling::S422)),
        3 => Ok(Some(Subsampling::S420)),
        _ => Err(fail_local(
            STATUS_INVALID_ARGUMENT,
            "INVALID_ARGUMENT",
            &format!(
                "Unknown subsampling code {code}. Use 0 (default), 1 (S444), 2 (S422) or 3 (S420)."
            ),
            "Galat subsampling code hai.",
        )),
    }
}

fn preset_from_code(code: i32) -> Result<Option<Preset>, i32> {
    match code {
        -1 | 0 => Ok(None),
        1 => Ok(Some(Preset::Web)),
        2 => Ok(Some(Preset::Social)),
        3 => Ok(Some(Preset::Print)),
        4 => Ok(Some(Preset::Archive)),
        _ => Err(fail_local(
            STATUS_INVALID_ARGUMENT,
            "INVALID_ARGUMENT",
            &format!(
                "Unknown preset code {code}. Use 0 (none), 1 (Web), 2 (Social), 3 (Print) or 4 (Archive)."
            ),
            "Galat preset code hai.",
        )),
    }
}

fn strip_from_code(code: i32) -> Result<StripMetadata, i32> {
    match code {
        -1 | 0 => Ok(StripMetadata::All),
        1 => Ok(StripMetadata::KeepExif),
        2 => Ok(StripMetadata::KeepIcc),
        3 => Ok(StripMetadata::KeepAll),
        _ => Err(fail_local(
            STATUS_INVALID_ARGUMENT,
            "INVALID_ARGUMENT",
            &format!(
                "Unknown strip_metadata code {code}. Use 0 (All), 1 (KeepExif), 2 (KeepIcc) or 3 (KeepAll)."
            ),
            "Galat strip_metadata code hai.",
        )),
    }
}

/// `true` unless the field is explicitly `0`; `-1` means "default".
fn bool_from_code(code: i32, default: bool, what: &str) -> Result<bool, i32> {
    match code {
        -1 => Ok(default),
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(fail_local(
            STATUS_INVALID_ARGUMENT,
            "INVALID_ARGUMENT",
            &format!("{what} must be -1 (default), 0 (false) or 1 (true), not {code}."),
            "Galat bool value hai.",
        )),
    }
}

/// `-1` means "unset" (`None`); any other negative value is
/// `INVALID_ARGUMENT`; `0` is rejected by `reject_zero` callers.
fn opt_u64(value: i64, what: &str) -> Result<Option<u64>, i32> {
    if value < 0 {
        if value == -1 {
            return Ok(None);
        }
        return Err(fail_local(
            STATUS_INVALID_ARGUMENT,
            "INVALID_ARGUMENT",
            &format!("{what} must not be negative."),
            "Galat value hai.",
        ));
    }
    Ok(Some(value as u64))
}

fn opt_u32_nonzero(value: i64, what: &str) -> Result<Option<u32>, i32> {
    match opt_u64(value, what)? {
        None => Ok(None),
        Some(0) => Err(fail_local(
            STATUS_INVALID_DIMENSIONS,
            "INVALID_DIMENSIONS",
            &format!("{what} must not be zero."),
            "Galat dimension hai.",
        )),
        Some(v) => {
            if v > u32::MAX as u64 {
                return Err(fail_local(
                    STATUS_INVALID_DIMENSIONS,
                    "INVALID_DIMENSIONS",
                    &format!("{what} is too large."),
                    "Galat dimension hai.",
                ));
            }
            Ok(Some(v as u32))
        }
    }
}

/// Translate one `DusimagezOptions` C struct into a core
/// `CompressOptions`, starting from the core default and overriding
/// only the fields the caller set. `NULL` means "all defaults".
/// On a validation failure the last error is already recorded and
/// the returned `i32` is the status to return.
fn options_from_c(ptr: *const DusimagezOptions) -> Result<CompressOptions, i32> {
    if ptr.is_null() {
        return Ok(CompressOptions::default());
    }
    let c = unsafe { &*ptr };
    let mut opts = CompressOptions::default();

    if c.quality != -1 {
        if !(1..=100).contains(&c.quality) {
            return Err(fail_local(
                STATUS_INVALID_QUALITY,
                "INVALID_QUALITY",
                &format!(
                    "Quality {} is invalid. Quality must be between 1 and 100.",
                    c.quality
                ),
                "Quality galat hai. Quality 1 se 100 ke beech honi chahiye.",
            ));
        }
        opts.quality = c.quality as u8;
    }
    opts.format = format_from_code(c.format)?;
    opts.progressive = bool_from_code(c.progressive, false, "progressive")?;
    opts.lossless = bool_from_code(c.lossless, false, "lossless")?;
    if c.effort != -1 {
        if !(0..=10).contains(&c.effort) {
            return Err(fail_local(
                STATUS_INVALID_ARGUMENT,
                "INVALID_ARGUMENT",
                &format!(
                    "Effort {} is invalid. Effort must be between 0 and 10.",
                    c.effort
                ),
                "Effort galat hai. Effort 0 se 10 ke beech hona chahiye.",
            ));
        }
        opts.effort = c.effort as u8;
    }
    if let Some(sub) = subsampling_from_code(c.subsampling)? {
        opts.subsampling = sub;
    }
    if c.png_compression_level != -1 {
        if !(0..=9).contains(&c.png_compression_level) {
            return Err(fail_local(
                STATUS_INVALID_ARGUMENT,
                "INVALID_ARGUMENT",
                &format!(
                    "PNG compression level {} is invalid. It must be between 0 and 9.",
                    c.png_compression_level
                ),
                "PNG compression level galat hai. Ye 0 se 9 ke beech hona chahiye.",
            ));
        }
        opts.png_compression_level = c.png_compression_level as u8;
    }
    opts.palette = bool_from_code(c.palette, false, "palette")?;
    opts.preset = preset_from_code(c.preset)?;
    opts.target_bytes = opt_u64(c.target_bytes, "target_bytes")?;
    if let Some(0) = opts.target_bytes {
        return Err(fail_local(
            STATUS_INVALID_ARGUMENT,
            "INVALID_ARGUMENT",
            "target_bytes must not be zero.",
            "target_bytes zero nahi ho sakta.",
        ));
    }
    opts.max_width = opt_u32_nonzero(c.max_width, "max_width")?;
    opts.max_height = opt_u32_nonzero(c.max_height, "max_height")?;
    opts.strip_metadata = strip_from_code(c.strip_metadata)?;
    opts.auto_orient = bool_from_code(c.auto_orient, true, "auto_orient")?;
    match (c.dpi_x, c.dpi_y) {
        (-1, -1) => {}
        (x, y) if x > 0 && y > 0 => opts.dpi = Some((x as u32, y as u32)),
        _ => {
            return Err(fail_local(
                STATUS_INVALID_ARGUMENT,
                "INVALID_ARGUMENT",
                "dpi needs both dpi_x and dpi_y set (both -1 for unset, or both greater than zero).",
                "dpi ke liye dpi_x aur dpi_y dono chahiye.",
            ));
        }
    }
    if c.max_pixels != -1 {
        match opt_u64(c.max_pixels, "max_pixels")? {
            Some(0) => {
                return Err(fail_local(
                    STATUS_INVALID_ARGUMENT,
                    "INVALID_ARGUMENT",
                    "max_pixels must not be zero.",
                    "max_pixels zero nahi ho sakta.",
                ));
            }
            Some(v) => opts.max_pixels = v,
            None => {}
        }
    }
    if c.max_dimension != -1
        && let Some(v) = opt_u32_nonzero(c.max_dimension, "max_dimension")?
    {
        opts.max_dimension = v;
    }
    Ok(opts)
}

// ---------------------------------------------------------------------------
// Pointer helpers + panic guard
// ---------------------------------------------------------------------------

/// Borrow a byte input: `len == 0` yields an empty slice (even for a
/// `NULL` pointer — the core then reports `EMPTY_INPUT`); a `NULL`
/// pointer with `len > 0` is `INVALID_ARGUMENT`.
fn input_slice<'a>(ptr: *const u8, len: usize) -> Result<&'a [u8], i32> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(fail_local(
            STATUS_INVALID_ARGUMENT,
            "INVALID_ARGUMENT",
            "Input pointer is NULL but input length is not zero.",
            "Input pointer NULL hai.",
        ));
    }
    Ok(unsafe { slice::from_raw_parts(ptr, len) })
}

/// Read the `count` inputs of a multi-input call. `count == 0` yields
/// an empty list (even for `NULL` pointers); a `NULL` array pointer
/// with `count > 0`, or a `NULL` entry with a non-zero length, is
/// `INVALID_ARGUMENT`.
fn read_inputs(
    inputs: *const *const u8,
    input_lens: *const usize,
    count: usize,
) -> Result<Vec<Vec<u8>>, i32> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if inputs.is_null() || input_lens.is_null() {
        return Err(fail_local(
            STATUS_INVALID_ARGUMENT,
            "INVALID_ARGUMENT",
            "Inputs pointer is NULL but input count is not zero.",
            "Inputs pointer NULL hai.",
        ));
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let p = unsafe { *inputs.add(i) };
        let len = unsafe { *input_lens.add(i) };
        out.push(input_slice(p, len)?.to_vec());
    }
    Ok(out)
}

/// Validate an output-pointer pair before doing any work, so a caller
/// bug is reported as `INVALID_ARGUMENT` even when the image itself
/// would also have failed.
fn check_out(out: *mut *mut u8, out_len: *mut usize) -> Result<(), i32> {
    if out.is_null() || out_len.is_null() {
        return Err(fail_local(
            STATUS_INVALID_ARGUMENT,
            "INVALID_ARGUMENT",
            "Output pointer is NULL.",
            "Output pointer NULL hai.",
        ));
    }
    Ok(())
}

/// Hand ownership of `bytes` to the caller: `*out` points at a heap
/// buffer of `*out_len` bytes, freed with `dusimagez_free`.
/// Clears the last error on success.
fn write_output(out: *mut *mut u8, out_len: *mut usize, bytes: Vec<u8>) -> i32 {
    let len = bytes.len();
    let ptr = Box::leak(bytes.into_boxed_slice()).as_mut_ptr();
    unsafe {
        *out = ptr;
        *out_len = len;
    }
    clear_last_error();
    STATUS_OK
}

/// Same as `write_output`, for a JSON string (UTF-8, `len` bytes,
/// NOT NUL-terminated).
fn write_json_output(out: *mut *mut u8, out_len: *mut usize, json: String) -> i32 {
    write_output(out, out_len, json.into_bytes())
}

/// Run `body`, converting a Rust panic (which should not happen — the
/// core never panics on bad input) into status 99 `UNKNOWN` instead
/// of unwinding across the FFI boundary.
fn guard(body: impl FnOnce() -> i32) -> i32 {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(code) => code,
        Err(_) => fail_unknown(),
    }
}

// ---------------------------------------------------------------------------
// Base64 (bulk outputs)
// ---------------------------------------------------------------------------

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64_CHARS[(n >> 18 & 63) as usize] as char);
        out.push(B64_CHARS[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64_CHARS[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64_CHARS[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Report / info filling
// ---------------------------------------------------------------------------

/// Fill a caller-allocated `DusimagezReport` from a core
/// `CompressReport`, handing the output bytes to the caller.
/// Clears the last error on success.
fn fill_report(report: *mut DusimagezReport, core: dusimagez::CompressReport) -> i32 {
    let output_len = core.output.len();
    let output_ptr = Box::leak(core.output.into_boxed_slice()).as_mut_ptr();
    let filled = DusimagezReport {
        output: output_ptr,
        output_len,
        original_size: core.original_size,
        output_size: core.output_size,
        saved_bytes: core.saved_bytes,
        saved_percent: core.saved_percent,
        format: format_code(&core.format),
        width: core.width,
        height: core.height,
        quality_used: core.quality_used,
        resized: i32::from(core.resized),
    };
    unsafe {
        *report = filled;
    }
    clear_last_error();
    STATUS_OK
}

/// Zero a report struct (failure path): `output` NULL, `output_len` 0.
fn zero_report(report: *mut DusimagezReport) {
    if report.is_null() {
        return;
    }
    unsafe {
        (*report).output = ptr::null_mut();
        (*report).output_len = 0;
    }
}

fn copy_colour_type(dst: &mut [c_char; 24], value: &str) {
    // The UDL contract is lowercase ("rgb8"); the core stores the
    // `Debug` name ("Rgb8"), so lowercase here — Go/PHP inherit this.
    let lowered = value.to_ascii_lowercase();
    let bytes = lowered.as_bytes();
    let n = bytes.len().min(23);
    for (i, &b) in bytes.iter().take(n).enumerate() {
        dst[i] = b as c_char;
    }
    dst[n] = 0;
    for slot in dst.iter_mut().skip(n + 1) {
        *slot = 0;
    }
}

// ---------------------------------------------------------------------------
// Exports — SPEC section 5
// ---------------------------------------------------------------------------

/// Re-encode at a fixed quality (and `format`, or the input format
/// when unset). `options` is a `DusimagezOptions` pointer, or `NULL`
/// for all defaults. When `options.target_bytes` is set, this behaves
/// exactly like `dusimagez_compress_to_size` (SPEC section 2).
///
/// # Safety
///
/// `input` must point at `input_len` readable bytes (or be NULL when
/// `input_len` is 0), `options` must be NULL or point at a valid
/// `DusimagezOptions`, and `out` / `out_len` must point at valid
/// writable locations. Passing a dangling non-NULL pointer is
/// undefined behaviour; NULL pointers are checked and reported as
/// `INVALID_ARGUMENT`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dusimagez_compress(
    input: *const u8,
    input_len: usize,
    options: *const DusimagezOptions,
    out: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    guard(|| {
        if let Err(status) = check_out(out, out_len) {
            return status;
        }
        let data = match input_slice(input, input_len) {
            Ok(d) => d,
            Err(status) => return status,
        };
        let opts = match options_from_c(options) {
            Ok(o) => o,
            Err(status) => return status,
        };
        let result = match opts.target_bytes {
            Some(target) => core_compress_to_size(data, target, &opts),
            None => core_compress_image(data, &opts),
        };
        match result {
            Ok(bytes) => write_output(out, out_len, bytes),
            Err(e) => fail(&e),
        }
    })
}

/// Re-encode so the result is at most `target_bytes` bytes
/// (iterative quality search; `TARGET_SIZE_UNREACHABLE` when even the
/// lowest quality exceeds the target). `options` is a
/// `DusimagezOptions` pointer, or `NULL` for all defaults.
///
/// # Safety
///
/// Same pointer rules as `dusimagez_compress`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dusimagez_compress_to_size(
    input: *const u8,
    input_len: usize,
    target_bytes: u64,
    options: *const DusimagezOptions,
    out: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    guard(|| {
        if let Err(status) = check_out(out, out_len) {
            return status;
        }
        let data = match input_slice(input, input_len) {
            Ok(d) => d,
            Err(status) => return status,
        };
        if target_bytes == 0 {
            return fail_local(
                STATUS_INVALID_ARGUMENT,
                "INVALID_ARGUMENT",
                "target_bytes must not be zero.",
                "target_bytes zero nahi ho sakta.",
            );
        }
        let opts = match options_from_c(options) {
            Ok(o) => o,
            Err(status) => return status,
        };
        match core_compress_to_size(data, target_bytes, &opts) {
            Ok(bytes) => write_output(out, out_len, bytes),
            Err(e) => fail(&e),
        }
    })
}

/// Re-encode and report the before/after numbers. `report` is a
/// caller-allocated `DusimagezReport` that this function fills in;
/// `report.output` is a heap buffer owned by the caller — free it
/// exactly once with `dusimagez_free(report.output,
/// report.output_len)`. On failure `report.output` is NULL and
/// `report.output_len` is 0. `options` is a `DusimagezOptions`
/// pointer, or `NULL` for all defaults.
///
/// # Safety
///
/// Same pointer rules as `dusimagez_compress`, plus `report` must
/// point at a valid writable `DusimagezReport`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dusimagez_compress_with_report(
    input: *const u8,
    input_len: usize,
    options: *const DusimagezOptions,
    report: *mut DusimagezReport,
) -> i32 {
    guard(|| {
        if report.is_null() {
            return fail_local(
                STATUS_INVALID_ARGUMENT,
                "INVALID_ARGUMENT",
                "Report pointer is NULL.",
                "Report pointer NULL hai.",
            );
        }
        let data = match input_slice(input, input_len) {
            Ok(d) => d,
            Err(status) => {
                zero_report(report);
                return status;
            }
        };
        let opts = match options_from_c(options) {
            Ok(o) => o,
            Err(status) => {
                zero_report(report);
                return status;
            }
        };
        match core_compress_with_report(data, &opts) {
            Ok(core_report) => fill_report(report, core_report),
            Err(e) => {
                zero_report(report);
                fail(&e)
            }
        }
    })
}

/// Image summary: format, dimensions, alpha, colour type, file size.
/// `info` is a caller-allocated `DusimagezInfo` that this function
/// fills in.
///
/// # Safety
///
/// Same pointer rules as `dusimagez_compress`, plus `info` must point
/// at a valid writable `DusimagezInfo`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dusimagez_get_info(
    input: *const u8,
    input_len: usize,
    info: *mut DusimagezInfo,
) -> i32 {
    guard(|| {
        if info.is_null() {
            return fail_local(
                STATUS_INVALID_ARGUMENT,
                "INVALID_ARGUMENT",
                "Info pointer is NULL.",
                "Info pointer NULL hai.",
            );
        }
        let data = match input_slice(input, input_len) {
            Ok(d) => d,
            Err(status) => return status,
        };
        match core_get_image_info(data) {
            Ok(core_info) => {
                let mut filled = DusimagezInfo {
                    format: format_code(&core_info.format),
                    width: core_info.width,
                    height: core_info.height,
                    has_alpha: i32::from(core_info.has_alpha),
                    colour_type: [0; 24],
                    file_size: core_info.file_size,
                };
                copy_colour_type(&mut filled.colour_type, core_info.colour_type.as_str());
                unsafe {
                    *info = filled;
                }
                clear_last_error();
                STATUS_OK
            }
            Err(e) => fail(&e),
        }
    })
}

/// Free a buffer returned by any `*out` parameter or `report.output`:
/// exactly the pointer and length that call returned, exactly once.
/// A `NULL` pointer or a zero length is a no-op.
///
/// # Safety
///
/// `ptr` / `len` must be exactly a buffer (pointer and length)
/// returned by a previous successful call of this library, freed
/// exactly once. Freeing any other pointer, or the same buffer
/// twice, is undefined behaviour. NULL / zero length is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dusimagez_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    drop(unsafe { Box::from_raw(ptr::slice_from_raw_parts_mut(ptr, len)) });
}

/// The last error on this thread, as JSON:
/// `{"code": "…", "message_en": "…", "message_hi": "…"}`.
/// Returns `NULL` when the last call on this thread succeeded (or no
/// FFI call has failed yet). Do NOT free the pointer; it stays valid
/// until the next FFI call on the same thread.
#[unsafe(no_mangle)]
pub extern "C" fn dusimagez_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| match cell.borrow().as_ref() {
        Some(s) => s.as_ptr(),
        None => ptr::null(),
    })
}

// ---------------------------------------------------------------------------
// Extra exports (beyond the six SPEC section 5 names)
// ---------------------------------------------------------------------------

/// Library version, e.g. `"0.1.0"` (lockstep with the core crate's
/// `CARGO_PKG_VERSION`). Do NOT free the pointer.
#[unsafe(no_mangle)]
pub extern "C" fn dusimagez_version() -> *const c_char {
    static VERSION_BYTES: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION_BYTES.as_ptr().cast::<c_char>()
}

/// Run `compress_with_report` over many inputs; one result per input,
/// same order, partial failures reported per item, never aborting the
/// batch (SPEC section 1 `bulk_compress`). `options` is a
/// `DusimagezOptions` pointer, or `NULL` for all defaults. A zero
/// `count` returns `"[]"` with status 0.
///
/// Returns a JSON array of per-item results (UTF-8, `*out_len` bytes,
/// NOT NUL-terminated — free it with `dusimagez_free`):
/// `{"index": 0, "ok": true, "output": "<base64>", "original_size":
/// …, "output_size": …, "saved_bytes": …, "saved_percent": …,
/// "format": "jpeg", "width": …, "height": …, "quality_used": …,
/// "resized": false}` or `{"index": 0, "ok": false, "error":
/// {"code": …, "message_en": …, "message_hi": …}}`. The function
/// itself returns status 0 whenever the JSON was produced — even when
/// every item failed; only a bad pointer / bad options argument
/// returns a non-zero status.
///
/// # Safety
///
/// Same pointer rules as `dusimagez_compress`, plus `inputs` /
/// `input_lens` must point at `count` readable entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dusimagez_bulk_compress(
    inputs: *const *const u8,
    input_lens: *const usize,
    count: usize,
    options: *const DusimagezOptions,
    out_json: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    guard(|| {
        if let Err(status) = check_out(out_json, out_len) {
            return status;
        }
        let bufs = match read_inputs(inputs, input_lens, count) {
            Ok(b) => b,
            Err(status) => return status,
        };
        if bufs.is_empty() {
            return write_json_output(out_json, out_len, "[]".to_string());
        }
        let opts = match options_from_c(options) {
            Ok(o) => o,
            Err(status) => return status,
        };
        let results: Vec<serde_json::Value> = core_bulk_compress(&bufs, &opts)
            .into_iter()
            .enumerate()
            .map(|(index, item)| match item {
                Ok(report) => serde_json::json!({
                    "index": index,
                    "ok": true,
                    "output": base64_encode(&report.output),
                    "original_size": report.original_size,
                    "output_size": report.output_size,
                    "saved_bytes": report.saved_bytes,
                    "saved_percent": report.saved_percent,
                    "format": format_str(&report.format),
                    "width": report.width,
                    "height": report.height,
                    "quality_used": report.quality_used,
                    "resized": report.resized,
                }),
                Err(e) => serde_json::json!({
                    "index": index,
                    "ok": false,
                    "error": {
                        "code": e.code(),
                        "message_en": e.message_en(),
                        "message_hi": e.message_hi(),
                    },
                }),
            })
            .collect();
        match serde_json::to_string(&results) {
            Ok(json) => write_json_output(out_json, out_len, json),
            Err(_) => fail_local(
                STATUS_INVALID_ARGUMENT,
                "INVALID_ARGUMENT",
                "Could not serialise bulk results.",
                "Bulk results serialise nahi ho sake.",
            ),
        }
    })
}
