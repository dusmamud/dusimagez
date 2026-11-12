package com.dusimagez

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
// Nested error subclasses, imported directly: Kotlin cannot access a
// nested class through a typealias (`DusimagezException.InvalidImage`
// is an "unresolved reference") — same rule as in Dusimagez.kt.
import uniffi.dusimagez.DusimagezException.EmptyInput
import uniffi.dusimagez.DusimagezException.InvalidImage
import uniffi.dusimagez.DusimagezException.InvalidQuality
import uniffi.dusimagez.DusimagezException.TargetSizeUnreachable

/**
 * JUnit tests for the Kotlin bindings. Two groups:
 * (a) pure-Kotlin checks (UDL defaults, enum values) — run anywhere;
 * (b) native round-trips — need the real Rust core and are skipped
 *     (not passed) when the native library is absent; see
 *     [NativeTestSupport].
 *
 * Fixtures come from [TestPng] (in-test encoder, deterministic).
 * The shared contract file is covered separately by TestVectorsTest.
 */
class DusimagezTest {

    // ---- Pure-Kotlin checks (no native library needed) ----

    @Test
    fun defaultsMatchSpecSection2() {
        val o = CompressOptions()
        assertEquals(80u.toUByte(), o.quality)
        assertNull(o.format)
        assertFalse(o.progressive)
        assertFalse(o.lossless)
        assertEquals(6u.toUByte(), o.effort)
        // Conditional default: null = let the core apply the
        // quality-dependent rule (S420 < 90, S444 otherwise).
        assertNull(o.subsampling)
        assertEquals(6u.toUByte(), o.pngCompressionLevel)
        assertFalse(o.palette)
        assertNull(o.preset)
        assertNull(o.targetBytes)
        assertNull(o.maxWidth)
        assertNull(o.maxHeight)
        assertEquals(StripMetadata.ALL, o.stripMetadata)
        assertTrue(o.autoOrient)
        assertNull(o.dpi)
        assertEquals(120_000_000uL, o.maxPixels)
        assertEquals(16_000u, o.maxDimension)
    }

    @Test
    fun enumValuesMatchUdl() {
        // Kotlin cases are UPPER_SNAKE (verified against the
        // uniffi-bindgen 0.29.5 output): WebP becomes WEB_P.
        assertEquals(6, ImageFormat.values().size)
        assertTrue(ImageFormat.values().contains(ImageFormat.JPEG))
        assertTrue(ImageFormat.values().contains(ImageFormat.WEB_P))
        assertEquals(setOf(Subsampling.S444, Subsampling.S422, Subsampling.S420), Subsampling.values().toSet())
        assertEquals(setOf(Preset.WEB, Preset.SOCIAL, Preset.PRINT, Preset.ARCHIVE), Preset.values().toSet())
        assertEquals(
            setOf(StripMetadata.ALL, StripMetadata.KEEP_EXIF, StripMetadata.KEEP_ICC, StripMetadata.KEEP_ALL),
            StripMetadata.values().toSet(),
        )
        // Dpi is a plain record: Dpi(width, height).
        assertEquals(300u, Dpi(300u, 300u).width)
    }

    @Test
    fun testPngFixturesAreValidPng() {
        val png = TestPng.solid(2, 2, 255, 0, 0)
        assertEquals(0x89.toByte(), png[0])
        assertEquals('P'.code.toByte(), png[1])
        assertEquals('N'.code.toByte(), png[2])
        assertEquals('G'.code.toByte(), png[3])
        // Deterministic: same seed -> same bytes.
        assertTrue(TestPng.noise(32, 32, 42).contentEquals(TestPng.noise(32, 32, 42)))
        assertFalse(TestPng.noise(32, 32, 42).contentEquals(TestPng.noise(32, 32, 43)))
    }

    // ---- Native round-trips (need the Rust core) ----

    @Test
    fun compressPngKeepsDimensionsAndReportMaths() {
        NativeTestSupport.assumeAvailable()
        val input = TestPng.gradient(64, 48)
        val report = Dusimagez.compressWithReport(input)
        assertEquals(input.size.toLong(), report.originalSize.toLong())
        assertEquals(report.output.size.toLong(), report.outputSize.toLong())
        // saved_bytes is saturating: 0 when the output is larger than
        // the input (can happen for tiny images), never wrapped.
        val expectedSaved = if (report.outputSize <= report.originalSize) {
            report.originalSize - report.outputSize
        } else {
            0uL
        }
        assertEquals(expectedSaved, report.savedBytes)
        assertEquals(ImageFormat.PNG, report.format)
        assertEquals(64u, report.width)
        assertEquals(48u, report.height)
        assertEquals(80u.toUByte(), report.qualityUsed)
        assertFalse(report.resized)
        // saved_percent = 100 * saved / original (0 when nothing saved).
        val expected = if (report.originalSize > 0uL) {
            100.0f * report.savedBytes.toFloat() / report.originalSize.toFloat()
        } else {
            0.0f
        }
        assertEquals(expected, report.savedPercent, 0.01f)
        // The report output decodes to the same dimensions.
        val info = Dusimagez.getImageInfo(report.output.toByteArray())
        assertEquals("png", info.format)
        assertEquals(64u, info.width)
        assertEquals(48u, info.height)
    }

    @Test
    fun compressToJpeg() {
        NativeTestSupport.assumeAvailable()
        val input = TestPng.gradient(100, 80)
        val out = Dusimagez.compressImage(
            input,
            CompressOptions(quality = 70u, format = ImageFormat.JPEG),
        )
        val info = Dusimagez.getImageInfo(out)
        assertEquals("jpeg", info.format)
        assertEquals(100u, info.width)
        assertEquals(80u, info.height)
        assertFalse(info.hasAlpha)
        assertEquals(out.size.toLong(), info.fileSize.toLong())
    }

    @Test
    fun compressToSizeReachesTargetAndUnreachable() {
        NativeTestSupport.assumeAvailable()
        val input = TestPng.noise(400, 300, 7)
        val opts = CompressOptions(format = ImageFormat.JPEG, quality = 90u)
        val out = Dusimagez.compressToSize(input, 200_000uL, opts)
        assertTrue("output must fit the target, was ${out.size}", out.size <= 200_000)
        val info = Dusimagez.getImageInfo(out)
        assertEquals(400u, info.width)
        assertEquals(300u, info.height)
        try {
            Dusimagez.compressToSize(input, 100uL, opts)
            fail("expected TARGET_SIZE_UNREACHABLE")
        } catch (e: TargetSizeUnreachable) {
            assertEquals("TARGET_SIZE_UNREACHABLE", e.code)
            assertTrue(e.messageEn.isNotEmpty())
            assertTrue(e.messageHi.isNotEmpty())
        }
    }

    @Test
    fun emptyInputIsEmptyInputError() {
        NativeTestSupport.assumeAvailable()
        try {
            Dusimagez.compressImage(byteArrayOf())
            fail("expected EMPTY_INPUT")
        } catch (e: EmptyInput) {
            assertEquals("EMPTY_INPUT", e.code)
            assertTrue(e.messageEn.isNotEmpty())
            assertTrue(e.messageHi.isNotEmpty())
        }
    }

    @Test
    fun garbageInputIsInvalidImageError() {
        NativeTestSupport.assumeAvailable()
        try {
            Dusimagez.compressImage("this is not an image".toByteArray(Charsets.UTF_8))
            fail("expected INVALID_IMAGE")
        } catch (e: InvalidImage) {
            assertEquals("INVALID_IMAGE", e.code)
        }
    }

    @Test
    fun invalidQualityIsInvalidQualityError() {
        NativeTestSupport.assumeAvailable()
        try {
            Dusimagez.compressImage(TestPng.solid(32, 32, 1, 2, 3), CompressOptions(quality = 0u))
            fail("expected INVALID_QUALITY")
        } catch (e: InvalidQuality) {
            assertEquals("INVALID_QUALITY", e.code)
        }
    }

    @Test
    fun maxWidthDownscalesOnlyAndMarksResized() {
        NativeTestSupport.assumeAvailable()
        val input = TestPng.gradient(200, 100)
        val report = Dusimagez.compressWithReport(input, CompressOptions(maxWidth = 100u))
        assertEquals(100u, report.width)
        assertEquals(50u, report.height)
        assertTrue(report.resized)
        // Never upscale: maxWidth larger than the input is a no-op.
        val noUpscale = Dusimagez.compressWithReport(input, CompressOptions(maxWidth = 400u))
        assertEquals(200u, noUpscale.width)
        assertFalse(noUpscale.resized)
    }

    @Test
    fun bulkCompressPartialFailureNeverAbortsBatch() {
        NativeTestSupport.assumeAvailable()
        val good = TestPng.gradient(64, 48)
        val results = Dusimagez.bulkCompress(listOf(good, byteArrayOf()))
        assertEquals(2, results.size)
        assertEquals(0u, results[0].index)
        assertTrue(results[0].ok)
        assertNotNull(results[0].report)
        assertEquals(1u, results[1].index)
        assertFalse(results[1].ok)
        assertNull(results[1].report)
        assertEquals("EMPTY_INPUT", results[1].errorCode)
        assertTrue(results[1].errorMessageEn?.isNotEmpty() == true)
        assertTrue(results[1].errorMessageHi?.isNotEmpty() == true)
        // Empty batch -> empty result (core semantics).
        assertTrue(Dusimagez.bulkCompress(emptyList()).isEmpty())
    }

    @Test
    fun getImageInfoOnSolidPng() {
        NativeTestSupport.assumeAvailable()
        val input = TestPng.solid(1, 1, 255, 0, 0)
        val info = Dusimagez.getImageInfo(input)
        assertEquals("png", info.format)
        assertEquals(1u, info.width)
        assertEquals(1u, info.height)
        assertFalse(info.hasAlpha)
        assertTrue(info.colourType.isNotEmpty())
        assertEquals(input.size.toLong(), info.fileSize.toLong())
    }
}
