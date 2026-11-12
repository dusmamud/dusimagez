# dusimagez (Python)

Offline image compressor — reduce image file size with maximum
control. One Rust core, no network, no uploads. MIT.

## Install

```bash
pip install dusimagez
pip install "dusimagez[pydantic]"  # optional: Pydantic v2 models
```

## Quickstart

```python
from dusimagez import (
    compress_image,
    compress_to_size,
    compress_with_report,
    get_image_info,
    bulk_compress,
)

png = open("input.png", "rb").read()

# Re-encode at a fixed quality (keeps the input format by default).
out = compress_image(png, quality=80)

# Convert to JPEG and squeeze under 200 KB.
small = compress_to_size(png, 200_000, format="jpeg")

# Full before/after report.
report = compress_with_report(png, format="jpeg", quality=80)
print(report.original_size, report.output_size, report.saved_percent)

# Summary without modifying anything.
info = get_image_info(png)
print(info.format, info.width, info.height)

# Batch: one result per input, same order; one bad input fails
# only its own item.
results = bulk_compress([png, bad_bytes, png], format="jpeg")
```

Options can be a dict, keyword arguments (kwargs win), a
`CompressOptions` dataclass, or a Pydantic model:

```python
from dusimagez import CompressOptions

opts = CompressOptions(quality=90, format="webp", preset="web")
out = compress_image(png, options=opts)
# or: out = compress_image(png, quality=90, format="webp")
```

## Options

| Option | Default | Notes |
| --- | --- | --- |
| `quality` | `80` | 1–100. `INVALID_QUALITY` outside. |
| `format` | keep input | `"jpeg"`, `"png"`, `"webp"`, `"avif"`, `"gif"`, `"bmp"` (`"tiff"` falls back to PNG) |
| `progressive` | `False` | Progressive JPEG / interlaced PNG |
| `lossless` | `False` | WebP/PNG only |
| `effort` | `6` | WebP 0–6, AVIF speed 0–10 |
| `subsampling` | core default | `"s444"`, `"s422"`, `"s420"` (S420 for JPEG quality < 90, S444 otherwise) |
| `png_compression_level` | `6` | 0–9 |
| `palette` | `False` | PNG quantise to palette |
| `preset` | `None` | `"web"`, `"social"`, `"print"`, `"archive"` |
| `target_bytes` | `None` | Same as `compress_to_size` via one call |
| `max_width` / `max_height` | `None` | Downscale-only, never upscale |
| `strip_metadata` | `"all"` | `"all"`, `"keep_exif"`, `"keep_icc"`, `"keep_all"` |
| `auto_orient` | `True` | Apply EXIF orientation on load |
| `dpi` | `None` | `(x, y)` pair, JPEG/PNG output |
| `max_pixels` | `120_000_000` | Refuse larger (`IMAGE_TOO_LARGE`) |
| `max_dimension` | `16000` | Refuse longer sides |

## Pydantic models (optional)

```python
from dusimagez.pydantic import CompressOptions, CompressReport, ImageInfo

opts = CompressOptions(quality=90, format="webp")
out = compress_image(png, options=opts)
```

Pydantic is never required to import or use the main package:
`import dusimagez` works without it, and the models raise a
helpful `ImportError` only when actually used without Pydantic
installed.

## Errors

Every failure raises `dusimagez.DusimagezError` with a stable
`.code` plus `.message_en` / `.message_hi`:

`INVALID_IMAGE`, `UNSUPPORTED_FORMAT`, `IMAGE_TOO_LARGE`,
`TARGET_SIZE_UNREACHABLE`, `INVALID_QUALITY`,
`INVALID_DIMENSIONS`, `INVALID_ARGUMENT`, `EMPTY_INPUT`,
`INVALID_TYPE` (non-bytes input), `OPERATION_FAILED`.

## Limits (v0.1.0, same as the Rust core)

NO HEIC. AVIF encode is lossy-only / 8-bit. GIF encode is
first-frame only for animated input. Whole images live in memory,
no streaming.

## Develop

```bash
# Build the wheel (from crates/dusimagez-py) and install it:
maturin develop
# Then run the wrapper tests from this directory:
pytest
```
