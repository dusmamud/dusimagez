"""Op tests — all five core operations through the real native
extension. Skip until the wheel is built (`maturin develop` in
crates/dusimagez-py), i.e. until the Rust core lands.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import dusimagez
from dusimagez import DusimagezError, check_report_maths
from _png import make_png
from conftest import NATIVE_AVAILABLE

pytestmark = pytest.mark.skipif(
    not NATIVE_AVAILABLE, reason="native wheel not built (core missing)"
)


def test_compress_image_png_round_trip():
    img = make_png("gradient", 64, 48, seed=4)
    out = dusimagez.compress_image(img, quality=80)
    assert isinstance(out, bytes) and len(out) > 0
    info = dusimagez.get_image_info(out)
    assert info.format == "png"
    assert info.width == 64 and info.height == 48


def test_compress_image_to_jpeg():
    img = make_png("gradient", 64, 48, seed=4)
    out = dusimagez.compress_image(img, format="jpeg", quality=80)
    info = dusimagez.get_image_info(out)
    assert info.format == "jpeg"
    assert info.width == 64 and info.height == 48


def test_compress_image_format_tiff_falls_back_to_png():
    # TIFF is decode-only: the core accepts it as an output format
    # and falls back to PNG (see crates/dusimagez/src/core.rs).
    img = make_png("gradient", 64, 48, seed=4)
    out = dusimagez.compress_image(img, format="tiff", quality=80)
    assert dusimagez.get_image_info(out).format == "png"


def test_compress_image_defaults_and_kwargs_agree():
    img = make_png("noise", 64, 48, seed=1)
    a = dusimagez.compress_image(img)
    b = dusimagez.compress_image(img, quality=80)
    assert a == b


def test_compress_image_with_options_dataclass_and_pydantic():
    from dusimagez import CompressOptions

    img = make_png("gradient", 64, 48, seed=4)
    out = dusimagez.compress_image(img, options=CompressOptions(quality=70, format="jpeg"))
    assert dusimagez.get_image_info(out).format == "jpeg"


def test_compress_to_size_respects_target():
    img = make_png("noise", 128, 96, seed=7)
    out = dusimagez.compress_to_size(img, 20_000, format="jpeg")
    assert len(out) <= 20_000
    assert dusimagez.get_image_info(out).format == "jpeg"


def test_compress_to_size_via_target_bytes_option():
    img = make_png("noise", 128, 96, seed=7)
    out = dusimagez.compress_image(img, format="jpeg", target_bytes=20_000)
    assert len(out) <= 20_000


def test_compress_with_report_maths():
    img = make_png("noise", 128, 96, seed=7)
    report = dusimagez.compress_with_report(img, format="jpeg", quality=80)
    assert report.original_size == len(img)
    assert report.output_size == len(report.output)
    assert report.format == "jpeg"
    assert report.width == 128 and report.height == 96
    assert report.quality_used == 80
    assert report.resized is False
    ok, problems = check_report_maths(report)
    assert ok, problems
    # Re-encoding the report output decodes to the same dimensions.
    assert dusimagez.get_image_info(report.output).width == 128


def test_compress_with_report_resize_flag():
    img = make_png("gradient", 100, 80, seed=2)
    report = dusimagez.compress_with_report(img, max_width=50)
    assert report.resized is True
    assert report.width == 50 and report.height == 40


def test_get_image_info_fields():
    img = make_png("gradient", 64, 48, seed=4)
    info = dusimagez.get_image_info(img)
    assert info.format == "png"
    assert info.width == 64
    assert info.height == 48
    assert info.has_alpha is False
    assert info.colour_type == "rgb8"
    assert info.file_size == len(img)


def test_bulk_compress_order_and_partial_failure():
    good = make_png("gradient", 64, 48, seed=4)
    bad = b"not an image at all"
    results = dusimagez.bulk_compress([good, bad, good], format="jpeg", quality=80)
    assert [r["index"] for r in results] == [0, 1, 2]
    assert results[0]["report"] is not None
    assert results[0]["error_code"] is None
    assert results[1]["report"] is None
    assert results[1]["error_code"] == "INVALID_IMAGE"
    assert results[2]["report"] is not None
    assert results[2]["report"].format == "jpeg"


def test_preset_web_applies():
    img = make_png("noise", 128, 96, seed=7)
    out = dusimagez.compress_image(img, format="jpeg", preset="web")
    assert dusimagez.get_image_info(out).format == "jpeg"
    assert len(out) > 0


def test_subsampling_and_strip_spellings():
    img = make_png("gradient", 64, 48, seed=4)
    out = dusimagez.compress_image(
        img, format="jpeg", subsampling="s444", strip_metadata="keep_exif"
    )
    assert dusimagez.get_image_info(out).format == "jpeg"
