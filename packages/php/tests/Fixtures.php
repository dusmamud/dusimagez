<?php

declare(strict_types=1);

namespace Dusimagez\Tests;

/**
 * Test fixtures, built in pure PHP — no binary blobs are committed.
 *
 * - 1x1 greyscale+alpha PNG (68 bytes, colour type "la8" — the core
 *   lowercases it), embedded
 *   as base64 (the same fixture the Rust FFI tests use).
 * - 100x80 solid JPEG (quality 90), embedded as base64.
 * - PNG/TIFF fixtures are generated on the fly (solidPng,
 *   gradientPng, noisePng, tiffBytes) with ext-zlib only.
 */
final class Fixtures
{
    public const PNG_1X1_BASE64 = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR4nGNgYAAAAAMAAbitOmMAAAAASUVORK5CYII=';

    /** 100x80 solid JPEG (200,100,50), quality 90. */
    public const JPEG_100X80_BASE64 = '/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAMCAgMCAgMDAwMEAwMEBQgFBQQEBQoHBwYIDAoMDAsKCwsNDhIQDQ4RDgsLEBYQERMUFRUVDA8XGBYUGBIUFRT/2wBDAQMEBAUEBQkFBQkUDQsNFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBT/wAARCABQAGQDASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwCpRRRX4ef0gFFFFABRRRQAUUUUAFFFFABRRRQAUUUUAFFFFABRRRQAUUUUAFFFFABRRRQAUUUUAFFFFABRRRQAUUUUAFFFFABRRRQAUUUUAFFFFABRRRQAUUUUAFFFFABRRRQAUUUUAFFFFABRRRQAUUUUAFFFFABRRRQAUUUUAFFFFABRRRQAUUUUAFFFFAH/2Q==';

    public static function png1x1(): string
    {
        $bytes = base64_decode(self::PNG_1X1_BASE64, true);
        \assert($bytes !== false);
        return $bytes;
    }

    public static function jpeg100x80(): string
    {
        $bytes = base64_decode(self::JPEG_100X80_BASE64, true);
        \assert($bytes !== false);
        return $bytes;
    }

    public static function garbage(): string
    {
        return 'this is not an image at all, just plain text bytes 1234567890';
    }

    public static function empty(): string
    {
        return '';
    }

    /** Encode one PNG chunk (length + type + data + CRC32). */
    private static function pngChunk(string $type, string $data): string
    {
        return pack('N', strlen($data)) . $type . $data . pack('N', crc32($type . $data));
    }

    /** Build an 8-bit RGB PNG from a row generator. */
    private static function pngFromRows(int $width, int $height, callable $pixel): string
    {
        $raw = '';
        for ($y = 0; $y < $height; $y++) {
            $raw .= "\x00"; // filter: none
            for ($x = 0; $x < $width; $x++) {
                [$r, $g, $b] = $pixel($x, $y);
                $raw .= \chr($r) . \chr($g) . \chr($b);
            }
        }
        $ihdr = pack('NNCCCCC', $width, $height, 8, 2, 0, 0, 0);
        return "\x89PNG\r\n\x1a\n"
            . self::pngChunk('IHDR', $ihdr)
            . self::pngChunk('IDAT', (string) gzcompress($raw))
            . self::pngChunk('IEND', '');
    }

    public static function solidPng(int $width, int $height, array $rgb = [10, 20, 30]): string
    {
        return self::pngFromRows($width, $height, static fn () => $rgb);
    }

    public static function gradientPng(int $width, int $height): string
    {
        return self::pngFromRows($width, $height, static function (int $x) use ($width): array {
            $v = $width > 1 ? (int) ($x * 255 / ($width - 1)) : 0;
            return [$v, $v, $v];
        });
    }

    public static function noisePng(int $width, int $height, int $seed): string
    {
        mt_srand($seed);
        $pixels = [];
        for ($y = 0; $y < $height; $y++) {
            for ($x = 0; $x < $width; $x++) {
                $pixels[] = [mt_rand(0, 255), mt_rand(0, 255), mt_rand(0, 255)];
            }
        }
        $i = 0;
        return self::pngFromRows($width, $height, static function () use (&$i, $pixels): array {
            return $pixels[$i++];
        });
    }

    /** Minimal uncompressed little-endian TIFF (8-bit RGB, one strip). */
    public static function tiffBytes(int $width, int $height, array $rgb = [10, 20, 30]): string
    {
        $pixels = str_repeat(\chr($rgb[0]) . \chr($rgb[1]) . \chr($rgb[2]), $width * $height);
        $headerSize = 8;
        $ifdEntries = 10;
        $ifdSize = 2 + $ifdEntries * 12 + 4;
        $bitsOffset = $headerSize + $ifdSize;
        $dataOffset = $bitsOffset + 6;
        $entries = [
            [256, 4, 1, $width],
            [257, 4, 1, $height],
            [258, 3, 3, $bitsOffset],
            [259, 3, 1, 1],
            [262, 3, 1, 2],
            [273, 4, 1, $dataOffset],
            [277, 3, 1, 3],
            [278, 4, 1, $height],
            [279, 4, 1, strlen($pixels)],
            [284, 3, 1, 1],
        ];
        $out = "II*\x00" . pack('V', $headerSize) . pack('v', $ifdEntries);
        foreach ($entries as [$tag, $type, $count, $value]) {
            $out .= pack('vvVV', $tag, $type, $count, $value);
        }
        $out .= pack('V', 0);
        $out .= pack('vvv', 8, 8, 8);
        return $out . $pixels;
    }
}
