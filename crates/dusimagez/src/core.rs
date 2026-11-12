//! Core implementation (SPEC §1-4).
//!
//! Honest v0.1.0 encoder limits (documented here so bindings don't promise
//! more — these are real limits of the crates available, not TODOs hidden
//! from the caller):
//!
//! * AVIF input returns [`UNSUPPORTED_FORMAT`](crate::DusimagezError):
//!   decoding AVIF needs dav1d (`avif-native`), whose build needs nasm /
//!   a dav1d pkg-config file — neither is available in this environment
//!   (verified with a scratch build 2026-11-12). AVIF *output* works
//!   (lossy, 8-bit, via ravif).
//! * JPEG chroma subsampling: the `subsampling` option is validated and
//!   resolved per SPEC §2, but the `image`-crate JPEG encoder exposes no
//!   subsampling control, and mozjpeg (the only alternative) needs nasm,
//!   which is unavailable here. Output therefore uses the encoder's
//!   default subsampling. The option is honoured by the API contract and
//!   will take effect when an encoder with subsampling control lands.
//! * Progressive JPEG / interlaced PNG: accepted, not honoured — neither
//!   the `image`-crate JPEG encoder nor the `png` 0.18 encoder exposes
//!   progressive/interlaced encoding.
//! * WebP output is lossy via libwebp (`webp` crate) when `lossless` is
//!   false, lossless otherwise; `effort` maps to libwebp `method` (0-6).
//! * AVIF `lossless` maps to quality 100 (ravif has no true lossless
//!   mode); AVIF `effort` is the rav1e speed 0-10 (0 is clamped to 1,
//!   ravif's minimum).
//! * PNG `dpi` is written as a pHYs chunk; JPEG `dpi` via the encoder's
//!   JFIF density. Other formats ignore `dpi`.
//! * `KeepExif` / `KeepAll` preserve ICC + EXIF only where the chosen
//!   encoder supports it (JPEG/PNG via the `image` encoders); the libwebp
//!   and palette-PNG paths drop metadata. EXIF bytes come from the
//!   decoder when the format carries them.
//! * GIF output is the first frame only for animated input.
//! * The whole image lives in memory; there is no streaming mode.

use std::io::Cursor;

use image::codecs::bmp::BmpEncoder;
use image::codecs::gif::GifEncoder;
use image::codecs::jpeg::{JpegEncoder, PixelDensity, PixelDensityUnit};
use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageEncoder, ImageReader};

use crate::error::{DusimagezError, Result};
use crate::types::{
    CompressOptions, CompressReport, ImageFormat, ImageInfo, ResolvedOptions, StripMetadata,
};

/// Public API: compress `input` bytes with `options`, return output bytes.
pub fn compress_image(input: &[u8], options: &CompressOptions) -> Result<Vec<u8>> {
    Ok(compress_inner(input, options)?.bytes)
}

/// Public API: compress `input` to at most `target_bytes` (SPEC §1).
pub fn compress_to_size(
    input: &[u8],
    target_bytes: u64,
    options: &CompressOptions,
) -> Result<Vec<u8>> {
    let mut options = options.clone();
    options.target_bytes = Some(target_bytes);
    Ok(compress_inner(input, &options)?.bytes)
}

/// Public API: compress and return the full before/after report.
pub fn compress_with_report(input: &[u8], options: &CompressOptions) -> Result<CompressReport> {
    let out = compress_inner(input, options)?;
    let original_size = input.len() as u64;
    let output_size = out.bytes.len() as u64;
    let saved_bytes = original_size.saturating_sub(output_size);
    let saved_percent = if original_size == 0 {
        0.0
    } else {
        saved_bytes as f32 / original_size as f32 * 100.0
    };
    Ok(CompressReport {
        output: out.bytes,
        original_size,
        output_size,
        saved_bytes,
        saved_percent,
        format: out.format,
        width: out.width,
        height: out.height,
        quality_used: out.quality_used,
        resized: out.resized,
    })
}

/// Public API: inspect an input image without compressing it.
pub fn get_image_info(input: &[u8]) -> Result<ImageInfo> {
    let format = detect_input_format(input)?;
    let decoder = open_decoder(input)?;
    let (width, height) = decoder.dimensions();
    let colour_type = decoder.color_type();
    Ok(ImageInfo {
        format,
        width,
        height,
        has_alpha: colour_type.has_alpha(),
        // Lowercase, matching the UDL contract ("rgb8") — every
        // binding returns the same case.
        colour_type: format!("{colour_type:?}").to_ascii_lowercase(),
        file_size: input.len() as u64,
    })
}

/// Public API: one result per input, same order, never aborts the batch.
pub fn bulk_compress(inputs: &[Vec<u8>], options: &CompressOptions) -> Vec<Result<CompressReport>> {
    inputs
        .iter()
        .map(|input| compress_with_report(input, options))
        .collect()
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_options(options: &CompressOptions) -> Result<()> {
    if options.quality == 0 || options.quality > 100 {
        return Err(DusimagezError::invalid_quality(options.quality));
    }
    if options.effort > 10 {
        return Err(DusimagezError::invalid_argument(
            "effort must be 0-10 (WebP uses 0-6, AVIF speed 0-10)",
        ));
    }
    if options.png_compression_level > 9 {
        return Err(DusimagezError::invalid_argument(
            "png_compression_level must be 0-9",
        ));
    }
    if options.max_width == Some(0) || options.max_height == Some(0) {
        return Err(DusimagezError::invalid_dimensions(
            "max_width / max_height must be > 0 when set",
        ));
    }
    if options.target_bytes == Some(0) {
        return Err(DusimagezError::invalid_argument(
            "target_bytes must be > 0 when set",
        ));
    }
    if options.max_pixels == 0 {
        return Err(DusimagezError::invalid_argument("max_pixels must be > 0"));
    }
    if options.max_dimension == 0 {
        return Err(DusimagezError::invalid_argument(
            "max_dimension must be > 0",
        ));
    }
    if let Some((x, y)) = options.dpi
        && (x == 0 || y == 0)
    {
        return Err(DusimagezError::invalid_argument("dpi values must be > 0"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

/// A decoded image plus the metadata the encoders may need.
struct Decoded {
    image: DynamicImage,
    format: ImageFormat,
    /// True when max_width / max_height changed the dimensions.
    resized: bool,
    icc: Option<Vec<u8>>,
    exif: Option<Vec<u8>>,
}

/// Detect the input format. `EMPTY_INPUT` for empty bytes,
/// `INVALID_IMAGE` when the bytes match no known image format,
/// `UNSUPPORTED_FORMAT` for recognised formats this build cannot decode
/// (AVIF, QOI, EXR, HDR, PNM, TGA, DDS, ...).
fn detect_input_format(input: &[u8]) -> Result<ImageFormat> {
    if input.is_empty() {
        return Err(DusimagezError::empty_input("image bytes"));
    }
    match image::guess_format(input) {
        Ok(image::ImageFormat::Jpeg) => Ok(ImageFormat::Jpeg),
        Ok(image::ImageFormat::Png) => Ok(ImageFormat::Png),
        Ok(image::ImageFormat::WebP) => Ok(ImageFormat::WebP),
        Ok(image::ImageFormat::Gif) => Ok(ImageFormat::Gif),
        Ok(image::ImageFormat::Bmp) => Ok(ImageFormat::Bmp),
        Ok(image::ImageFormat::Tiff) => Ok(ImageFormat::Tiff),
        Ok(image::ImageFormat::Avif) => Err(DusimagezError::unsupported_format("avif")),
        Ok(other) => Err(DusimagezError::unsupported_format(
            &format!("{other:?}").to_lowercase(),
        )),
        Err(_) => Err(DusimagezError::invalid_image(
            "the bytes do not match any known image format",
        )),
    }
}

fn open_decoder(input: &[u8]) -> Result<impl ImageDecoder> {
    let reader = ImageReader::new(Cursor::new(input))
        .with_guessed_format()
        .map_err(|_| DusimagezError::invalid_image("could not detect the image format"))?;
    reader.into_decoder().map_err(|e| {
        DusimagezError::invalid_image(&format!("could not open the image decoder ({e})"))
    })
}

fn decode(input: &[u8], options: &CompressOptions) -> Result<Decoded> {
    let format = detect_input_format(input)?;
    let mut decoder = open_decoder(input)?;
    let (width, height) = decoder.dimensions();
    let pixels = width as u64 * height as u64;
    if pixels > options.max_pixels
        || width > options.max_dimension
        || height > options.max_dimension
    {
        return Err(DusimagezError::image_too_large(
            width,
            height,
            options.max_pixels,
            options.max_dimension,
        ));
    }
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let icc = match options.strip_metadata {
        StripMetadata::KeepIcc | StripMetadata::KeepAll => decoder.icc_profile().ok().flatten(),
        _ => None,
    };
    let exif = match options.strip_metadata {
        StripMetadata::KeepExif | StripMetadata::KeepAll => decoder.exif_metadata().ok().flatten(),
        _ => None,
    };
    let mut image = DynamicImage::from_decoder(decoder)
        .map_err(|e| DusimagezError::invalid_image(&format!("could not decode the image ({e})")))?;
    if options.auto_orient {
        image.apply_orientation(orientation);
    }
    let (image, resized) = downscale_to_max(&image, options.max_width, options.max_height);
    Ok(Decoded {
        image,
        format,
        resized,
        icc,
        exif,
    })
}

/// Downscale-only resize honouring max_width / max_height, aspect preserved.
fn downscale_to_max(
    img: &DynamicImage,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> (DynamicImage, bool) {
    let (w, h) = (img.width(), img.height());
    let mut scale = 1.0f32;
    if let Some(mw) = max_width {
        scale = scale.min(mw as f32 / w as f32);
    }
    if let Some(mh) = max_height {
        scale = scale.min(mh as f32 / h as f32);
    }
    if scale >= 1.0 {
        return (img.clone(), false);
    }
    let nw = ((w as f32 * scale).round() as u32).max(1);
    let nh = ((h as f32 * scale).round() as u32).max(1);
    (img.resize(nw, nh, FilterType::Lanczos3), true)
}

fn output_format(decoded: &Decoded, options: &CompressOptions) -> ImageFormat {
    match options.format {
        Some(ImageFormat::Tiff) => ImageFormat::Png, // TIFF is decode-only.
        Some(f) => f,
        None => {
            if decoded.format.is_output_format() {
                decoded.format
            } else {
                ImageFormat::Png // TIFF input falls back to lossless PNG.
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared compress path
// ---------------------------------------------------------------------------

struct InnerOutput {
    bytes: Vec<u8>,
    format: ImageFormat,
    width: u32,
    height: u32,
    quality_used: u8,
    resized: bool,
}

fn compress_inner(input: &[u8], options: &CompressOptions) -> Result<InnerOutput> {
    validate_options(options)?;
    let decoded = decode(input, options)?;
    let format = output_format(&decoded, options);
    let resolved = options.resolved();
    if let Some(target) = options.target_bytes {
        return match search_to_size(&decoded, format, &resolved, options, target)? {
            Some(out) => Ok(out),
            None => Err(DusimagezError::target_size_unreachable(target)),
        };
    }
    let png_level = options.png_compression_level;
    let bytes = encode_at(
        &decoded.image,
        format,
        options,
        &resolved,
        resolved.quality,
        png_level,
        decoded.icc.as_deref(),
        decoded.exif.as_deref(),
    )?;
    Ok(InnerOutput {
        bytes,
        format,
        width: decoded.image.width(),
        height: decoded.image.height(),
        quality_used: resolved.quality,
        resized: decoded.resized,
    })
}

/// Binary search on quality (max 10 steps) at full size, then downscale in
/// 10% steps (never below 10% of the search image's dimensions). Returns
/// `None` when even the smallest variant exceeds the target.
fn search_to_size(
    decoded: &Decoded,
    format: ImageFormat,
    resolved: &ResolvedOptions,
    options: &CompressOptions,
    target: u64,
) -> Result<Option<InnerOutput>> {
    if let Some((bytes, quality)) = search_quality(
        &decoded.image,
        format,
        options,
        resolved,
        target,
        decoded.icc.as_deref(),
        decoded.exif.as_deref(),
    )? {
        return Ok(Some(InnerOutput {
            bytes,
            format,
            width: decoded.image.width(),
            height: decoded.image.height(),
            quality_used: quality,
            resized: decoded.resized,
        }));
    }
    let (ow, oh) = (decoded.image.width(), decoded.image.height());
    for pct in (1..=9).rev() {
        let scale = pct as f32 / 10.0;
        let w = ((ow as f32 * scale).round() as u32).max(1);
        let h = ((oh as f32 * scale).round() as u32).max(1);
        if w == ow && h == oh {
            continue;
        }
        let small = decoded.image.resize_exact(w, h, FilterType::Lanczos3);
        if let Some((bytes, quality)) = search_quality(
            &small,
            format,
            options,
            resolved,
            target,
            decoded.icc.as_deref(),
            decoded.exif.as_deref(),
        )? {
            return Ok(Some(InnerOutput {
                bytes,
                format,
                width: w,
                height: h,
                quality_used: quality,
                resized: true,
            }));
        }
    }
    Ok(None)
}

/// Binary search for the highest quality (1-100) whose output is at most
/// `target` bytes. At most 10 encodes. `None` when even quality 1 is too
/// big. Output size is not strictly monotonic in quality for every format
/// (PNG levels are stepwise, GIF/BMP ignore quality), but the search still
/// terminates and returns the best found.
fn search_quality(
    img: &DynamicImage,
    format: ImageFormat,
    options: &CompressOptions,
    resolved: &ResolvedOptions,
    target: u64,
    icc: Option<&[u8]>,
    exif: Option<&[u8]>,
) -> Result<Option<(Vec<u8>, u8)>> {
    let mut lo: i32 = 1;
    let mut hi: i32 = 100;
    let mut best: Option<(Vec<u8>, u8)> = None;
    for _ in 0..10 {
        if lo > hi {
            break;
        }
        let mid = (lo + hi) / 2;
        // PNG level follows quality during the search (0-9 mapped from 1-100).
        let png_level = (((mid - 1) * 9) / 99) as u8;
        let out = encode_at(
            img, format, options, resolved, mid as u8, png_level, icc, exif,
        )?;
        if (out.len() as u64) <= target {
            best = Some((out, mid as u8));
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    Ok(best)
}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// Encode `img` to `format` at `quality`. `png_level` is the PNG
/// compression level 0-9 (the caller maps quality -> level during a
/// target-size search).
#[allow(clippy::too_many_arguments)]
fn encode_at(
    img: &DynamicImage,
    format: ImageFormat,
    options: &CompressOptions,
    resolved: &ResolvedOptions,
    quality: u8,
    png_level: u8,
    icc: Option<&[u8]>,
    exif: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut out = match format {
        ImageFormat::Jpeg => encode_jpeg(img, options, quality, icc, exif)?,
        ImageFormat::Png => {
            if options.palette {
                encode_png_palette(img, quality, png_level)?
            } else {
                encode_png(img, options, png_level, icc, exif)?
            }
        }
        ImageFormat::WebP => encode_webp(img, options, resolved, quality)?,
        ImageFormat::Avif => encode_avif(img, options, resolved, quality)?,
        ImageFormat::Gif => encode_gif(img)?,
        ImageFormat::Bmp => encode_bmp(img)?,
        ImageFormat::Tiff => {
            return Err(DusimagezError::invalid_argument(
                "TIFF is decode-only and cannot be an output format",
            ));
        }
    };
    if format == ImageFormat::Png
        && let Some(dpi) = options.dpi
    {
        out = insert_png_phys(out, dpi);
    }
    Ok(out)
}

fn encode_jpeg(
    img: &DynamicImage,
    options: &CompressOptions,
    quality: u8,
    icc: Option<&[u8]>,
    exif: Option<&[u8]>,
) -> Result<Vec<u8>> {
    // JPEG has no alpha: flatten onto white first.
    let rgb = flatten_to_rgb8(img);
    let mut cursor = Cursor::new(Vec::new());
    let mut encoder = JpegEncoder::new_with_quality(&mut cursor, quality);
    if let Some((x, y)) = options.dpi {
        encoder.set_pixel_density(PixelDensity {
            density: (x.min(u16::MAX as u32) as u16, y.min(u16::MAX as u32) as u16),
            unit: PixelDensityUnit::Inches,
        });
    }
    if let Some(profile) = icc {
        let _ = encoder.set_icc_profile(profile.to_vec());
    }
    if let Some(data) = exif {
        let _ = encoder.set_exif_metadata(data.to_vec());
    }
    DynamicImage::ImageRgb8(rgb)
        .write_with_encoder(encoder)
        .map_err(|e| DusimagezError::invalid_argument(&format!("JPEG encoding failed ({e})")))?;
    Ok(cursor.into_inner())
}

fn png_compression(level: u8) -> CompressionType {
    match level {
        0..=2 => CompressionType::Fast,
        3..=6 => CompressionType::Default,
        _ => CompressionType::Best,
    }
}

fn encode_png(
    img: &DynamicImage,
    options: &CompressOptions,
    png_level: u8,
    icc: Option<&[u8]>,
    exif: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let _ = options; // (kept for symmetry; PNG ignores progressive in v0.1.0)
    let mut cursor = Cursor::new(Vec::new());
    let mut encoder = PngEncoder::new_with_quality(
        &mut cursor,
        png_compression(png_level),
        PngFilterType::Adaptive,
    );
    if let Some(profile) = icc {
        let _ = encoder.set_icc_profile(profile.to_vec());
    }
    if let Some(data) = exif {
        let _ = encoder.set_exif_metadata(data.to_vec());
    }
    img.write_with_encoder(encoder)
        .map_err(|e| DusimagezError::invalid_argument(&format!("PNG encoding failed ({e})")))?;
    Ok(cursor.into_inner())
}

/// PNG with a 256-colour palette via libimagequant, written with the `png`
/// crate (the `image` PngEncoder has no indexed-colour path). Alpha is
/// preserved through a tRNS chunk when the palette uses transparency.
fn encode_png_palette(img: &DynamicImage, quality: u8, png_level: u8) -> Result<Vec<u8>> {
    let rgba = img.to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let pixels: Vec<imagequant::RGBA> = rgba
        .as_raw()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| imagequant::RGBA {
            r: c[0],
            g: c[1],
            b: c[2],
            a: c[3],
        })
        .collect();
    let mut attr = imagequant::Attributes::new();
    attr.set_quality(0, quality)
        .map_err(|e| DusimagezError::invalid_argument(&format!("palette quantise failed ({e})")))?;
    attr.set_max_colors(256)
        .map_err(|e| DusimagezError::invalid_argument(&format!("palette quantise failed ({e})")))?;
    let mut iq_image = attr
        .new_image(pixels, w, h, 0.0)
        .map_err(|e| DusimagezError::invalid_argument(&format!("palette quantise failed ({e})")))?;
    let mut result = attr
        .quantize(&mut iq_image)
        .map_err(|e| DusimagezError::invalid_argument(&format!("palette quantise failed ({e})")))?;
    let (palette, indices) = result
        .remapped(&mut iq_image)
        .map_err(|e| DusimagezError::invalid_argument(&format!("palette quantise failed ({e})")))?;

    let mut palette_bytes = Vec::with_capacity(palette.len() * 3);
    let mut trns: Vec<u8> = palette.iter().map(|p| p.a).collect();
    for p in &palette {
        palette_bytes.extend_from_slice(&[p.r, p.g, p.b]);
    }
    // tRNS must not have trailing fully-opaque entries.
    while trns.last() == Some(&255) {
        trns.pop();
    }

    let mut cursor = Cursor::new(Vec::new());
    let mut encoder = png::Encoder::new(&mut cursor, w as u32, h as u32);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(match png_level {
        0..=2 => png::Compression::Fast,
        3..=6 => png::Compression::Balanced,
        _ => png::Compression::High,
    });
    encoder.set_filter(png::Filter::Adaptive);
    encoder.set_palette(palette_bytes);
    if !trns.is_empty() {
        encoder.set_trns(trns);
    }
    let mut writer = encoder
        .write_header()
        .map_err(|e| DusimagezError::invalid_argument(&format!("PNG encoding failed ({e})")))?;
    writer
        .write_image_data(&indices)
        .map_err(|e| DusimagezError::invalid_argument(&format!("PNG encoding failed ({e})")))?;
    drop(writer);
    Ok(cursor.into_inner())
}

fn encode_webp(
    img: &DynamicImage,
    options: &CompressOptions,
    resolved: &ResolvedOptions,
    quality: u8,
) -> Result<Vec<u8>> {
    let rgba = img.to_rgba8();
    let encoder = webp::Encoder::from_rgba(rgba.as_raw(), img.width(), img.height());
    if options.lossless {
        return Ok(encoder.encode_lossless().to_vec());
    }
    let mut config = webp::WebPConfig::new()
        .map_err(|e| DusimagezError::invalid_argument(&format!("WebP encoding failed ({e:?})")))?;
    config.lossless = 0;
    config.quality = quality as f32;
    // libwebp `method` (effort) is 0-6; the option range is 0-10 for AVIF,
    // so clamp for WebP.
    config.method = resolved.effort.min(6) as i32;
    let mem = encoder
        .encode_advanced(&config)
        .map_err(|e| DusimagezError::invalid_argument(&format!("WebP encoding failed ({e:?})")))?;
    Ok(mem.to_vec())
}

fn encode_avif(
    img: &DynamicImage,
    options: &CompressOptions,
    resolved: &ResolvedOptions,
    quality: u8,
) -> Result<Vec<u8>> {
    let rgba = img.to_rgba8();
    let pixels: Vec<rgb::RGBA<u8>> = rgba
        .as_raw()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| rgb::RGBA::new(c[0], c[1], c[2], c[3]))
        .collect();
    let img_vec = imgref::Img::new(pixels, img.width() as usize, img.height() as usize);
    // ravif has no true lossless mode: lossless maps to quality 100.
    let q = if options.lossless {
        100.0
    } else {
        quality as f32
    };
    // ravif speed asserts 1..=10; the option allows 0, which clamps to 1.
    let speed = resolved.effort.clamp(1, 10);
    let encoder = ravif::Encoder::new().with_quality(q).with_speed(speed);
    let encoded = encoder
        .encode_rgba(img_vec.as_ref())
        .map_err(|e| DusimagezError::invalid_argument(&format!("AVIF encoding failed ({e})")))?;
    Ok(encoded.avif_file)
}

fn encode_gif(img: &DynamicImage) -> Result<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    let encoder = GifEncoder::new(&mut cursor);
    img.write_with_encoder(encoder)
        .map_err(|e| DusimagezError::invalid_argument(&format!("GIF encoding failed ({e})")))?;
    Ok(cursor.into_inner())
}

fn encode_bmp(img: &DynamicImage) -> Result<Vec<u8>> {
    let rgb = flatten_to_rgb8(img);
    let mut cursor = Cursor::new(Vec::new());
    let encoder = BmpEncoder::new(&mut cursor);
    DynamicImage::ImageRgb8(rgb)
        .write_with_encoder(encoder)
        .map_err(|e| DusimagezError::invalid_argument(&format!("BMP encoding failed ({e})")))?;
    Ok(cursor.into_inner())
}

/// Flatten onto a white background (for formats without alpha).
fn flatten_to_rgb8(img: &DynamicImage) -> image::RgbImage {
    let rgba = img.to_rgba8();
    let mut out = image::RgbImage::new(img.width(), img.height());
    for (x, y, px) in rgba.enumerate_pixels() {
        let a = px[3] as u32;
        let blend = |c: u8| -> u8 { ((c as u32 * a + 255 * (255 - a)) / 255) as u8 };
        out.put_pixel(x, y, image::Rgb([blend(px[0]), blend(px[1]), blend(px[2])]));
    }
    out
}

// ---------------------------------------------------------------------------
// PNG DPI (pHYs chunk)
// ---------------------------------------------------------------------------

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Insert a pHYs chunk (pixels per metre, unit = metre) right after IHDR.
/// Returns the input unchanged when it is not a well-formed PNG.
fn insert_png_phys(mut bytes: Vec<u8>, dpi: (u32, u32)) -> Vec<u8> {
    if bytes.len() < 8 + 25 || bytes[..8] != *PNG_SIGNATURE || &bytes[12..16] != b"IHDR" {
        return bytes;
    }
    let ihdr_len = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let ihdr_end = 8 + 4 + 4 + ihdr_len + 4;
    if bytes.len() < ihdr_end {
        return bytes;
    }
    // DPI -> pixels per metre (1 inch = 0.0254 m).
    let ppm = |d: u32| -> u32 { ((d as u64 * 10_000 + 127) / 254) as u32 };
    let mut data = Vec::with_capacity(9);
    data.extend_from_slice(&ppm(dpi.0).to_be_bytes());
    data.extend_from_slice(&ppm(dpi.1).to_be_bytes());
    data.push(1); // unit: metre
    let mut chunk = Vec::with_capacity(12 + 9);
    chunk.extend_from_slice(&9u32.to_be_bytes());
    chunk.extend_from_slice(b"pHYs");
    chunk.extend_from_slice(&data);
    let mut crc_input = Vec::with_capacity(4 + 9);
    crc_input.extend_from_slice(b"pHYs");
    crc_input.extend_from_slice(&data);
    chunk.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    bytes.splice(ihdr_end..ihdr_end, chunk);
    bytes
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use image::{GenericImageView, ImageBuffer, Rgb};

    fn noise_png(width: u32, height: u32) -> Vec<u8> {
        let mut state: u32 = 0x9E37_79B9;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |_, _| {
            let v = next();
            Rgb([(v >> 16) as u8, (v >> 8) as u8, v as u8])
        });
        let mut cursor = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(img)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    #[test]
    fn crc32_matches_known_vector() {
        // CRC-32 of "123456789" is 0xCBF43926.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn png_phys_chunk_round_trips() {
        let png = noise_png(8, 8);
        let with_dpi = insert_png_phys(png, (300, 300));
        // pHYs chunk sits right after the 33-byte IHDR chunk (8 sig + 25):
        // length at 33..37, type "pHYs" at 37..41, x ppm at 41..45.
        assert_eq!(&with_dpi[37..41], b"pHYs");
        let ppm = u32::from_be_bytes(with_dpi[41..45].try_into().unwrap());
        assert!(
            (11_800..=11_820).contains(&ppm),
            "300 dpi ~= 11811 ppm, got {ppm}"
        );
        // The result still decodes.
        let decoded = image::load_from_memory(&with_dpi).unwrap();
        assert_eq!(decoded.dimensions(), (8, 8));
    }

    #[test]
    fn subsampling_is_recorded_in_resolved_options() {
        let mut o = CompressOptions::default();
        o.quality = 95;
        assert_eq!(o.resolved().subsampling, crate::types::Subsampling::S444);
        let _ = o;
    }
}
