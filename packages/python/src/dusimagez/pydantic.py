"""Optional Pydantic v2 models for dusimagez (thin wrapper).

The models live in the wheel's ``dusimagez.pydantic`` (built from
``crates/dusimagez-py``); this module re-exports them from the
installed wheel when it is on ``sys.path``. Pydantic is never
required to import the main ``dusimagez`` package.
"""

import importlib.util
import os
import sys

__all__ = [
    "CompressOptions",
    "CompressReport",
    "ImageInfo",
]


def _load_wheel_pydantic():
    here = os.path.dirname(os.path.abspath(__file__))
    for entry in sys.path:
        if not entry:
            continue
        package_dir = os.path.join(entry, "dusimagez")
        if os.path.abspath(package_dir) == os.path.abspath(here):
            continue
        path = os.path.join(package_dir, "pydantic.py")
        if not os.path.isfile(path):
            continue
        spec = importlib.util.spec_from_file_location(
            "dusimagez._wheel_pydantic", path
        )
        if spec is None or spec.loader is None:
            continue
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module
    raise ImportError(
        "dusimagez Pydantic models need the dusimagez wheel: "
        "pip install 'dusimagez[pydantic]'"
    )


_wheel = _load_wheel_pydantic()

CompressOptions = _wheel.CompressOptions
CompressReport = _wheel.CompressReport
ImageInfo = _wheel.ImageInfo
