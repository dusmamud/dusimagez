//! `dusimagez` CLI: `compress`, `info`, `mock` (+ `--version`, `--help`).
//! Flags mirror `CompressOptions` 1:1.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use dusimagez::{
    CompressOptions, ImageFormat, Preset, StripMetadata, Subsampling, compress_with_report,
    get_image_info,
};

#[derive(Debug, Parser)]
#[command(
    name = "dusimagez",
    version,
    about = "Offline image compressor — reduce file size with maximum control."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Compress an image.
    Compress {
        /// Input image file.
        input: PathBuf,
        /// Output image file (format follows the extension unless --format is given).
        output: PathBuf,
        /// Quality 1-100 (JPEG/WebP/AVIF). Default 80.
        #[arg(long)]
        quality: Option<u8>,
        /// Output format: jpeg, png, webp, avif, gif, bmp. If omitted, the
        /// output file's extension decides; unknown extension keeps the
        /// input format.
        #[arg(long, value_enum)]
        format: Option<FormatArg>,
        /// Compress to at most this many bytes.
        #[arg(long)]
        target_bytes: Option<u64>,
        /// Progressive JPEG (accepted; the v0.1.0 encoder writes baseline JPEG).
        #[arg(long)]
        progressive: bool,
        /// Lossless output (WebP/PNG only; AVIF maps to quality 100).
        #[arg(long)]
        lossless: bool,
        /// Effort: WebP method 0-6, AVIF speed 0-10. Default 6.
        #[arg(long)]
        effort: Option<u8>,
        /// JPEG chroma subsampling: 444, 422 or 420.
        #[arg(long)]
        subsampling: Option<SubsamplingArg>,
        /// PNG compression level 0-9. Default 6.
        #[arg(long)]
        png_level: Option<u8>,
        /// PNG: quantise to a 256-colour palette.
        #[arg(long)]
        palette: bool,
        /// Preset: web, social, print, archive.
        #[arg(long, value_enum)]
        preset: Option<PresetArg>,
        /// Downscale-only max width (aspect preserved, never upscale).
        #[arg(long)]
        max_width: Option<u32>,
        /// Downscale-only max height (aspect preserved, never upscale).
        #[arg(long)]
        max_height: Option<u32>,
        /// Metadata: all (strip), keep-exif, keep-icc, keep-all.
        #[arg(long, value_enum, default_value = "all")]
        strip: StripArg,
        /// Do not apply EXIF orientation on load.
        #[arg(long)]
        no_auto_orient: bool,
        /// Output DPI as WxH, e.g. 300x300 (JPEG/PNG only).
        #[arg(long, value_parser = parse_dpi)]
        dpi: Option<(u32, u32)>,
        /// Refuse images with more pixels than this. Default 120000000.
        #[arg(long)]
        max_pixels: Option<u64>,
        /// Refuse images with either side longer than this. Default 16000.
        #[arg(long)]
        max_dimension: Option<u32>,
    },
    /// Print format, dimensions and size of an image.
    Info {
        /// Input image file.
        input: PathBuf,
    },
    /// Generate a deterministic test image (fixed seed).
    Mock {
        /// Kind of test image: solid, gradient or noise.
        #[arg(long, value_enum)]
        kind: MockKind,
        /// Width in pixels.
        #[arg(long)]
        width: u32,
        /// Height in pixels.
        #[arg(long)]
        height: u32,
        /// Output file (format follows the extension, default PNG).
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Jpeg,
    Png,
    Webp,
    Avif,
    Gif,
    Bmp,
}

impl FormatArg {
    fn as_format(self) -> ImageFormat {
        match self {
            Self::Jpeg => ImageFormat::Jpeg,
            Self::Png => ImageFormat::Png,
            Self::Webp => ImageFormat::WebP,
            Self::Avif => ImageFormat::Avif,
            Self::Gif => ImageFormat::Gif,
            Self::Bmp => ImageFormat::Bmp,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SubsamplingArg {
    #[value(name = "444")]
    S444,
    #[value(name = "422")]
    S422,
    #[value(name = "420")]
    S420,
}

impl SubsamplingArg {
    fn as_subsampling(self) -> Subsampling {
        match self {
            Self::S444 => Subsampling::S444,
            Self::S422 => Subsampling::S422,
            Self::S420 => Subsampling::S420,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PresetArg {
    Web,
    Social,
    Print,
    Archive,
}

impl PresetArg {
    fn as_preset(self) -> Preset {
        match self {
            Self::Web => Preset::Web,
            Self::Social => Preset::Social,
            Self::Print => Preset::Print,
            Self::Archive => Preset::Archive,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum StripArg {
    All,
    KeepExif,
    KeepIcc,
    KeepAll,
}

impl StripArg {
    fn as_strip(self) -> StripMetadata {
        match self {
            Self::All => StripMetadata::All,
            Self::KeepExif => StripMetadata::KeepExif,
            Self::KeepIcc => StripMetadata::KeepIcc,
            Self::KeepAll => StripMetadata::KeepAll,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum MockKind {
    Solid,
    Gradient,
    Noise,
}

fn parse_dpi(value: &str) -> Result<(u32, u32), String> {
    let (w, h) = value
        .split_once(['x', 'X'])
        .ok_or_else(|| "dpi must be WxH, e.g. 300x300".to_string())?;
    let w: u32 = w
        .parse()
        .map_err(|_| "dpi width must be a number".to_string())?;
    let h: u32 = h
        .parse()
        .map_err(|_| "dpi height must be a number".to_string())?;
    if w == 0 || h == 0 {
        return Err("dpi values must be > 0".to_string());
    }
    Ok((w, h))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<(), String> {
    match command {
        Command::Compress {
            input,
            output,
            quality,
            format,
            target_bytes,
            progressive,
            lossless,
            effort,
            subsampling,
            png_level,
            palette,
            preset,
            max_width,
            max_height,
            strip,
            no_auto_orient,
            dpi,
            max_pixels,
            max_dimension,
        } => {
            let bytes = std::fs::read(&input)
                .map_err(|e| format!("cannot read {}: {e}", input.display()))?;
            // --format wins; otherwise the output file's extension decides;
            // an unknown/missing extension keeps the input format.
            let inferred_format = output
                .extension()
                .and_then(|e| e.to_str())
                .and_then(ImageFormat::from_name)
                .filter(|f| f.is_output_format());
            let options = CompressOptions {
                quality: quality.unwrap_or(80),
                format: format.map(FormatArg::as_format).or(inferred_format),
                target_bytes,
                progressive,
                lossless,
                effort: effort.unwrap_or(6),
                subsampling: subsampling
                    .map(SubsamplingArg::as_subsampling)
                    .unwrap_or(Subsampling::S420),
                png_compression_level: png_level.unwrap_or(6),
                palette,
                preset: preset.map(PresetArg::as_preset),
                max_width,
                max_height,
                strip_metadata: strip.as_strip(),
                auto_orient: !no_auto_orient,
                dpi,
                max_pixels: max_pixels.unwrap_or(120_000_000),
                max_dimension: max_dimension.unwrap_or(16000),
            };

            let report = compress_with_report(&bytes, &options)
                .map_err(|e| format!("[{}] {}", e.code(), e.message_en()))?;
            std::fs::write(&output, &report.output)
                .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
            println!(
                "{} ({} bytes) -> {} ({} bytes)",
                input.display(),
                report.original_size,
                output.display(),
                report.output_size
            );
            println!(
                "saved {} bytes ({:.1}%) · {} {}x{} · quality {}",
                report.saved_bytes,
                report.saved_percent,
                report.format,
                report.width,
                report.height,
                report.quality_used,
            );
            if report.resized {
                println!("resized: yes");
            }
            Ok(())
        }
        Command::Info { input } => {
            let bytes = std::fs::read(&input)
                .map_err(|e| format!("cannot read {}: {e}", input.display()))?;
            let info =
                get_image_info(&bytes).map_err(|e| format!("[{}] {}", e.code(), e.message_en()))?;
            println!("format: {}", info.format);
            println!("dimensions: {}x{}", info.width, info.height);
            println!("has_alpha: {}", info.has_alpha);
            println!("colour_type: {}", info.colour_type);
            println!("size: {} bytes", info.file_size);
            Ok(())
        }
        Command::Mock {
            kind,
            width,
            height,
            out,
        } => {
            if width == 0 || height == 0 {
                return Err("width and height must be > 0".to_string());
            }
            let format = out
                .extension()
                .and_then(|e| e.to_str())
                .and_then(ImageFormat::from_name)
                .unwrap_or(ImageFormat::Png);
            let image = generate_mock(kind, width, height);
            let bytes = if format == ImageFormat::Avif {
                // The `image` crate build here has no AVIF encoder; go
                // through the core (ravif) instead.
                let mut png_cursor = std::io::Cursor::new(Vec::new());
                image
                    .write_to(&mut png_cursor, image::ImageFormat::Png)
                    .map_err(|e| format!("cannot encode mock image: {e}"))?;
                let avif_options = CompressOptions {
                    format: Some(ImageFormat::Avif),
                    ..CompressOptions::default()
                };
                dusimagez::compress_image(&png_cursor.into_inner(), &avif_options)
                    .map_err(|e| format!("[{}] {}", e.code(), e.message_en()))?
            } else {
                let mut cursor = std::io::Cursor::new(Vec::new());
                image
                    .write_to(&mut cursor, image_format(format))
                    .map_err(|e| format!("cannot encode mock image: {e}"))?;
                cursor.into_inner()
            };
            std::fs::write(&out, bytes)
                .map_err(|e| format!("cannot write {}: {e}", out.display()))?;
            println!(
                "wrote {} ({}x{}, {:?}, {})",
                out.display(),
                width,
                height,
                kind,
                format
            );
            Ok(())
        }
    }
}

fn image_format(format: ImageFormat) -> image::ImageFormat {
    match format {
        ImageFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::WebP => image::ImageFormat::WebP,
        ImageFormat::Avif => image::ImageFormat::Avif,
        ImageFormat::Gif => image::ImageFormat::Gif,
        ImageFormat::Bmp => image::ImageFormat::Bmp,
        ImageFormat::Tiff => image::ImageFormat::Tiff,
    }
}

/// Deterministic mock image. Noise uses a fixed-seed xorshift32, so the
/// same arguments always produce the same bytes.
fn generate_mock(kind: MockKind, width: u32, height: u32) -> image::DynamicImage {
    use image::{ImageBuffer, Rgb};
    match kind {
        MockKind::Solid => {
            let img = ImageBuffer::from_pixel(width, height, Rgb([255u8, 0, 0]));
            image::DynamicImage::ImageRgb8(img)
        }
        MockKind::Gradient => {
            let img = ImageBuffer::from_fn(width, height, |x, y| {
                let r = if width > 1 {
                    (x * 255 / (width - 1)) as u8
                } else {
                    0
                };
                let g = if height > 1 {
                    (y * 255 / (height - 1)) as u8
                } else {
                    0
                };
                Rgb([r, g, 128])
            });
            image::DynamicImage::ImageRgb8(img)
        }
        MockKind::Noise => {
            let mut state: u32 = 0x9E37_79B9;
            let img = ImageBuffer::from_fn(width, height, |_, _| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                Rgb([(state >> 16) as u8, (state >> 8) as u8, state as u8])
            });
            image::DynamicImage::ImageRgb8(img)
        }
    }
}
