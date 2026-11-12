//! Python bindings for dusimagez — thin PyO3 wrappers over the Rust core.
//!
//! All image logic lives in the `dusimagez` crate; this crate only
//! converts bytes / option dicts to and from Python. Build with
//! `maturin develop` / `maturin build` (from this directory).
//!
//! Error contract: every Rust `DusimagezError` is raised as a Python
//! `ValueError` whose message is `CODE\x1f message_en \x1f message_hi`.
//! The Python layer (`python/dusimagez/__init__.py`) parses that into
//! `dusimagez.DusimagezError`. Options arrive as a Python dict matching
//! the Rust `CompressOptions` field names (snake_case keys); missing keys
//! fall back to the Rust `Default`, so callers can pass a partial dict.
//!
//! The native side is forgiving about friendly spellings, so the
//! Python layer and direct `_native` callers agree: `format` accepts
//! `"jpeg"` / `"jpg"` / `"png"` / `"webp"` / `"avif"` / `"gif"` /
//! `"bmp"` / `"tiff"` (or the canonical `"Jpeg"` …); `subsampling`
//! accepts `"s444"` / `"444"` / `"4:4:4"` (and the `s422` / `s420`
//! forms); `preset` accepts `"web"` / `"social"` / `"print"` /
//! `"archive"`; `strip_metadata` accepts `"all"` / `"keep_exif"` /
//! `"keep_icc"` / `"keep_all"` (snake or kebab, or the canonical
//! `"KeepExif"` / `"KeepIcc"` / `"KeepAll"`). `dpi` is a `[x, y]`
//! list. (`format="tiff"` is accepted: the core falls back to PNG
//! output for it, TIFF being decode-only.)
//!
//! `compress_with_report` returns a dict
//! `{output: bytes, original_size, output_size, saved_bytes,
//! saved_percent, format, width, height, quality_used, resized}`
//! with `format` lowercased. `get_image_info` returns
//! `{format, width, height, has_alpha, colour_type, file_size}`
//! with `colour_type` lowercased (`"rgb8"`). `bulk_compress`
//! returns one dict per input, in order:
//! `{"index", "report": dict | None, "error_code": str | None,
//! "error": str | None}` — one bad input fails only its own item.
//!
//! Core surface used (read from `crates/dusimagez/src/` — the core
//! types carry NO serde derives, so options are built field by field
//! here and reports are converted to dicts by hand):
//! `dusimagez::{CompressOptions, CompressReport, DusimagezError,
//! ImageFormat, ImageInfo, Preset, StripMetadata, Subsampling,
//! bulk_compress, compress_image, compress_to_size,
//! compress_with_report, get_image_info}` with
//! `DusimagezError::{code, message_en, message_hi}` and
//! `ImageFormat::from_name`.

// clippy 1.99 flags `PyResult<Py<PyAny>>` returns from #[pyfunction]s
// as useless conversions — a PyO3 false positive (the conversion is
// what turns the value into a Python object). Allowed crate-wide.
#![allow(clippy::useless_conversion)]

use dusimagez::{
    CompressOptions, CompressReport, ImageFormat, ImageInfo, Preset, StripMetadata, Subsampling,
    bulk_compress as core_bulk_compress, compress_image as core_compress_image,
    compress_to_size as core_compress_to_size, compress_with_report as core_compress_with_report,
    get_image_info as core_get_image_info,
};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

/// Separator for the native error payload. `\x1f` never appears in
/// the Rust message strings.
const SEP: char = '\x1f';

fn native_err(e: &dusimagez::DusimagezError) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
        "{}{}{}{}{}",
        e.code(),
        SEP,
        e.message_en(),
        SEP,
        e.message_hi()
    ))
}

fn invalid_argument(detail: &str) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
        "INVALID_ARGUMENT{}Invalid argument: {}{}Galat argument hai: {}",
        SEP, detail, SEP, detail
    ))
}

fn invalid_field(field: &str, expected: &str) -> PyErr {
    invalid_argument(&format!("Option \"{field}\" must be {expected}."))
}

/// Convert a Python value to a JSON value. Bytes become an array of
/// byte values.
fn py_to_json(obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if obj.is_none() {
        return Ok(serde_json::Value::Null);
    }
    if let Ok(b) = obj.extract::<bool>() {
        // extract::<bool> only succeeds for real bools in PyO3.
        return Ok(serde_json::Value::Bool(b));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(serde_json::Value::Number(i.into()));
    }
    if let Ok(u) = obj.extract::<u64>() {
        return Ok(serde_json::Value::Number(u.into()));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(serde_json::Value::String(s));
    }
    // bytes / bytearray only (the Python layer normalises memoryview
    // to bytes first, so the native side never sees it).
    if obj.is_instance_of::<PyBytes>() || obj.cast::<pyo3::types::PyByteArray>().is_ok() {
        let bytes: Vec<u8> = obj.extract()?;
        return Ok(serde_json::Value::Array(
            bytes
                .into_iter()
                .map(|b| serde_json::Value::Number(b.into()))
                .collect(),
        ));
    }
    if let Ok(items) = obj.extract::<Vec<Py<PyAny>>>() {
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(py_to_json(item.bind(obj.py()))?);
        }
        return Ok(serde_json::Value::Array(out));
    }
    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut out = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            out.insert(key, py_to_json(&v)?);
        }
        return Ok(serde_json::Value::Object(out));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "cannot convert {} to a JSON value",
        obj.get_type()
    )))
}

// ---------------------------------------------------------------------------
// Options parsing: partial dict -> CompressOptions, field by field.
// The core types have no serde impls. Each known key is read off the
// dict; unknown keys are ignored; a wrong type is INVALID_ARGUMENT.
// Range checks (quality 1-100, effort 0-10, …) belong to the core's
// `validate_options` — values that fit the Rust field type are
// passed through so the core reports its own stable code.
// ---------------------------------------------------------------------------

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

fn as_u8(v: &serde_json::Value, field: &str) -> PyResult<u8> {
    match v.as_u64().and_then(|n| u8::try_from(n).ok()) {
        Some(n) => Ok(n),
        None => Err(invalid_field(field, "an integer 0-255")),
    }
}

fn as_u64(v: &serde_json::Value, field: &str) -> PyResult<u64> {
    match v.as_u64() {
        Some(n) => Ok(n),
        None => Err(invalid_field(field, "a non-negative integer")),
    }
}

fn as_u32(v: &serde_json::Value, field: &str) -> PyResult<u32> {
    match v.as_u64().and_then(|n| u32::try_from(n).ok()) {
        Some(n) => Ok(n),
        None => Err(invalid_field(field, "a non-negative integer")),
    }
}

fn as_bool(v: &serde_json::Value, field: &str) -> PyResult<bool> {
    match v.as_bool() {
        Some(b) => Ok(b),
        None => Err(invalid_field(field, "a boolean")),
    }
}

fn as_string<'a>(v: &'a serde_json::Value, field: &str) -> PyResult<&'a str> {
    match v.as_str() {
        Some(s) => Ok(s),
        None => Err(invalid_field(field, "a string")),
    }
}

fn parse_subsampling(s: &str) -> PyResult<Subsampling> {
    match s.trim().to_ascii_lowercase().as_str() {
        "s444" | "444" | "4:4:4" => Ok(Subsampling::S444),
        "s422" | "422" | "4:2:2" => Ok(Subsampling::S422),
        "s420" | "420" | "4:2:0" => Ok(Subsampling::S420),
        _ => Err(invalid_argument(&format!(
            "Unknown subsampling \"{s}\". Use s444, s422 or s420."
        ))),
    }
}

fn parse_preset(s: &str) -> PyResult<Preset> {
    match s.trim().to_ascii_lowercase().as_str() {
        "web" => Ok(Preset::Web),
        "social" => Ok(Preset::Social),
        "print" => Ok(Preset::Print),
        "archive" => Ok(Preset::Archive),
        _ => Err(invalid_argument(&format!(
            "Unknown preset \"{s}\". Use web, social, print or archive."
        ))),
    }
}

fn parse_strip_metadata(s: &str) -> PyResult<StripMetadata> {
    match s.trim().to_ascii_lowercase().as_str() {
        "all" => Ok(StripMetadata::All),
        "keep_exif" | "keep-exif" | "keepexif" => Ok(StripMetadata::KeepExif),
        "keep_icc" | "keep-icc" | "keepicc" => Ok(StripMetadata::KeepIcc),
        "keep_all" | "keep-all" | "keepall" => Ok(StripMetadata::KeepAll),
        _ => Err(invalid_argument(&format!(
            "Unknown strip_metadata \"{s}\". Use all, keep_exif, keep_icc or keep_all."
        ))),
    }
}

fn parse_dpi(v: &serde_json::Value) -> PyResult<(u32, u32)> {
    let err = || invalid_field("dpi", "a [x, y] pair of non-negative integers");
    let arr = v.as_array().ok_or_else(err)?;
    if arr.len() != 2 {
        return Err(err());
    }
    let x = as_u32(&arr[0], "dpi")?;
    let y = as_u32(&arr[1], "dpi")?;
    Ok((x, y))
}

/// Parse an options dict into `CompressOptions`. `None` / `{}` mean
/// "all defaults".
fn parse_options(options: Option<&Bound<'_, PyDict>>) -> PyResult<CompressOptions> {
    let mut opts = CompressOptions::default();
    let Some(dict) = options else {
        return Ok(opts);
    };
    let provided = py_to_json(dict.as_any())?;
    let Some(obj) = provided.as_object() else {
        return Err(invalid_argument("Options must be a dict."));
    };
    if let Some(v) = field(obj, "quality") {
        opts.quality = as_u8(v, "quality")?;
    }
    if let Some(v) = field(obj, "format") {
        let s = as_string(v, "format")?;
        opts.format = match ImageFormat::from_name(s) {
            Some(f) => Some(f),
            None => {
                return Err(invalid_argument(&format!(
                    "Unknown format \"{s}\". Use jpeg, png, webp, avif, gif, bmp or tiff."
                )));
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

/// Lowercase format name for Python dicts: `Jpeg` -> `"jpeg"`,
/// `WebP` -> `"webp"`. (The core's `extension()` returns `"jpg"`
/// for JPEG; bindings report `"jpeg"`.)
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

fn report_to_py(py: Python<'_>, report: &CompressReport) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    dict.set_item("output", PyBytes::new(py, &report.output))?;
    dict.set_item("original_size", report.original_size)?;
    dict.set_item("output_size", report.output_size)?;
    dict.set_item("saved_bytes", report.saved_bytes)?;
    dict.set_item("saved_percent", report.saved_percent)?;
    dict.set_item("format", format_name(report.format))?;
    dict.set_item("width", report.width)?;
    dict.set_item("height", report.height)?;
    dict.set_item("quality_used", report.quality_used)?;
    dict.set_item("resized", report.resized)?;
    Ok(dict.into_any().unbind())
}

fn image_info_to_py(py: Python<'_>, info: &ImageInfo) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    dict.set_item("format", format_name(info.format))?;
    dict.set_item("width", info.width)?;
    dict.set_item("height", info.height)?;
    dict.set_item("has_alpha", info.has_alpha)?;
    dict.set_item("colour_type", info.colour_type.to_ascii_lowercase())?;
    dict.set_item("file_size", info.file_size)?;
    Ok(dict.into_any().unbind())
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(signature = (image, options=None))]
fn compress_image(
    py: Python<'_>,
    image: Vec<u8>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<Py<PyAny>> {
    let opts: CompressOptions = parse_options(options)?;
    let out = core_compress_image(&image, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new(py, &out).into_any().unbind())
}

#[pyfunction]
#[pyo3(signature = (image, target_bytes, options=None))]
fn compress_to_size(
    py: Python<'_>,
    image: Vec<u8>,
    target_bytes: u64,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<Py<PyAny>> {
    let opts: CompressOptions = parse_options(options)?;
    let out = core_compress_to_size(&image, target_bytes, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new(py, &out).into_any().unbind())
}

#[pyfunction]
#[pyo3(signature = (image, options=None))]
fn compress_with_report(
    py: Python<'_>,
    image: Vec<u8>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<Py<PyAny>> {
    let opts: CompressOptions = parse_options(options)?;
    let report = core_compress_with_report(&image, &opts).map_err(|e| native_err(&e))?;
    report_to_py(py, &report)
}

#[pyfunction]
#[pyo3(signature = (image))]
fn get_image_info(py: Python<'_>, image: Vec<u8>) -> PyResult<Py<PyAny>> {
    let info = core_get_image_info(&image).map_err(|e| native_err(&e))?;
    image_info_to_py(py, &info)
}

/// Batch helper: delegates to the core's `bulk_compress` (SPEC §1) —
/// one result per input, same order, failures per item, never
/// aborting. Returns one dict per input:
/// `{"index", "report": dict | None, "error_code": str | None,
/// "error": str | None}`.
#[pyfunction]
#[pyo3(signature = (images, options=None))]
fn bulk_compress(
    py: Python<'_>,
    images: Vec<Vec<u8>>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<Py<PyAny>> {
    let opts: CompressOptions = parse_options(options)?;
    let list = PyList::empty(py);
    for (index, item) in core_bulk_compress(&images, &opts).into_iter().enumerate() {
        let dict = PyDict::new(py);
        dict.set_item("index", index).expect("dict set");
        match item {
            Ok(report) => {
                dict.set_item("report", report_to_py(py, &report)?)
                    .expect("dict set");
                dict.set_item("error_code", py.None()).expect("dict set");
                dict.set_item("error", py.None()).expect("dict set");
            }
            Err(e) => {
                dict.set_item("report", py.None()).expect("dict set");
                dict.set_item("error_code", e.code()).expect("dict set");
                dict.set_item("error", e.message_en()).expect("dict set");
                dict.set_item("error_hi", e.message_hi()).expect("dict set");
            }
        }
        list.append(dict).expect("list append");
    }
    Ok(list.into_any().unbind())
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress_image, m)?)?;
    m.add_function(wrap_pyfunction!(compress_to_size, m)?)?;
    m.add_function(wrap_pyfunction!(compress_with_report, m)?)?;
    m.add_function(wrap_pyfunction!(get_image_info, m)?)?;
    m.add_function(wrap_pyfunction!(bulk_compress, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
