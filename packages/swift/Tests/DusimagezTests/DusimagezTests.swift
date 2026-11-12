import XCTest
@testable import Dusimagez

/// XCTest for the Swift bindings. Fixtures come from `TestPng`
/// (in-test encoder, deterministic). The shared contract file is
/// covered separately by `TestVectors`.
///
/// These tests need the real Rust core: the static library
/// (`libdusimagez.a` / `libuniffi_dusimagez.a`) is built by CI on
/// macos-latest and linked with `-Xlinker -L… -Xlinker
/// -ldusimagez` (see README.md). If the library is missing, linking
/// fails loudly — the tests never silently pass without the core.
final class DusimagezTests: XCTestCase {

    // ---- Pure-Swift checks (no native call) ----

    func testDefaultsMatchSpecSection2() {
        let o = CompressOptions()
        XCTAssertEqual(o.quality, 80)
        XCTAssertNil(o.format)
        XCTAssertFalse(o.progressive)
        XCTAssertFalse(o.lossless)
        XCTAssertEqual(o.effort, 6)
        // Conditional default: nil = let the core apply the
        // quality-dependent rule (S420 < 90, S444 otherwise).
        XCTAssertNil(o.subsampling)
        XCTAssertEqual(o.pngCompressionLevel, 6)
        XCTAssertFalse(o.palette)
        XCTAssertNil(o.preset)
        XCTAssertNil(o.targetBytes)
        XCTAssertNil(o.maxWidth)
        XCTAssertNil(o.maxHeight)
        XCTAssertEqual(o.stripMetadata, .all)
        XCTAssertTrue(o.autoOrient)
        XCTAssertNil(o.dpi)
        XCTAssertEqual(o.maxPixels, 120_000_000)
        XCTAssertEqual(o.maxDimension, 16_000)
    }

    func testEnumCasesMatchUdl() {
        // Swift cases are lowerCamelCase (verified against the
        // uniffi-bindgen 0.29.5 output): WebP becomes .webP.
        // (Generated UniFFI enums are plain enums, no rawValue.)
        let formats: [ImageFormat] = [.jpeg, .png, .webP, .avif, .gif, .bmp]
        XCTAssertEqual(formats.count, 6)
        XCTAssertEqual([Subsampling.s444, .s422, .s420].count, 3)
        XCTAssertEqual([Preset.web, .social, .print, .archive].count, 4)
        XCTAssertEqual(
            [StripMetadata.all, .keepExif, .keepIcc, .keepAll].count,
            4
        )
        XCTAssertEqual(Dpi(width: 300, height: 300).width, 300)
    }

    func testPngFixturesAreValidPng() {
        let png = TestPng.solid(width: 2, height: 2, r: 255, g: 0, b: 0)
        XCTAssertEqual([UInt8](png.prefix(4)), [0x89, 0x50, 0x4E, 0x47])
        // Deterministic: same seed -> same bytes.
        XCTAssertEqual(
            TestPng.noise(width: 32, height: 32, seed: 42),
            TestPng.noise(width: 32, height: 32, seed: 42)
        )
        XCTAssertNotEqual(
            TestPng.noise(width: 32, height: 32, seed: 42),
            TestPng.noise(width: 32, height: 32, seed: 43)
        )
    }

    // ---- Native round-trips (need the Rust core) ----

    func testCompressPngKeepsDimensionsAndReportMaths() throws {
        let input = TestPng.gradient(width: 64, height: 48)
        let report = try DusimagezAPI.compressWithReport(input)
        XCTAssertEqual(report.originalSize, UInt64(input.count))
        XCTAssertEqual(report.outputSize, UInt64(report.output.count))
        // savedBytes is saturating: 0 when the output is larger than
        // the input (can happen for tiny images), never wrapped.
        let expectedSaved: UInt64 = report.outputSize <= report.originalSize
            ? report.originalSize - report.outputSize
            : 0
        XCTAssertEqual(report.savedBytes, expectedSaved)
        XCTAssertEqual(report.format, .png)
        XCTAssertEqual(report.width, 64)
        XCTAssertEqual(report.height, 48)
        XCTAssertEqual(report.qualityUsed, 80)
        XCTAssertFalse(report.resized)
        // saved_percent = 100 * saved / original (0 when nothing saved).
        let expected: Float = report.originalSize > 0
            ? 100.0 * Float(report.savedBytes) / Float(report.originalSize)
            : 0.0
        XCTAssertEqual(report.savedPercent, expected, accuracy: 0.01)
        // The report output decodes to the same dimensions.
        let info = try DusimagezAPI.getImageInfo(Data(report.output))
        XCTAssertEqual(info.format, "png")
        XCTAssertEqual(info.width, 64)
        XCTAssertEqual(info.height, 48)
    }

    func testCompressToJpeg() throws {
        let input = TestPng.gradient(width: 100, height: 80)
        let out = try DusimagezAPI.compressImage(
            input,
            options: CompressOptions(quality: 70, format: .jpeg)
        )
        let info = try DusimagezAPI.getImageInfo(out)
        XCTAssertEqual(info.format, "jpeg")
        XCTAssertEqual(info.width, 100)
        XCTAssertEqual(info.height, 80)
        XCTAssertFalse(info.hasAlpha)
        XCTAssertEqual(info.fileSize, UInt64(out.count))
    }

    func testCompressToSizeReachesTargetAndUnreachable() throws {
        let input = TestPng.noise(width: 400, height: 300, seed: 7)
        let opts = CompressOptions(quality: 90, format: .jpeg)
        let out = try DusimagezAPI.compressToSize(input, targetBytes: 200_000, options: opts)
        XCTAssertLessThanOrEqual(out.count, 200_000)
        let info = try DusimagezAPI.getImageInfo(out)
        XCTAssertEqual(info.width, 400)
        XCTAssertEqual(info.height, 300)
        XCTAssertThrowsError(
            try DusimagezAPI.compressToSize(input, targetBytes: 100, options: opts)
        ) { error in
            guard let e = error as? DusimagezError else {
                return XCTFail("wrong error type: \(error)")
            }
            XCTAssertEqual(e.code, "TARGET_SIZE_UNREACHABLE")
            XCTAssertFalse(e.messageEn.isEmpty)
            XCTAssertFalse(e.messageHi.isEmpty)
        }
    }

    func testEmptyInputIsEmptyInputError() {
        XCTAssertThrowsError(try DusimagezAPI.compressImage(Data())) { error in
            guard let e = error as? DusimagezError else {
                return XCTFail("wrong error type: \(error)")
            }
            XCTAssertEqual(e.code, "EMPTY_INPUT")
            XCTAssertFalse(e.messageEn.isEmpty)
            XCTAssertFalse(e.messageHi.isEmpty)
        }
    }

    func testGarbageInputIsInvalidImageError() {
        XCTAssertThrowsError(
            try DusimagezAPI.compressImage(Data("this is not an image".utf8))
        ) { error in
            guard let e = error as? DusimagezError else {
                return XCTFail("wrong error type: \(error)")
            }
            XCTAssertEqual(e.code, "INVALID_IMAGE")
        }
    }

    func testInvalidQualityIsInvalidQualityError() {
        XCTAssertThrowsError(
            try DusimagezAPI.compressImage(
                TestPng.solid(width: 32, height: 32, r: 1, g: 2, b: 3),
                options: CompressOptions(quality: 0)
            )
        ) { error in
            guard let e = error as? DusimagezError else {
                return XCTFail("wrong error type: \(error)")
            }
            XCTAssertEqual(e.code, "INVALID_QUALITY")
        }
    }

    func testMaxWidthDownscalesOnlyAndMarksResized() throws {
        let input = TestPng.gradient(width: 200, height: 100)
        let report = try DusimagezAPI.compressWithReport(
            input,
            options: CompressOptions(maxWidth: 100)
        )
        XCTAssertEqual(report.width, 100)
        XCTAssertEqual(report.height, 50)
        XCTAssertTrue(report.resized)
        // Never upscale: maxWidth larger than the input is a no-op.
        let noUpscale = try DusimagezAPI.compressWithReport(
            input,
            options: CompressOptions(maxWidth: 400)
        )
        XCTAssertEqual(noUpscale.width, 200)
        XCTAssertFalse(noUpscale.resized)
    }

    func testBulkCompressPartialFailureNeverAbortsBatch() {
        let good = TestPng.gradient(width: 64, height: 48)
        let results = DusimagezAPI.bulkCompress([good, Data()])
        XCTAssertEqual(results.count, 2)
        XCTAssertEqual(results[0].index, 0)
        XCTAssertTrue(results[0].ok)
        XCTAssertNotNil(results[0].report)
        XCTAssertEqual(results[1].index, 1)
        XCTAssertFalse(results[1].ok)
        XCTAssertNil(results[1].report)
        XCTAssertEqual(results[1].errorCode, "EMPTY_INPUT")
        XCTAssertFalse(results[1].errorMessageEn?.isEmpty ?? true)
        XCTAssertFalse(results[1].errorMessageHi?.isEmpty ?? true)
        // Empty batch -> empty result (core semantics).
        XCTAssertTrue(DusimagezAPI.bulkCompress([]).isEmpty)
    }

    func testGetImageInfoOnSolidPng() throws {
        let input = TestPng.solid(width: 1, height: 1, r: 255, g: 0, b: 0)
        let info = try DusimagezAPI.getImageInfo(input)
        XCTAssertEqual(info.format, "png")
        XCTAssertEqual(info.width, 1)
        XCTAssertEqual(info.height, 1)
        XCTAssertFalse(info.hasAlpha)
        XCTAssertFalse(info.colourType.isEmpty)
        XCTAssertEqual(info.fileSize, UInt64(input.count))
    }
}
