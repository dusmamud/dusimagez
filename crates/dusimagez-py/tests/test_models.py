"""Tests for the pure-Python layer: dataclasses, options merging,
report maths. No native extension needed — these run even before
the Rust core lands.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import dusimagez
from dusimagez import (
    CompressOptions,
    CompressReport,
    DusimagezError,
    ImageInfo,
    check_report_maths,
)


def test_options_defaults_mirror_spec():
    opts = CompressOptions()
    assert opts.quality == 80
    assert opts.format is None
    assert opts.progressive is False
    assert opts.lossless is False
    assert opts.effort == 6
    assert opts.subsampling is None  # core default (S420 < q90, else S444)
    assert opts.png_compression_level == 6
    assert opts.palette is False
    assert opts.preset is None
    assert opts.target_bytes is None
    assert opts.max_width is None
    assert opts.max_height is None
    assert opts.strip_metadata == "all"
    assert opts.auto_orient is True
    assert opts.dpi is None
    assert opts.max_pixels == 120_000_000
    assert opts.max_dimension == 16000


def test_options_dict_merges_kwargs_and_dataclass():
    merged = dusimagez._options_dict(CompressOptions(quality=70), {"format": "jpeg"})
    assert merged["quality"] == 70
    assert merged["format"] == "jpeg"
    # kwargs win over the dataclass
    merged = dusimagez._options_dict(CompressOptions(quality=70), {"quality": 90})
    assert merged["quality"] == 90


def test_options_dict_omits_none_subsampling():
    merged = dusimagez._options_dict(CompressOptions())
    assert "subsampling" not in merged
    merged = dusimagez._options_dict(CompressOptions(subsampling="s444"))
    assert merged["subsampling"] == "s444"


def test_options_dict_accepts_plain_dict_and_dpi():
    merged = dusimagez._options_dict({"quality": 60, "dpi": (300, 300)})
    assert merged["quality"] == 60
    assert merged["dpi"] == [300, 300]


def test_report_from_dict_and_maths():
    report = CompressReport.from_dict({
        "output": b"x" * 800,
        "original_size": 1000,
        "output_size": 800,
        "saved_bytes": 200,
        "saved_percent": 20.0,
        "format": "jpeg",
        "width": 64,
        "height": 48,
        "quality_used": 80,
        "resized": False,
    })
    assert report.saved_bytes == 200
    assert report.format == "jpeg"
    ok, problems = check_report_maths(report)
    assert ok, problems
    ok, problems = check_report_maths({
        "output": b"x" * 800,
        "original_size": 1000,
        "output_size": 800,
        "saved_bytes": 200,
        "saved_percent": 20.0,
    })
    assert ok, problems


def test_report_maths_catches_bad_saved_bytes():
    ok, problems = check_report_maths({
        "output": b"x" * 800,
        "original_size": 1000,
        "output_size": 800,
        "saved_bytes": 150,  # should be 200
        "saved_percent": 20.0,
    })
    assert not ok
    assert any("saved_bytes" in p for p in problems)


def test_report_maths_catches_bad_percent():
    ok, problems = check_report_maths({
        "output": b"x" * 800,
        "original_size": 1000,
        "output_size": 800,
        "saved_bytes": 200,
        "saved_percent": 90.0,  # should be ~20
    })
    assert not ok
    assert any("saved_percent" in p for p in problems)


def test_report_maths_catches_output_size_mismatch():
    ok, problems = check_report_maths({
        "output": b"x" * 800,
        "original_size": 1000,
        "output_size": 799,
        "saved_bytes": 201,
        "saved_percent": 20.1,
    })
    assert not ok
    assert any("output_size" in p for p in problems)


def test_image_info_from_dict():
    info = ImageInfo.from_dict({
        "format": "png",
        "width": 64,
        "height": 48,
        "has_alpha": False,
        "colour_type": "rgb8",
        "file_size": 1234,
    })
    assert info.format == "png"
    assert info.width == 64
    assert info.colour_type == "rgb8"


def test_error_carries_code_and_both_messages():
    err = DusimagezError("INVALID_QUALITY", "Bad quality.", "Galat quality hai.")
    assert err.code == "INVALID_QUALITY"
    assert err.message_en == "Bad quality."
    assert err.message_hi == "Galat quality hai."
    assert err.reason_en == "Bad quality."
    assert "INVALID_QUALITY" in str(err)


def test_from_native_parses_payload():
    err = dusimagez._from_native(
        ValueError("INVALID_IMAGE\x1fNot an image.\x1fImage nahi hai.")
    )
    assert isinstance(err, DusimagezError)
    assert err.code == "INVALID_IMAGE"
    assert err.message_en == "Not an image."
    assert err.message_hi == "Image nahi hai."


def test_from_native_falls_back_on_plain_error():
    err = dusimagez._from_native(ValueError("boom"))
    assert err.code == "OPERATION_FAILED"
