# dusimagez — PHP binding

PHP 8.1+ (FFI) wrapper over the dusimagez Rust core, via the C API in
`crates/dusimagez-ffi`. No image logic lives here — this package only
marshals options and results, so PHP results agree with Rust, Python,
JS, Go, Kotlin and Swift.

## Install — read this first: Composer-direct install is NOT possible

This package needs a compiled Rust shared library
(`libdusimagez_ffi.so` / `.dylib` / `dusimagez_ffi.dll`). Composer
cannot build or fetch that library, so `composer require
dusmamud/dusimagez` alone is **not** enough — the class will throw a
`RuntimeException` ("native library not found") until the library
exists. That is by design, and this README says so plainly instead of
pretending otherwise.

### Exact steps

```bash
# 1. Get the sources (the library is built from them, never downloaded
#    as a prebuilt binary) and install the PHP dependencies.
git clone https://github.com/dusmamud/dusimagez.git
cd dusimagez/packages/php
composer install

# 2. Build the native library (needs a Rust toolchain: https://rustup.rs).
bash scripts/build-ffi.sh
# -> ../../target/release/libdusimagez_ffi.so   (Linux)
# -> ../../target/release/libdusimagez_ffi.dylib (macOS)
# -> ../../target/release/dusimagez_ffi.dll      (Windows)

# 3. Use it. In a repo checkout the wrapper finds the library
#    automatically (target/release/ above). Otherwise point it at the
#    library explicitly — constructor argument, or the
#    DUSIMAGEZ_FFI_LIB environment variable, or copy the library to
#    packages/php/lib/:
cp ../../target/release/libdusimagez_ffi.so lib/
# or:
export DUSIMAGEZ_FFI_LIB=/path/to/libdusimagez_ffi.so
```

Requirements: PHP >= 8.1 with `ext-ffi` and `ext-json` enabled.
PHPUnit 10.5 is a dev dependency (`composer install` pulls it in).

## Use

```php
use Dusimagez\Dusimagez;

$dz = new Dusimagez();                       // or new Dusimagez('/path/to/libdusimagez_ffi.so')
$out = $dz->compress($bytes, ['quality' => 70]);
$rep = $dz->compressWithReport($bytes);      // report array, output in $rep['output']
$info = $dz->getInfo($bytes);                // format, width, height, has_alpha, colour_type, file_size
$out = $dz->compressToSize($bytes, 200_000); // at most 200 kB
$results = $dz->bulkCompress([$a, $bytes]);  // per-item results, same order
```

Options use snake_case keys matching the Rust `CompressOptions`
fields: `quality`, `format`, `progressive`, `lossless`, `effort`,
`subsampling`, `png_compression_level`, `palette`, `preset`,
`target_bytes`, `max_width`, `max_height`, `strip_metadata`,
`auto_orient`, `dpi`, `max_pixels`, `max_dimension`. Enums accept
lowercase spellings (`"jpeg"`, `"s420"`, `"social"`, `"keep-exif"`)
or the Rust spellings (`"Jpeg"`, `"S420"`, `"Social"`, `"KeepExif"`);
`"jpg"` is an alias of `"jpeg"`. `dpi` takes an int (both axes) or a
`[x, y]` pair. When `target_bytes` is set, `compress()` behaves like
`compressToSize()` (SPEC §2). Unknown option keys throw
`INVALID_ARGUMENT` instead of being silently ignored.

Errors: every failure throws a `DusimagezError` carrying the stable
`$code` (same codes as the Rust core, SPEC §4) plus `$messageEn` /
`$messageHi`:

```php
try {
    $dz->compress($bytes);
} catch (\Dusimagez\DusimagezError $e) {
    if ($e->code === 'INVALID_IMAGE') { /* ... */ }
    echo $e->messageHi;
}
```

## Tests

```bash
composer test   # vendor/bin/phpunit
```

`OptionsTest` is pure PHP (no library needed). `ApiTest` and
`VectorsTest` need the built library — when it is missing they
**skip with a clear message, never fake-pass**. `VectorsTest` runs
`shared/test-vectors.json` when it exists (W5's deliverable):
`solid` / `gradient` / `noise` fixtures come from the `dusimagez
mock` CLI (`cargo build -p dusimagez-cli --release`, or set
`$DUSIMAGEZ_CLI`); derived fixtures (jpeg, tiff, garbage, empty)
are built in-test. Without the CLI, fixtures are generated in-test
in pure PHP and size-bound assertions are skipped for those cases
(logged, never silently passed). If the vectors file itself is
missing, the test runs embedded-fixture fallback cases instead
(also logged).

## Honest limits (same as the Rust core, v0.1.0)

No HEIC. AVIF encode is lossy-only / 8-bit. GIF encode is
first-frame only for animated input. The whole image lives in
memory; there is no streaming mode.
