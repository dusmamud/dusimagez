<?php

declare(strict_types=1);

namespace Dusimagez\Tests;

use Dusimagez\Dusimagez;
use Dusimagez\DusimagezError;
use PHPUnit\Framework\TestCase;

/**
 * FFI-backed API tests. Skipped (never fake-passed) when the native
 * library is missing — build it with scripts/build-ffi.sh first.
 */
final class ApiTest extends TestCase
{
    private Dusimagez $dz;

    protected function setUp(): void
    {
        if (!Dusimagez::isLibAvailable()) {
            $this->markTestSkipped('dusimagez native library not built (bash packages/php/scripts/build-ffi.sh)');
        }
        $this->dz = new Dusimagez();
    }

    public function testVersion(): void
    {
        $this->assertSame('0.1.0', $this->dz->version());
    }

    public function testCompressPng(): void
    {
        $out = $this->dz->compress(Fixtures::png1x1());
        $this->assertNotSame('', $out);
        $this->assertSame("\x89PNG\r\n\x1a\n", substr($out, 0, 8));
    }

    public function testCompressWithReport(): void
    {
        $png = Fixtures::png1x1();
        $rep = $this->dz->compressWithReport($png);
        $this->assertSame(strlen($png), $rep['original_size']);
        $this->assertSame(strlen($rep['output']), $rep['output_size']);
        $this->assertSame('png', $rep['format']);
        $this->assertSame(1, $rep['width']);
        $this->assertSame(1, $rep['height']);
        $this->assertSame(80, $rep['quality_used']);
        $this->assertSame($rep['original_size'] - $rep['output_size'], $rep['saved_bytes']);
    }

    public function testCompressToSizeUnreachable(): void
    {
        // A 1x1 image cannot shrink below 10 bytes.
        try {
            $this->dz->compressToSize(Fixtures::png1x1(), 10);
            $this->fail('target 10 should be unreachable');
        } catch (DusimagezError $e) {
            $this->assertSame('TARGET_SIZE_UNREACHABLE', $e->code);
            $this->assertNotSame('', $e->messageEn);
            $this->assertNotSame('', $e->messageHi);
        }
    }

    public function testGetInfo(): void
    {
        $png = Fixtures::png1x1();
        $info = $this->dz->getInfo($png);
        $this->assertSame('png', $info['format']);
        $this->assertSame(1, $info['width']);
        $this->assertSame(1, $info['height']);
        $this->assertTrue($info['has_alpha']);
        $this->assertSame('la8', $info['colour_type']);
        $this->assertSame(strlen($png), $info['file_size']);
    }

    public function testGetInfoJpeg(): void
    {
        $info = $this->dz->getInfo(Fixtures::jpeg100x80());
        $this->assertSame('jpeg', $info['format']);
        $this->assertSame(100, $info['width']);
        $this->assertSame(80, $info['height']);
        $this->assertFalse($info['has_alpha']);
    }

    public function testEmptyInput(): void
    {
        try {
            $this->dz->compress('');
            $this->fail('empty input should throw');
        } catch (DusimagezError $e) {
            $this->assertSame('EMPTY_INPUT', $e->code);
        }
        try {
            $this->dz->getInfo('');
            $this->fail('empty getInfo should throw');
        } catch (DusimagezError $e) {
            $this->assertSame('EMPTY_INPUT', $e->code);
        }
    }

    public function testGarbageIsInvalidImage(): void
    {
        try {
            $this->dz->compress(Fixtures::garbage());
            $this->fail('garbage should throw');
        } catch (DusimagezError $e) {
            $this->assertSame('INVALID_IMAGE', $e->code);
        }
    }

    public function testInvalidQuality(): void
    {
        try {
            $this->dz->compress(Fixtures::png1x1(), ['quality' => 0]);
            $this->fail('quality 0 should throw');
        } catch (DusimagezError $e) {
            $this->assertSame('INVALID_QUALITY', $e->code);
        }
    }

    public function testBulkCompressMixed(): void
    {
        $results = $this->dz->bulkCompress([Fixtures::png1x1(), Fixtures::garbage()]);
        $this->assertCount(2, $results);
        $this->assertTrue($results[0]['ok']);
        $this->assertNotSame('', $results[0]['output']);
        $this->assertFalse($results[1]['ok']);
        $this->assertSame('INVALID_IMAGE', $results[1]['error']['code']);
        $this->assertSame([], $this->dz->bulkCompress([]));
    }
}
