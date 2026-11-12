"""Shared-vector tests — every language runs `shared/test-vectors.json`
and must agree (SPEC §5–6).

W5 owns `shared/`. The file is a dict: `{"cases": [...],
"fixtures": [...], "version": "0.1.0"}`. Case fields (see the file's
`_header`): `id`, `op` (`compress` | `compress_to_size` |
`compress_with_report` | `get_info` | `bulk`), `fixture` /
`fixtures` (names into the `fixtures` table), `options` (Rust
snake_case, enum spellings CANONICAL: `"Web"`, `"S444"`, `"All"`),
`expect` (format / width / height / max_output_bytes /
min_saved_percent / quality_used / resized / error_code /
results for bulk).

Fixtures resolve three ways: a committed file under
`shared/fixtures/` (`"file"`), inline pseudo-fixtures (`empty` =
0 bytes, `garbage` = the ASCII text in `generate`), or a
`dusimagez mock ...` command — translated to the real CLI flags
(`--out`, format follows the extension, fixed seed) and run with
the CLI binary from `target/debug/dusimagez` (built by W3's
crate; if it is not built yet, those cases skip, never
fake-pass) with output into a temp dir, never into `shared/`.
"""

import json
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import dusimagez
from conftest import NATIVE_AVAILABLE

REPO_ROOT = Path(__file__).resolve().parents[3]
VECTORS_PATH = REPO_ROOT / "shared" / "test-vectors.json"
GARBAGE_TEXT = b"this is not an image at all, just plain text bytes 1234567890"


def _cli_binary():
    for candidate in (REPO_ROOT / "target" / "debug" / "dusimagez",
                      REPO_ROOT / "target" / "release" / "dusimagez"):
        if candidate.is_file():
            return candidate
    return None


def _load_vectors():
    if not VECTORS_PATH.exists():
        pytest.skip("shared/test-vectors.json not present yet (W5 owns shared/)")
    if not NATIVE_AVAILABLE:
        pytest.skip("native wheel not built (core missing)")
    data = json.loads(VECTORS_PATH.read_text())
    assert isinstance(data, dict) and data.get("cases") and data.get("fixtures")
    return data


def _resolve_fixtures(data, tmpdir):
    """Name -> bytes for every fixture in the vectors file."""
    out = {}
    cli = _cli_binary()
    for fixture in data["fixtures"]:
        name = fixture["name"]
        kind = fixture.get("kind")
        if kind == "empty":
            out[name] = b""
        elif kind == "garbage":
            out[name] = GARBAGE_TEXT
        elif fixture.get("file"):
            path = REPO_ROOT / fixture["file"]
            assert path.is_file(), f"committed fixture missing: {path}"
            out[name] = path.read_bytes()
        elif fixture.get("generate", "").startswith("dusimagez mock"):
            if cli is None:
                continue  # unresolved: cases using it skip below
            # Generate strings use the real mock CLI flags: --kind /
            # --width / --height / --out (format follows the
            # extension, fixed seed). Parse --out (legacy --output
            # as fallback).
            parts = shlex.split(fixture["generate"])

            def _flag(name):
                return parts[parts.index(name) + 1] if name in parts else None

            ext = Path(_flag("--out") or _flag("--output") or "out.png").suffix or ".png"
            target = Path(tmpdir) / f"{name}{ext}"
            argv = [str(cli), "mock",
                    "--kind", _flag("--kind"),
                    "--width", _flag("--width"),
                    "--height", _flag("--height"),
                    "--out", str(target)]
            proc = subprocess.run(argv, capture_output=True, text=True, timeout=120)
            assert proc.returncode == 0, \
                f"mock generate failed for {name}: {proc.stderr}"
            out[name] = target.read_bytes()
        else:  # pragma: no cover - unknown fixture shape
            raise AssertionError(f"unknown fixture shape: {fixture}")
    return out


def _check_expect(case_id, data, options, expect, out, input_len, report=None):
    """Assert one case's `expect` against output bytes (and a report)."""
    if "error_code" in expect:
        raise AssertionError(f"{case_id}: expected error {expect['error_code']} but op succeeded")
    if report is None and ("quality_used" in expect or "resized" in expect):
        report = dusimagez.compress_with_report(data, **options)
    if report is not None:
        if "quality_used" in expect:
            assert report.quality_used == expect["quality_used"], \
                f"{case_id}: quality_used {report.quality_used} != {expect['quality_used']}"
        if "resized" in expect:
            assert report.resized == expect["resized"], \
                f"{case_id}: resized {report.resized} != {expect['resized']}"
    if "max_output_bytes" in expect:
        assert len(out) <= expect["max_output_bytes"], \
            f"{case_id}: output {len(out)} bytes > max {expect['max_output_bytes']}"
    if "min_saved_percent" in expect:
        saved = 100.0 * (input_len - len(out)) / input_len if input_len else 0.0
        assert saved >= expect["min_saved_percent"], \
            f"{case_id}: saved {saved:.1f}% < min {expect['min_saved_percent']}%"
    if any(k in expect for k in ("format", "width", "height")):
        info = dusimagez.get_image_info(out)
        if "format" in expect:
            assert info.format == expect["format"], \
                f"{case_id}: format {info.format} != {expect['format']}"
        if "width" in expect:
            assert info.width == expect["width"], \
                f"{case_id}: width {info.width} != {expect['width']}"
        if "height" in expect:
            assert info.height == expect["height"], \
                f"{case_id}: height {info.height} != {expect['height']}"


def _run_case(case, fixtures):
    case_id = case["id"]
    op = case["op"]
    options = dict(case.get("options") or {})
    expect = case.get("expect") or {}
    if op == "bulk":
        names = case["fixtures"]
        missing = [n for n in names if n not in fixtures]
        if missing:
            pytest.skip(f"{case_id}: fixtures not generatable yet: {missing}")
        inputs = [fixtures[n] for n in names]
        results = dusimagez.bulk_compress(inputs, **options)
        assert len(results) == len(case["expect"]["results"]), case_id
        for item, want in zip(results, case["expect"]["results"]):
            if "error_code" in want:
                assert item["report"] is None, f"{case_id}: expected failure"
                assert item["error_code"] == want["error_code"], \
                    f"{case_id}: error {item['error_code']} != {want['error_code']}"
            else:
                assert item["report"] is not None, f"{case_id}: expected success"
                _check_expect(case_id, inputs[item["index"]], options, want,
                              item["report"].output, len(inputs[item["index"]]),
                              report=item["report"])
        return
    name = case["fixture"]
    if name not in fixtures:
        pytest.skip(f"{case_id}: fixture {name} not generatable yet (mock CLI not built?)")
    data = fixtures[name]
    try:
        if op == "compress":
            out = dusimagez.compress_image(data, **options)
            _check_expect(case_id, data, options, expect, out, len(data))
        elif op == "compress_to_size":
            target = options.pop("target_bytes")
            out = dusimagez.compress_to_size(data, target, **options)
            _check_expect(case_id, data, options, expect, out, len(data))
        elif op == "compress_with_report":
            report = dusimagez.compress_with_report(data, **options)
            _check_expect(case_id, data, options, expect, report.output,
                          len(data), report=report)
        elif op == "get_info":
            info = dusimagez.get_image_info(data)
            if "format" in expect:
                assert info.format == expect["format"], case_id
            if "width" in expect:
                assert info.width == expect["width"], case_id
            if "height" in expect:
                assert info.height == expect["height"], case_id
        else:  # pragma: no cover - unknown op
            pytest.skip(f"{case_id}: op {op!r} not covered by the Python runner")
    except dusimagez.DusimagezError as err:
        assert err.code == expect.get("error_code"), \
            f"{case_id}: error {err.code} != expected {expect.get('error_code')}"


def test_shared_vectors():
    data = _load_vectors()
    with tempfile.TemporaryDirectory(prefix="dusimagez-vectors-") as tmpdir:
        fixtures = _resolve_fixtures(data, tmpdir)
        for case in data["cases"]:
            _run_case(case, fixtures)
