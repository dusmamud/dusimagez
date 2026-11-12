<?php

declare(strict_types=1);

namespace Dusimagez\Tests;

use Dusimagez\Dusimagez;
use Dusimagez\DusimagezError;
use PHPUnit\Framework\TestCase;

/**
 * Pure-PHP tests for option normalisation — no native library needed.
 */
final class OptionsTest extends TestCase
{
    public function testDefaults(): void
    {
        $o = Dusimagez::normalizeOptions([]);
        $this->assertSame(80, $o['quality']);
        $this->assertSame(0, $o['format']);
        $this->assertSame(0, $o['progressive']);
        $this->assertSame(0, $o['lossless']);
        $this->assertSame(6, $o['effort']);
        $this->assertSame(0, $o['subsampling']);
        $this->assertSame(6, $o['png_compression_level']);
        $this->assertSame(0, $o['palette']);
        $this->assertSame(0, $o['preset']);
        $this->assertSame(-1, $o['target_bytes']);
        $this->assertSame(-1, $o['max_width']);
        $this->assertSame(-1, $o['max_height']);
        $this->assertSame(0, $o['strip_metadata']);
        $this->assertSame(1, $o['auto_orient']);
        $this->assertSame(-1, $o['dpi_x']);
        $this->assertSame(-1, $o['dpi_y']);
        $this->assertSame(120000000, $o['max_pixels']);
        $this->assertSame(16000, $o['max_dimension']);
    }

    public function testFormatSpellings(): void
    {
        $this->assertSame(1, Dusimagez::normalizeOptions(['format' => 'jpeg'])['format']);
        $this->assertSame(1, Dusimagez::normalizeOptions(['format' => 'Jpeg'])['format']);
        $this->assertSame(1, Dusimagez::normalizeOptions(['format' => 'jpg'])['format']);
        $this->assertSame(2, Dusimagez::normalizeOptions(['format' => 'png'])['format']);
        $this->assertSame(3, Dusimagez::normalizeOptions(['format' => 'webp'])['format']);
        $this->assertSame(4, Dusimagez::normalizeOptions(['format' => 'avif'])['format']);
        $this->assertSame(7, Dusimagez::normalizeOptions(['format' => 'tiff'])['format']);
        $this->assertSame(0, Dusimagez::normalizeOptions(['format' => null])['format']);
        $this->assertSame(0, Dusimagez::normalizeOptions(['format' => 'keep'])['format']);
    }

    public function testEnumSpellings(): void
    {
        $this->assertSame(3, Dusimagez::normalizeOptions(['subsampling' => '420'])['subsampling']);
        $this->assertSame(3, Dusimagez::normalizeOptions(['subsampling' => 'S420'])['subsampling']);
        $this->assertSame(1, Dusimagez::normalizeOptions(['subsampling' => 's444'])['subsampling']);
        $this->assertSame(2, Dusimagez::normalizeOptions(['preset' => 'social'])['preset']);
        $this->assertSame(2, Dusimagez::normalizeOptions(['preset' => 'Social'])['preset']);
        $this->assertSame(4, Dusimagez::normalizeOptions(['preset' => 'archive'])['preset']);
        $this->assertSame(1, Dusimagez::normalizeOptions(['strip_metadata' => 'keep-exif'])['strip_metadata']);
        $this->assertSame(1, Dusimagez::normalizeOptions(['strip_metadata' => 'keep_exif'])['strip_metadata']);
        $this->assertSame(2, Dusimagez::normalizeOptions(['strip_metadata' => 'KeepIcc'])['strip_metadata']);
        $this->assertSame(3, Dusimagez::normalizeOptions(['strip_metadata' => 'keep-all'])['strip_metadata']);
    }

    public function testDpiForms(): void
    {
        $o = Dusimagez::normalizeOptions(['dpi' => 300]);
        $this->assertSame([300, 300], [$o['dpi_x'], $o['dpi_y']]);
        $o = Dusimagez::normalizeOptions(['dpi' => [72, 150]]);
        $this->assertSame([72, 150], [$o['dpi_x'], $o['dpi_y']]);
        $o = Dusimagez::normalizeOptions(['dpi' => null]);
        $this->assertSame([-1, -1], [$o['dpi_x'], $o['dpi_y']]);
    }

    public function testScalars(): void
    {
        $o = Dusimagez::normalizeOptions([
            'quality' => 70,
            'effort' => 3,
            'png_compression_level' => 9,
            'palette' => true,
            'progressive' => true,
            'auto_orient' => false,
            'target_bytes' => 200000,
            'max_width' => 800,
            'max_height' => 600,
            'max_pixels' => 1000,
            'max_dimension' => 500,
        ]);
        $this->assertSame(70, $o['quality']);
        $this->assertSame(3, $o['effort']);
        $this->assertSame(9, $o['png_compression_level']);
        $this->assertSame(1, $o['palette']);
        $this->assertSame(1, $o['progressive']);
        $this->assertSame(0, $o['auto_orient']);
        $this->assertSame(200000, $o['target_bytes']);
        $this->assertSame(800, $o['max_width']);
        $this->assertSame(600, $o['max_height']);
        $this->assertSame(1000, $o['max_pixels']);
        $this->assertSame(500, $o['max_dimension']);
    }

    public function testInvalidQuality(): void
    {
        $this->expectException(DusimagezError::class);
        $this->expectExceptionMessage('INVALID_QUALITY');
        Dusimagez::normalizeOptions(['quality' => 0]);
    }

    public function testInvalidQualityHigh(): void
    {
        try {
            Dusimagez::normalizeOptions(['quality' => 101]);
            $this->fail('quality 101 should throw');
        } catch (DusimagezError $e) {
            $this->assertSame('INVALID_QUALITY', $e->code);
        }
    }

    public function testUnknownOptionKey(): void
    {
        try {
            Dusimagez::normalizeOptions(['qualty' => 70]);
            $this->fail('unknown key should throw');
        } catch (DusimagezError $e) {
            $this->assertSame('INVALID_ARGUMENT', $e->code);
        }
    }

    public function testUnknownFormat(): void
    {
        try {
            Dusimagez::normalizeOptions(['format' => 'heic']);
            $this->fail('heic should throw');
        } catch (DusimagezError $e) {
            $this->assertSame('INVALID_ARGUMENT', $e->code);
        }
    }

    public function testBadDpi(): void
    {
        try {
            Dusimagez::normalizeOptions(['dpi' => [300, 0]]);
            $this->fail('dpi [300, 0] should throw');
        } catch (DusimagezError $e) {
            $this->assertSame('INVALID_ARGUMENT', $e->code);
        }
    }

    public function testZeroDimension(): void
    {
        try {
            Dusimagez::normalizeOptions(['max_width' => 0]);
            $this->fail('max_width 0 should throw');
        } catch (DusimagezError $e) {
            $this->assertSame('INVALID_DIMENSIONS', $e->code);
        }
    }
}
