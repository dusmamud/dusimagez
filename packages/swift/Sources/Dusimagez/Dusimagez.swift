import Foundation

// Idiomatic Swift wrapper over the UniFFI-generated bindings
// (`dusimagez.swift`, generated from crates/dusimagez/src/dusimagez.udl
// — see scripts/regenerate-bindings.sh). The generated API works in
// [UInt8]; this wrapper speaks Data, which is what iOS/macOS callers
// actually hold.
//
// Generated naming, verified against uniffi-bindgen 0.29.5 output:
// * UDL snake_case fields become camelCase (`max_width` -> `maxWidth`).
// * Enum cases become lowerCamelCase (`Jpeg` -> `.jpeg`, `WebP` ->
//   `.webP`, `S420` -> `.s420`, `KeepExif` -> `.keepExif`).
// * The error type keeps its UDL name `DusimagezError` (enum with
//   associated values) — unlike Kotlin, which names it
//   `DusimagezException`.
// * `CompressOptions()` with no arguments gives the UDL defaults
//   (SPEC §2). `subsampling` defaults to nil = "let the core apply
//   the quality-dependent rule" — do not substitute your own default.
//
// The wrapper methods take Data with an unlabeled first parameter
// and call the generated top-level functions with the `input:`
// label, so there is no ambiguity with the wrapper itself.

/// Stable error code, e.g. `"INVALID_IMAGE"` (== variant name in SNAKE_UPPER).
public extension DusimagezError {
    public var code: String {
        switch self {
        case let .InvalidImage(code, _, _): return code
        case let .UnsupportedFormat(code, _, _): return code
        case let .ImageTooLarge(code, _, _): return code
        case let .TargetSizeUnreachable(code, _, _): return code
        case let .InvalidQuality(code, _, _): return code
        case let .InvalidDimensions(code, _, _): return code
        case let .InvalidArgument(code, _, _): return code
        case let .EmptyInput(code, _, _): return code
        }
    }

    /// English error message from the core.
    public var messageEn: String {
        switch self {
        case let .InvalidImage(_, messageEn, _): return messageEn
        case let .UnsupportedFormat(_, messageEn, _): return messageEn
        case let .ImageTooLarge(_, messageEn, _): return messageEn
        case let .TargetSizeUnreachable(_, messageEn, _): return messageEn
        case let .InvalidQuality(_, messageEn, _): return messageEn
        case let .InvalidDimensions(_, messageEn, _): return messageEn
        case let .InvalidArgument(_, messageEn, _): return messageEn
        case let .EmptyInput(_, messageEn, _): return messageEn
        }
    }

    /// Hindi error message from the core.
    public var messageHi: String {
        switch self {
        case let .InvalidImage(_, _, messageHi): return messageHi
        case let .UnsupportedFormat(_, _, messageHi): return messageHi
        case let .ImageTooLarge(_, _, messageHi): return messageHi
        case let .TargetSizeUnreachable(_, _, messageHi): return messageHi
        case let .InvalidQuality(_, _, messageHi): return messageHi
        case let .InvalidDimensions(_, _, messageHi): return messageHi
        case let .InvalidArgument(_, _, messageHi): return messageHi
        case let .EmptyInput(_, _, messageHi): return messageHi
        }
    }
}

/// Offline image compressor — the dusimagez Rust core over UniFFI.
/// Every function is a thin call into the core; no logic lives here.
public enum Dusimagez {

    /// Re-encode an image at a fixed quality (and `options.format`,
    /// or the input format when unset). Throws `DusimagezError`
    /// with a stable code (`INVALID_IMAGE`, `INVALID_QUALITY`, ...).
    public static func compressImage(
        _ input: Data,
        options: CompressOptions = CompressOptions()
    ) throws -> Data {
        Data(try compressImage(input: [UInt8](input), options: options))
    }

    /// Re-encode an image so the result is at most `targetBytes`
    /// bytes, at the highest quality that fits. Throws
    /// `DusimagezError.targetSizeUnreachable` (code
    /// `TARGET_SIZE_UNREACHABLE`) when even the lowest quality +
    /// downscale cannot reach the target.
    public static func compressToSize(
        _ input: Data,
        targetBytes: UInt64,
        options: CompressOptions = CompressOptions()
    ) throws -> Data {
        Data(try compressToSize(
            input: [UInt8](input),
            targetBytes: targetBytes,
            options: options
        ))
    }

    /// Compress and return the full before/after `CompressReport`
    /// (sizes, savings, output format/dimensions, quality actually
    /// used, whether it resized) alongside the output bytes.
    public static func compressWithReport(
        _ input: Data,
        options: CompressOptions = CompressOptions()
    ) throws -> CompressReport {
        try compressWithReport(input: [UInt8](input), options: options)
    }

    /// Read an image's summary without modifying it.
    public static func getImageInfo(_ input: Data) throws -> ImageInfo {
        try getImageInfo(input: [UInt8](input))
    }

    /// Compress many images in one call. One result per input, same
    /// order; a failing item has `ok == false` plus `errorCode` /
    /// `errorMessageEn` / `errorMessageHi` and never aborts the
    /// batch. Never throws.
    public static func bulkCompress(
        _ inputs: [Data],
        options: CompressOptions = CompressOptions()
    ) -> [BulkItemResult] {
        bulkCompress(inputs: inputs.map { [UInt8]($0) }, options: options)
    }
}
