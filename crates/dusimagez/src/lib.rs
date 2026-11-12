//! dusimagez — offline image compressor (v0.1.0).
//!
//! One Rust core: compress an image, compress to an exact target file
//! size, or get a full before/after report — with pro controls (quality,
//! chroma subsampling, progressive JPEG, PNG palette quantise, WebP/AVIF
//! effort, metadata strip/keep, presets, downscale-only resize, batch).
//! No upload, no network at runtime.
//!
//! ```rust,no_run
//! use dusimagez::{CompressOptions, compress_image};
//!
//! let bytes = std::fs::read("input.jpg").unwrap();
//! let out = compress_image(&bytes, &CompressOptions::default()).unwrap();
//! std::fs::write("output.jpg", out).unwrap();
//! ```
//!
//! ## Formats
//!
//! In: JPEG, PNG, WebP, GIF, BMP, TIFF. Out: JPEG, PNG, WebP, AVIF, GIF,
//! BMP. No HEIC in v0.1.0. AVIF encode is lossy-only / 8-bit. GIF encode
//! is first-frame only for animated input.
//!
//! ## Honest v0.1.0 limits
//!
//! * **AVIF input is not decodable** in this build: it needs dav1d
//!   (`avif-native`), which does not build in this environment, so AVIF
//!   input returns `UNSUPPORTED_FORMAT`. AVIF *output* works.
//! * **JPEG subsampling** is validated and resolved per SPEC §2, but the
//!   `image`-crate JPEG encoder exposes no subsampling control (and
//!   mozjpeg needs nasm, unavailable here), so output uses the encoder's
//!   default subsampling.
//! * **Progressive JPEG / interlaced PNG** are accepted but not honoured
//!   — neither encoder exposes the control.
//! * **PNG `dpi`** is written as a pHYs chunk; **JPEG `dpi`** via JFIF
//!   density. Other formats ignore `dpi`.
//! * `KeepExif` / `KeepAll` preserve metadata only where the chosen
//!   encoder supports it (JPEG/PNG via the `image` encoders).
//! * The whole image lives in memory; there is no streaming mode.
//!
//! See `SPEC.md` for the full contract.

mod core;
mod error;
mod types;

pub use core::{
    bulk_compress, compress_image, compress_to_size, compress_with_report, get_image_info,
};
pub use error::{DusimagezError, ErrorCode};
pub use types::{
    CompressOptions, CompressReport, ImageFormat, ImageInfo, Preset, ResolvedOptions,
    StripMetadata, Subsampling,
};
