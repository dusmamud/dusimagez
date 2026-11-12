# dusimagez

Offline image compressor — reduce image file size with maximum control.
One Rust core, bindings for Rust, JS/TypeScript (WASM), Python, Go, PHP,
Kotlin (Android) and Swift (iOS). No upload, no network at runtime. MIT.

## What it does

Compress one image, compress to an exact target file size, or get a full
before/after report — with pro controls: quality, chroma subsampling,
progressive JPEG, PNG palette quantise, WebP/AVIF effort, metadata
strip/keep, presets (Web / Social / Print / Archive), downscale-only
resize, and batch mode.

## Quickstart

```bash
dusimagez compress input.jpg output.jpg --quality 80
dusimagez compress input.png output.png --target-bytes 200000
dusimagez info input.jpg
```

Rust:

```rust
let out = dusimagez::compress_image(&bytes, &CompressOptions::default())?;
```

See `SPEC.md` for the full contract (API, options, formats, error codes),
and the docs site for per-language quickstarts.

## Formats

In: JPEG, PNG, WebP, GIF, BMP, TIFF, AVIF. Out: JPEG, PNG, WebP, AVIF,
GIF, BMP. No HEIC in v0.1.0. AVIF encode is lossy-only / 8-bit.
GIF encode is first-frame only for animated input.

## Status

v0.1.0 — see `CHANGELOG.md`.
