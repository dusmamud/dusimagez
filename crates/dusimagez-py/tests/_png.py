"""Minimal PNG encoder (RGB8, filter 0 per row) — test fixtures only.

Pure stdlib (`zlib` + `struct`): no binary blobs committed, no
dependency on the Rust core or its `mock` CLI. Deterministic:
same (kind, width, height, seed) always gives the same bytes.
"""

import struct
import zlib


def _chunk(kind: bytes, data: bytes) -> bytes:
    out = struct.pack(">I", len(data)) + kind + data
    out += struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    return out


def _pixels(kind: str, width: int, height: int, seed: int) -> bytes:
    if kind == "solid":
        r, g, b = (seed * 37 + 11) % 256, (seed * 91 + 7) % 256, (seed * 53 + 3) % 256
        row = bytes([r, g, b]) * width
        return row * height
    if kind == "gradient":
        out = bytearray()
        for y in range(height):
            for x in range(width):
                out += bytes([x * 255 // max(width - 1, 1), y * 255 // max(height - 1, 1), 128])
        return bytes(out)
    if kind == "noise":
        # Deterministic LCG "photo-like" noise — hard to compress, so
        # `compress_to_size` has something real to squeeze.
        state = (seed * 2654435761 + 97) & 0xFFFFFFFF
        out = bytearray()
        for _ in range(width * height * 3):
            state = (state * 1664525 + 1013904223) & 0xFFFFFFFF
            out.append((state >> 24) & 0xFF)
        return bytes(out)
    raise ValueError(f"unknown fixture kind {kind!r}")


def make_png(kind: str = "noise", width: int = 64, height: int = 48, seed: int = 42) -> bytes:
    """Encode a tiny RGB8 PNG. `kind` is `solid`, `gradient` or `noise`."""
    raw = bytearray()
    px = _pixels(kind, width, height, seed)
    stride = width * 3
    for y in range(height):
        raw.append(0)  # filter: None
        raw += px[y * stride:(y + 1) * stride]
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", ihdr)
        + _chunk(b"IDAT", zlib.compress(bytes(raw)))
        + _chunk(b"IEND", b"")
    )
