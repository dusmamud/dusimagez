<?php

declare(strict_types=1);

// PHPUnit bootstrap: Composer autoloader (src/ + tests/), plus a
// warning — not a failure — when the native library is missing, so
// the pure-PHP tests (OptionsTest) can still run. Any test that needs
// the FFI library skips itself with a clear message instead (see
// ApiTest / VectorsTest) — never a fake pass.
require __DIR__ . '/../vendor/autoload.php';

use Dusimagez\Dusimagez;

if (!Dusimagez::isLibAvailable()) {
    fwrite(
        STDERR,
        "\n[phpunit bootstrap] WARNING: dusimagez native library not found.\n"
        . "[phpunit bootstrap] Build it with: bash packages/php/scripts/build-ffi.sh\n"
        . "[phpunit bootstrap] FFI-backed tests will be skipped until then.\n\n",
    );
}
