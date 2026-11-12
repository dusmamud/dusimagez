//! UniFFI scaffolding crate for dusimagez (v0.1.0).
//!
//! This crate exists for one reason: the Kotlin (Android) and Swift
//! (iOS/macOS) bindings need a `cdylib` named `uniffi_dusimagez` that
//! ships the UniFFI scaffolding for the UDL in
//! `crates/dusimagez/src/dusimagez.udl`. There is **no logic here** —
//! every type is a thin adapter over the core crate's types, and every
//! function converts its arguments and delegates to
//! [`dusimagez`].
//!
//! UDL divergences (see the UDL header for the full notes):
//! * `CompressOptions.dpi`: UDL `Dpi?` record <-> core `Option<(u32, u32)>`.
//! * `CompressOptions.subsampling`: UDL `null` means "let the core apply
//!   the quality-dependent rule" — the adapter passes the core's
//!   not-explicit sentinel (`S420`), which [`dusimagez`] resolves to
//!   S420 for quality < 90 and S444 otherwise.
//! * `ImageInfo.format` / `colour_type`: UDL strings, lowercase — the
//!   adapter lowercases the core's `Debug` colour-type name
//!   (`"Rgb8"` -> `"rgb8"`) and uses the format's lowercase extension.
//! * `bulk_compress` never throws: per-item outcomes ride in
//!   [`BulkItemResult`].
//!
//! The generated Kotlin loads this library as `uniffi_dusimagez`
//! (JNA); the Swift side links the same symbols via the
//! `DusimagezFFI` modulemap. Do not rename `[lib] name`.

use std::fmt;

use dusimagez::{
    CompressOptions as CoreOptions, CompressReport as CoreReport, ImageInfo as CoreInfo,
};

// ---------------------------------------------------------------------------
// Value types — names and fields must match the UDL exactly.
// ---------------------------------------------------------------------------

/// Output image format (UDL `enum ImageFormat`). TIFF is deliberately
/// absent: input-only, never an output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Jpeg,
    Png,
    WebP,
    Avif,
    Gif,
    Bmp,
}

/// Chroma subsampling for JPEG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subsampling {
    S444,
    S422,
    S420,
}

/// Preset: sets quality / effort / subsampling defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Preset {
    Web,
    Social,
    Print,
    Archive,
}

/// What metadata survives on output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StripMetadata {
    All,
    KeepExif,
    KeepIcc,
    KeepAll,
}

/// DPI pair (UDL `dictionary Dpi`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dpi {
    pub width: u32,
    pub height: u32,
}

/// Full before/after report for one compression (UDL `CompressReport`).
#[derive(Debug, Clone, PartialEq)]
pub struct CompressReport {
    pub output: Vec<u8>,
    pub original_size: u64,
    pub output_size: u64,
    pub saved_bytes: u64,
    pub saved_percent: f32,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub quality_used: u8,
    pub resized: bool,
}

/// Summary of an image (UDL `ImageInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
    pub colour_type: String,
    pub file_size: u64,
}

/// One item of a `bulk_compress` result (UDL `BulkItemResult`).
#[derive(Debug, Clone, PartialEq)]
pub struct BulkItemResult {
    pub index: u32,
    pub ok: bool,
    pub report: Option<CompressReport>,
    pub error_code: Option<String>,
    pub error_message_en: Option<String>,
    pub error_message_hi: Option<String>,
}

/// Pro compressor options (UDL `CompressOptions`, SPEC §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressOptions {
    pub quality: u8,
    pub format: Option<ImageFormat>,
    pub progressive: bool,
    pub lossless: bool,
    pub effort: u8,
    /// `None` = let the core apply the quality-dependent rule.
    pub subsampling: Option<Subsampling>,
    pub png_compression_level: u8,
    pub palette: bool,
    pub preset: Option<Preset>,
    pub target_bytes: Option<u64>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub strip_metadata: StripMetadata,
    pub auto_orient: bool,
    pub dpi: Option<Dpi>,
    pub max_pixels: u64,
    pub max_dimension: u32,
}

// ---------------------------------------------------------------------------
// Errors — UDL `[Error] interface DusimagezError`; one variant per
// stable code, each carrying code + EN + HI messages.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DusimagezError {
    InvalidImage {
        code: String,
        message_en: String,
        message_hi: String,
    },
    UnsupportedFormat {
        code: String,
        message_en: String,
        message_hi: String,
    },
    ImageTooLarge {
        code: String,
        message_en: String,
        message_hi: String,
    },
    TargetSizeUnreachable {
        code: String,
        message_en: String,
        message_hi: String,
    },
    InvalidQuality {
        code: String,
        message_en: String,
        message_hi: String,
    },
    InvalidDimensions {
        code: String,
        message_en: String,
        message_hi: String,
    },
    InvalidArgument {
        code: String,
        message_en: String,
        message_hi: String,
    },
    EmptyInput {
        code: String,
        message_en: String,
        message_hi: String,
    },
}

impl fmt::Display for DusimagezError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (code, message_en) = match self {
            Self::InvalidImage {
                code, message_en, ..
            }
            | Self::UnsupportedFormat {
                code, message_en, ..
            }
            | Self::ImageTooLarge {
                code, message_en, ..
            }
            | Self::TargetSizeUnreachable {
                code, message_en, ..
            }
            | Self::InvalidQuality {
                code, message_en, ..
            }
            | Self::InvalidDimensions {
                code, message_en, ..
            }
            | Self::InvalidArgument {
                code, message_en, ..
            }
            | Self::EmptyInput {
                code, message_en, ..
            } => (code, message_en),
        };
        write!(f, "[{code}] {message_en}")
    }
}

impl std::error::Error for DusimagezError {}

impl From<dusimagez::DusimagezError> for DusimagezError {
    fn from(e: dusimagez::DusimagezError) -> Self {
        let code = e.code().to_owned();
        let message_en = e.message_en().to_owned();
        let message_hi = e.message_hi().to_owned();
        match e.code() {
            "INVALID_IMAGE" => Self::InvalidImage {
                code,
                message_en,
                message_hi,
            },
            "UNSUPPORTED_FORMAT" => Self::UnsupportedFormat {
                code,
                message_en,
                message_hi,
            },
            "IMAGE_TOO_LARGE" => Self::ImageTooLarge {
                code,
                message_en,
                message_hi,
            },
            "TARGET_SIZE_UNREACHABLE" => Self::TargetSizeUnreachable {
                code,
                message_en,
                message_hi,
            },
            "INVALID_QUALITY" => Self::InvalidQuality {
                code,
                message_en,
                message_hi,
            },
            "INVALID_DIMENSIONS" => Self::InvalidDimensions {
                code,
                message_en,
                message_hi,
            },
            "INVALID_ARGUMENT" => Self::InvalidArgument {
                code,
                message_en,
                message_hi,
            },
            _ => Self::EmptyInput {
                code,
                message_en,
                message_hi,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Conversions to/from the core types.
// ---------------------------------------------------------------------------

impl From<ImageFormat> for dusimagez::ImageFormat {
    fn from(f: ImageFormat) -> Self {
        match f {
            ImageFormat::Jpeg => Self::Jpeg,
            ImageFormat::Png => Self::Png,
            ImageFormat::WebP => Self::WebP,
            ImageFormat::Avif => Self::Avif,
            ImageFormat::Gif => Self::Gif,
            ImageFormat::Bmp => Self::Bmp,
        }
    }
}

impl From<dusimagez::ImageFormat> for ImageFormat {
    fn from(f: dusimagez::ImageFormat) -> Self {
        match f {
            // TIFF is decode-only and can never be an *output* format;
            // the core falls back to PNG for TIFF input, so this arm is
            // defensive only.
            dusimagez::ImageFormat::Tiff => Self::Png,
            dusimagez::ImageFormat::Jpeg => Self::Jpeg,
            dusimagez::ImageFormat::Png => Self::Png,
            dusimagez::ImageFormat::WebP => Self::WebP,
            dusimagez::ImageFormat::Avif => Self::Avif,
            dusimagez::ImageFormat::Gif => Self::Gif,
            dusimagez::ImageFormat::Bmp => Self::Bmp,
        }
    }
}

impl From<Subsampling> for dusimagez::Subsampling {
    fn from(s: Subsampling) -> Self {
        match s {
            Subsampling::S444 => Self::S444,
            Subsampling::S422 => Self::S422,
            Subsampling::S420 => Self::S420,
        }
    }
}

impl From<Preset> for dusimagez::Preset {
    fn from(p: Preset) -> Self {
        match p {
            Preset::Web => Self::Web,
            Preset::Social => Self::Social,
            Preset::Print => Self::Print,
            Preset::Archive => Self::Archive,
        }
    }
}

impl From<StripMetadata> for dusimagez::StripMetadata {
    fn from(s: StripMetadata) -> Self {
        match s {
            StripMetadata::All => Self::All,
            StripMetadata::KeepExif => Self::KeepExif,
            StripMetadata::KeepIcc => Self::KeepIcc,
            StripMetadata::KeepAll => Self::KeepAll,
        }
    }
}

impl From<CompressOptions> for CoreOptions {
    fn from(o: CompressOptions) -> Self {
        Self {
            quality: o.quality,
            format: o.format.map(dusimagez::ImageFormat::from),
            progressive: o.progressive,
            lossless: o.lossless,
            effort: o.effort,
            // None = core applies the quality-dependent rule: the
            // core treats S420 as "not explicit" and resolves it.
            subsampling: o
                .subsampling
                .map_or(dusimagez::Subsampling::S420, dusimagez::Subsampling::from),
            png_compression_level: o.png_compression_level,
            palette: o.palette,
            preset: o.preset.map(dusimagez::Preset::from),
            target_bytes: o.target_bytes,
            max_width: o.max_width,
            max_height: o.max_height,
            strip_metadata: o.strip_metadata.into(),
            auto_orient: o.auto_orient,
            dpi: o.dpi.map(|d| (d.width, d.height)),
            max_pixels: o.max_pixels,
            max_dimension: o.max_dimension,
        }
    }
}

impl From<CoreReport> for CompressReport {
    fn from(r: CoreReport) -> Self {
        Self {
            output: r.output,
            original_size: r.original_size,
            output_size: r.output_size,
            saved_bytes: r.saved_bytes,
            saved_percent: r.saved_percent,
            format: r.format.into(),
            width: r.width,
            height: r.height,
            quality_used: r.quality_used,
            resized: r.resized,
        }
    }
}

impl From<CoreInfo> for ImageInfo {
    fn from(i: CoreInfo) -> Self {
        Self {
            // Lowercase short name, e.g. "jpeg" / "tiff". Do NOT use
            // `extension()` here: it returns "jpg" for JPEG, but the
            // UDL contract is "jpeg".
            format: match i.format {
                dusimagez::ImageFormat::Jpeg => "jpeg".to_owned(),
                other => other.extension().to_owned(),
            },
            width: i.width,
            height: i.height,
            has_alpha: i.has_alpha,
            // Core stores the `Debug` name ("Rgb8"); the UDL
            // contract is lowercase ("rgb8").
            colour_type: i.colour_type.to_ascii_lowercase(),
            file_size: i.file_size,
        }
    }
}

// ---------------------------------------------------------------------------
// The five SPEC §1 functions — signatures must match the UDL exactly.
// ---------------------------------------------------------------------------

/// Compress one image (UDL `compress_image`).
pub fn compress_image(input: Vec<u8>, options: CompressOptions) -> Result<Vec<u8>, DusimagezError> {
    dusimagez::compress_image(&input, &options.into()).map_err(DusimagezError::from)
}

/// Compress to an exact target file size (UDL `compress_to_size`).
pub fn compress_to_size(
    input: Vec<u8>,
    target_bytes: u64,
    options: CompressOptions,
) -> Result<Vec<u8>, DusimagezError> {
    dusimagez::compress_to_size(&input, target_bytes, &options.into()).map_err(DusimagezError::from)
}

/// Compress with a full before/after report (UDL `compress_with_report`).
pub fn compress_with_report(
    input: Vec<u8>,
    options: CompressOptions,
) -> Result<CompressReport, DusimagezError> {
    dusimagez::compress_with_report(&input, &options.into())
        .map(CompressReport::from)
        .map_err(DusimagezError::from)
}

/// Summarise an image (UDL `get_image_info`).
pub fn get_image_info(input: Vec<u8>) -> Result<ImageInfo, DusimagezError> {
    dusimagez::get_image_info(&input)
        .map(ImageInfo::from)
        .map_err(DusimagezError::from)
}

/// Batch compress; per-item outcomes, never aborts (UDL `bulk_compress`).
pub fn bulk_compress(inputs: Vec<Vec<u8>>, options: CompressOptions) -> Vec<BulkItemResult> {
    let core_options: CoreOptions = options.into();
    dusimagez::bulk_compress(&inputs, &core_options)
        .into_iter()
        .enumerate()
        .map(|(index, result)| match result {
            Ok(report) => BulkItemResult {
                index: index as u32,
                ok: true,
                report: Some(report.into()),
                error_code: None,
                error_message_en: None,
                error_message_hi: None,
            },
            Err(e) => BulkItemResult {
                index: index as u32,
                ok: false,
                report: None,
                error_code: Some(e.code().to_owned()),
                error_message_en: Some(e.message_en().to_owned()),
                error_message_hi: Some(e.message_hi().to_owned()),
            },
        })
        .collect()
}

uniffi::include_scaffolding!("dusimagez");
