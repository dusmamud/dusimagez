package com.dusimagez

import org.junit.Assume.assumeTrue
import org.junit.Test
import java.io.File

/**
 * Runs `shared/test-vectors.json` (W5's contract file) against the
 * Kotlin bindings — the same vectors Rust, Python, JS, Go, PHP and
 * Swift must all agree on (SPEC §5-6).
 *
 * Honest-skip policy (never a fake pass):
 * - Vectors file absent -> the whole test is skipped via a JUnit
 *   assumption (reported as skipped).
 * - A case whose fixture file is absent under `shared/fixtures/` ->
 *   that case is skipped and counted; the run prints
 *   `ran=X skipped=Y` and only fails on a real mismatch.
 * - An unknown `op` or an unreadable case -> skipped, counted, and
 *   named in the output (so a schema drift is visible, not silent).
 *
 * The expected schema (tolerant — only fields that are present are
 * checked; both snake_case and camelCase option keys are accepted):
 * ```
 * { "vectors": [ { "fixture": "gradient-64x48.png",
 *                  "op": "compress_image",
 *                  "options": { "quality": 70, "format": "jpeg" },
 *                  "target_bytes": 200000,            // compress_to_size only
 *                  "expected": { "format": "jpeg", "width": 64, "height": 48,
 *                                "max_size": 200000 } } ] }
 * ```
 * A top-level array is also accepted. `expected.error_code` asserts
 * the case throws that stable code.
 */
class TestVectorsTest {

    @Test
    fun runSharedVectors() {
        val vectorsFile = findVectorsFile()
        assumeTrue("shared/test-vectors.json not present yet (W5)", vectorsFile != null)
        NativeTestSupport.assumeAvailable()
        val root = TestJson.parse(vectorsFile!!.readText())
        val cases = TestJson.array(TestJson.obj(root)["vectors"].let { it ?: root })
        assumeTrue("no vector cases found in ${vectorsFile.path}", cases.isNotEmpty())

        var ran = 0
        var skipped = 0
        val skippedNames = ArrayList<String>()
        for ((i, rawCase) in cases.withIndex()) {
            val case = TestJson.obj(rawCase)
            val name = TestJson.string(case["name"]) ?: "case[$i]"
            val fixtureName = TestJson.string(case["fixture"])
            val op = TestJson.string(case["op"])
            if (fixtureName == null || op == null) {
                skipped++
                skippedNames.add("$name (missing fixture/op)")
                continue
            }
            val fixture = File(vectorsFile.parentFile, "fixtures/$fixtureName")
            if (!fixture.exists()) {
                skipped++
                skippedNames.add("$name (fixture $fixtureName absent)")
                continue
            }
            val input = fixture.readBytes()
            val options = optionsFromJson(TestJson.obj(case["options"]))
            val expected = TestJson.obj(case["expected"])
            try {
                runCase(op, input, options, case, expected)
                ran++
            } catch (e: UnsupportedOperationException) {
                // Unknown op: skip, don't fail (schema drift is visible).
                skipped++
                skippedNames.add("$name (${e.message})")
            }
        }
        println("dusimagez shared vectors (Kotlin): ran=$ran skipped=$skipped $skippedNames")
    }

    /**
     * Runs one vector case. When `expected.error_code` is set, the op
     * must throw [DusimagezException] with exactly that stable code;
     * otherwise the op must succeed and match the expected fields.
     */
    private fun runCase(
        op: String,
        input: ByteArray,
        options: CompressOptions,
        case: Map<String, Any?>,
        expected: Map<String, Any?>,
    ) {
        val expectedError = TestJson.string(expected["error_code"])
        try {
            dispatchOp(op, input, options, case, expected)
        } catch (e: DusimagezException) {
            if (expectedError != null) {
                org.junit.Assert.assertEquals(
                    "error_code for op '$op'",
                    expectedError,
                    e.code,
                )
                return
            }
            throw e
        }
        if (expectedError != null) {
            throw AssertionError("case expected error $expectedError but op '$op' succeeded")
        }
    }

    private fun dispatchOp(
        op: String,
        input: ByteArray,
        options: CompressOptions,
        case: Map<String, Any?>,
        expected: Map<String, Any?>,
    ) {
        when (op) {
            "compress_image" -> {
                val out = Dusimagez.compressImage(input, options)
                checkExpectedImage(out, expected)
            }
            "compress_to_size" -> {
                val target = TestJson.long(case["target_bytes"])
                    ?: TestJson.long(TestJson.obj(case["options"])["target_bytes"])
                    ?: throw UnsupportedOperationException("compress_to_size without target_bytes")
                val out = Dusimagez.compressToSize(input, target.toULong(), options)
                checkExpectedImage(out, expected)
                val maxSize = TestJson.long(expected["max_size"])
                if (maxSize != null) {
                    org.junit.Assert.assertTrue(
                        "output ${out.size} exceeds max_size $maxSize",
                        out.size <= maxSize,
                    )
                }
            }
            "compress_with_report" -> {
                val report = Dusimagez.compressWithReport(input, options)
                checkExpectedImage(report.output.toByteArray(), expected)
                TestJson.long(expected["quality_used"])?.let {
                    org.junit.Assert.assertEquals(it.toInt().toUByte(), report.qualityUsed)
                }
            }
            "get_image_info" -> {
                val info = Dusimagez.getImageInfo(input)
                checkExpectedInfo(info, expected)
            }
            "bulk_compress" -> {
                val results = Dusimagez.bulkCompress(listOf(input), options)
                org.junit.Assert.assertEquals(1, results.size)
                org.junit.Assert.assertTrue(results[0].ok)
                results[0].report?.let { checkExpectedImage(it.output.toByteArray(), expected) }
            }
            else -> throw UnsupportedOperationException("unknown op '$op'")
        }
    }

    private fun checkExpectedImage(out: ByteArray, expected: Map<String, Any?>) {
        if (expected.isEmpty()) return
        val info = Dusimagez.getImageInfo(out)
        checkExpectedInfo(info, expected)
    }

    private fun checkExpectedInfo(info: ImageInfo, expected: Map<String, Any?>) {
        TestJson.string(expected["format"])?.let {
            org.junit.Assert.assertEquals("format", it, info.format)
        }
        TestJson.long(expected["width"])?.let {
            org.junit.Assert.assertEquals("width", it.toInt().toUInt(), info.width)
        }
        TestJson.long(expected["height"])?.let {
            org.junit.Assert.assertEquals("height", it.toInt().toUInt(), info.height)
        }
    }

    /** Maps a JSON options object onto CompressOptions (tolerant of key casing). */
    private fun optionsFromJson(json: Map<String, Any?>): CompressOptions {
        if (json.isEmpty()) return CompressOptions()
        fun <T> pick(vararg keys: String): Any? = keys.firstNotNullOfOrNull { json[it] }
        val base = CompressOptions()
        return CompressOptions(
            quality = TestJson.long(pick("quality"))?.toInt()?.toUByte() ?: base.quality,
            format = TestJson.string(pick("format"))?.let(::parseImageFormat),
            progressive = TestJson.bool(pick("progressive")) ?: base.progressive,
            lossless = TestJson.bool(pick("lossless")) ?: base.lossless,
            effort = TestJson.long(pick("effort"))?.toInt()?.toUByte() ?: base.effort,
            subsampling = TestJson.string(pick("subsampling"))?.let(::parseSubsampling),
            pngCompressionLevel = TestJson.long(
                pick("png_compression_level", "pngCompressionLevel"),
            )?.toInt()?.toUByte() ?: base.pngCompressionLevel,
            palette = TestJson.bool(pick("palette")) ?: base.palette,
            preset = TestJson.string(pick("preset"))?.let(::parsePreset),
            targetBytes = TestJson.long(pick("target_bytes", "targetBytes"))?.toULong(),
            maxWidth = TestJson.long(pick("max_width", "maxWidth"))?.toInt()?.toUInt(),
            maxHeight = TestJson.long(pick("max_height", "maxHeight"))?.toInt()?.toUInt(),
            stripMetadata = TestJson.string(pick("strip_metadata", "stripMetadata"))
                ?.let(::parseStripMetadata) ?: base.stripMetadata,
            autoOrient = TestJson.bool(pick("auto_orient", "autoOrient")) ?: base.autoOrient,
            dpi = TestJson.obj(pick("dpi")).let { d ->
                val w = TestJson.long(d["width"])
                val h = TestJson.long(d["height"])
                if (w != null && h != null) Dpi(w.toInt().toUInt(), h.toInt().toUInt()) else null
            },
            maxPixels = TestJson.long(pick("max_pixels", "maxPixels"))?.toULong() ?: base.maxPixels,
            maxDimension = TestJson.long(pick("max_dimension", "maxDimension"))?.toInt()?.toUInt()
                ?: base.maxDimension,
        )
    }

    private fun parseImageFormat(s: String): ImageFormat = when (s.lowercase()) {
        "jpeg", "jpg" -> ImageFormat.JPEG
        "png" -> ImageFormat.PNG
        "webp" -> ImageFormat.WEB_P
        "avif" -> ImageFormat.AVIF
        "gif" -> ImageFormat.GIF
        "bmp" -> ImageFormat.BMP
        else -> throw UnsupportedOperationException("unknown format '$s'")
    }

    private fun parseSubsampling(s: String): Subsampling = when (s.uppercase()) {
        "S444" -> Subsampling.S444
        "S422" -> Subsampling.S422
        "S420" -> Subsampling.S420
        else -> throw UnsupportedOperationException("unknown subsampling '$s'")
    }

    private fun parsePreset(s: String): Preset = when (s.lowercase()) {
        "web" -> Preset.WEB
        "social" -> Preset.SOCIAL
        "print" -> Preset.PRINT
        "archive" -> Preset.ARCHIVE
        else -> throw UnsupportedOperationException("unknown preset '$s'")
    }

    private fun parseStripMetadata(s: String): StripMetadata = when (s.lowercase()) {
        "all" -> StripMetadata.ALL
        "keepexif" -> StripMetadata.KEEP_EXIF
        "keepicc" -> StripMetadata.KEEP_ICC
        "keepall" -> StripMetadata.KEEP_ALL
        else -> throw UnsupportedOperationException("unknown strip_metadata '$s'")
    }

    private fun findVectorsFile(): File? {
        val override = System.getProperty("dusimagez.repo.root")
            ?.takeIf { it.isNotEmpty() } ?: System.getenv("DUSIMAGEZ_REPO_ROOT")
        val starts = listOfNotNull(
            override?.let { File(it) },
            File(System.getProperty("user.dir")),
        )
        for (start in starts) {
            var dir: File? = start
            while (dir != null) {
                val candidate = File(dir, "shared/test-vectors.json")
                if (candidate.isFile) return candidate
                dir = dir.parentFile
            }
        }
        return null
    }
}
