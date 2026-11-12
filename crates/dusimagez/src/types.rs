//! Public types: options, enums, report and info structs (SPEC §1-2).
//!
//! Preset rule (documented, deterministic): a preset supplies the *base*
//! values for quality / effort / subsampling. A field that still holds its
//! global default (quality 80, effort 6, subsampling S420) is filled from
//! the preset; a field set to anything else counts as an explicit override
//! and wins over the preset. So `quality: 70` + `preset: Print` keeps 70,
//! while `quality: 80` (the default) + `preset: Print` becomes 95.

use std::fmt;

/// Image format. `Tiff` is decode-only: it can be an input format and
/// appears in [`ImageInfo`], but it is never an output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Jpeg,
    Png,
    WebP,
    Avif,
    Gif,
    Bmp,
    Tiff,
}

impl ImageFormat {
    /// Lowercase file extension without the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Avif => "avif",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
        }
    }

    /// MIME type.
    #[must_use]
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
            Self::Gif => "image/gif",
            Self::Bmp => "image/bmp",
            Self::Tiff => "image/tiff",
        }
    }

    /// Parse a user-supplied name: `jpg`/`jpeg`, `png`, `webp`, `avif`,
    /// `gif`, `bmp`, `tiff` (case-insensitive, optional leading dot).
    /// Returns `None` for anything else.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "webp" => Some(Self::WebP),
            "avif" => Some(Self::Avif),
            "gif" => Some(Self::Gif),
            "bmp" => Some(Self::Bmp),
            "tiff" | "tif" => Some(Self::Tiff),
            _ => None,
        }
    }

    /// Can this format be an output format? (Everything except TIFF.)
    #[must_use]
    pub const fn is_output_format(self) -> bool {
        !matches!(self, Self::Tiff)
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Jpeg => "JPEG",
            Self::Png => "PNG",
            Self::WebP => "WebP",
            Self::Avif => "AVIF",
            Self::Gif => "GIF",
            Self::Bmp => "BMP",
            Self::Tiff => "TIFF",
        };
        f.write_str(name)
    }
}

/// JPEG chroma subsampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Subsampling {
    /// 4:4:4 — no chroma downsampling (best quality, largest files).
    S444,
    /// 4:2:2 — horizontal chroma halved.
    S422,
    /// 4:2:0 — chroma halved both ways (smallest files).
    #[default]
    S420,
}

/// Named preset: base quality / effort / subsampling defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Preset {
    /// Web: quality 80, effort 4, 4:2:0.
    Web,
    /// Social: quality 85, effort 4, 4:2:0.
    Social,
    /// Print: quality 95, effort 6, 4:4:4.
    Print,
    /// Archive: quality 100, effort 6, 4:4:4.
    Archive,
}

impl Preset {
    /// (quality, effort, subsampling) base values for this preset.
    #[must_use]
    pub const fn defaults(self) -> (u8, u8, Subsampling) {
        match self {
            Self::Web => (80, 4, Subsampling::S420),
            Self::Social => (85, 4, Subsampling::S420),
            Self::Print => (95, 6, Subsampling::S444),
            Self::Archive => (100, 6, Subsampling::S444),
        }
    }
}

/// What metadata to keep in the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StripMetadata {
    /// Strip everything (default).
    #[default]
    All,
    /// Keep EXIF (and ICC where the encoder supports it).
    KeepExif,
    /// Keep the ICC colour profile.
    KeepIcc,
    /// Keep EXIF and ICC.
    KeepAll,
}

/// Pro compression options (SPEC §2).
#[derive(Debug, Clone, PartialEq)]
pub struct CompressOptions {
    /// Quality 1-100 (JPEG/WebP/AVIF). Default 80.
    pub quality: u8,
    /// Output format. `None` keeps the input format (TIFF input falls
    /// back to PNG, which is lossless).
    pub format: Option<ImageFormat>,
    /// JPEG progressive / PNG interlaced. Accepted and stored; the v0.1.0
    /// encoders do not expose progressive/interlaced control, so output
    /// is baseline JPEG / non-interlaced PNG (see crate docs).
    pub progressive: bool,
    /// Lossless output (WebP/PNG only). WebP encodes lossless; AVIF maps
    /// to quality 100 (ravif has no true lossless mode). Ignored for
    /// JPEG/GIF/BMP.
    pub lossless: bool,
    /// Effort: WebP method 0-6; AVIF speed 0-10. Default 6.
    pub effort: u8,
    /// JPEG chroma subsampling. Default S420; the effective default is
    /// S420 for quality < 90 and S444 otherwise (see [`Self::resolved`]).
    pub subsampling: Subsampling,
    /// PNG compression level 0-9. Default 6.
    pub png_compression_level: u8,
    /// PNG: quantise to a 256-colour palette with libimagequant.
    /// Default false.
    pub palette: bool,
    /// Preset supplying base quality / effort / subsampling. Explicit
    /// fields (anything != the global default) override the preset.
    pub preset: Option<Preset>,
    /// Target output size in bytes; same behaviour as `compress_to_size`.
    pub target_bytes: Option<u64>,
    /// Downscale-only max width (aspect preserved, never upscale).
    pub max_width: Option<u32>,
    /// Downscale-only max height (aspect preserved, never upscale).
    pub max_height: Option<u32>,
    /// Metadata handling. Default `All` (strip).
    pub strip_metadata: StripMetadata,
    /// Apply EXIF orientation on load. Default true.
    pub auto_orient: bool,
    /// Output DPI (width, height). Honoured for JPEG (JFIF density) and
    /// PNG (pHYs chunk); ignored for other formats.
    pub dpi: Option<(u32, u32)>,
    /// Refuse images with more pixels than this. Default 120_000_000.
    pub max_pixels: u64,
    /// Refuse images with either side longer than this. Default 16000.
    pub max_dimension: u32,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            quality: 80,
            format: None,
            progressive: false,
            lossless: false,
            effort: 6,
            subsampling: Subsampling::S420,
            png_compression_level: 6,
            palette: false,
            preset: None,
            target_bytes: None,
            max_width: None,
            max_height: None,
            strip_metadata: StripMetadata::All,
            auto_orient: true,
            dpi: None,
            max_pixels: 120_000_000,
            max_dimension: 16_000,
        }
    }
}

/// Options after preset resolution — what the encoders actually use.
/// Public so bindings can share the same resolution rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedOptions {
    pub quality: u8,
    pub effort: u8,
    pub subsampling: Subsampling,
}

impl CompressOptions {
    /// Resolve preset + quality-dependent subsampling default into the
    /// concrete values the encoders use.
    #[must_use]
    pub fn resolved(&self) -> ResolvedOptions {
        let mut quality = self.quality;
        let mut effort = self.effort;
        // S420 is the global default, so a stored S420 may mean "not set".
        let mut subsampling = self.subsampling;
        let mut subsampling_explicit = self.subsampling != Subsampling::S420;
        if let Some(preset) = self.preset {
            let (pq, pe, ps) = preset.defaults();
            if quality == 80 {
                quality = pq;
            }
            if effort == 6 {
                effort = pe;
            }
            if !subsampling_explicit {
                subsampling = ps;
                subsampling_explicit = true;
            }
        }
        if !subsampling_explicit {
            // SPEC §2 default: S420 for JPEG quality < 90, S444 otherwise.
            subsampling = if quality < 90 {
                Subsampling::S420
            } else {
                Subsampling::S444
            };
        }
        ResolvedOptions {
            quality,
            effort,
            subsampling,
        }
    }
}

/// Before/after report for one compression (SPEC §1).
#[derive(Debug, Clone, PartialEq)]
pub struct CompressReport {
    /// Compressed bytes.
    pub output: Vec<u8>,
    /// Input size in bytes.
    pub original_size: u64,
    /// Output size in bytes.
    pub output_size: u64,
    /// `original_size - output_size`, saturating at 0.
    pub saved_bytes: u64,
    /// `saved_bytes / original_size * 100.0` (0.0 for empty input,
    /// which never happens — empty input is an error).
    pub saved_percent: f32,
    /// Output format.
    pub format: ImageFormat,
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Quality the output was actually encoded at.
    pub quality_used: u8,
    /// True when the output dimensions differ from the input dimensions
    /// (max_width/max_height downscale or target-size downscale).
    pub resized: bool,
}

/// Information about an input image (SPEC §1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
    /// Detected input format.
    pub format: ImageFormat,
    /// Width in pixels (after EXIF orientation is *not* applied — this is
    /// the stored width; use a compress call for oriented dimensions).
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// True when the colour type carries an alpha channel.
    pub has_alpha: bool,
    /// Colour type as reported by the decoder, lowercase, e.g. `"rgb8"`.
    pub colour_type: String,
    /// Input size in bytes.
    pub file_size: u64,
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec() {
        let o = CompressOptions::default();
        assert_eq!(o.quality, 80);
        assert_eq!(o.effort, 6);
        assert_eq!(o.png_compression_level, 6);
        assert_eq!(o.max_pixels, 120_000_000);
        assert_eq!(o.max_dimension, 16_000);
        assert!(o.auto_orient);
        assert_eq!(o.strip_metadata, StripMetadata::All);
    }

    #[test]
    fn subsampling_default_follows_quality() {
        let mut o = CompressOptions::default();
        o.quality = 80;
        assert_eq!(o.resolved().subsampling, Subsampling::S420);
        o.quality = 95;
        assert_eq!(o.resolved().subsampling, Subsampling::S444);
        o.quality = 89;
        assert_eq!(o.resolved().subsampling, Subsampling::S420);
        o.quality = 90;
        assert_eq!(o.resolved().subsampling, Subsampling::S444);
    }

    #[test]
    fn preset_fills_defaults_but_explicit_fields_win() {
        let mut o = CompressOptions::default();
        o.preset = Some(Preset::Print);
        let r = o.resolved();
        assert_eq!(r.quality, 95);
        assert_eq!(r.effort, 6);
        assert_eq!(r.subsampling, Subsampling::S444);

        let mut o = CompressOptions::default();
        o.preset = Some(Preset::Print);
        o.quality = 70;
        assert_eq!(o.resolved().quality, 70);
        assert_eq!(o.resolved().subsampling, Subsampling::S444);

        let mut o = CompressOptions::default();
        o.preset = Some(Preset::Web);
        o.subsampling = Subsampling::S444;
        assert_eq!(o.resolved().subsampling, Subsampling::S444);
        assert_eq!(o.resolved().quality, 80);
    }

    #[test]
    fn format_names_round_trip() {
        assert_eq!(ImageFormat::from_name("jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_name("JPEG"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_name(".png"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_name("tif"), Some(ImageFormat::Tiff));
        assert_eq!(ImageFormat::from_name("heic"), None);
        assert!(!ImageFormat::Tiff.is_output_format());
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Avif.mime_type(), "image/avif");
    }
}
