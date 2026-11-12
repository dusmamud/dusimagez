"""Tests for the packages/python thin wrapper.

The wrapper delegates to the compiled ``dusimagez`` wheel
(maturin develop / pip install), so the op tests assert the
delegation: same bytes, same info, same error codes. They skip
until the wheel is installed (i.e. until the Rust core lands).
The pure-Python checks (report maths, missing-wheel error) run
without the wheel.

The full API (dataclasses, enum spellings, validation) is covered
by crates/dusimagez-py/tests/.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))
sys.path.insert(0, str(Path(__file__).resolve().parents[3]
                       / "crates" / "dusimagez-py" / "tests"))

import dusimagez  # noqa: E402
from dusimagez import DusimagezError, check_report_maths  # noqa: E402
from _png import make_png  # noqa: E402

WHEEL_PRESENT = dusimagez._wheel is not None
needs_wheel = pytest.mark.skipif(
    not WHEEL_PRESENT, reason="dusimagez wheel not installed (core missing)"
)


# ---------------------------------------------------------------------------
# Pure checks — run without the wheel.
# ---------------------------------------------------------------------------

def test_report_maths_pure():
    ok, problems = check_report_maths({
        "output": b"x" * 800,
        "original_size": 1000,
        "output_size": 800,
        "saved_bytes": 200,
        "saved_percent": 20.0,
    })
    assert ok, problems
    ok, problems = check_report_maths({
        "output": b"x" * 800,
        "original_size": 1000,
        "output_size": 800,
        "saved_bytes": 150,
        "saved_percent": 20.0,
    })
    assert not ok


@pytest.mark.skipif(WHEEL_PRESENT, reason="only without the wheel")
def test_missing_wheel_raises_helpful_import_error():
    with pytest.raises(ImportError, match="not installed"):
        dusimagez.compress_image(b"bytes")
    with pytest.raises(ImportError, match="not installed"):
        dusimagez.get_image_info(b"bytes")


# ---------------------------------------------------------------------------
# Delegation checks — need the wheel.
# ---------------------------------------------------------------------------

@needs_wheel
def test_wrapper_reexports_native():
    img = make_png("gradient", 64, 48, seed=4)
    assert dusimagez.get_image_info(
        dusimagez.compress_image(img, quality=80)).width == 64
    assert dusimagez.get_image_info(
        dusimagez.compress_image(img, format="jpeg")).format == "jpeg"
    out = dusimagez.compress_to_size(make_png("noise", 128, 96, seed=7),
                                     20_000, format="jpeg")
    assert len(out) <= 20_000
    report = dusimagez.compress_with_report(img, format="jpeg", quality=80)
    assert report.format == "jpeg"
    ok, problems = check_report_maths(report)
    assert ok, problems


@needs_wheel
def test_wrapper_info_and_bulk():
    img = make_png("gradient", 64, 48, seed=4)
    info = dusimagez.get_image_info(img)
    assert info.format == "png" and info.width == 64 and info.height == 48
    results = dusimagez.bulk_compress([img, b"not an image", img],
                                      format="jpeg", quality=80)
    assert [r["index"] for r in results] == [0, 1, 2]
    assert results[0]["report"] is not None
    assert results[1]["report"] is None
    assert results[1]["error_code"] == "INVALID_IMAGE"
    assert results[2]["report"] is not None


@needs_wheel
def test_wrapper_error_codes():
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(b"this is not an image at all, just text")
    assert excinfo.value.code == "INVALID_IMAGE"
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(b"")
    assert excinfo.value.code == "EMPTY_INPUT"


@needs_wheel
def test_wrapper_pydantic_models():
    pydantic = pytest.importorskip("pydantic")
    from dusimagez.pydantic import CompressOptions

    opts = CompressOptions(quality=90, format="webp")
    img = make_png("gradient", 64, 48, seed=4)
    out = dusimagez.compress_image(img, options=opts)
    assert dusimagez.get_image_info(out).format == "webp"
    with pytest.raises(pydantic.ValidationError):
        CompressOptions(quality=0)
