# dusimagez — Swift (iOS/macOS) bindings

UniFFI Swift bindings for the dusimagez Rust core. SwiftPM package.
Fully offline — no network at runtime.

> **macOS only.** iOS/macOS targets need macOS: build and test on
> `macos-latest` in CI (see `mobile.yml`). Do **not** claim a Linux
> iOS build — there is none.

## Layout

```
packages/swift/
  Package.swift                  # Dusimagez + DusimagezFFI targets
  Sources/Dusimagez/
    dusimagez.swift              # GENERATED — do not hand-edit
    DusimagezAPI.swift           # idiomatic Data-based wrapper
  Sources/DusimagezFFI/
    dusimagezFFI.h               # GENERATED C header (uniffi-bindgen)
    module.modulemap             # GENERATED modulemap
  Tests/DusimagezTests/
    DusimagezTests.swift         # XCTest
    TestPng.swift                # in-test PNG encoder
    TestVectors.swift            # shared/test-vectors.json runner
  scripts/regenerate-bindings.sh
```

The generated files are **checked in** on purpose: the package builds
without `uniffi-bindgen` at consumer build time. Regenerate only when
`crates/dusimagez-uniffi/src/dusimagez.udl` changes.

## Usage

```swift
import Dusimagez

let input = try Data(contentsOf: URL(fileURLWithPath: "input.jpg"))

// Default options (quality 80, keep input format).
let out = try DusimagezAPI.compressImage(input)

// To JPEG at quality 70, max 1600 px wide (downscale-only).
let jpeg = try DusimagezAPI.compressImage(
    input,
    options: CompressOptions(quality: 70, format: .jpeg, maxWidth: 1600)
)

// Full report: sizes, savings, actual quality, whether it resized.
let report = try DusimagezAPI.compressWithReport(input)
print("saved \(report.savedBytes) bytes (\(report.savedPercent)%)")

// Target size — throws DusimagezError.targetSizeUnreachable when
// even the lowest quality + downscale cannot reach it.
let small = try DusimagezAPI.compressToSize(input, targetBytes: 200_000)

// Info without decoding the whole image into pixels.
let info = try DusimagezAPI.getImageInfo(input)
print("\(info.format) \(info.width)x\(info.height)")

// Batch: one result per input, same order; a bad item fails only
// its own entry (ok == false + errorCode), never the whole batch.
let results = DusimagezAPI.bulkCompress([input, Data()])
```

Errors are `DusimagezError` (generated — Swift keeps the UDL name,
unlike Kotlin, which names it `DusimagezException`), one case per
stable code, each carrying `code` (e.g. `"INVALID_IMAGE"`),
`messageEn` and `messageHi`. The wrapper rethrows them unchanged,
and adds a `DusimagezError.code` / `.messageEn` / `.messageHi`
computed property so callers can read the stable code without a
`switch` over all eight cases. Enum cases are lowerCamelCase
(`.jpeg`, `.webP`, `.s420`, `.keepExif`) — the UDL variant names
are preserved in meaning, re-cased to Swift convention.

Defaults come from the UDL (`CompressOptions` defaults match SPEC §2),
with two notes:
- `subsampling` defaults to `nil`, which means "let the core apply
  the quality-dependent rule" (S420 for JPEG quality < 90, S444
  otherwise). Do not substitute your own default.
- `dpi` is a `Dpi(width:height:)` record or `nil` (Rust tuples do
  not exist in UDL).

## Build & test

```bash
cd packages/swift
# 1. Build the Rust static library first (the scaffolding crate
#    crates/dusimagez-uniffi — its [lib] name is uniffi_dusimagez).
#    CI does this on macos-latest before running swift test.
cargo build --release -p dusimagez-uniffi
# 2. Test, pointing the linker at the .a:
swift test -Xlinker -L../../target/release -Xlinker -luniffi_dusimagez
```

Requirements: macOS 12+ / iOS 15+, Xcode 15+ (Swift 5.9+), Rust
1.88+ with the target above (`rustup target add
aarch64-apple-darwin` for Apple Silicon; `x86_64-apple-darwin` for
Intel). For a real iOS device/simulator build, build the static
library for `aarch64-apple-ios` / `aarch64-apple-ios-sim` (e.g. via
`cargo-lipo`) and link that instead — the Swift code is unchanged.

If the static library is missing, linking fails loudly — the tests
never silently pass without the real core.

## Regenerate the bindings

```bash
packages/swift/scripts/regenerate-bindings.sh   # from the repo root
```

Requires `uniffi-bindgen` 0.29 (same version the core's `uniffi`
dependency pins):

```bash
cargo install uniffi-bindgen --version 0.29
```

After regenerating, check that the wrapper (`DusimagezAPI.swift`) and
the tests still compile — a UDL change (new field, renamed variant)
shows up here first.

## Tests

`Tests/DusimagezTests/`:

- `DusimagezTests.swift` — round-trips on fixtures built in-test
  (see `TestPng.swift`): compress PNG→JPEG, report maths,
  `compress_to_size` guarantee + unreachable error, error codes
  (`EMPTY_INPUT`, `INVALID_IMAGE`, `INVALID_QUALITY`),
  `get_image_info`, bulk with one good + one bad input.
- `TestVectors.swift` — runs `shared/test-vectors.json` when it
  exists (W5's contract file): each case is looked up by fixture
  name under `shared/fixtures/`; cases whose fixture file is absent
  are **skipped with XCTSkip** (reported as skipped, never as
  passed) and a missing vectors file skips the whole test. Nothing
  here fakes a pass.
- `TestPng.swift` — a tiny in-test PNG encoder (solid / gradient /
  noise, deterministic seed) so tests need no binary blobs and no
  extra core API. Vectors are parsed with Foundation's
  `JSONSerialization`.

## Honest limits (v0.1.0, same as the core)

No HEIC. AVIF encode is lossy-only / 8-bit. GIF encode is
first-frame only for animated input. Whole image in memory, no
streaming. Android is the Kotlin module (`packages/kotlin`), not
this package.
