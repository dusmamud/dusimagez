"""dusimagez — offline image compressor (Python).

Compress one image, compress to an exact target file size, or get a
full before/after report — with pro controls: quality, chroma
subsampling, progressive JPEG, PNG palette quantise, WebP/AVIF
effort, metadata strip/keep, presets (Web / Social / Print /
Archive), downscale-only resize, and batch mode. Every operation
takes bytes in and returns bytes out, and every failure raises
:class:`DusimagezError` with a stable ``code`` plus ``message_en`` /
``message_hi`` — never a bare ``TypeError``.

The heavy lifting is the Rust core, reached through the compiled
``dusimagez._native`` extension module. This module is the friendly
Python API on top of it: keyword arguments mirroring the Rust option
fields, an ``options`` dict (or dataclass / Pydantic model) override,
and input-type validation before any native call.

Honest limits (v0.1.0, same as the Rust core): NO HEIC; AVIF encode
is lossy-only / 8-bit; GIF encode is first-frame only for animated
input; whole images live in memory, no streaming.
"""

from __future__ import annotations

from dataclasses import dataclass

try:
    from . import _native
except ImportError:  # pragma: no cover - built wheel always ships it
    _native = None  # type: ignore[assignment]

__version__ = "0.1.0"

__all__ = [
    "DusimagezError",
    "CompressOptions",
    "CompressReport",
    "ImageInfo",
    "compress_image",
    "compress_to_size",
    "compress_with_report",
    "get_image_info",
    "bulk_compress",
    "check_report_maths",
    "__version__",
]


class DusimagezError(Exception):
    """Every dusimagez failure, with a stable machine-readable code."""

    def __init__(self, code, message_en="", message_hi=""):
        super().__init__(f"{code}: {message_en}")
        self.code = code
        self.message_en = message_en
        self.message_hi = message_hi
        # Aliases: the native payload calls these messages; some
        # callers expect the `reason_*` names.
        self.reason_en = message_en
        self.reason_hi = message_hi


def _invalid_type(what_en, what_hi):
    return DusimagezError("INVALID_TYPE", what_en, what_hi)


def _invalid_argument(what_en, what_hi):
    return DusimagezError("INVALID_ARGUMENT", what_en, what_hi)


def _from_native(exc):
    """Parse the native `CODE\\x1f message_en \\x1f message_hi` payload."""
    parts = str(exc).split("\x1f")
    if len(parts) == 3 and parts[0]:
        return DusimagezError(parts[0], parts[1], parts[2])
    return DusimagezError(
        "OPERATION_FAILED",
        f"Operation failed: {exc}",
        f"Operation fail hua: {exc}",
    )


def _call(fn_name, *args):
    """Call a native function by name, translating every failure to
    DusimagezError. The name (not a bound method) is passed so the
    missing-module check runs before any attribute access on
    `_native`."""
    if _native is None:  # pragma: no cover
        raise DusimagezError(
            "OPERATION_FAILED",
            "The dusimagez native module is not available in this install. "
            "Build the wheel: maturin develop (from crates/dusimagez-py).",
            "Is install me dusimagez ka native module nahi hai. "
            "Wheel build karein: crates/dusimagez-py se maturin develop.",
        )
    try:
        return getattr(_native, fn_name)(*args)
    except DusimagezError:
        raise
    except Exception as exc:  # native ValueError payload, or a bug
        raise _from_native(exc) from None


# ---------------------------------------------------------------------------
# Options — dataclass mirrors the Rust CompressOptions (SPEC §2).
# Field names are snake_case and match the Rust core exactly.
# `subsampling=None` means "let the core decide" (S420 for JPEG
# quality < 90, S444 otherwise) — the key is omitted, not sent as
# null, so the Rust Default applies.
# ---------------------------------------------------------------------------

_FORMATS = {
    "jpeg", "jpg", "png", "webp", "avif", "gif", "bmp", "tiff",
    "Jpeg", "Png", "WebP", "Avif", "Gif", "Bmp", "Tiff",
}
_SUBSAMPLINGS = {
    "s444", "s422", "s420", "444", "422", "420",
    "4:4:4", "4:2:2", "4:2:0", "S444", "S422", "S420",
}
_PRESETS = {"web", "social", "print", "archive", "Web", "Social", "Print", "Archive"}
_STRIP_METADATA = {
    "all", "keep_exif", "keep_icc", "keep_all",
    "keep-exif", "keep-icc", "keep-all",
    "All", "KeepExif", "KeepIcc", "KeepAll",
}


@dataclass
class CompressOptions:
    """Pro controls for compression (SPEC §2). Defaults mirror Rust."""

    quality: int = 80
    format: str | None = None
    progressive: bool = False
    lossless: bool = False
    effort: int = 6
    subsampling: str | None = None
    png_compression_level: int = 6
    palette: bool = False
    preset: str | None = None
    target_bytes: int | None = None
    max_width: int | None = None
    max_height: int | None = None
    strip_metadata: str = "all"
    auto_orient: bool = True
    dpi: tuple[int, int] | None = None
    max_pixels: int = 120_000_000
    max_dimension: int = 16000


@dataclass
class CompressReport:
    """Before/after report from :func:`compress_with_report`."""

    output: bytes
    original_size: int
    output_size: int
    saved_bytes: int
    saved_percent: float
    format: str
    width: int
    height: int
    quality_used: int
    resized: bool

    @classmethod
    def from_dict(cls, d):
        return cls(
            output=bytes(d["output"]),
            original_size=int(d["original_size"]),
            output_size=int(d["output_size"]),
            saved_bytes=int(d["saved_bytes"]),
            saved_percent=float(d["saved_percent"]),
            format=str(d["format"]),
            width=int(d["width"]),
            height=int(d["height"]),
            quality_used=int(d["quality_used"]),
            resized=bool(d["resized"]),
        )


@dataclass
class ImageInfo:
    """Summary of an image, from :func:`get_image_info`."""

    format: str
    width: int
    height: int
    has_alpha: bool
    colour_type: str
    file_size: int

    @classmethod
    def from_dict(cls, d):
        return cls(
            format=str(d["format"]),
            width=int(d["width"]),
            height=int(d["height"]),
            has_alpha=bool(d["has_alpha"]),
            colour_type=str(d["colour_type"]),
            file_size=int(d["file_size"]),
        )


def check_report_maths(report):
    """Pure check: does a report's maths add up?

    Accepts a :class:`CompressReport` or a report dict. Returns
    ``(ok, problems)`` — ``problems`` is a list of human-readable
    mismatches (empty when the maths holds). ``saved_percent`` is
    compared with a 0.5-point tolerance (the core stores it as f32).
    """
    if isinstance(report, dict):
        original = int(report["original_size"])
        output = int(report["output_size"])
        saved = int(report["saved_bytes"])
        percent = float(report["saved_percent"])
    else:
        original = report.original_size
        output = report.output_size
        saved = report.saved_bytes
        percent = report.saved_percent
    problems = []
    output_len = len(report["output"]) if isinstance(report, dict) else len(report.output)
    if output != output_len:
        problems.append("output_size does not match len(output)")
    if saved != original - output:
        problems.append(
            f"saved_bytes {saved} != original_size - output_size "
            f"({original} - {output} = {original - output})"
        )
    expected_percent = (100.0 * saved / original) if original else 0.0
    if abs(percent - expected_percent) > 0.5:
        problems.append(
            f"saved_percent {percent} != 100*saved/original ({expected_percent:.2f})"
        )
    return (not problems, problems)


# ---------------------------------------------------------------------------
# Input / options coercion
# ---------------------------------------------------------------------------

def _as_bytes(image, what="Image bytes", allow_empty=False):
    if isinstance(image, (bytes, bytearray, memoryview)):
        data = bytes(image)
    else:
        raise _invalid_type(
            f"{what} must be bytes.",
            f"{what} bytes honi chahiye.",
        )
    if len(data) == 0 and not allow_empty:
        raise DusimagezError(
            "EMPTY_INPUT", f"{what} is empty.", f"{what} khaali hai."
        )
    return data


def _options_dict(options=None, kwargs=None):
    """Merge an options dict / dataclass / Pydantic model with kwargs.

    kwargs win. Validates the cheap checks (quality, effort, PNG
    level, dimensions, enum spellings); the core does the real
    validation and reports the same stable codes.
    """
    merged = {}
    if options is not None:
        if hasattr(options, "model_dump"):  # Pydantic v2 model
            merged.update(options.model_dump(exclude_none=False))
        elif hasattr(options, "__dataclass_fields__"):
            import dataclasses

            merged.update(dataclasses.asdict(options))
        elif isinstance(options, dict):
            merged.update(options)
        else:
            raise _invalid_argument(
                "options must be a dict, a CompressOptions dataclass, "
                "or a Pydantic CompressOptions model.",
                "options dict, CompressOptions dataclass, ya Pydantic "
                "CompressOptions model honi chahiye.",
            )
    if kwargs:
        merged.update(kwargs)
    # `subsampling=None` means "core default" — omit the key so the
    # Rust Default applies (the field is not an Option in Rust).
    if merged.get("subsampling") is None:
        merged.pop("subsampling", None)

    quality = merged.get("quality", 80)
    if not isinstance(quality, int) or isinstance(quality, bool) \
            or quality < 1 or quality > 100:
        raise DusimagezError(
            "INVALID_QUALITY",
            f"Quality {quality} is invalid. Quality must be between 1 and 100.",
            f"Quality {quality} galat hai. Quality 1 se 100 ke beech honi chahiye.",
        )
    effort = merged.get("effort", 6)
    if not isinstance(effort, int) or isinstance(effort, bool) \
            or effort < 0 or effort > 10:
        raise _invalid_argument(
            f"Effort {effort} is invalid. Effort must be between 0 and 10.",
            f"Effort {effort} galat hai. Effort 0 se 10 ke beech hona chahiye.",
        )
    level = merged.get("png_compression_level", 6)
    if not isinstance(level, int) or isinstance(level, bool) \
            or level < 0 or level > 9:
        raise _invalid_argument(
            f"PNG compression level {level} is invalid. It must be between 0 and 9.",
            f"PNG compression level {level} galat hai. Ye 0 se 9 ke beech hona chahiye.",
        )
    fmt = merged.get("format")
    if fmt is not None and fmt not in _FORMATS:
        raise _invalid_argument(
            f'Unknown format "{fmt}". Use jpeg, png, webp, avif, gif or bmp.',
            f'Unknown format "{fmt}" hai. jpeg, png, webp, avif, gif ya bmp use karein.',
        )
    subsampling = merged.get("subsampling")
    if subsampling is not None and subsampling not in _SUBSAMPLINGS:
        raise _invalid_argument(
            f'Unknown subsampling "{subsampling}". Use s444, s422 or s420.',
            f'Unknown subsampling "{subsampling}" hai. s444, s422 ya s420 use karein.',
        )
    preset = merged.get("preset")
    if preset is not None and preset not in _PRESETS:
        raise _invalid_argument(
            f'Unknown preset "{preset}". Use web, social, print or archive.',
            f'Unknown preset "{preset}" hai. web, social, print ya archive use karein.',
        )
    strip = merged.get("strip_metadata", "all")
    if strip not in _STRIP_METADATA:
        raise _invalid_argument(
            f'Unknown strip_metadata "{strip}". Use all, keep_exif, keep_icc or keep_all.',
            f'Unknown strip_metadata "{strip}" hai. all, keep_exif, keep_icc ya keep_all use karein.',
        )
    for key in ("target_bytes", "max_pixels"):
        value = merged.get(key)
        if value is not None and (
            not isinstance(value, int) or isinstance(value, bool) or value < 0
        ):
            raise _invalid_argument(
                f"{key} must be a non-negative integer.",
                f"{key} non-negative integer hona chahiye.",
            )
    for key in ("max_width", "max_height", "max_dimension"):
        value = merged.get(key)
        if value is not None and (
            not isinstance(value, int) or isinstance(value, bool) or value <= 0
        ):
            raise DusimagezError(
                "INVALID_DIMENSIONS",
                f"{key} must be a positive integer.",
                f"{key} positive integer hona chahiye.",
            )
    dpi = merged.get("dpi")
    if dpi is not None:
        if (
            not isinstance(dpi, (list, tuple))
            or len(dpi) != 2
            or any(
                not isinstance(v, int) or isinstance(v, bool) or v <= 0
                for v in dpi
            )
        ):
            raise _invalid_argument(
                "dpi must be a (x, y) pair of positive integers.",
                "dpi (x, y) positive integers ka pair hona chahiye.",
            )
        merged["dpi"] = [int(dpi[0]), int(dpi[1])]
    return merged


def _target_bytes_arg(target_bytes):
    if not isinstance(target_bytes, int) or isinstance(target_bytes, bool) \
            or target_bytes < 0:
        raise _invalid_argument(
            "Target size must be a non-negative number of bytes.",
            "Target size bytes ki non-negative number honi chahiye.",
        )
    return target_bytes


# ---------------------------------------------------------------------------
# Public API — all synchronous, all raise DusimagezError on failure.
# ---------------------------------------------------------------------------

def compress_image(image, options=None, **kwargs):
    """Re-encode an image at a fixed quality (and `format`, or the
    input format when unset). Returns output bytes."""
    data = _as_bytes(image)
    opts = _options_dict(options, kwargs)
    return bytes(_call("compress_image", data, opts))


def compress_to_size(image, target_bytes, options=None, **kwargs):
    """Re-encode an image so the result is at most `target_bytes`
    bytes, at the highest quality that fits. Raises
    `TARGET_SIZE_UNREACHABLE` when even the lowest quality is too
    big. Returns output bytes."""
    data = _as_bytes(image)
    target = _target_bytes_arg(target_bytes)
    opts = _options_dict(options, kwargs)
    return bytes(_call("compress_to_size", data, target, opts))


def compress_with_report(image, options=None, **kwargs):
    """Compress and return the full before/after :class:`CompressReport`."""
    data = _as_bytes(image)
    opts = _options_dict(options, kwargs)
    return CompressReport.from_dict(_call("compress_with_report", data, opts))


def get_image_info(image):
    """Read an image's summary without modifying it."""
    data = _as_bytes(image)
    return ImageInfo.from_dict(_call("get_image_info", data))


def bulk_compress(images, options=None, **kwargs):
    """Compress many images; one result per input, same order.

    Returns a list of dicts: ``{"index", "report": CompressReport |
    None, "error_code": str | None, "error": str | None,
    "error_hi": str | None}``. One bad input fails only its own
    item, never the whole batch. An empty input list returns ``[]``.
    """
    if not isinstance(images, (list, tuple)):
        raise _invalid_type(
            "Bulk inputs must be a list of bytes.",
            "Bulk inputs bytes ki list honi chahiye.",
        )
    if len(images) == 0:
        return []
    # Empty items are allowed through: the native bulk call reports
    # them per-item (EMPTY_INPUT) instead of aborting the batch.
    bufs = [
        _as_bytes(item, f"Bulk input {i}", allow_empty=True)
        for i, item in enumerate(images)
    ]
    opts = _options_dict(options, kwargs)
    out = []
    for item in _call("bulk_compress", bufs, opts):
        report = item["report"]
        out.append({
            "index": item["index"],
            "report": CompressReport.from_dict(report) if report is not None else None,
            "error_code": item["error_code"],
            "error": item["error"],
            "error_hi": item.get("error_hi"),
        })
    return out
