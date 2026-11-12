import Foundation
import XCTest
@testable import Dusimagez

/// Runs `shared/test-vectors.json` (W5's contract file) against the
/// Swift bindings — the same vectors Rust, Python, JS, Go, PHP and
/// Kotlin must all agree on (SPEC §5-6).
///
/// Honest-skip policy (never a fake pass):
/// - Vectors file absent -> the whole test is skipped with XCTSkip.
/// - A case whose fixture file is absent under `shared/fixtures/` ->
///   that case is skipped and counted; the run prints
///   `ran=X skipped=Y` and only fails on a real mismatch.
/// - An unknown `op` or an unreadable case -> skipped, counted, and
///   named in the output (so a schema drift is visible, not silent).
///
/// The expected schema (tolerant — only fields that are present are
/// checked):
/// ```
/// { "vectors": [ { "fixture": "gradient-64x48.png",
///                  "op": "compress_image",
///                  "options": { "quality": 70, "format": "jpeg" },
///                  "target_bytes": 200000,            // compress_to_size only
///                  "expected": { "format": "jpeg", "width": 64, "height": 48,
///                                "max_size": 200000 } } ] }
/// ```
/// A top-level array is also accepted. `expected.error_code` asserts
/// the case throws that stable code.
final class TestVectors: XCTestCase {

    func testRunSharedVectors() throws {
        guard let vectorsFile = findVectorsFile() else {
            throw XCTSkip("shared/test-vectors.json not present yet (W5)")
        }
        let data = try Data(contentsOf: vectorsFile)
        let root = try JSONSerialization.jsonObject(with: data)
        let cases: [[String: Any]]
        if let dict = root as? [String: Any], let v = dict["vectors"] as? [[String: Any]] {
            cases = v
        } else if let arr = root as? [[String: Any]] {
            cases = arr
        } else {
            throw XCTSkip("no vector cases found in \(vectorsFile.path)")
        }
        guard !cases.isEmpty else {
            throw XCTSkip("no vector cases found in \(vectorsFile.path)")
        }

        var ran = 0
        var skipped = 0
        var skippedNames: [String] = []
        for (i, c) in cases.enumerated() {
            let name = c["name"] as? String ?? "case[\(i)]"
            guard
                let fixtureName = c["fixture"] as? String,
                let op = c["op"] as? String
            else {
                skipped += 1
                skippedNames.append("\(name) (missing fixture/op)")
                continue
            }
            let fixture = vectorsFile.deletingLastPathComponent()
                .appendingPathComponent("fixtures/\(fixtureName)")
            guard FileManager.default.fileExists(atPath: fixture.path) else {
                skipped += 1
                skippedNames.append("\(name) (fixture \(fixtureName) absent)")
                continue
            }
            let input = try Data(contentsOf: fixture)
            let options = optionsFromJson(c["options"] as? [String: Any] ?? [:])
            let expected = c["expected"] as? [String: Any] ?? [:]
            do {
                try runCase(op: op, input: input, options: options, case: c, expected: expected)
                ran += 1
            } catch VectorSkip.unknownOp(let message) {
                skipped += 1
                skippedNames.append("\(name) (\(message))")
            }
        }
        print("dusimagez shared vectors (Swift): ran=\(ran) skipped=\(skipped) \(skippedNames)")
    }

    private enum VectorSkip: Error {
        case unknownOp(String)
    }

    private func runCase(
        op: String,
        input: Data,
        options: CompressOptions,
        case c: [String: Any],
        expected: [String: Any]
    ) throws {
        let expectedError = expected["error_code"] as? String
        do {
            try dispatchOp(op: op, input: input, options: options, case: c, expected: expected)
        } catch let e as DusimagezError {
            if let expectedError {
                XCTAssertEqual(e.code, expectedError, "error_code for op '\(op)'")
                return
            }
            throw e
        }
        if let expectedError {
            XCTFail("case expected error \(expectedError) but op '\(op)' succeeded")
        }
    }

    private func dispatchOp(
        op: String,
        input: Data,
        options: CompressOptions,
        case c: [String: Any],
        expected: [String: Any]
    ) throws {
        switch op {
        case "compress_image":
            let out = try Dusimagez.compressImage(input, options: options)
            try checkExpectedImage(out, expected: expected)
        case "compress_to_size":
            guard
                let target = (c["target_bytes"] as? NSNumber)?.uint64Value
                    ?? (options.targetBytes)
            else {
                throw VectorSkip.unknownOp("compress_to_size without target_bytes")
            }
            let out = try Dusimagez.compressToSize(input, targetBytes: target, options: options)
            try checkExpectedImage(out, expected: expected)
            if let maxSize = (expected["max_size"] as? NSNumber)?.uint64Value {
                XCTAssertLessThanOrEqual(
                    out.count, Int(maxSize),
                    "output \(out.count) exceeds max_size \(maxSize)"
                )
            }
        case "compress_with_report":
            let report = try Dusimagez.compressWithReport(input, options: options)
            try checkExpectedImage(Data(report.output), expected: expected)
            if let qualityUsed = (expected["quality_used"] as? NSNumber)?.uint8Value {
                XCTAssertEqual(report.qualityUsed, qualityUsed)
            }
        case "get_image_info":
            let info = try Dusimagez.getImageInfo(input)
            checkExpectedInfo(info, expected: expected)
        case "bulk_compress":
            let results = Dusimagez.bulkCompress([input], options: options)
            XCTAssertEqual(results.count, 1)
            XCTAssertTrue(results[0].ok)
            if let report = results[0].report {
                try checkExpectedImage(Data(report.output), expected: expected)
            }
        default:
            throw VectorSkip.unknownOp("unknown op '\(op)'")
        }
    }

    private func checkExpectedImage(_ out: Data, expected: [String: Any]) throws {
        guard !expected.isEmpty else { return }
        let info = try Dusimagez.getImageInfo(out)
        checkExpectedInfo(info, expected: expected)
    }

    private func checkExpectedInfo(_ info: ImageInfo, expected: [String: Any]) {
        if let format = expected["format"] as? String {
            XCTAssertEqual(info.format, format, "format")
        }
        if let width = (expected["width"] as? NSNumber)?.uint32Value {
            XCTAssertEqual(info.width, width, "width")
        }
        if let height = (expected["height"] as? NSNumber)?.uint32Value {
            XCTAssertEqual(info.height, height, "height")
        }
    }

    /// Maps a JSON options object onto CompressOptions.
    private func optionsFromJson(_ json: [String: Any]) -> CompressOptions {
        guard !json.isEmpty else { return CompressOptions() }
        func num(_ keys: String...) -> NSNumber? {
            keys.compactMap { json[$0] as? NSNumber }.first
        }
        func str(_ keys: String...) -> String? {
            keys.compactMap { json[$0] as? String }.first
        }
        func flag(_ keys: String...) -> Bool? {
            keys.compactMap { json[$0] as? Bool }.first
        }
        let base = CompressOptions()
        var dpi: Dpi?
        if
            let d = json["dpi"] as? [String: Any],
            let w = (d["width"] as? NSNumber)?.uint32Value,
            let h = (d["height"] as? NSNumber)?.uint32Value
        {
            dpi = Dpi(width: w, height: h)
        }
        return CompressOptions(
            quality: num("quality")?.uint8Value ?? base.quality,
            format: str("format").flatMap(parseImageFormat),
            progressive: flag("progressive") ?? base.progressive,
            lossless: flag("lossless") ?? base.lossless,
            effort: num("effort")?.uint8Value ?? base.effort,
            subsampling: str("subsampling").flatMap(parseSubsampling),
            pngCompressionLevel: num("png_compression_level", "pngCompressionLevel")?.uint8Value
                ?? base.pngCompressionLevel,
            palette: flag("palette") ?? base.palette,
            preset: str("preset").flatMap(parsePreset),
            targetBytes: num("target_bytes", "targetBytes")?.uint64Value,
            maxWidth: num("max_width", "maxWidth")?.uint32Value,
            maxHeight: num("max_height", "maxHeight")?.uint32Value,
            stripMetadata: str("strip_metadata", "stripMetadata").flatMap(parseStripMetadata)
                ?? base.stripMetadata,
            autoOrient: flag("auto_orient", "autoOrient") ?? base.autoOrient,
            dpi: dpi,
            maxPixels: num("max_pixels", "maxPixels")?.uint64Value ?? base.maxPixels,
            maxDimension: num("max_dimension", "maxDimension")?.uint32Value ?? base.maxDimension
        )
    }

    private func parseImageFormat(_ s: String) -> ImageFormat? {
        switch s.lowercased() {
        case "jpeg", "jpg": return .jpeg
        case "png": return .png
        case "webp": return .webP
        case "avif": return .avif
        case "gif": return .gif
        case "bmp": return .bmp
        default: return nil
        }
    }

    private func parseSubsampling(_ s: String) -> Subsampling? {
        switch s.uppercased() {
        case "S444": return .s444
        case "S422": return .s422
        case "S420": return .s420
        default: return nil
        }
    }

    private func parsePreset(_ s: String) -> Preset? {
        switch s.lowercased() {
        case "web": return .web
        case "social": return .social
        case "print": return .print
        case "archive": return .archive
        default: return nil
        }
    }

    private func parseStripMetadata(_ s: String) -> StripMetadata? {
        switch s.lowercased() {
        case "all": return .all
        case "keepexif": return .keepExif
        case "keepicc": return .keepIcc
        case "keepall": return .keepAll
        default: return nil
        }
    }

    private func findVectorsFile() -> URL? {
        // 1. Explicit override (CI / local): DUSIMAGEZ_REPO_ROOT.
        if let root = ProcessInfo.processInfo.environment["DUSIMAGEZ_REPO_ROOT"],
           !root.isEmpty
        {
            let candidate = URL(fileURLWithPath: root)
                .appendingPathComponent("shared/test-vectors.json")
            if FileManager.default.fileExists(atPath: candidate.path) {
                return candidate
            }
        }
        // 2. Walk up from this test file: Tests/DusimagezTests/ ->
        //    packages/swift/ -> packages/ -> repo root.
        var dir = URL(fileURLWithPath: #file).deletingLastPathComponent()
        for _ in 0..<6 {
            let candidate = dir.appendingPathComponent("shared/test-vectors.json")
            if FileManager.default.fileExists(atPath: candidate.path) {
                return candidate
            }
            dir.deleteLastPathComponent()
        }
        return nil
    }
}
