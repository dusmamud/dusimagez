"""Pytest config for crates/dusimagez-py/tests.

The tests import the `dusimagez` package from this crate's
`python/` source tree (the same code the maturin wheel ships).
When the native extension is built (`maturin develop`), the
package's `_native` import succeeds and the op tests run; without
it, op tests skip and only the pure-Python tests run.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "python"))

import dusimagez  # noqa: E402

NATIVE_AVAILABLE = dusimagez._native is not None
