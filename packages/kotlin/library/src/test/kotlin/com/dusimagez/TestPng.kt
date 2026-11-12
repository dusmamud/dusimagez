package com.dusimagez

import java.io.ByteArrayOutputStream
import java.util.zip.CRC32
import java.util.zip.Deflater
import kotlin.random.Random

/**
 * Tiny in-test PNG encoder (RGB8, filter 0 per scanline), so tests
 * need no binary blobs and no extra core API. Deterministic for a
 * given kind + size + seed — the same builders the CLI `mock` command
 * uses, reimplemented here in ~60 lines. The bytes are verified valid
 * by the core itself: every native test first round-trips the fixture
 * through `getImageInfo` and asserts the expected dimensions.
 */
object TestPng {

    /** Solid colour image. */
    fun solid(width: Int, height: Int, r: Int, g: Int, b: Int): ByteArray =
        encode(width, height) { _, _ -> Triple(r, g, b) }

    /** Smooth gradient (red across, green down, blue diagonal). */
    fun gradient(width: Int, height: Int): ByteArray =
        encode(width, height) { x, y ->
            Triple(
                if (width > 1) x * 255 / (width - 1) else 0,
                if (height > 1) y * 255 / (height - 1) else 0,
                (x + y) % 256,
            )
        }

    /** Photo-like noise: deliberately hard to compress, so
     * `compress_to_size` has something to work on. */
    fun noise(width: Int, height: Int, seed: Long): ByteArray {
        val random = Random(seed)
        return encode(width, height) { _, _ ->
            Triple(random.nextInt(256), random.nextInt(256), random.nextInt(256))
        }
    }

    private fun encode(
        width: Int,
        height: Int,
        pixel: (x: Int, y: Int) -> Triple<Int, Int, Int>,
    ): ByteArray {
        require(width > 0 && height > 0) { "width and height must be > 0" }
        val raw = ByteArrayOutputStream()
        for (y in 0 until height) {
            raw.write(0) // filter: None
            for (x in 0 until width) {
                val (r, g, b) = pixel(x, y)
                raw.write(r)
                raw.write(g)
                raw.write(b)
            }
        }
        val deflater = Deflater(6)
        deflater.setInput(raw.toByteArray())
        deflater.finish()
        val compressed = ByteArrayOutputStream()
        val buf = ByteArray(8192)
        while (!deflater.finished()) {
            val n = deflater.deflate(buf)
            compressed.write(buf, 0, n)
        }
        deflater.end()

        val out = ByteArrayOutputStream()
        out.write(byteArrayOf(0x89.toByte(), 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A)) // PNG signature
        val ihdr = ByteArrayOutputStream()
        ihdr.write(intBytes(width))
        ihdr.write(intBytes(height))
        ihdr.write(byteArrayOf(8, 2, 0, 0, 0)) // bit depth 8, colour type 2 (RGB)
        out.write(chunk("IHDR", ihdr.toByteArray()))
        out.write(chunk("IDAT", compressed.toByteArray()))
        out.write(chunk("IEND", ByteArray(0)))
        return out.toByteArray()
    }

    private fun intBytes(value: Int): ByteArray =
        byteArrayOf(
            (value ushr 24).toByte(),
            (value ushr 16).toByte(),
            (value ushr 8).toByte(),
            value.toByte(),
        )

    private fun chunk(type: String, data: ByteArray): ByteArray {
        val out = ByteArrayOutputStream()
        out.write(intBytes(data.size))
        val typeBytes = type.toByteArray(Charsets.US_ASCII)
        out.write(typeBytes)
        out.write(data)
        val crc = CRC32()
        crc.update(typeBytes)
        crc.update(data)
        out.write(intBytes(crc.value.toInt()))
        return out.toByteArray()
    }
}
