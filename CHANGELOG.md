# Changelog

## 0.1.0 — Unreleased

- Initial release: offline image compressor (Rust core).
- `compress_image`, `compress_to_size`, `compress_with_report`,
  `get_image_info`, `bulk_compress`.
- Pro controls: quality, subsampling, progressive, lossless, effort,
  PNG palette/level, presets, target size, metadata strip/keep,
  auto-orient, DPI, max pixels/dimensions, downscale-only resize.
- Bindings: JS/WASM, Python (PyO3), CLI, C FFI, Go (cgo), PHP (FFI),
  Kotlin (Android, UniFFI), Swift (iOS, UniFFI).
- Docs + WASM playground.
