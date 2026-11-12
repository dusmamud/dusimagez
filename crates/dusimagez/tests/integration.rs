//! Integration tests: round-trips, target-size guarantee, report maths,
//! error codes, presets, downscale-only, bulk partial failure.
//! Fixtures are built in memory (no binary blobs in the repo).

use std::io::Cursor;

use dusimagez::{
    CompressOptions, ImageFormat, Preset, StripMetadata, bulk_compress, compress_image,
    compress_to_size, compress_with_report, get_image_info,
};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, Rgba};

fn xorshift(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

/// Photo-like noise: hard to compress, so target-size tests are meaningful.
fn noise_png(width: u32, height: u32) -> Vec<u8> {
    let mut state: u32 = 0x9E37_79B9;
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |_, _| {
        let v = xorshift(&mut state);
        Rgb([(v >> 16) as u8, (v >> 8) as u8, v as u8])
    });
    encode_png(&DynamicImage::ImageRgb8(img))
}

fn solid_png(width: u32, height: u32) -> Vec<u8> {
    let img = ImageBuffer::from_pixel(width, height, Rgb([37u8, 99, 235]));
    encode_png(&DynamicImage::ImageRgb8(img))
}

fn gradient_rgba_png(width: u32, height: u32) -> Vec<u8> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
        Rgba([
            (x * 255 / width.max(1)) as u8,
            (y * 255 / height.max(1)) as u8,
            128,
            200,
        ])
    });
    encode_png(&DynamicImage::ImageRgba8(img))
}

fn encode_png(img: &DynamicImage) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    cursor.into_inner()
}

fn decode(bytes: &[u8]) -> DynamicImage {
    image::load_from_memory(bytes).expect("output must decode")
}

fn opts() -> CompressOptions {
    CompressOptions::default()
}

#[test]
fn round_trip_jpeg_dimensions_preserved() {
    let input = noise_png(320, 240);
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.quality = 80;
    let out = compress_image(&input, &o).unwrap();
    assert_eq!(&out[..3], &[0xFF, 0xD8, 0xFF], "must be a JPEG");
    assert_eq!(decode(&out).dimensions(), (320, 240));
}

#[test]
fn round_trip_png_dimensions_preserved() {
    let input = noise_png(200, 150);
    let mut o = opts();
    o.format = Some(ImageFormat::Png);
    let out = compress_image(&input, &o).unwrap();
    assert_eq!(&out[1..4], b"PNG");
    assert_eq!(decode(&out).dimensions(), (200, 150));
}

#[test]
fn round_trip_webp_lossy_and_lossless() {
    let input = gradient_rgba_png(160, 120);
    let mut o = opts();
    o.format = Some(ImageFormat::WebP);
    o.quality = 75;
    let lossy = compress_image(&input, &o).unwrap();
    assert_eq!(decode(&lossy).dimensions(), (160, 120));

    o.lossless = true;
    let lossless = compress_image(&input, &o).unwrap();
    assert_eq!(decode(&lossless).dimensions(), (160, 120));
    // Lossless WebP of a smooth gradient should not be larger than the
    // lossy one by an absurd margin; just assert it decodes and is WebP.
    assert_eq!(&lossless[8..12], b"WEBP");
}

#[test]
fn round_trip_gif_and_bmp() {
    let input = solid_png(64, 48);
    for format in [ImageFormat::Gif, ImageFormat::Bmp] {
        let mut o = opts();
        o.format = Some(format);
        let out = compress_image(&input, &o).unwrap();
        assert_eq!(
            decode(&out).dimensions(),
            (64, 48),
            "{format:?} must decode"
        );
    }
}

#[test]
fn avif_output_has_avif_box() {
    let input = gradient_rgba_png(64, 48);
    let mut o = opts();
    o.format = Some(ImageFormat::Avif);
    o.quality = 70;
    let out = compress_image(&input, &o).unwrap();
    // ftyp box at offset 4, brand "avif" at offset 8.
    assert_eq!(&out[4..8], b"ftyp");
    assert_eq!(&out[8..12], b"avif");
    // AVIF input is not decodable in this build (no dav1d): the same bytes
    // fed back must report UNSUPPORTED_FORMAT, not INVALID_IMAGE.
    let err = get_image_info(&out).unwrap_err();
    assert_eq!(err.code(), "UNSUPPORTED_FORMAT");
}

#[test]
fn tiff_input_decodes_and_defaults_to_png() {
    let img = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(80, 60, Rgb([9u8, 200, 30])));
    let mut cursor = Cursor::new(Vec::new());
    {
        let encoder = image::codecs::tiff::TiffEncoder::new(&mut cursor);
        img.write_with_encoder(encoder).unwrap();
    }
    let tiff = cursor.into_inner();
    let out = compress_image(&tiff, &opts()).unwrap();
    assert_eq!(&out[1..4], b"PNG", "TIFF input defaults to PNG output");
    assert_eq!(decode(&out).dimensions(), (80, 60));
}

#[test]
fn target_size_guarantee() {
    // JPEG: quality search actually shrinks output at full size.
    let input = noise_png(400, 300);
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    let out = compress_to_size(&input, 30_000, &o).unwrap();
    assert!(out.len() as u64 <= 30_000, "output {} > 30000", out.len());
    assert_eq!(decode(&out).dimensions(), (400, 300));
}

#[test]
fn target_size_png_downscales_to_fit() {
    // PNG is lossless: quality only moves the DEFLATE level, so a 400x300
    // noise PNG cannot reach 30_000 bytes at full size — the search must
    // downscale instead of failing.
    let input = noise_png(400, 300);
    let mut o = opts();
    o.format = Some(ImageFormat::Png);
    let report = compress_with_report(&input, &{
        o.target_bytes = Some(30_000);
        o
    })
    .unwrap();
    assert!(report.output_size <= 30_000);
    assert!(report.resized);
    assert!(report.width < 400 && report.height < 300);
}

#[test]
fn target_size_via_options_matches() {
    let input = noise_png(400, 300);
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.target_bytes = Some(25_000);
    let out = compress_image(&input, &o).unwrap();
    assert!(out.len() as u64 <= 25_000);
}

#[test]
fn target_size_unreachable_errors() {
    let input = noise_png(400, 300);
    let err = compress_to_size(&input, 100, &opts()).unwrap_err();
    assert_eq!(err.code(), "TARGET_SIZE_UNREACHABLE");
    assert!(!err.message_hi().is_empty());
}

#[test]
fn target_size_forces_downscale() {
    let input = noise_png(800, 600);
    let report = compress_with_report(&input, &{
        let mut o = opts();
        o.format = Some(ImageFormat::Jpeg);
        o.target_bytes = Some(3_000);
        o
    })
    .unwrap();
    assert!(report.output_size <= 3_000);
    assert!(
        report.resized,
        "reaching 3000 bytes must downscale 800x600 noise"
    );
    assert!(report.width < 800 && report.height < 600);
}

#[test]
fn report_maths_are_consistent() {
    let input = noise_png(320, 240);
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.quality = 50;
    let report = compress_with_report(&input, &o).unwrap();
    assert_eq!(report.original_size, input.len() as u64);
    assert_eq!(report.output_size, report.output.len() as u64);
    assert_eq!(
        report.saved_bytes,
        report.original_size.saturating_sub(report.output_size)
    );
    let expected = report.saved_bytes as f32 / report.original_size as f32 * 100.0;
    assert!((report.saved_percent - expected).abs() < 0.01);
    assert_eq!(report.format, ImageFormat::Jpeg);
    assert_eq!((report.width, report.height), (320, 240));
    assert_eq!(report.quality_used, 50);
    assert!(!report.resized);
}

#[test]
fn error_codes() {
    assert_eq!(
        compress_image(&[], &opts()).unwrap_err().code(),
        "EMPTY_INPUT"
    );
    assert_eq!(
        compress_image(b"this is not an image at all", &opts())
            .unwrap_err()
            .code(),
        "INVALID_IMAGE"
    );

    let input = solid_png(32, 32);
    let mut o = opts();
    o.quality = 0;
    assert_eq!(
        compress_image(&input, &o).unwrap_err().code(),
        "INVALID_QUALITY"
    );
    o.quality = 101;
    assert_eq!(
        compress_image(&input, &o).unwrap_err().code(),
        "INVALID_QUALITY"
    );

    let mut o = opts();
    o.effort = 11;
    assert_eq!(
        compress_image(&input, &o).unwrap_err().code(),
        "INVALID_ARGUMENT"
    );
    let mut o = opts();
    o.png_compression_level = 10;
    assert_eq!(
        compress_image(&input, &o).unwrap_err().code(),
        "INVALID_ARGUMENT"
    );
    let mut o = opts();
    o.max_width = Some(0);
    assert_eq!(
        compress_image(&input, &o).unwrap_err().code(),
        "INVALID_DIMENSIONS"
    );
    let mut o = opts();
    o.target_bytes = Some(0);
    assert_eq!(
        compress_image(&input, &o).unwrap_err().code(),
        "INVALID_ARGUMENT"
    );
}

#[test]
fn error_messages_have_en_and_hi() {
    let err = compress_image(&[], &opts()).unwrap_err();
    assert_eq!(err.code(), "EMPTY_INPUT");
    assert!(err.message_en().contains("empty"));
    assert!(
        err.message_hi()
            .chars()
            .any(|c| ('\u{0900}'..='\u{097F}').contains(&c))
    );
}

#[test]
fn image_too_large_is_refused() {
    let input = noise_png(200, 200);
    let mut o = opts();
    o.max_pixels = 10_000; // 200x200 = 40000 pixels
    assert_eq!(
        compress_image(&input, &o).unwrap_err().code(),
        "IMAGE_TOO_LARGE"
    );
    let mut o = opts();
    o.max_dimension = 100;
    assert_eq!(
        compress_image(&input, &o).unwrap_err().code(),
        "IMAGE_TOO_LARGE"
    );
}

#[test]
fn preset_behaviour() {
    let input = noise_png(200, 150);
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.preset = Some(Preset::Print);
    let report = compress_with_report(&input, &o).unwrap();
    assert_eq!(report.quality_used, 95, "Print preset quality is 95");

    // Explicit quality overrides the preset.
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.preset = Some(Preset::Print);
    o.quality = 70;
    let report = compress_with_report(&input, &o).unwrap();
    assert_eq!(report.quality_used, 70);

    // Web preset keeps the default quality 80.
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.preset = Some(Preset::Web);
    let report = compress_with_report(&input, &o).unwrap();
    assert_eq!(report.quality_used, 80);
}

#[test]
fn downscale_only_never_upscales() {
    let input = noise_png(400, 200);
    // Smaller cap: downscales, aspect preserved, resized = true.
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.max_width = Some(100);
    let report = compress_with_report(&input, &o).unwrap();
    assert_eq!((report.width, report.height), (100, 50));
    assert!(report.resized);

    // Larger cap: no upscale, resized = false.
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.max_width = Some(4000);
    o.max_height = Some(4000);
    let report = compress_with_report(&input, &o).unwrap();
    assert_eq!((report.width, report.height), (400, 200));
    assert!(!report.resized);

    // max_height alone constrains the long side.
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.max_height = Some(100);
    let report = compress_with_report(&input, &o).unwrap();
    assert_eq!((report.width, report.height), (200, 100));
}

#[test]
fn bulk_partial_failure_keeps_order() {
    let good = noise_png(64, 48);
    let inputs = vec![good.clone(), b"garbage bytes".to_vec(), good.clone()];
    let results = bulk_compress(&inputs, &opts());
    assert_eq!(results.len(), 3);
    assert!(results[0].is_ok());
    assert_eq!(results[1].as_ref().unwrap_err().code(), "INVALID_IMAGE");
    assert!(results[2].is_ok());
}

#[test]
fn palette_png_decodes() {
    let input = gradient_rgba_png(120, 90);
    let mut o = opts();
    o.format = Some(ImageFormat::Png);
    o.palette = true;
    let out = compress_image(&input, &o).unwrap();
    assert_eq!(&out[1..4], b"PNG");
    assert_eq!(decode(&out).dimensions(), (120, 90));
    // IHDR colour type byte (offset 25) must be 3 = indexed colour:
    // this proves the output really is a palette PNG, not RGBA.
    assert_eq!(out[25], 3, "palette PNG must use indexed colour type");
}

#[test]
fn get_image_info_reports_fields() {
    let input = gradient_rgba_png(120, 90);
    let info = get_image_info(&input).unwrap();
    assert_eq!(info.format, ImageFormat::Png);
    assert_eq!((info.width, info.height), (120, 90));
    assert!(info.has_alpha);
    assert_eq!(info.colour_type, "rgba8");
    assert_eq!(info.file_size, input.len() as u64);

    let jpeg_input = {
        let mut o = opts();
        o.format = Some(ImageFormat::Jpeg);
        compress_image(&input, &o).unwrap()
    };
    let info = get_image_info(&jpeg_input).unwrap();
    assert_eq!(info.format, ImageFormat::Jpeg);
    assert!(!info.has_alpha);
    assert_eq!(info.colour_type, "rgb8");
}

#[test]
fn dpi_is_written_for_jpeg_and_png() {
    let input = noise_png(64, 48);
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.dpi = Some((300, 300));
    let jpeg = compress_image(&input, &o).unwrap();
    assert_eq!(decode(&jpeg).dimensions(), (64, 48));

    let mut o = opts();
    o.format = Some(ImageFormat::Png);
    o.dpi = Some((300, 300));
    let png = compress_image(&input, &o).unwrap();
    // pHYs chunk right after the 33-byte IHDR chunk: type at 37..41.
    assert_eq!(&png[37..41], b"pHYs");
    assert_eq!(decode(&png).dimensions(), (64, 48));
}

#[test]
fn keep_metadata_options_are_accepted() {
    let input = noise_png(64, 48);
    for strip in [
        StripMetadata::All,
        StripMetadata::KeepExif,
        StripMetadata::KeepIcc,
        StripMetadata::KeepAll,
    ] {
        let mut o = opts();
        o.format = Some(ImageFormat::Jpeg);
        o.strip_metadata = strip;
        let out = compress_image(&input, &o).unwrap();
        assert_eq!(decode(&out).dimensions(), (64, 48));
    }
}

#[test]
fn quality_boundaries_are_accepted() {
    let input = noise_png(64, 48);
    for quality in [1u8, 100] {
        let mut o = opts();
        o.format = Some(ImageFormat::Jpeg);
        o.quality = quality;
        let report = compress_with_report(&input, &o).unwrap();
        assert_eq!(report.quality_used, quality);
    }
}

#[test]
fn no_auto_orient_still_compresses() {
    let input = noise_png(64, 48);
    let mut o = opts();
    o.format = Some(ImageFormat::Jpeg);
    o.auto_orient = false;
    assert_eq!(
        decode(&compress_image(&input, &o).unwrap()).dimensions(),
        (64, 48)
    );
}
