"""Optional Pydantic v2 models for dusimagez.

Pydantic is **never** required to import or use the main
``dusimagez`` package. This module is pure Python — it does not
import the native extension — and it imports cleanly without Pydantic
installed; the models below raise an informative ``ImportError`` only
when actually instantiated without Pydantic.

Usage (requires ``pip install dusimagez[pydantic]``)::

    from dusimagez.pydantic import CompressOptions
    from dusimagez import compress_image

    opts = CompressOptions(quality=90, format="webp")
    out = compress_image(png, options=opts)

Field names are snake_case and match the Rust core options exactly,
with the same defaults (SPEC §2). Friendly spellings (``"jpeg"``,
``"s420"``, ``"web"``, ``"keep_exif"``) are normalised by the main
package before they reach the native module.
"""

from typing import Literal

try:
    from pydantic import BaseModel, Field
    _HAS_PYDANTIC = True
except ImportError:  # pragma: no cover - depends on env
    _HAS_PYDANTIC = False

    class BaseModel:  # type: ignore[no-redef]
        """Placeholder: raises a helpful error only when used."""

        def __init__(self, *args, **kwargs):
            raise ImportError(
                "dusimagez Pydantic models need Pydantic v2: "
                "pip install 'dusimagez[pydantic]'"
            )

    def Field(default=None, **kwargs):  # type: ignore[no-redef]  # noqa: N802
        return default


__all__ = [
    "CompressOptions",
    "CompressReport",
    "ImageInfo",
]


# ---------------------------------------------------------------------------
# Shared field types
# ---------------------------------------------------------------------------

ImageFormatName = Literal["jpeg", "jpg", "png", "webp", "avif", "gif", "bmp", "tiff"]
SubsamplingName = Literal["s444", "s422", "s420"]
PresetName = Literal["web", "social", "print", "archive"]
StripMetadataName = Literal["all", "keep_exif", "keep_icc", "keep_all"]


# ---------------------------------------------------------------------------
# Models — defaults mirror SPEC §2 exactly.
# ---------------------------------------------------------------------------

class CompressOptions(BaseModel):
    """Pro controls for compression (SPEC §2)."""

    quality: int = Field(default=80, ge=1, le=100)
    format: ImageFormatName | None = None
    progressive: bool = False
    lossless: bool = False
    effort: int = Field(default=6, ge=0, le=10)
    subsampling: SubsamplingName | None = None
    png_compression_level: int = Field(default=6, ge=0, le=9)
    palette: bool = False
    preset: PresetName | None = None
    target_bytes: int | None = Field(default=None, ge=0)
    max_width: int | None = Field(default=None, gt=0)
    max_height: int | None = Field(default=None, gt=0)
    strip_metadata: StripMetadataName = "all"
    auto_orient: bool = True
    dpi: tuple[int, int] | None = None
    max_pixels: int = Field(default=120_000_000, ge=0)
    max_dimension: int = Field(default=16000, gt=0)


class CompressReport(BaseModel):
    """Before/after report from ``compress_with_report``."""

    output: bytes
    original_size: int = Field(ge=0)
    output_size: int = Field(ge=0)
    saved_bytes: int = Field(ge=0)
    saved_percent: float = Field(ge=0, le=100)
    format: str
    width: int = Field(ge=0)
    height: int = Field(ge=0)
    quality_used: int = Field(ge=1, le=100)
    resized: bool


class ImageInfo(BaseModel):
    """Summary of an image, from ``get_image_info``."""

    format: str
    width: int = Field(ge=0)
    height: int = Field(ge=0)
    has_alpha: bool
    colour_type: str
    file_size: int = Field(ge=0)
