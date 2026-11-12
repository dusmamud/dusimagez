<?php

declare(strict_types=1);

namespace Dusimagez;

/**
 * dusimagez — offline image compressor (PHP FFI wrapper over the Rust core).
 *
 * The image work lives in the Rust core (`crates/dusimagez`), reached
 * here through the C API in `crates/dusimagez-ffi`. Every operation
 * takes bytes in and returns bytes out, synchronously — the FFI calls
 * are synchronous, so there is nothing to `await`.
 *
 * FFI contract (see `crates/dusimagez-ffi/dusimagez.h`; the whole C
 * declaration lives in the single `CDEF` constant below, so a name
 * fix is a one-line change):
 * - Image functions return a status int (`0` = OK, otherwise one of
 *   the `STATUS_CODES` below) and write their output to `*out` /
 *   `*out_len`. The wrapper reads the output bytes, frees the buffer
 *   with `dusimagez_free`, and returns the bytes as a PHP string. On
 *   a non-zero status it reads `dusimagez_last_error()` (a JSON string
 *   `{"code","message_en","message_hi"}`) and throws a
 *   `DusimagezError` — never a raw FFI error, never a raw JSON string.
 * - `compress_with_report` fills a caller-allocated
 *   `DusimagezReport` struct; `get_info` fills a `DusimagezInfo`
 *   struct. This wrapper converts them to PHP arrays.
 * - `bulk_compress` returns a JSON array of per-item results whose
 *   image outputs are base64-encoded; this wrapper decodes them to
 *   strings. One bad input fails only its own item.
 * - Options are plain PHP arrays with snake_case keys matching the
 *   Rust `CompressOptions` field names (see `SPEC.md` section 2 and
 *   `normalizeOptions()`): `quality`, `format`, `progressive`,
 *   `lossless`, `effort`, `subsampling`, `png_compression_level`,
 *   `palette`, `preset`, `target_bytes`, `max_width`, `max_height`,
 *   `strip_metadata`, `auto_orient`, `dpi`, `max_pixels`,
 *   `max_dimension`. Enums accept lowercase spellings (`"jpeg"`,
 *   `"s420"`, `"social"`, `"keep-exif"`) or the Rust spellings
 *   (`"Jpeg"`, `"S420"`, `"Social"`, `"KeepExif"`); `"jpg"` is
 *   accepted as an alias of `"jpeg"`. Unknown option keys throw
 *   `INVALID_ARGUMENT` instead of being silently ignored.
 * - When `target_bytes` is set in the options, `compress()` behaves
 *   like `compressToSize()` (SPEC section 2).
 *
 * Input rule: functions accept PHP strings only (enforced by the
 * `string` type hints — a wrong type raises a `TypeError`). Empty
 * strings are passed to the core, which reports `EMPTY_INPUT`.
 *
 * Honest limits (same as the Rust core, v0.1.0): NO HEIC. AVIF
 * encode is lossy-only / 8-bit. GIF encode is first-frame only for
 * animated input. The whole image lives in memory; no streaming.
 *
 * The native library is NOT bundled with this package (see
 * README.md): build it with `scripts/build-ffi.sh` first. The
 * library path resolves in this order: explicit constructor
 * argument, the `DUSIMAGEZ_FFI_LIB` environment variable,
 * `packages/php/lib/libdusimagez_ffi.so` (`.dylib` on macOS,
 * `dusimagez_ffi.dll` on Windows), then
 * `target/release/libdusimagez_ffi.so` in a repo checkout.
 */
final class Dusimagez
{
    /**
     * The whole FFI contract in one place. `uint8_t`, `int32_t`,
     * `uint32_t`, `uint64_t`, `int64_t` and `size_t` are PHP FFI's
     * predefined stdint types. The struct field order matches
     * `crates/dusimagez-ffi/dusimagez.h` exactly — do not reorder.
     */
    public const CDEF = <<<'C'
        typedef struct {
            int32_t quality;
            int32_t format;
            int32_t progressive;
            int32_t lossless;
            int32_t effort;
            int32_t subsampling;
            int32_t png_compression_level;
            int32_t palette;
            int32_t preset;
            int64_t target_bytes;
            int64_t max_width;
            int64_t max_height;
            int32_t strip_metadata;
            int32_t auto_orient;
            int64_t dpi_x;
            int64_t dpi_y;
            int64_t max_pixels;
            int64_t max_dimension;
        } DusimagezOptions;
        typedef struct {
            uint8_t *output;
            size_t output_len;
            uint64_t original_size;
            uint64_t output_size;
            uint64_t saved_bytes;
            float saved_percent;
            int32_t format;
            uint32_t width;
            uint32_t height;
            uint8_t quality_used;
            int32_t resized;
        } DusimagezReport;
        typedef struct {
            int32_t format;
            uint32_t width;
            uint32_t height;
            int32_t has_alpha;
            char colour_type[24];
            uint64_t file_size;
        } DusimagezInfo;
        int32_t dusimagez_compress(const uint8_t *input, size_t input_len, const DusimagezOptions *options, uint8_t **out, size_t *out_len);
        int32_t dusimagez_compress_to_size(const uint8_t *input, size_t input_len, uint64_t target_bytes, const DusimagezOptions *options, uint8_t **out, size_t *out_len);
        int32_t dusimagez_compress_with_report(const uint8_t *input, size_t input_len, const DusimagezOptions *options, DusimagezReport *report);
        int32_t dusimagez_get_info(const uint8_t *input, size_t input_len, DusimagezInfo *info);
        void dusimagez_free(uint8_t *ptr, size_t len);
        const char *dusimagez_last_error(void);
        const char *dusimagez_version(void);
        int32_t dusimagez_bulk_compress(const uint8_t *const *inputs, const size_t *input_lens, size_t count, const DusimagezOptions *options, uint8_t **out_json, size_t *out_len);
        C;

    /** FFI status code -> stable error code (crates/dusimagez-ffi/dusimagez.h). */
    private const STATUS_CODES = [
        1 => 'INVALID_IMAGE',
        2 => 'UNSUPPORTED_FORMAT',
        3 => 'IMAGE_TOO_LARGE',
        4 => 'TARGET_SIZE_UNREACHABLE',
        5 => 'INVALID_QUALITY',
        6 => 'INVALID_DIMENSIONS',
        7 => 'INVALID_ARGUMENT',
        8 => 'EMPTY_INPUT',
        99 => 'UNKNOWN',
    ];

    /** Format code -> lowercase name. */
    private const FORMAT_NAMES = [
        0 => '',
        1 => 'jpeg',
        2 => 'png',
        3 => 'webp',
        4 => 'avif',
        5 => 'gif',
        6 => 'bmp',
        7 => 'tiff',
    ];

    private \FFI $ffi;

    /**
     * @param string|null $libPath explicit path to the native library
     *   (libdusimagez_ffi.so / .dylib / dusimagez_ffi.dll). When null,
     *   see resolveLibPath() for the search order.
     */
    public function __construct(?string $libPath = null)
    {
        $this->ffi = \FFI::cdef(self::CDEF, self::resolveLibPath($libPath));
    }

    /**
     * Resolve the native library path. Throws RuntimeException with
     * the searched locations when nothing is found — build the
     * library first with scripts/build-ffi.sh (see README.md).
     */
    public static function resolveLibPath(?string $explicit = null): string
    {
        $tried = [];
        $ext = match (\PHP_OS_FAMILY) {
            'Windows' => 'dll',
            'Darwin' => 'dylib',
            default => 'so',
        };
        $file = $ext === 'dll' ? 'dusimagez_ffi.dll' : "libdusimagez_ffi.{$ext}";
        $candidates = [];
        if ($explicit !== null && $explicit !== '') {
            $candidates[] = $explicit;
        }
        $env = getenv('DUSIMAGEZ_FFI_LIB');
        if ($env !== false && $env !== '') {
            $candidates[] = $env;
        }
        $candidates[] = __DIR__ . "/lib/{$file}";
        $candidates[] = dirname(__DIR__, 3) . "/target/release/{$file}";
        foreach ($candidates as $candidate) {
            $tried[] = $candidate;
            if (is_file($candidate)) {
                return $candidate;
            }
        }
        throw new \RuntimeException(
            'dusimagez native library not found. Tried: ' . implode(', ', $tried)
            . '. Build it with: bash packages/php/scripts/build-ffi.sh',
        );
    }

    /** True when a native library can be found (see resolveLibPath). */
    public static function isLibAvailable(?string $libPath = null): bool
    {
        try {
            self::resolveLibPath($libPath);
            return true;
        } catch (\RuntimeException) {
            return false;
        }
    }

    /** Library version, e.g. "0.1.0" (lockstep with the core crate). */
    public function version(): string
    {
        // PHP FFI already converts a `char *` return to a PHP string
        // (null when the C function returns NULL) — do NOT wrap it in
        // FFI::string(), which requires CData and throws a TypeError
        // on a string. Same rule as dusrelimage's PHP wrapper.
        $ptr = $this->ffi->dusimagez_version();
        return $ptr === null ? '' : $ptr;
    }

    // ------------------------------------------------------------------
    // Options
    // ------------------------------------------------------------------

    /**
     * Normalise a user options array into the resolved C values
     * (keys are the DusimagezOptions struct field names). Pure PHP —
     * no FFI needed, so it is unit-testable on its own.
     *
     * Defaults (the Rust CompressOptions defaults): quality 80,
     * format keep (0), progressive/lossless/palette false, effort 6,
     * subsampling default (0), png_compression_level 6, preset none
     * (0), target_bytes unset (-1), max_width/max_height unset (-1),
     * strip_metadata All (0), auto_orient true, dpi unset (-1/-1),
     * max_pixels 120000000, max_dimension 16000.
     *
     * @throws DusimagezError with INVALID_QUALITY, INVALID_DIMENSIONS
     *   or INVALID_ARGUMENT on a bad value; unknown keys also throw
     *   INVALID_ARGUMENT instead of being silently ignored.
     */
    public static function normalizeOptions(array $options): array
    {
        $known = [
            'quality', 'format', 'progressive', 'lossless', 'effort',
            'subsampling', 'png_compression_level', 'palette', 'preset',
            'target_bytes', 'max_width', 'max_height', 'strip_metadata',
            'auto_orient', 'dpi', 'max_pixels', 'max_dimension',
        ];
        foreach (array_keys($options) as $key) {
            if (!in_array($key, $known, true)) {
                throw new DusimagezError(
                    'INVALID_ARGUMENT',
                    "Unknown option \"{$key}\".",
                    "Galat option hai.",
                );
            }
        }
        $o = $options;
        $out = [
            'quality' => 80,
            'format' => 0,
            'progressive' => 0,
            'lossless' => 0,
            'effort' => 6,
            'subsampling' => 0,
            'png_compression_level' => 6,
            'palette' => 0,
            'preset' => 0,
            'target_bytes' => -1,
            'max_width' => -1,
            'max_height' => -1,
            'strip_metadata' => 0,
            'auto_orient' => 1,
            'dpi_x' => -1,
            'dpi_y' => -1,
            'max_pixels' => 120000000,
            'max_dimension' => 16000,
        ];

        if (array_key_exists('quality', $o)) {
            $q = $o['quality'];
            if (!is_int($q) || $q < 1 || $q > 100) {
                throw new DusimagezError(
                    'INVALID_QUALITY',
                    "Quality {$q} is invalid. Quality must be between 1 and 100.",
                    'Quality galat hai. Quality 1 se 100 ke beech honi chahiye.',
                );
            }
            $out['quality'] = $q;
        }
        if (array_key_exists('format', $o)) {
            $out['format'] = self::formatCode($o['format']);
        }
        foreach (['progressive', 'lossless', 'palette'] as $boolKey) {
            if (array_key_exists($boolKey, $o)) {
                $out[$boolKey] = self::boolCode($o[$boolKey], $boolKey);
            }
        }
        if (array_key_exists('effort', $o)) {
            $e = $o['effort'];
            if (!is_int($e) || $e < 0 || $e > 10) {
                throw self::invalidArgument(
                    "Effort {$e} is invalid. Effort must be between 0 and 10.",
                    'Effort galat hai. Effort 0 se 10 ke beech hona chahiye.',
                );
            }
            $out['effort'] = $e;
        }
        if (array_key_exists('subsampling', $o)) {
            $out['subsampling'] = self::subsamplingCode($o['subsampling']);
        }
        if (array_key_exists('png_compression_level', $o)) {
            $l = $o['png_compression_level'];
            if (!is_int($l) || $l < 0 || $l > 9) {
                throw self::invalidArgument(
                    "PNG compression level {$l} is invalid. It must be between 0 and 9.",
                    'PNG compression level galat hai. Ye 0 se 9 ke beech hona chahiye.',
                );
            }
            $out['png_compression_level'] = $l;
        }
        if (array_key_exists('preset', $o)) {
            $out['preset'] = self::presetCode($o['preset']);
        }
        if (array_key_exists('target_bytes', $o)) {
            $t = $o['target_bytes'];
            if ($t === null) {
                $out['target_bytes'] = -1;
            } elseif (!is_int($t) || $t <= 0) {
                throw self::invalidArgument(
                    'target_bytes must be greater than zero.',
                    'target_bytes zero se bada hona chahiye.',
                );
            } else {
                $out['target_bytes'] = $t;
            }
        }
        foreach (['max_width', 'max_height'] as $dimKey) {
            if (array_key_exists($dimKey, $o)) {
                $d = $o[$dimKey];
                if ($d === null) {
                    $out[$dimKey] = -1;
                } elseif (!is_int($d) || $d <= 0) {
                    throw new DusimagezError(
                        'INVALID_DIMENSIONS',
                        "{$dimKey} must be greater than zero.",
                        'Galat dimension hai.',
                    );
                } else {
                    $out[$dimKey] = $d;
                }
            }
        }
        if (array_key_exists('strip_metadata', $o)) {
            $out['strip_metadata'] = self::stripCode($o['strip_metadata']);
        }
        if (array_key_exists('auto_orient', $o)) {
            $out['auto_orient'] = self::boolCode($o['auto_orient'], 'auto_orient');
        }
        if (array_key_exists('dpi', $o)) {
            [$x, $y] = self::dpiPair($o['dpi']);
            $out['dpi_x'] = $x;
            $out['dpi_y'] = $y;
        }
        if (array_key_exists('max_pixels', $o)) {
            $m = $o['max_pixels'];
            if (!is_int($m) || $m <= 0) {
                throw self::invalidArgument(
                    'max_pixels must be greater than zero.',
                    'max_pixels zero se bada hona chahiye.',
                );
            }
            $out['max_pixels'] = $m;
        }
        if (array_key_exists('max_dimension', $o)) {
            $m = $o['max_dimension'];
            if (!is_int($m) || $m <= 0) {
                throw self::invalidArgument(
                    'max_dimension must be greater than zero.',
                    'max_dimension zero se bada hona chahiye.',
                );
            }
            $out['max_dimension'] = $m;
        }
        return $out;
    }

    private static function invalidArgument(string $en, string $hi): DusimagezError
    {
        return new DusimagezError('INVALID_ARGUMENT', $en, $hi);
    }

    private static function boolCode(mixed $value, string $key): int
    {
        if ($value === true || $value === 1) {
            return 1;
        }
        if ($value === false || $value === 0) {
            return 0;
        }
        throw self::invalidArgument(
            "\"{$key}\" must be a bool.",
            'Galat bool value hai.',
        );
    }

    private static function formatCode(mixed $value): int
    {
        if ($value === null || $value === '' || $value === 'keep') {
            return 0;
        }
        if (is_int($value) && $value >= 0 && $value <= 7) {
            return $value;
        }
        if (is_string($value)) {
            $map = [
                'jpeg' => 1, 'jpg' => 1, 'png' => 2, 'webp' => 3,
                'avif' => 4, 'gif' => 5, 'bmp' => 6, 'tiff' => 7,
            ];
            $lower = strtolower($value);
            if (isset($map[$lower])) {
                return $map[$lower];
            }
        }
        throw self::invalidArgument(
            "Unknown image format \"{$value}\". Use jpeg, png, webp, avif, gif, bmp or tiff.",
            'Galat image format hai.',
        );
    }

    private static function subsamplingCode(mixed $value): int
    {
        if ($value === null || $value === '' || $value === 0) {
            return 0;
        }
        if (is_int($value) && $value >= 0 && $value <= 3) {
            return $value;
        }
        if (is_string($value)) {
            $map = ['444' => 1, 's444' => 1, '422' => 2, 's422' => 2, '420' => 3, 's420' => 3];
            $lower = strtolower($value);
            if (isset($map[$lower])) {
                return $map[$lower];
            }
        }
        throw self::invalidArgument(
            "Unknown subsampling \"{$value}\". Use 444, 422 or 420.",
            'Galat subsampling hai.',
        );
    }

    private static function presetCode(mixed $value): int
    {
        if ($value === null || $value === '' || $value === 'none') {
            return 0;
        }
        if (is_int($value) && $value >= 0 && $value <= 4) {
            return $value;
        }
        if (is_string($value)) {
            $map = ['web' => 1, 'social' => 2, 'print' => 3, 'archive' => 4];
            $lower = strtolower($value);
            if (isset($map[$lower])) {
                return $map[$lower];
            }
        }
        throw self::invalidArgument(
            "Unknown preset \"{$value}\". Use web, social, print or archive.",
            'Galat preset hai.',
        );
    }

    private static function stripCode(mixed $value): int
    {
        if ($value === null || $value === '' || $value === 'all') {
            return 0;
        }
        if (is_int($value) && $value >= 0 && $value <= 3) {
            return $value;
        }
        if (is_string($value)) {
            $map = [
                'keep-exif' => 1, 'keep_exif' => 1, 'keepexif' => 1,
                'keep-icc' => 2, 'keep_icc' => 2, 'keepicc' => 2,
                'keep-all' => 3, 'keep_all' => 3, 'keepall' => 3,
            ];
            $lower = strtolower($value);
            if (isset($map[$lower])) {
                return $map[$lower];
            }
        }
        throw self::invalidArgument(
            "Unknown strip_metadata \"{$value}\". Use all, keep-exif, keep-icc or keep-all.",
            'Galat strip_metadata hai.',
        );
    }

    /** @return array{0: int, 1: int} (-1/-1 when unset). */
    private static function dpiPair(mixed $value): array
    {
        if ($value === null) {
            return [-1, -1];
        }
        if (is_int($value)) {
            if ($value <= 0) {
                throw self::invalidArgument(
                    'dpi must be greater than zero.',
                    'dpi zero se bada hona chahiye.',
                );
            }
            return [$value, $value];
        }
        if (is_array($value) && count($value) === 2) {
            [$x, $y] = array_values($value);
            if (is_int($x) && is_int($y) && $x > 0 && $y > 0) {
                return [$x, $y];
            }
        }
        throw self::invalidArgument(
            'dpi must be an int, or a [x, y] pair of ints greater than zero.',
            'dpi ke liye ek int ya [x, y] pair chahiye.',
        );
    }

    private static function formatName(int $code): string
    {
        return self::FORMAT_NAMES[$code] ?? '';
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /**
     * Re-encode at a fixed quality (and `format`, or the input format
     * when unset). When `target_bytes` is set in `$options`, this
     * behaves like compressToSize() (SPEC section 2).
     */
    public function compress(string $input, array $options = []): string
    {
        $cOpts = $this->newOptions($options);
        $out = \FFI::new('uint8_t*[1]');
        $outLen = \FFI::new('size_t[1]');
        $status = $this->ffi->dusimagez_compress(
            $this->inputBuffer($input),
            strlen($input),
            \FFI::addr($cOpts),
            $out,
            $outLen,
        );
        $this->throwForStatus($status);
        return $this->takeBytes($out[0], (int) $outLen[0]);
    }

    /**
     * Re-encode so the result is at most `$targetBytes` bytes.
     * Throws TARGET_SIZE_UNREACHABLE when even the lowest quality
     * exceeds the target.
     */
    public function compressToSize(string $input, int $targetBytes, array $options = []): string
    {
        if ($targetBytes <= 0) {
            throw self::invalidArgument(
                'target_bytes must be greater than zero.',
                'target_bytes zero se bada hona chahiye.',
            );
        }
        $cOpts = $this->newOptions($options);
        $out = \FFI::new('uint8_t*[1]');
        $outLen = \FFI::new('size_t[1]');
        $status = $this->ffi->dusimagez_compress_to_size(
            $this->inputBuffer($input),
            strlen($input),
            $targetBytes,
            \FFI::addr($cOpts),
            $out,
            $outLen,
        );
        $this->throwForStatus($status);
        return $this->takeBytes($out[0], (int) $outLen[0]);
    }

    /**
     * Re-encode and return the before/after report:
     * `output`, `original_size`, `output_size`, `saved_bytes`,
     * `saved_percent`, `format`, `width`, `height`, `quality_used`,
     * `resized`.
     *
     * @return array{output: string, original_size: int, output_size: int, saved_bytes: int, saved_percent: float, format: string, width: int, height: int, quality_used: int, resized: bool}
     */
    public function compressWithReport(string $input, array $options = []): array
    {
        $cOpts = $this->newOptions($options);
        // Instance new(): static FFI::new() cannot see typedefs from
        // this instance's cdef (ParserException 'Undefined C type').
        $rep = $this->ffi->new('DusimagezReport');
        $status = $this->ffi->dusimagez_compress_with_report(
            $this->inputBuffer($input),
            strlen($input),
            \FFI::addr($cOpts),
            \FFI::addr($rep),
        );
        $this->throwForStatus($status);
        $output = $this->takeBytes($rep->output, (int) $rep->output_len);
        return [
            'output' => $output,
            'original_size' => (int) $rep->original_size,
            'output_size' => (int) $rep->output_size,
            'saved_bytes' => (int) $rep->saved_bytes,
            'saved_percent' => (float) $rep->saved_percent,
            'format' => self::formatName((int) $rep->format),
            'width' => (int) $rep->width,
            'height' => (int) $rep->height,
            'quality_used' => (int) $rep->quality_used,
            'resized' => (bool) $rep->resized,
        ];
    }

    /**
     * Image summary: `format`, `width`, `height`, `has_alpha`,
     * `colour_type`, `file_size`.
     *
     * @return array{format: string, width: int, height: int, has_alpha: bool, colour_type: string, file_size: int}
     */
    public function getInfo(string $input): array
    {
        // Instance new(): see compressWithReport() above.
        $info = $this->ffi->new('DusimagezInfo');
        $status = $this->ffi->dusimagez_get_info(
            $this->inputBuffer($input),
            strlen($input),
            \FFI::addr($info),
        );
        $this->throwForStatus($status);
        return [
            'format' => self::formatName((int) $info->format),
            'width' => (int) $info->width,
            'height' => (int) $info->height,
            'has_alpha' => (bool) $info->has_alpha,
            'colour_type' => \FFI::string($info->colour_type),
            'file_size' => (int) $info->file_size,
        ];
    }

    /**
     * Run compress over every input; one result per input, same
     * order. Each item is either
     * `['ok' => true, 'output' => ..., 'original_size' => ...,
     * 'output_size' => ..., 'saved_bytes' => ..., 'saved_percent' =>
     * ..., 'format' => ..., 'width' => ..., 'height' => ...,
     * 'quality_used' => ..., 'resized' => ...]` or
     * `['ok' => false, 'error' => ['code' => ..., 'message_en' =>
     * ..., 'message_hi' => ...]]`. An empty input list returns `[]`.
     */
    public function bulkCompress(array $inputs, array $options = []): array
    {
        $count = count($inputs);
        if ($count === 0) {
            return [];
        }
        $cOpts = $this->newOptions($options);
        $bufs = [];
        $ptrs = \FFI::new("uint8_t*[{$count}]");
        $lens = \FFI::new("size_t[{$count}]");
        foreach ($inputs as $i => $input) {
            if (!is_string($input)) {
                throw new DusimagezError(
                    'INVALID_TYPE',
                    'Bulk inputs must be strings.',
                    'Bulk inputs string hone chahiye.',
                );
            }
            // Keep each buffer alive for the whole call: $bufs holds
            // the owning CData, $ptrs only points at it.
            $bufs[$i] = $this->inputBuffer($input);
            // addr() of the array itself is uint8_t(*)[N]; the slot
            // wants uint8_t*, so take the address of element 0.
            $ptrs[$i] = $bufs[$i] === null ? null : \FFI::addr($bufs[$i][0]);
            $lens[$i] = strlen($input);
        }
        $out = \FFI::new('uint8_t*[1]');
        $outLen = \FFI::new('size_t[1]');
        $status = $this->ffi->dusimagez_bulk_compress($ptrs, $lens, $count, \FFI::addr($cOpts), $out, $outLen);
        $this->throwForStatus($status);
        $json = $this->takeBytes($out[0], (int) $outLen[0]);
        $items = json_decode($json, true);
        if (!is_array($items)) {
            throw new DusimagezError(
                'OPERATION_FAILED',
                'Could not parse bulk results.',
                'Bulk results parse nahi ho sake.',
            );
        }
        $results = [];
        foreach ($items as $item) {
            $index = (int) ($item['index'] ?? count($results));
            if (!($item['ok'] ?? false)) {
                $err = $item['error'] ?? [];
                $results[$index] = [
                    'ok' => false,
                    'error' => [
                        'code' => $err['code'] ?? 'UNKNOWN',
                        'message_en' => $err['message_en'] ?? '',
                        'message_hi' => $err['message_hi'] ?? '',
                    ],
                ];
                continue;
            }
            $output = base64_decode((string) ($item['output'] ?? ''), true);
            if ($output === false) {
                throw new DusimagezError(
                    'OPERATION_FAILED',
                    'Could not decode bulk output.',
                    'Bulk output decode nahi ho saka.',
                );
            }
            $results[$index] = [
                'ok' => true,
                'output' => $output,
                'original_size' => (int) ($item['original_size'] ?? 0),
                'output_size' => (int) ($item['output_size'] ?? 0),
                'saved_bytes' => (int) ($item['saved_bytes'] ?? 0),
                'saved_percent' => (float) ($item['saved_percent'] ?? 0.0),
                'format' => (string) ($item['format'] ?? ''),
                'width' => (int) ($item['width'] ?? 0),
                'height' => (int) ($item['height'] ?? 0),
                'quality_used' => (int) ($item['quality_used'] ?? 0),
                'resized' => (bool) ($item['resized'] ?? false),
            ];
        }
        ksort($results);
        return array_values($results);
    }

    // ------------------------------------------------------------------
    // Internals
    // ------------------------------------------------------------------

    /** Build a DusimagezOptions struct from a user options array. */
    private function newOptions(array $options): \FFI\CData
    {
        $norm = self::normalizeOptions($options);
        // Instance new(): static FFI::new() cannot see typedefs from
        // this instance's cdef (ParserException 'Undefined C type').
        $c = $this->ffi->new('DusimagezOptions');
        foreach ($norm as $key => $value) {
            $c->{$key} = $value;
        }
        return $c;
    }

    /**
     * Copy `$input` into a C buffer for the call, or null for an empty
     * string (the core then reports EMPTY_INPUT). The returned CData
     * must stay alive until the FFI call returns.
     */
    private function inputBuffer(string $input): ?\FFI\CData
    {
        $len = strlen($input);
        if ($len === 0) {
            return null;
        }
        $buf = \FFI::new("uint8_t[{$len}]");
        \FFI::memcpy($buf, $input, $len);
        return $buf;
    }

    /**
     * Copy `length` bytes out of a Rust-owned buffer, then free it.
     * The copy happens before the free, so the returned string is
     * fully PHP-owned.
     */
    private function takeBytes(mixed $ptr, int $length): string
    {
        if ($ptr === null || $length === 0) {
            return '';
        }
        $out = \FFI::string($ptr, $length);
        $this->ffi->dusimagez_free($ptr, $length);
        return $out;
    }

    /** Read dusimagez_last_error() (null when the last call succeeded). */
    private function lastError(): ?DusimagezError
    {
        $ptr = $this->ffi->dusimagez_last_error();
        if ($ptr === null) {
            return null;
        }
        // $ptr is already a PHP string here: dusimagez_last_error()
        // returns `const char *`, which PHP FFI converts to string
        // (see version() above) — FFI::string() would throw TypeError.
        $decoded = json_decode($ptr, true);
        if (!is_array($decoded) || empty($decoded['code'])) {
            return null;
        }
        return new DusimagezError(
            (string) $decoded['code'],
            (string) ($decoded['message_en'] ?? $decoded['reason_en'] ?? ''),
            (string) ($decoded['message_hi'] ?? $decoded['reason_hi'] ?? ''),
        );
    }

    /** Throw the DusimagezError for a non-zero FFI status. */
    private function throwForStatus(int $status): void
    {
        if ($status === 0) {
            return;
        }
        $err = $this->lastError();
        if ($err !== null) {
            throw $err;
        }
        $code = self::STATUS_CODES[$status] ?? 'OPERATION_FAILED';
        throw new DusimagezError($code, 'Operation failed.', 'Operation fail ho gaya.');
    }
}
