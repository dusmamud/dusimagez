"""Pydantic v2 model tests — run with or without Pydantic installed.

Without Pydantic, instantiating a model raises an ImportError with
a helpful message (the lazy pattern); with Pydantic, the models
validate ranges and literals.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "python"))

from dusimagez import pydantic as models  # noqa: E402

try:
    import pydantic  # noqa: F401

    HAS_PYDANTIC = True
except ImportError:
    HAS_PYDANTIC = False


@pytest.mark.skipif(not HAS_PYDANTIC, reason="pydantic not installed")
def test_options_defaults():
    opts = models.CompressOptions()
    assert opts.quality == 80
    assert opts.format is None
    assert opts.effort == 6
    assert opts.subsampling is None
    assert opts.png_compression_level == 6
    assert opts.strip_metadata == "all"
    assert opts.auto_orient is True
    assert opts.max_pixels == 120_000_000
    assert opts.max_dimension == 16000


@pytest.mark.skipif(not HAS_PYDANTIC, reason="pydantic not installed")
def test_options_rejects_bad_quality():
    with pytest.raises(Exception):
        models.CompressOptions(quality=0)
    with pytest.raises(Exception):
        models.CompressOptions(quality=101)


@pytest.mark.skipif(not HAS_PYDANTIC, reason="pydantic not installed")
def test_options_rejects_bad_format_and_enums():
    with pytest.raises(Exception):
        models.CompressOptions(format="heic")  # not an image format at all
    with pytest.raises(Exception):
        models.CompressOptions(subsampling="s111")
    with pytest.raises(Exception):
        models.CompressOptions(preset="tiny")
    with pytest.raises(Exception):
        models.CompressOptions(png_compression_level=10)


@pytest.mark.skipif(not HAS_PYDANTIC, reason="pydantic not installed")
def test_options_accepts_friendly_spellings():
    opts = models.CompressOptions(
        quality=90, format="webp", subsampling="s444",
        preset="web", strip_metadata="keep_exif",
    )
    assert opts.format == "webp"
    assert opts.subsampling == "s444"


@pytest.mark.skipif(not HAS_PYDANTIC, reason="pydantic not installed")
def test_report_and_info_models():
    report = models.CompressReport(
        output=b"x" * 10, original_size=100, output_size=10,
        saved_bytes=90, saved_percent=90.0, format="jpeg",
        width=64, height=48, quality_used=80, resized=False,
    )
    assert report.saved_percent == 90.0
    with pytest.raises(Exception):
        models.CompressReport(
            output=b"x", original_size=100, output_size=10,
            saved_bytes=90, saved_percent=150.0, format="jpeg",
            width=64, height=48, quality_used=80, resized=False,
        )
    info = models.ImageInfo(
        format="png", width=64, height=48, has_alpha=False,
        colour_type="rgb8", file_size=1234,
    )
    assert info.format == "png"


@pytest.mark.skipif(not HAS_PYDANTIC, reason="pydantic not installed")
def test_model_dump_feeds_options_dict():
    import dusimagez

    opts = models.CompressOptions(quality=70, format="jpeg")
    merged = dusimagez._options_dict(opts)
    assert merged["quality"] == 70
    assert merged["format"] == "jpeg"


@pytest.mark.skipif(HAS_PYDANTIC, reason="only without pydantic")
def test_without_pydantic_instantiation_raises_helpful_error():
    with pytest.raises(ImportError, match="Pydantic v2"):
        models.CompressOptions(quality=80)
