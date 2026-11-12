# dusimagez-py

Python bindings for dusimagez — thin PyO3 wrappers over the Rust
core (`crates/dusimagez`), plus the friendly Python package shipped
in the wheel (`python/dusimagez/`).

## Build the wheel

```bash
# From this directory (needs the Rust core at ../dusimagez):
maturin develop          # install into the active venv, for tests
maturin build --release  # wheel in target/wheels/
```

## Layout

- `src/lib.rs` — the `_native` extension module: `compress_image`,
  `compress_to_size`, `compress_with_report`, `get_image_info`,
  `bulk_compress`. Options arrive as dicts (snake_case, partial —
  missing keys fall back to the Rust `Default`); friendly enum
  spellings (`"jpeg"`, `"s420"`, `"web"`, `"keep_exif"`) are
  normalised to the serde names.
- `python/dusimagez/__init__.py` — the friendly API: dataclass
  `CompressOptions`, `DusimagezError` (stable `.code` + EN/HI
  messages), kwargs-or-options calls, input validation.
- `python/dusimagez/pydantic.py` — optional Pydantic v2 models.
  Pydantic is never required to import or use the package.
- `tests/` — pytest: ops, options, errors, pydantic, report maths.
  Op tests need the built wheel (`maturin develop`); model/report
  tests run without it.

## Error contract

Every Rust `DusimagezError` crosses the boundary as a `ValueError`
with message `CODE\x1f message_en \x1f message_hi`; the Python
layer parses it into `DusimagezError`. Codes (SPEC §4):
`INVALID_IMAGE`, `UNSUPPORTED_FORMAT`, `IMAGE_TOO_LARGE`,
`TARGET_SIZE_UNREACHABLE`, `INVALID_QUALITY`, `INVALID_DIMENSIONS`,
`INVALID_ARGUMENT`, `EMPTY_INPUT`.

Core surface used (read from `crates/dusimagez/src/`): the core
types carry NO serde derives, so options are built field by field
in `src/lib.rs` and reports are converted to dicts by hand.
