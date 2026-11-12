# dusimagez — Kotlin (Android) bindings

UniFFI Kotlin bindings for the dusimagez Rust core. Android library
module (`com.dusimagez`), shipped as an AAR. Fully offline — no
network at runtime.

## Layout

```
packages/kotlin/
  settings.gradle.kts            # includes :library
  build.gradle.kts               # AGP 8.5.2 / Kotlin 2.0.21
  library/
    build.gradle.kts             # com.dusimagez, minSdk 24, JNA dep
    src/main/AndroidManifest.xml
    src/main/kotlin/uniffi/dusimagez/dusimagez.kt   # GENERATED — do not hand-edit
    src/main/kotlin/com/dusimagez/Dusimagez.kt          # idiomatic ByteArray wrapper
    src/test/kotlin/com/dusimagez/                      # JUnit tests
  scripts/regenerate-bindings.sh
```

The generated file is **checked in** on purpose: the library builds
without `uniffi-bindgen` at consumer build time. Regenerate only when
`crates/dusimagez-uniffi/src/dusimagez.udl` changes.

## Usage

```kotlin
import com.dusimagez.Dusimagez
import com.dusimagez.CompressOptions
import com.dusimagez.ImageFormat

val input: ByteArray = File("input.jpg").readBytes()

// Default options (quality 80, keep input format).
val out: ByteArray = Dusimagez.compressImage(input)

// To JPEG at quality 70, max 1600 px wide (downscale-only).
val jpeg: ByteArray = Dusimagez.compressImage(
    input,
    CompressOptions(quality = 70u, format = ImageFormat.JPEG, maxWidth = 1600u)
)

// Full report: sizes, savings, actual quality, whether it resized.
val report = Dusimagez.compressWithReport(input)
println("saved ${report.savedBytes} bytes (${report.savedPercent}%)")

// Target size — fails with DusimagezError.TargetSizeUnreachable
// when even the lowest quality + downscale cannot reach it.
val small: ByteArray = Dusimagez.compressToSize(input, 200_000uL)

// Info without decoding the whole image into pixels.
val info = Dusimagez.getImageInfo(input)
println("${info.format} ${info.width}x${info.height}")

// Batch: one result per input, same order; a bad item fails only
// its own entry (ok == false + errorCode), never the whole batch.
val results = Dusimagez.bulkCompress(listOf(input, byteArrayOf()))
```

Errors are `DusimagezException` (generated, `uniffi.dusimagez` —
note the `Exception` suffix: UniFFI Kotlin names the error type
`<Name>Exception`, while Swift keeps `DusimagezError`), a sealed
class with one subclass per stable code, each carrying `code` (e.g.
`"INVALID_IMAGE"`), `messageEn` and `messageHi`. The wrapper
rethrows them unchanged, and adds `DusimagezException.code` /
`.messageEn` / `.messageHi` extensions so callers can read the
stable code without a `when` over all eight subclasses.

Defaults come from the UDL (`CompressOptions` defaults match SPEC §2),
with two notes:
- `subsampling` defaults to `null`, which means "let the core apply
  the quality-dependent rule" (S420 for JPEG quality < 90, S444
  otherwise). Do not substitute your own default.
- `dpi` is a `Dpi(width, height)` record or `null` (Rust tuples do
  not exist in UDL).

## Build

```bash
cd packages/kotlin
gradle :library:assembleRelease   # AAR in library/build/outputs/aar/
gradle :library:testDebugUnitTest # JUnit tests (needs the native lib, below)
```

Requirements: JDK 17, Android SDK (compileSdk 34, minSdk 24),
Gradle 8.7+ (no wrapper is committed; use the system Gradle or
`gradle/actions/setup-gradle` as CI does).

## The native library (required at runtime and for unit tests)

The Kotlin code calls the Rust core through JNI/JNA. The `.so` is
**not** in this repo; CI builds it from the Rust core and the AAR
ships one per ABI (`armeabi-v7a`, `arm64-v8a`, `x86`, `x86_64`).
The generated bindings load the library as `uniffi_dusimagez`, so
the cdylib that ships the UniFFI scaffolding must be named
`uniffi_dusimagez` (crate `[lib] name = "uniffi_dusimagez"`) —
consumers can override it with the
`uniffi.component.dusimagez.libraryOverride` system property.
For local JVM unit tests, build the host library first:

```bash
# From the repo root — the UniFFI scaffolding lives in
# crates/dusimagez-uniffi (its [lib] name is uniffi_dusimagez).
cargo build --release -p dusimagez-uniffi   # produces libuniffi_dusimagez.so
DUSIMAGEZ_NATIVE_DIR=/path/to/target/release \
  gradle :library:testDebugUnitTest
```

`DUSIMAGEZ_NATIVE_DIR` is read by `library/build.gradle.kts` and
passed to the tests as `jna.library.path`. If the library cannot be
loaded, the native tests are **skipped via JUnit assumptions**
(reported as skipped, never as passed) — build the cdylib and set
`DUSIMAGEZ_NATIVE_DIR` so they actually run. The pure-Kotlin tests
(PNG fixture encoder, vector-file parsing) run without the native
library.

## Regenerate the bindings

```bash
packages/kotlin/scripts/regenerate-bindings.sh   # from the repo root
```

Requires `uniffi-bindgen` 0.29 (same version the core's `uniffi`
dependency pins):

```bash
cargo install uniffi-bindgen --version 0.29
```

After regenerating, check that the wrapper (`Dusimagez.kt`) and the
tests still compile — a UDL change (new field, renamed variant) shows
up here first.

## Tests

`library/src/test/kotlin/com/dusimagez/`:

- `DusimagezTest.kt` — round-trips on fixtures built in-test (see
  `TestPng.kt`): compress PNG→JPEG, report maths, `compress_to_size`
  guarantee + unreachable error, error codes (`EMPTY_INPUT`,
  `INVALID_IMAGE`, `INVALID_QUALITY`), `get_image_info`, bulk with
  one good + one bad input.
- `TestVectorsTest.kt` — runs `shared/test-vectors.json` when it
  exists (W5's contract file): each case is looked up by fixture
  name under `shared/fixtures/`; cases whose fixture file is absent
  are **skipped via JUnit assumptions** (reported as skipped, never
  as passed) and a missing vectors file skips the whole test class.
  Nothing here fakes a pass.
- `TestPng.kt` — a tiny in-test PNG encoder (solid / gradient /
  noise, deterministic seed) so tests need no binary blobs and no
  extra core API. `TestJson.kt` — a minimal JSON parser for the
  vectors file (objects/arrays/strings/numbers/booleans/null).

## Honest limits (v0.1.0, same as the core)

No HEIC. AVIF encode is lossy-only / 8-bit. GIF encode is
first-frame only for animated input. Whole image in memory, no
streaming. iOS is the Swift package (`packages/swift`), not this
module.
