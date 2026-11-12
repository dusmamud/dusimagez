//! WASM bindings for dusimagez — thin wrappers over the Rust core.
//!
//! All image logic lives in the `dusimagez` crate; this crate only
//! converts between JavaScript values and Rust types. Build with:
//! `wasm-pack build --target nodejs` (the npm package's tests) or
//! `wasm-pack build --target web` (website playground).
//!
//! Contract:
//! * Binary results are `Uint8Array`s.
//! * On failure the function throws a JSON string:
//!   `{"ok":false,"code":"INVALID_IMAGE","message_en":"…",
//!   "message_hi":"…"}` — the TS wrapper parses this into a
//!   `DusimagezError` with the same stable `code`.
//! * Options are plain JS objects. `undefined` / `null` / `{}` mean
//!   "all defaults": a partial object is overlaid on the Rust
//!   `Default` field by field, so the playground's `{ quality: 70 }`
//!   works. The TS wrapper still sends complete objects.
//! * Enum fields accept friendly lowercase spellings
//!   (`"jpeg"`, `"s420"`, `"web"`, `"keep_exif"`) as well as the
//!   canonical names (`"Jpeg"`, `"S420"`, `"Web"`, `"KeepExif"`).
//!   `format` also accepts `"tiff"` — the core falls back to PNG
//!   output for it (TIFF is decode-only).
//! * `compress_with_report` and `bulk_compress` return JSON strings.
//!   A report's `output` bytes are base64 inside the JSON (a raw
//!   `Vec<u8>` would serialise as a huge number array); the TS
//!   wrapper decodes them back to `Uint8Array`s. `format` is
//!   lowercase (`"jpeg"`, `"png"`, …).
//! * `target_bytes` for `compress_to_size` is `f64` on the JS side,
//!   NOT `u64`: wasm-bindgen would demand a `BigInt` for `u64`, but
//!   callers pass a plain JS number.
//! * `get_image_info` takes only the input bytes (SPEC §1) and
//!   returns a JSON string:
//!   `{ format, width, height, has_alpha, colour_type, file_size }`.
//!   `format` is lowercase (`"jpeg"` …); `colour_type` is lowercased
//!   (`"rgb8"`, `"rgba8"`).
//!
//! Core surface used (read from `crates/dusimagez/src/` — the core
//! types carry NO serde derives, so options are built field by
//! field here and reports are serialised to JSON by hand):
//! `dusimagez::{CompressOptions, CompressReport, DusimagezError,
//! ImageFormat, ImageInfo, Preset, StripMetadata, Subsampling,
//! bulk_compress, compress_image, compress_to_size,
//! compress_with_report, get_image_info}` with
//! `DusimagezError::{code, message_en, message_hi}` and
//! `ImageFormat::from_name`.

use dusimagez::{
    CompressOptions, CompressReport, DusimagezError, ImageFormat, Preset, StripMetadata,
    Subsampling, bulk_compress as core_bulk_compress, compress_image as core_compress_image,
    compress_to_size as core_compress_to_size, compress_with_report as core_compress_with_report,
    get_image_info as core_get_image_info,
};
use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

fn err_js(e: &DusimagezError) -> JsValue {
    JsValue::from_str(
        &serde_json::json!({
            "ok": false,
            "code": e.code(),
            "message_en": e.message_en(),
            "message_hi": e.message_hi(),
        })
        .to_string(),
    )
}

fn invalid_argument(en: &str, hi: &str) -> JsValue {
    JsValue::from_str(
        &serde_json::json!({
            "ok": false,
            "code": "INVALID_ARGUMENT",
            "message_en": en,
            "message_hi": hi,
        })
        .to_string(),
    )
}

// ---------------------------------------------------------------------------
// Options parsing: partial JS object -> CompressOptions, field by field.
// The core types have no serde impls, so there is no merge-over-JSON;
// each known key is read off the object, unknown keys are ignored,
// and a wrong JSON type is INVALID_ARGUMENT. Range checks (quality
// 1-100, effort 0-10, …) belong to the core's `validate_options` —
// values that fit the Rust field type are passed through so the
// core reports its own stable code.
// ---------------------------------------------------------------------------

fn invalid_field(field: &str, expected: &str) -> JsValue {
    invalid_argument(
        &format!("Option \"{field}\" must be {expected}."),
        &format!("Option \"{field}\" {expected} hona chahiye."),
    )
}

/// Read an optional field: missing / explicit null -> `None`.
fn field<'a>(
    obj: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<&'a serde_json::Value> {
    match obj.get(key) {
        None | Some(serde_json::Value::Null) => None,
        Some(v) => Some(v),
    }
}

fn as_u8(v: &serde_json::Value, field: &str) -> Result<u8, JsValue> {
    match v.as_u64().and_then(|n| u8::try_from(n).ok()) {
        Some(n) => Ok(n),
        None => Err(invalid_field(field, "an integer 0-255")),
    }
}

fn as_u64(v: &serde_json::Value, field: &str) -> Result<u64, JsValue> {
    match v.as_u64() {
        Some(n) => Ok(n),
        None => Err(invalid_field(field, "a non-negative integer")),
    }
}

fn as_u32(v: &serde_json::Value, field: &str) -> Result<u32, JsValue> {
    match v.as_u64().and_then(|n| u32::try_from(n).ok()) {
        Some(n) => Ok(n),
        None => Err(invalid_field(field, "a non-negative integer")),
    }
}

fn as_bool(v: &serde_json::Value, field: &str) -> Result<bool, JsValue> {
    match v.as_bool() {
        Some(b) => Ok(b),
        None => Err(invalid_field(field, "a boolean")),
    }
}

fn as_string<'a>(v: &'a serde_json::Value, field: &str) -> Result<&'a str, JsValue> {
    match v.as_str() {
        Some(s) => Ok(s),
        None => Err(invalid_field(field, "a string")),
    }
}

fn parse_subsampling(s: &str) -> Result<Subsampling, JsValue> {
    match s.trim().to_ascii_lowercase().as_str() {
        "s444" | "444" | "4:4:4" => Ok(Subsampling::S444),
        "s422" | "422" | "4:2:2" => Ok(Subsampling::S422),
        "s420" | "420" | "4:2:0" => Ok(Subsampling::S420),
        _ => Err(invalid_argument(
            &format!("Unknown subsampling \"{s}\". Use s444, s422 or s420."),
            &format!("Unknown subsampling \"{s}\" hai. s444, s422 ya s420 use karein."),
        )),
    }
}

fn parse_preset(s: &str) -> Result<Preset, JsValue> {
    match s.trim().to_ascii_lowercase().as_str() {
        "web" => Ok(Preset::Web),
        "social" => Ok(Preset::Social),
        "print" => Ok(Preset::Print),
        "archive" => Ok(Preset::Archive),
        _ => Err(invalid_argument(
            &format!("Unknown preset \"{s}\". Use web, social, print or archive."),
            &format!("Unknown preset \"{s}\" hai. web, social, print ya archive use karein."),
        )),
    }
}

fn parse_strip_metadata(s: &str) -> Result<StripMetadata, JsValue> {
    match s.trim().to_ascii_lowercase().as_str() {
        "all" => Ok(StripMetadata::All),
        "keep_exif" | "keep-exif" | "keepexif" => Ok(StripMetadata::KeepExif),
        "keep_icc" | "keep-icc" | "keepicc" => Ok(StripMetadata::KeepIcc),
        "keep_all" | "keep-all" | "keepall" => Ok(StripMetadata::KeepAll),
        _ => Err(invalid_argument(
            &format!("Unknown strip_metadata \"{s}\". Use all, keep_exif, keep_icc or keep_all."),
            &format!(
                "Unknown strip_metadata \"{s}\" hai. all, keep_exif, keep_icc ya keep_all use karein."
            ),
        )),
    }
}

fn parse_dpi(v: &serde_json::Value) -> Result<(u32, u32), JsValue> {
    let err = || invalid_field("dpi", "a [x, y] pair of non-negative integers");
    let arr = v.as_array().ok_or_else(err)?;
    if arr.len() != 2 {
        return Err(err());
    }
    let x = as_u32(&arr[0], "dpi")?;
    let y = as_u32(&arr[1], "dpi")?;
    Ok((x, y))
}

/// Parse a JS options value into `CompressOptions`. `undefined` /
/// `null` / `{}` mean "all defaults".
fn parse_options(value: &JsValue) -> Result<CompressOptions, JsValue> {
    let mut opts = CompressOptions::default();
    if value.is_undefined() || value.is_null() {
        return Ok(opts);
    }
    let provided: serde_json::Value =
        serde_wasm_bindgen::from_value(value.clone()).map_err(|e| {
            invalid_argument(
                &format!("Invalid options: {e}"),
                &format!("Galat options hain: {e}"),
            )
        })?;
    let Some(obj) = provided.as_object() else {
        return Err(invalid_argument(
            "Options must be an object.",
            "Options object hona chahiye.",
        ));
    };
    if let Some(v) = field(obj, "quality") {
        opts.quality = as_u8(v, "quality")?;
    }
    if let Some(v) = field(obj, "format") {
        let s = as_string(v, "format")?;
        opts.format = match ImageFormat::from_name(s) {
            Some(f) => Some(f),
            None => {
                return Err(invalid_argument(
                    &format!(
                        "Unknown format \"{s}\". Use jpeg, png, webp, avif, gif, bmp or tiff."
                    ),
                    &format!(
                        "Unknown format \"{s}\" hai. jpeg, png, webp, avif, gif, bmp ya tiff use karein."
                    ),
                ));
            }
        };
    }
    if let Some(v) = field(obj, "progressive") {
        opts.progressive = as_bool(v, "progressive")?;
    }
    if let Some(v) = field(obj, "lossless") {
        opts.lossless = as_bool(v, "lossless")?;
    }
    if let Some(v) = field(obj, "effort") {
        opts.effort = as_u8(v, "effort")?;
    }
    if let Some(v) = field(obj, "subsampling") {
        opts.subsampling = parse_subsampling(as_string(v, "subsampling")?)?;
    }
    if let Some(v) = field(obj, "png_compression_level") {
        opts.png_compression_level = as_u8(v, "png_compression_level")?;
    }
    if let Some(v) = field(obj, "palette") {
        opts.palette = as_bool(v, "palette")?;
    }
    if let Some(v) = field(obj, "preset") {
        opts.preset = Some(parse_preset(as_string(v, "preset")?)?);
    }
    if let Some(v) = field(obj, "target_bytes") {
        opts.target_bytes = Some(as_u64(v, "target_bytes")?);
    }
    if let Some(v) = field(obj, "max_width") {
        opts.max_width = Some(as_u32(v, "max_width")?);
    }
    if let Some(v) = field(obj, "max_height") {
        opts.max_height = Some(as_u32(v, "max_height")?);
    }
    if let Some(v) = field(obj, "strip_metadata") {
        opts.strip_metadata = parse_strip_metadata(as_string(v, "strip_metadata")?)?;
    }
    if let Some(v) = field(obj, "auto_orient") {
        opts.auto_orient = as_bool(v, "auto_orient")?;
    }
    if let Some(v) = field(obj, "dpi") {
        opts.dpi = Some(parse_dpi(v)?);
    }
    if let Some(v) = field(obj, "max_pixels") {
        opts.max_pixels = as_u64(v, "max_pixels")?;
    }
    if let Some(v) = field(obj, "max_dimension") {
        opts.max_dimension = as_u32(v, "max_dimension")?;
    }
    Ok(opts)
}

fn bytes_array(value: &JsValue) -> Result<Vec<Vec<u8>>, JsValue> {
    if !Array::is_array(value) {
        return Err(invalid_argument(
            "Inputs must be an array of Uint8Array.",
            "Inputs Uint8Array ki array honi chahiye.",
        ));
    }
    let arr = Array::from(value);
    let mut out = Vec::with_capacity(arr.length() as usize);
    for item in arr.iter() {
        out.push(Uint8Array::new(&item).to_vec());
    }
    Ok(out)
}

mod base64_for_wasm {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    pub fn encode(data: &[u8]) -> String {
        let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
        for chunk in data.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
            out.push(CHARS[(n >> 18 & 63) as usize] as char);
            out.push(CHARS[(n >> 12 & 63) as usize] as char);
            out.push(if chunk.len() > 1 {
                CHARS[(n >> 6 & 63) as usize] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                CHARS[(n & 63) as usize] as char
            } else {
                '='
            });
        }
        out
    }
}

/// Lowercase format name for JSON: `Jpeg` -> `"jpeg"`, `WebP` ->
/// `"webp"`, `Tiff` -> `"tiff"`. (The core's `extension()` returns
/// `"jpg"` for JPEG; bindings report `"jpeg"`.)
fn format_name(format: ImageFormat) -> &'static str {
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

/// A `CompressReport` as JSON for JS: `output` is base64 (not a
/// number array), `format` is lowercase.
fn report_json(report: &CompressReport) -> serde_json::Value {
    serde_json::json!({
        "output": base64_for_wasm::encode(&report.output),
        "original_size": report.original_size,
        "output_size": report.output_size,
        "saved_bytes": report.saved_bytes,
        "saved_percent": report.saved_percent,
        "format": format_name(report.format),
        "width": report.width,
        "height": report.height,
        "quality_used": report.quality_used,
        "resized": report.resized,
    })
}

/// An `ImageInfo` as JSON for JS: `format` lowercase, `colour_type`
/// lowercased (`"Rgb8"` -> `"rgb8"`).
fn info_json(info: &dusimagez::ImageInfo) -> serde_json::Value {
    serde_json::json!({
        "format": format_name(info.format),
        "width": info.width,
        "height": info.height,
        "has_alpha": info.has_alpha,
        "colour_type": info.colour_type.to_ascii_lowercase(),
        "file_size": info.file_size,
    })
}

/// One failed bulk item as a JSON value.
fn bulk_item_error(index: usize, e: &DusimagezError) -> serde_json::Value {
    serde_json::json!({
        "index": index,
        "ok": false,
        "error": {
            "code": e.code(),
            "message_en": e.message_en(),
            "message_hi": e.message_hi(),
        },
    })
}

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Re-encode an image at a fixed quality (and `format`, or the input
/// format when unset).
#[wasm_bindgen]
pub fn compress_image(input: &[u8], options: JsValue) -> Result<Vec<u8>, JsValue> {
    let opts = parse_options(&options)?;
    core_compress_image(input, &opts).map_err(|e| err_js(&e))
}

/// Re-encode an image so the result is at most `target_bytes` bytes.
/// `target_bytes` is `f64` (plain JS number) — see the header note.
#[wasm_bindgen]
pub fn compress_to_size(
    input: &[u8],
    target_bytes: f64,
    options: JsValue,
) -> Result<Vec<u8>, JsValue> {
    if !target_bytes.is_finite() || target_bytes < 0.0 || target_bytes > u64::MAX as f64 {
        return Err(invalid_argument(
            "Target size must be a non-negative number of bytes.",
            "Target size bytes ki non-negative number honi chahiye.",
        ));
    }
    let opts = parse_options(&options)?;
    core_compress_to_size(input, target_bytes as u64, &opts).map_err(|e| err_js(&e))
}

/// Compress and return the full before/after report as a JSON string:
/// `{ output (base64), original_size, output_size, saved_bytes,
/// saved_percent, format, width, height, quality_used, resized }`.
#[wasm_bindgen]
pub fn compress_with_report(input: &[u8], options: JsValue) -> Result<String, JsValue> {
    let opts = parse_options(&options)?;
    let report = core_compress_with_report(input, &opts).map_err(|e| err_js(&e))?;
    Ok(serde_json::to_string(&report_json(&report)).unwrap())
}

/// Image summary as a JSON string:
/// `{ format, width, height, has_alpha, colour_type, file_size }`.
#[wasm_bindgen]
pub fn get_image_info(input: &[u8]) -> Result<String, JsValue> {
    let info = core_get_image_info(input).map_err(|e| err_js(&e))?;
    Ok(serde_json::to_string(&info_json(&info)).unwrap())
}

/// Compress many inputs; one result per input, same order, partial
/// failures reported per item, never aborting the batch (SPEC §1).
/// Returns a JSON string of items:
/// `{ index, ok, report?: {...}, error?: { code, message_en,
/// message_hi } }`. An empty input list returns `[]`.
#[wasm_bindgen]
pub fn bulk_compress(inputs: JsValue, options: JsValue) -> Result<String, JsValue> {
    let bufs = bytes_array(&inputs)?;
    if bufs.is_empty() {
        return Ok("[]".to_string());
    }
    let opts = parse_options(&options)?;
    let results: Vec<serde_json::Value> = core_bulk_compress(&bufs, &opts)
        .into_iter()
        .enumerate()
        .map(|(index, item)| match item {
            Ok(report) => serde_json::json!({
                "index": index,
                "ok": true,
                "report": report_json(&report),
            }),
            Err(e) => bulk_item_error(index, &e),
        })
        .collect();
    Ok(serde_json::to_string(&results).unwrap())
}
