import Foundation

/// Tiny in-test PNG encoder (RGB8, filter 0 per scanline), so tests
/// need no binary blobs and no extra core API. Deterministic for a
/// given kind + size + seed — the same builders the CLI `mock` command
/// uses, reimplemented here. The bytes are verified valid by the core
/// itself: every test first round-trips the fixture through
/// `getImageInfo` and asserts the expected dimensions.
enum TestPng {

    /// Solid colour image.
    static func solid(width: Int, height: Int, r: UInt8, g: UInt8, b: UInt8) -> Data {
        encode(width: width, height: height) { _, _ in (r, g, b) }
    }

    /// Smooth gradient (red across, green down, blue diagonal).
    static func gradient(width: Int, height: Int) -> Data {
        encode(width: width, height: height) { x, y in
            (
                width > 1 ? UInt8(x * 255 / (width - 1)) : 0,
                height > 1 ? UInt8(y * 255 / (height - 1)) : 0,
                UInt8((x + y) % 256)
            )
        }
    }

    /// Photo-like noise: deliberately hard to compress, so
    /// `compressToSize` has something to work on.
    static func noise(width: Int, height: Int, seed: UInt64) -> Data {
        var rng = SplitMix64(seed: seed)
        return encode(width: width, height: height) { _, _ in
            (UInt8(rng.next() & 0xFF), UInt8(rng.next() & 0xFF), UInt8(rng.next() & 0xFF))
        }
    }

    private static func encode(
        width: Int,
        height: Int,
        pixel: (Int, Int) -> (UInt8, UInt8, UInt8)
    ) -> Data {
        precondition(width > 0 && height > 0, "width and height must be > 0")
        var raw = Data()
        raw.reserveCapacity(height * (1 + width * 3))
        for y in 0..<height {
            raw.append(0) // filter: None
            for x in 0..<width {
                let (r, g, b) = pixel(x, y)
                raw.append(contentsOf: [r, g, b])
            }
        }
        // zlib stream via stored DEFLATE blocks (see zlibStored below)
        // — no Compression framework dependency: `Data.compressed`
        // is not available to a SwiftPM test target here, and a
        // stored-block stream is always valid, just uncompressed.
        let compressed = zlibStored(raw)
        var out = Data([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) // PNG signature
        var ihdr = Data()
        ihdr.append(contentsOf: withUnsafeBytes(of: UInt32(width).bigEndian, Array.init))
        ihdr.append(contentsOf: withUnsafeBytes(of: UInt32(height).bigEndian, Array.init))
        ihdr.append(contentsOf: [8, 2, 0, 0, 0]) // bit depth 8, colour type 2 (RGB)
        out.append(chunk(type: "IHDR", data: ihdr))
        out.append(chunk(type: "IDAT", data: compressed))
        out.append(chunk(type: "IEND", data: Data()))
        return out
    }

    private static func chunk(type: String, data: Data) -> Data {
        var out = Data()
        let typeBytes = Data(type.utf8)
        out.append(contentsOf: withUnsafeBytes(of: UInt32(data.count).bigEndian, Array.init))
        out.append(typeBytes)
        out.append(data)
        var crcData = typeBytes
        crcData.append(data)
        let crc = crc32(crcData)
        out.append(contentsOf: withUnsafeBytes(of: crc.bigEndian, Array.init))
        return out
    }

    /// Minimal zlib encoder (RFC 1950) using DEFLATE stored blocks
    /// (RFC 1951 §3.2.4): 0x78 0x01 header, then ≤65,535-byte raw
    /// blocks (BFINAL + BTYPE=00, LEN/NLEN, bytes), then Adler-32.
    /// Deterministic; any zlib decoder accepts stored blocks.
    static func zlibStored(_ data: Data) -> Data {
        var out = Data()
        out.append(contentsOf: [0x78, 0x01]) // CMF/FLG, no preset dict
        var offset = 0
        while true {
            let remaining = data.count - offset
            let take = min(remaining, 65_535)
            let isFinal = remaining <= 65_535
            out.append(isFinal ? 1 : 0) // BFINAL, BTYPE=00, byte-aligned
            let len = UInt16(take)
            out.append(contentsOf: withUnsafeBytes(of: len.littleEndian, Array.init))
            out.append(contentsOf: withUnsafeBytes(of: (~len).littleEndian, Array.init))
            out.append(data[offset ..< offset + take])
            offset += take
            if isFinal { break }
        }
        out.append(contentsOf: withUnsafeBytes(of: adler32(data).bigEndian, Array.init))
        return out
    }

    /// Adler-32 (RFC 1950), big-endian in the stream.
    static func adler32(_ data: Data) -> UInt32 {
        var a: UInt32 = 1
        var b: UInt32 = 0
        for byte in data {
            a = (a + UInt32(byte)) % 65_521
            b = (b + a) % 65_521
        }
        return (b << 16) | a
    }

    /// CRC-32 (IEEE, polynomial 0xEDB88320), bitwise — no table needed.
    static func crc32(_ data: Data) -> UInt32 {
        var crc: UInt32 = 0xFFFF_FFFF
        for byte in data {
            crc ^= UInt32(byte)
            for _ in 0..<8 {
                crc = (crc & 1) != 0 ? (crc >> 1) ^ 0xEDB8_8320 : crc >> 1
            }
        }
        return crc ^ 0xFFFF_FFFF
    }
}

/// Deterministic PRNG (SplitMix64) — no GameplayKit dependency.
struct SplitMix64 {
    private var state: UInt64

    init(seed: UInt64) {
        state = seed
    }

    mutating func next() -> UInt64 {
        state &+= 0x9E37_79B9_7F4A_7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
        z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
        return z ^ (z >> 31)
    }
}
