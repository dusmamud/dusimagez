"""dusimagez — offline image compressor (thin wrapper).

This is the source-tree wrapper around the compiled ``dusimagez``
extension. The image work lives in the Rust core, reached through
``dusimagez._native`` (shipped by the maturin wheel built from
``crates/dusimagez-py``); this module re-exports that wheel's
package with the same ``DusimagezError`` translation, accepting
either an ``options`` dict or keyword arguments (kwargs win).

Install the wheel (``pip install dusimagez``) — this wrapper on
its own has no image code in it. In a development checkout the
native module is found in the installed wheel on ``sys.path``.

``check_report_maths`` is pure Python and works even without the
wheel installed.
"""

from __future__ import annotations

import importlib.util
import os
import sys

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


def _load_wheel():
    """Find the maturin wheel's ``dusimagez`` package on ``sys.path``.

    The wheel's ``__init__.py`` is loaded as ``dusimagez_wheel_impl``
    (with its own directory as the submodule search path, so its
    ``from . import _native`` finds the compiled extension). This
    file's own directory is skipped.
    """
    here = os.path.dirname(os.path.abspath(__file__))
    for entry in sys.path:
        if not entry:
            continue
        package_dir = os.path.join(entry, "dusimagez")
        if os.path.abspath(package_dir) == os.path.abspath(here):
            continue
        init = os.path.join(package_dir, "__init__.py")
        if not os.path.isfile(init):
            continue
        spec = importlib.util.spec_from_file_location(
            "dusimagez_wheel_impl", init,
            submodule_search_locations=[package_dir],
        )
        if spec is None or spec.loader is None:
            continue
        module = importlib.util.module_from_spec(spec)
        sys.modules.setdefault("dusimagez_wheel_impl", module)
        try:
            spec.loader.exec_module(module)
        except ImportError:
            continue
        return module
    return None


_wheel = _load_wheel()


if _wheel is not None:
    DusimagezError = _wheel.DusimagezError
    CompressOptions = _wheel.CompressOptions
    CompressReport = _wheel.CompressReport
    ImageInfo = _wheel.ImageInfo
    compress_image = _wheel.compress_image
    compress_to_size = _wheel.compress_to_size
    compress_with_report = _wheel.compress_with_report
    get_image_info = _wheel.get_image_info
    bulk_compress = _wheel.bulk_compress
    check_report_maths = _wheel.check_report_maths
else:  # pragma: no cover - wheel installed in normal use

    class DusimagezError(Exception):
        """Every dusimagez failure, with a stable machine-readable code."""

        def __init__(self, code, message_en="", message_hi=""):
            super().__init__(f"{code}: {message_en}")
            self.code = code
            self.message_en = message_en
            self.message_hi = message_hi
            self.reason_en = message_en
            self.reason_hi = message_hi

    def _missing(*args, **kwargs):
        raise ImportError(
            "The dusimagez native module is not installed: "
            "pip install dusimagez "
            "(or build the wheel: maturin develop, from crates/dusimagez-py)"
        )

    compress_image = _missing
    compress_to_size = _missing
    compress_with_report = _missing
    get_image_info = _missing
    bulk_compress = _missing

    def check_report_maths(report):
        """Pure check: does a report's maths add up? (No wheel needed.)

        Accepts a report dict (or an object with ``original_size`` /
        ``output_size`` / ``saved_bytes`` / ``saved_percent`` /
        ``output``). Returns ``(ok, problems)``.
        """
        if isinstance(report, dict):
            original = int(report["original_size"])
            output = int(report["output_size"])
            saved = int(report["saved_bytes"])
            percent = float(report["saved_percent"])
            output_len = len(report["output"])
        else:
            original = report.original_size
            output = report.output_size
            saved = report.saved_bytes
            percent = report.saved_percent
            output_len = len(report.output)
        problems = []
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
