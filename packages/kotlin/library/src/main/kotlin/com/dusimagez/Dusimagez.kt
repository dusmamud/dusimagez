package com.dusimagez

// Idiomatic Kotlin wrapper over the UniFFI-generated bindings
// (`uniffi.dusimagez`, generated from crates/dusimagez/src/dusimagez.udl
// — see scripts/regenerate-bindings.sh). The generated API works in
// List<UByte>; this wrapper speaks ByteArray, which is what Android
// callers actually hold, and re-exports the generated types so
// callers only ever import com.dusimagez.
//
// Generated naming, verified against uniffi-bindgen 0.29.5 output:
// * UDL snake_case fields become camelCase (`max_width` -> `maxWidth`).
// * Enum variants become UPPER_SNAKE (`Jpeg` -> `JPEG`, `WebP` ->
//   `WEB_P`, `S420` stays `S420`, `KeepExif` -> `KEEP_EXIF`).
// * The error type is `DusimagezException` (sealed class, one
//   subclass per variant) — Kotlin appends `Exception`, unlike Swift.
// * `CompressOptions()` with no arguments gives the UDL defaults
//   (SPEC §2). `subsampling` defaults to null = "let the core apply
//   the quality-dependent rule" — do not substitute your own default.

import uniffi.dusimagez.BulkItemResult as GeneratedBulkItemResult
import uniffi.dusimagez.CompressOptions as GeneratedCompressOptions
import uniffi.dusimagez.CompressReport as GeneratedCompressReport
import uniffi.dusimagez.Dpi as GeneratedDpi
import uniffi.dusimagez.DusimagezException as GeneratedDusimagezException
// Nested error subclasses, imported directly: Kotlin cannot access a
// nested class through a typealias (`DusimagezException.InvalidImage`
// is an "unresolved reference"), so the `when` branches below match
// on these imports instead.
import uniffi.dusimagez.DusimagezException.EmptyInput
import uniffi.dusimagez.DusimagezException.ImageTooLarge
import uniffi.dusimagez.DusimagezException.InvalidArgument
import uniffi.dusimagez.DusimagezException.InvalidDimensions
import uniffi.dusimagez.DusimagezException.InvalidImage
import uniffi.dusimagez.DusimagezException.InvalidQuality
import uniffi.dusimagez.DusimagezException.TargetSizeUnreachable
import uniffi.dusimagez.DusimagezException.UnsupportedFormat
import uniffi.dusimagez.ImageFormat as GeneratedImageFormat
import uniffi.dusimagez.ImageInfo as GeneratedImageInfo
import uniffi.dusimagez.Preset as GeneratedPreset
import uniffi.dusimagez.StripMetadata as GeneratedStripMetadata
import uniffi.dusimagez.Subsampling as GeneratedSubsampling
import uniffi.dusimagez.bulkCompress as generatedBulkCompress
import uniffi.dusimagez.compressImage as generatedCompressImage
import uniffi.dusimagez.compressToSize as generatedCompressToSize
import uniffi.dusimagez.compressWithReport as generatedCompressWithReport
import uniffi.dusimagez.getImageInfo as generatedGetImageInfo

/** Pro compressor options (UDL `CompressOptions`, SPEC §2). */
typealias CompressOptions = GeneratedCompressOptions

/** Full before/after report (UDL `CompressReport`, SPEC §1). */
typealias CompressReport = GeneratedCompressReport

/** Image summary (UDL `ImageInfo`, SPEC §1). */
typealias ImageInfo = GeneratedImageInfo

/** One item of a `bulkCompress` result (UDL `BulkItemResult`). */
typealias BulkItemResult = GeneratedBulkItemResult

/** DPI pair (UDL `Dpi`; `null` in options = unset). */
typealias Dpi = GeneratedDpi

/** Output image format. Kotlin cases are UPPER_SNAKE: `JPEG`, `PNG`, `WEB_P`, `AVIF`, `GIF`, `BMP`. */
typealias ImageFormat = GeneratedImageFormat

/** Chroma subsampling: `S444`, `S422`, `S420`. */
typealias Subsampling = GeneratedSubsampling

/** Preset: `WEB`, `SOCIAL`, `PRINT`, `ARCHIVE`. */
typealias Preset = GeneratedPreset

/** Metadata handling: `ALL`, `KEEP_EXIF`, `KEEP_ICC`, `KEEP_ALL`. */
typealias StripMetadata = GeneratedStripMetadata

/**
 * Error type (generated `DusimagezException`, sealed class — one
 * subclass per stable code, each carrying `code` + `messageEn` +
 * `messageHi`). The `code` / `messageEn` / `messageHi` extensions
 * below read those fields without a `when` over all eight subclasses.
 */
typealias DusimagezException = GeneratedDusimagezException

/** Stable error code, e.g. `"INVALID_IMAGE"` (== variant name in SNAKE_UPPER). */
val DusimagezException.code: String
    get() = when (this) {
        is InvalidImage -> code
        is UnsupportedFormat -> code
        is ImageTooLarge -> code
        is TargetSizeUnreachable -> code
        is InvalidQuality -> code
        is InvalidDimensions -> code
        is InvalidArgument -> code
        is EmptyInput -> code
    }

/** English error message from the core. */
val DusimagezException.messageEn: String
    get() = when (this) {
        is InvalidImage -> messageEn
        is UnsupportedFormat -> messageEn
        is ImageTooLarge -> messageEn
        is TargetSizeUnreachable -> messageEn
        is InvalidQuality -> messageEn
        is InvalidDimensions -> messageEn
        is InvalidArgument -> messageEn
        is EmptyInput -> messageEn
    }

/** Hindi error message from the core. */
val DusimagezException.messageHi: String
    get() = when (this) {
        is InvalidImage -> messageHi
        is UnsupportedFormat -> messageHi
        is ImageTooLarge -> messageHi
        is TargetSizeUnreachable -> messageHi
        is InvalidQuality -> messageHi
        is InvalidDimensions -> messageHi
        is InvalidArgument -> messageHi
        is EmptyInput -> messageHi
    }

internal fun ByteArray.toUByteList(): List<UByte> = map { it.toUByte() }
internal fun List<UByte>.toByteArray(): ByteArray = ByteArray(size) { this[it].toByte() }

/**
 * Offline image compressor — the dusimagez Rust core over UniFFI.
 * Every function is a thin call into the core; no logic lives here.
 */
object Dusimagez {

    /**
     * Re-encode an image at a fixed quality (and `options.format`, or
     * the input format when unset). Throws [DusimagezException] with
     * a stable code (`INVALID_IMAGE`, `INVALID_QUALITY`, ...).
     */
    @Throws(DusimagezException::class)
    fun compressImage(input: ByteArray, options: CompressOptions = CompressOptions()): ByteArray =
        generatedCompressImage(input.toUByteList(), options).toByteArray()

    /**
     * Re-encode an image so the result is at most `targetBytes` bytes,
     * at the highest quality that fits. Throws
     * `DusimagezException.TargetSizeUnreachable` (code
     * `TARGET_SIZE_UNREACHABLE`) when even the lowest quality +
     * downscale cannot reach the target.
     */
    @Throws(DusimagezException::class)
    fun compressToSize(
        input: ByteArray,
        targetBytes: ULong,
        options: CompressOptions = CompressOptions(),
    ): ByteArray = generatedCompressToSize(input.toUByteList(), targetBytes, options).toByteArray()

    /**
     * Compress and return the full before/after [CompressReport]
     * (sizes, savings, output format/dimensions, quality actually
     * used, whether it resized) alongside the output bytes.
     */
    @Throws(DusimagezException::class)
    fun compressWithReport(
        input: ByteArray,
        options: CompressOptions = CompressOptions(),
    ): CompressReport = generatedCompressWithReport(input.toUByteList(), options)

    /** Read an image's summary without modifying it. */
    @Throws(DusimagezException::class)
    fun getImageInfo(input: ByteArray): ImageInfo = generatedGetImageInfo(input.toUByteList())

    /**
     * Compress many images in one call. One result per input, same
     * order; a failing item has `ok == false` plus `errorCode` /
     * `errorMessageEn` / `errorMessageHi` and never aborts the batch.
     * Never throws.
     */
    fun bulkCompress(
        inputs: List<ByteArray>,
        options: CompressOptions = CompressOptions(),
    ): List<BulkItemResult> =
        generatedBulkCompress(inputs.map { it.toUByteList() }, options)
}
