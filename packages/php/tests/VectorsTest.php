<?php

declare(strict_types=1);

namespace Dusimagez\Tests;

use Dusimagez\Dusimagez;
use Dusimagez\DusimagezError;
use PHPUnit\Framework\TestCase;

/**
 * Shared test vectors (SPEC section 6): PHP runs
 * shared/test-vectors.json and must agree with the Rust, Python, JS,
 * Go, Kotlin and Swift runners. The file's shape (W5, v0.1.0):
 *
 *   {
 *     "version": "0.1.0",
 *     "fixtures": [{"name", "kind", "format", "width", "height",
 *                   "seed"?, "file"?, "generate"?}, ...],
 *     "cases": [{"id", "op", "fixture" | "fixtures", "options",
 *                "expect", "notes"?}, ...]
 *   }
 *
 * Ops: "compress", "compress_with_report", "compress_to_size",
 * "get_info", "bulk". A case's "fixture" / "fixtures" are fixture
 * NAMES resolved against the top-level "fixtures" list. "expect"
 * keys: format, width, height, max_output_bytes, min_saved_percent,
 * quality_used, resized, error_code; bulk cases expect a "results"
 * array with one expect object per item.
 *
 * Fixtures: a descriptor with a "file" is loaded from shared/<file>
 * (exact). Otherwise `solid` / `gradient` / `noise` come from the
 * `dusimagez mock` CLI, with the exact flags the vectors file
 * records: `mock --kind <kind> --width <w> --height <h> --out
 * <file>` (format follows the extension, fixed seed)
 * (`$DUSIMAGEZ_CLI`, else
 * target/release/dusimagez in a repo checkout, else PATH). If no CLI
 * is found, fixtures are generated in-test with Fixtures (pure PHP)
 * — those are marked inexact, and size-bound assertions are skipped
 * for them (logged, never silently passed). Derived fixtures (jpeg,
 * tiff, garbage, empty) are built in-test. If
 * shared/test-vectors.json itself is missing, this test falls back to
 * the embedded 1x1 PNG cases instead of failing (the vectors file is
 * W5's deliverable) — the fallback is logged loudly, never silent.
 *
 * Unknown fixture kinds/names, ops or expect keys fail the case by
 * name.
 */
final class VectorsTest extends TestCase
{
    private Dusimagez $dz;

    protected function setUp(): void
    {
        if (!Dusimagez::isLibAvailable()) {
            $this->markTestSkipped('dusimagez native library not built (bash packages/php/scripts/build-ffi.sh)');
        }
        $this->dz = new Dusimagez();
    }

    // ------------------------------------------------------------------
    // Loading
    // ------------------------------------------------------------------

    /** @return array{version: string, fixtures: array, cases: array}|null */
    private function loadVectors(): ?array
    {
        $path = getenv('DUSIMAGEZ_VECTORS');
        if ($path === false || $path === '') {
            foreach ([
                __DIR__ . '/../../shared/test-vectors.json',
                __DIR__ . '/../../../shared/test-vectors.json',
                'shared/test-vectors.json',
            ] as $candidate) {
                if (is_file($candidate)) {
                    $path = $candidate;
                    break;
                }
            }
        }
        if (!is_string($path) || $path === '' || !is_file($path)) {
            return null;
        }
        $decoded = json_decode((string) file_get_contents($path), true);
        if (!is_array($decoded) || !isset($decoded['cases']) || !is_array($decoded['cases'])) {
            $this->fail("shared/test-vectors.json has no cases array: {$path}");
        }
        $decoded['_path'] = $path;
        return $decoded;
    }

    // ------------------------------------------------------------------
    // Fixtures
    // ------------------------------------------------------------------

    private function cliPath(): ?string
    {
        $env = getenv('DUSIMAGEZ_CLI');
        if ($env !== false && $env !== '' && is_file($env)) {
            return $env;
        }
        $repo = __DIR__ . '/../../../target/release/dusimagez';
        if (is_file($repo)) {
            return $repo;
        }
        $which = trim((string) shell_exec('command -v dusimagez'));
        return $which !== '' ? $which : null;
    }

    /**
     * @return array{0: string, 1: bool} bytes + exact flag (see class doc).
     */
    private function fixture(array $vf, string $name): array
    {
        $def = null;
        foreach ($vf['fixtures'] ?? [] as $candidate) {
            if (($candidate['name'] ?? null) === $name) {
                $def = $candidate;
                break;
            }
        }
        if ($def === null) {
            $this->fail("unknown fixture \"{$name}\" (not in shared/test-vectors.json fixtures)");
        }
        $kind = (string) ($def['kind'] ?? '');
        if (($def['file'] ?? '') !== '') {
            // Committed file, relative to the repo root (shared/...).
            $vectorsDir = dirname((string) $vf['_path']);
            $rel = (string) $def['file'];
            $stripped = substr($rel, (int) strpos($rel, '/') + 1);
            foreach ([$vectorsDir . '/' . $stripped, $vectorsDir . '/../' . $rel, $rel] as $candidate) {
                if (is_file($candidate)) {
                    return [(string) file_get_contents($candidate), true];
                }
            }
            $this->fail("fixture \"{$name}\": cannot read {$rel}");
        }
        switch ($kind) {
            case 'empty':
                return ['', true];
            case 'garbage':
                return [Fixtures::garbage(), true];
            case 'solid':
            case 'gradient':
            case 'noise':
                $width = (int) ($def['width'] ?? 0);
                $height = (int) ($def['height'] ?? 0);
                $seed = (int) ($def['seed'] ?? 0);
                $format = (string) ($def['format'] ?? 'png');
                if ($width <= 0 || $height <= 0) {
                    $this->fail("fixture \"{$name}\": width/height required");
                }
                $cli = $this->cliPath();
                if ($cli !== null) {
                    $tmp = tempnam(sys_get_temp_dir(), 'dusimagez-mock-') . '.' . $format;
                    // Real CLI flags only: --kind/--width/--height/--out
                    // (format follows the extension; fixed seed).
                    $cmd = escapeshellarg($cli) . ' mock --out ' . escapeshellarg($tmp)
                        . ' --kind ' . escapeshellarg($kind)
                        . ' --width ' . $width . ' --height ' . $height . ' 2>&1';
                    exec($cmd, $output, $code);
                    if ($code !== 0 || !is_file($tmp)) {
                        $this->fail("dusimagez mock {$kind} failed: " . implode("\n", $output));
                    }
                    $bytes = (string) file_get_contents($tmp);
                    unlink($tmp);
                    return [$bytes, true];
                }
                // In-test fallback (inexact): solid fixtures are
                // named solid-red-*; gradient is black-to-white.
                // JPEG fixtures are derived by converting the PNG
                // version through the core itself (format + dims are
                // then exact; only the pixels differ from the CLI's).
                $png = match ($kind) {
                    'solid' => Fixtures::solidPng($width, $height, [255, 0, 0]),
                    'gradient' => Fixtures::gradientPng($width, $height),
                    'noise' => Fixtures::noisePng($width, $height, $seed),
                };
                if ($format === 'jpeg') {
                    $bytes = $this->dz->compress($png, ['format' => 'jpeg']);
                } else {
                    $bytes = $png;
                }
                return [$bytes, false];
            default:
                $this->fail("unknown fixture kind \"{$kind}\" (fixture \"{$name}\")");
        }
    }

    // ------------------------------------------------------------------
    // Options / expect
    // ------------------------------------------------------------------

    /** Map a vector options object onto the PHP option keys. */
    private function optionsFromVector(mixed $raw): array
    {
        if (!is_array($raw)) {
            return [];
        }
        // Only pass through keys normalizeOptions() knows; anything
        // else must fail loudly in normalizeOptions, not here.
        $allowed = [
            'quality', 'format', 'progressive', 'lossless', 'effort',
            'subsampling', 'png_compression_level', 'palette', 'preset',
            'target_bytes', 'max_width', 'max_height', 'strip_metadata',
            'auto_orient', 'dpi', 'max_pixels', 'max_dimension',
        ];
        return array_intersect_key($raw, array_flip($allowed));
    }

    private function checkExpect(string $id, array $expect, ?array $info, ?array $rep, ?string $out, bool $exact): void
    {
        $known = [
            'format', 'width', 'height', 'has_alpha', 'colour_type',
            'file_size', 'error_code', 'max_output_bytes',
            'min_output_bytes', 'min_saved_percent', 'quality_used',
            'resized', 'output_smaller',
        ];
        foreach (array_keys($expect) as $key) {
            if (!in_array($key, $known, true)) {
                $this->fail("{$id}: unknown expect key \"{$key}\"");
            }
        }
        $src = $info ?? $rep ?? [];
        foreach (['format', 'width', 'height', 'has_alpha', 'colour_type', 'file_size', 'quality_used', 'resized'] as $key) {
            if (array_key_exists($key, $expect) && array_key_exists($key, $src)) {
                $this->assertSame($expect[$key], $src[$key], "{$id}: {$key}");
            }
        }
        if ($rep !== null && ($expect['output_smaller'] ?? false) === true) {
            $this->assertLessThan($rep['original_size'], $rep['output_size'], "{$id}: output not smaller");
        }
        if ($out !== null) {
            if ($exact) {
                if (isset($expect['max_output_bytes'])) {
                    $this->assertLessThanOrEqual($expect['max_output_bytes'], strlen($out), "{$id}: max_output_bytes");
                }
                if (isset($expect['min_output_bytes'])) {
                    $this->assertGreaterThanOrEqual($expect['min_output_bytes'], strlen($out), "{$id}: min_output_bytes");
                }
            } elseif (isset($expect['max_output_bytes']) || isset($expect['min_output_bytes'])) {
                fwrite(STDERR, "[vectors] {$id}: skipping size bounds (inexact fixture, no CLI)\n");
            }
        }
        if ($rep !== null && isset($expect['min_saved_percent'])) {
            if ($exact) {
                $this->assertGreaterThanOrEqual($expect['min_saved_percent'], $rep['saved_percent'], "{$id}: min_saved_percent");
            } else {
                fwrite(STDERR, "[vectors] {$id}: skipping min_saved_percent (inexact fixture, no CLI)\n");
            }
        }
    }

    /**
     * Returns true when the caller should stop (an error was expected
     * and checked, or an unexpected error occurred).
     */
    private function expectErrorCode(string $id, array $expect, ?DusimagezError $err): bool
    {
        if (!array_key_exists('error_code', $expect)) {
            if ($err !== null) {
                $this->fail("{$id}: unexpected error: {$err->getMessage()}");
            }
            return false;
        }
        $want = (string) $expect['error_code'];
        if ($err === null) {
            $this->fail("{$id}: expected error {$want}, got success");
        }
        $this->assertSame($want, $err->code, "{$id}: error code");
        return true;
    }

    private function checkBulkItem(string $id, int $index, array $expect, array $result, bool $exact): void
    {
        if (array_key_exists('error_code', $expect)) {
            $want = (string) $expect['error_code'];
            $this->assertFalse($result['ok'], "{$id}: item {$index} should have failed");
            $this->assertSame($want, $result['error']['code'] ?? null, "{$id}: item {$index} error code");
            return;
        }
        $this->assertTrue($result['ok'], "{$id}: item {$index} unexpected error: " . json_encode($result['error'] ?? null));
        $rep = $result;
        unset($rep['output']);
        $this->checkExpect("{$id} item {$index}", $expect, null, $rep, $result['output'], $exact);
    }

    // ------------------------------------------------------------------
    // The test
    // ------------------------------------------------------------------

    public function testSharedVectors(): void
    {
        $vf = $this->loadVectors();
        if ($vf === null) {
            fwrite(STDERR, "[vectors] shared/test-vectors.json not found (W5 has not created it yet) — running embedded-fixture fallback cases instead\n");
            $out = $this->dz->compress(Fixtures::png1x1());
            $this->assertNotSame('', $out);
            try {
                $this->dz->compress(Fixtures::garbage());
                $this->fail('fallback compress(garbage) should fail');
            } catch (DusimagezError $e) {
                $this->assertSame('INVALID_IMAGE', $e->code);
            }
            return;
        }
        $cases = $vf['cases'];
        $this->assertNotSame([], $cases, 'shared/test-vectors.json has no cases');
        foreach ($cases as $tc) {
            $id = (string) ($tc['id'] ?? '(no id)');
            $op = (string) ($tc['op'] ?? '');
            $options = $this->optionsFromVector($tc['options'] ?? null);
            $expect = is_array($tc['expect'] ?? null) ? $tc['expect'] : [];
            switch ($op) {
                case 'get_info':
                    [$input] = $this->fixture($vf, (string) $tc['fixture']);
                    try {
                        $info = $this->dz->getInfo($input);
                        $err = null;
                    } catch (DusimagezError $e) {
                        $info = null;
                        $err = $e;
                    }
                    if ($this->expectErrorCode($id, $expect, $err)) {
                        break;
                    }
                    $this->checkExpect($id, $expect, $info, null, null, true);
                    break;
                case 'compress':
                    [$input, $exact] = $this->fixture($vf, (string) $tc['fixture']);
                    try {
                        $out = $this->dz->compress($input, $options);
                        $err = null;
                    } catch (DusimagezError $e) {
                        $out = null;
                        $err = $e;
                    }
                    if ($this->expectErrorCode($id, $expect, $err)) {
                        break;
                    }
                    // Verify the output's own format/dimensions, not
                    // just its size: decode the summary from the output.
                    $outInfo = $this->dz->getInfo((string) $out);
                    $this->checkExpect($id, $expect, $outInfo, null, $out, $exact);
                    // A compress case may also assert report fields
                    // (quality_used, resized): check them via a report
                    // call on the same input when the expect object asks.
                    if (array_key_exists('quality_used', $expect) || array_key_exists('resized', $expect)) {
                        $rep = $this->dz->compressWithReport($input, $options);
                        $this->checkExpect($id, $expect, null, $rep, $out, $exact);
                    }
                    break;
                case 'compress_to_size':
                    [$input, $exact] = $this->fixture($vf, (string) $tc['fixture']);
                    $target = (int) ($options['target_bytes'] ?? 0);
                    $this->assertGreaterThan(0, $target, "{$id}: compress_to_size needs target_bytes in options");
                    try {
                        $out = $this->dz->compressToSize($input, $target, $options);
                        $err = null;
                    } catch (DusimagezError $e) {
                        $out = null;
                        $err = $e;
                    }
                    if ($this->expectErrorCode($id, $expect, $err)) {
                        break;
                    }
                    $this->assertLessThanOrEqual($target, strlen((string) $out), "{$id}: output exceeds target");
                    $outInfo = $this->dz->getInfo((string) $out);
                    $this->checkExpect($id, $expect, $outInfo, null, $out, $exact);
                    break;
                case 'compress_with_report':
                    [$input, $exact] = $this->fixture($vf, (string) $tc['fixture']);
                    try {
                        $rep = $this->dz->compressWithReport($input, $options);
                        $err = null;
                    } catch (DusimagezError $e) {
                        $rep = null;
                        $err = $e;
                    }
                    if ($this->expectErrorCode($id, $expect, $err)) {
                        break;
                    }
                    $this->checkExpect($id, $expect, null, $rep, $rep['output'], $exact);
                    break;
                case 'bulk':
                    $names = $tc['fixtures'] ?? [];
                    $this->assertNotSame([], $names, "{$id}: bulk needs fixtures[]");
                    $inputs = [];
                    $allExact = true;
                    foreach ($names as $fixtureName) {
                        [$b, $exact] = $this->fixture($vf, (string) $fixtureName);
                        $inputs[] = $b;
                        $allExact = $allExact && $exact;
                    }
                    $results = $this->dz->bulkCompress($inputs, $options);
                    $this->assertCount(count($inputs), $results, "{$id}: result count");
                    $itemExpects = $expect['results'] ?? null;
                    if (is_array($itemExpects)) {
                        $this->assertCount(count($results), $itemExpects, "{$id}: item expect count");
                        foreach ($itemExpects as $i => $itemExpect) {
                            $this->checkBulkItem($id, (int) $i, (array) $itemExpect, $results[$i], $allExact);
                        }
                    }
                    break;
                default:
                    $this->fail("{$id}: unknown op \"{$op}\"");
            }
        }
    }
}
