"""Tests for the Python-side validation errors.

The cheap checks (empty input, wrong type, quality range, enum
spellings) run in the Python layer before any native call, so they
pass without the native extension. The native-error cases skip
until the wheel is built.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import dusimagez
from dusimagez import DusimagezError
from _png import make_png
from conftest import NATIVE_AVAILABLE


def assert_code(excinfo, code):
    assert excinfo.value.code == code


def test_empty_input_is_empty_input():
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(b"")
    assert_code(excinfo, "EMPTY_INPUT")
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.get_image_info(bytearray())
    assert_code(excinfo, "EMPTY_INPUT")


def test_non_bytes_input_is_invalid_type():
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image("not bytes")
    assert_code(excinfo, "INVALID_TYPE")
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(None)
    assert_code(excinfo, "INVALID_TYPE")


def test_memoryview_and_bytearray_accepted_as_input_shape():
    # bytearray / memoryview must pass input validation: with the
    # native module they compress fine; without it they reach the
    # native call and fail only on the missing module — never on
    # INVALID_TYPE.
    for data in (bytearray(make_png()), memoryview(make_png())):
        if NATIVE_AVAILABLE:
            out = dusimagez.compress_image(data)
            assert isinstance(out, bytes) and len(out) > 0
        else:
            with pytest.raises(DusimagezError) as excinfo:
                dusimagez.compress_image(data)
            assert excinfo.value.code == "OPERATION_FAILED"


def test_quality_out_of_range_is_invalid_quality():
    for bad in (0, 101, -5):
        with pytest.raises(DusimagezError) as excinfo:
            dusimagez.compress_image(make_png(), quality=bad)
        assert_code(excinfo, "INVALID_QUALITY")
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(make_png(), quality=80.5)
    assert_code(excinfo, "INVALID_QUALITY")


def test_unknown_format_is_invalid_argument():
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(make_png(), format="heic")
    assert_code(excinfo, "INVALID_ARGUMENT")


def test_unknown_subsampling_preset_strip_are_invalid_argument():
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(make_png(), subsampling="s111")
    assert_code(excinfo, "INVALID_ARGUMENT")
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(make_png(), preset="tiny")
    assert_code(excinfo, "INVALID_ARGUMENT")
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(make_png(), strip_metadata="keep_all_exif")
    assert_code(excinfo, "INVALID_ARGUMENT")


def test_bad_dimensions_are_invalid_dimensions():
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(make_png(), max_width=0)
    assert_code(excinfo, "INVALID_DIMENSIONS")
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(make_png(), max_height=-10)
    assert_code(excinfo, "INVALID_DIMENSIONS")


def test_bad_dpi_is_invalid_argument():
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(make_png(), dpi=(300,))
    assert_code(excinfo, "INVALID_ARGUMENT")
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(make_png(), dpi=(0, 300))
    assert_code(excinfo, "INVALID_ARGUMENT")


def test_negative_target_bytes_is_invalid_argument():
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_to_size(make_png(), -1)
    assert_code(excinfo, "INVALID_ARGUMENT")


def test_bulk_non_list_is_invalid_type():
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.bulk_compress(b"not a list")
    assert_code(excinfo, "INVALID_TYPE")


def test_bulk_empty_list_returns_empty():
    assert dusimagez.bulk_compress([]) == []


@pytest.mark.skipif(not NATIVE_AVAILABLE, reason="native wheel not built (core missing)")
def test_invalid_image_bytes_is_invalid_image():
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_image(b"this is not an image at all, just text bytes")
    assert excinfo.value.code == "INVALID_IMAGE"
    assert excinfo.value.message_en
    assert excinfo.value.message_hi


@pytest.mark.skipif(not NATIVE_AVAILABLE, reason="native wheel not built (core missing)")
def test_unreachable_target_size():
    img = make_png("noise", 128, 128, seed=7)
    with pytest.raises(DusimagezError) as excinfo:
        dusimagez.compress_to_size(img, 10)
    assert excinfo.value.code == "TARGET_SIZE_UNREACHABLE"
