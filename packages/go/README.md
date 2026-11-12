# dusimagez — Go binding

Go (cgo) wrapper over the dusimagez Rust core, via the C API in
`crates/dusimagez-ffi`. No image logic lives here — this package only
marshals options and results, so Go results agree with Rust, Python,
JS, PHP, Kotlin and Swift.

## Prerequisites

- Go 1.22+
- A C toolchain (cgo) and a Rust toolchain (cargo) to build the
  native library — the library is **not** bundled with this package.

## Build

```bash
bash scripts/build-ffi.sh   # cargo build -p dusimagez-ffi --release
go build ./...              # or: make build
```

`build-ffi.sh` leaves the static library at
`target/release/libdusimagez_ffi.a` (Linux/macOS) or
`target/release/dusimagez_ffi.lib` (Windows), which the package links
with `#cgo LDFLAGS` (see `ffi.go`). No prebuilt binary is ever
downloaded.

## Use

```go
import "github.com/dusmamud/dusimagez/go"

opts := dusimagez.DefaultCompressOptions()
opts.Quality = 70
out, err := dusimagez.Compress(input, &opts)
if err != nil {
    // *dusimagez.DusimagezError with a stable Code
    // (INVALID_IMAGE, EMPTY_INPUT, ...) + EN/HI messages.
    if errors.Is(err, dusimagez.ErrInvalidImage) { ... }
}

rep, err := dusimagez.CompressWithReport(input, nil) // nil = defaults
info, err := dusimagez.GetInfo(input)
out, err := dusimagez.CompressToSize(input, 200_000, nil)
results, err := dusimagez.BulkCompress(inputs, nil) // per-item results
```

Start from `DefaultCompressOptions()` and change what you need; a
nil `*CompressOptions` means "all defaults". Option fields that are
pointers (`Format`, `Preset`, `TargetBytes`, `MaxWidth`, `MaxHeight`,
`DPI`) are nil when unset. When `TargetBytes` is set, `Compress`
behaves like `CompressToSize` (SPEC §2).

Errors: every failure is a `*DusimagezError` carrying the stable
`Code` (same codes as the Rust core, SPEC §4) plus `MessageEn` /
`MessageHi`. Match by code with `errors.Is(err,
dusimagez.ErrInvalidImage)` — the `Is` method compares codes.

## Tests

```bash
go test ./...   # or: make test
```

`dusimagez_test.go` covers defaults, option validation, error
matching, and the native calls on an embedded 1x1 PNG.
`vectors_test.go` runs `shared/test-vectors.json` when it exists
(W5's deliverable): `solid` / `gradient` / `noise` fixtures come
from the `dusimagez mock` CLI (`cargo build -p dusimagez-cli`
`--release`, or set `$DUSIMAGEZ_CLI`); derived fixtures (jpeg, tiff,
garbage, empty) are built in-test. If the CLI is missing, fixtures
are generated in-test with `image/png` and size-bound assertions are
skipped for those cases (logged, never silently passed). If the
vectors file itself is missing, the test runs embedded-fixture
fallback cases instead (also logged).

## Honest limits (same as the Rust core, v0.1.0)

No HEIC. AVIF encode is lossy-only / 8-bit. GIF encode is
first-frame only for animated input. The whole image lives in
memory; there is no streaming mode.
