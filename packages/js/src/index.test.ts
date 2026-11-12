/**
 * dusimagez JS tests — WASM wrapper over the Rust core.
 *
 * Every test runs through the real wasm build (`npm run build` via
 * `pretest`), so these assert the whole chain: TS validation ->
 * camelCase-to-snake_case translation -> wasm -> Rust core.
 * Dimension assertions go through `getImageInfo` (the wasm
 * round-trip).
 *
 * The whole suite is skipped when `wasm-pkg/` is not built yet
 * (the Rust core has not landed, so `wasm-pack build` cannot run).
 * That skip is the honest signal — never a fake pass.
 *
 * Fixtures: tiny PNGs generated in-test with a minimal encoder
 * (zlib deflate + CRC32, RGB8, no interlace) — no binary blobs
 * committed. A solid red 10x10, a 64x48 gradient, and a 64x48
 * photo-like noise image (deterministic hash noise, so
 * `compressToSize` has something real to squeeze).
 */
import { existsSync } from "node:fs";
import { join } from "node:path";
import { deflateSync } from "node:zlib";
import { describe, expect, it } from "vitest";
import { compressOptionsSchema } from "./zod";

// Vitest runs with cwd = packages/js, so paths resolve from there
// (import.meta.url is unusable: the package compiles as CommonJS).
const wasmBuilt = existsSync(join(process.cwd(), "wasm-pkg", "dusimagez_wasm.js"));

// The static import of `./index` would pull in `../wasm-pkg` at
// collection time, so the API is loaded lazily inside each test.
async function loadApi(): Promise<typeof import("./index")> {
  return import("./index");
}

// ---------------------------------------------------------------------------
// Minimal PNG encoder (RGB8, filter 0 per row) — test fixtures only.
// ---------------------------------------------------------------------------

const CRC_TABLE: Uint32Array = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(buf: Buffer): number {
  let c = 0xffffffff;
  for (const byte of buf) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type: string, data: Buffer): Buffer {
  const typeBuf = Buffer.from(type, "ascii");
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

function makePng(
  kind: "solid" | "gradient" | "noise",
  width: number,
  height: number,
  seed = 42,
): Uint8Array {
  const px = Buffer.alloc(width * height * 3);
  if (kind === "solid") {
    for (let i = 0; i < width * height; i++) {
      px[i * 3] = 255;
      px[i * 3 + 1] = 0;
      px[i * 3 + 2] = 0;
    }
  } else if (kind === "gradient") {
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const i = (y * width + x) * 3;
        px[i] = Math.floor((x * 255) / Math.max(width - 1, 1));
        px[i + 1] = Math.floor((y * 255) / Math.max(height - 1, 1));
        px[i + 2] = 128;
      }
    }
  } else {
    let state = (seed * 2654435761 + 97) >>> 0;
    for (let i = 0; i < px.length; i++) {
      state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
      px[i] = (state >>> 24) & 0xff;
    }
  }
  const raw = Buffer.alloc(height * (1 + width * 3));
  for (let y = 0; y < height; y++) {
    raw[y * (1 + width * 3)] = 0; // filter: None
    px.copy(raw, y * (1 + width * 3) + 1, y * width * 3, (y + 1) * width * 3);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 2; // colour type RGB
  const png = Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
  return new Uint8Array(png);
}

// ---------------------------------------------------------------------------
// Op tests (skipped until the wasm package is built).
// ---------------------------------------------------------------------------

describe.skipIf(!wasmBuilt)("ops (wasm)", () => {
  it("version() reports 0.1.0", async () => {
    const api = await loadApi();
    expect(api.version()).toBe("0.1.0");
  });

  it("compressImage: PNG round-trip keeps dimensions", async () => {
    const api = await loadApi();
    const img = makePng("gradient", 64, 48, 4);
    const out = api.compressImage(img, { quality: 80 });
    expect(out).toBeInstanceOf(Uint8Array);
    expect(out.byteLength).toBeGreaterThan(0);
    const info = api.getImageInfo(out);
    expect(info.format).toBe("png");
    expect(info.width).toBe(64);
    expect(info.height).toBe(48);
  });

  it("compressImage: converts to JPEG", async () => {
    const api = await loadApi();
    const img = makePng("gradient", 64, 48, 4);
    const out = api.compressImage(img, { format: "jpeg", quality: 80 });
    const info = api.getImageInfo(out);
    expect(info.format).toBe("jpeg");
    expect(info.width).toBe(64);
    expect(info.height).toBe(48);
  });

  it("compressImage: format tiff falls back to PNG output", async () => {
    const api = await loadApi();
    const img = makePng("gradient", 64, 48, 4);
    const out = api.compressImage(img, { format: "tiff", quality: 80 });
    expect(api.getImageInfo(out).format).toBe("png");
  });

  it("compressImage: defaults and explicit quality 80 agree", async () => {
    const api = await loadApi();
    const img = makePng("noise", 64, 48, 1);
    expect(api.compressImage(img)).toEqual(api.compressImage(img, { quality: 80 }));
  });

  it("compressToSize: result is at most the target", async () => {
    const api = await loadApi();
    const img = makePng("noise", 128, 96, 7);
    const out = api.compressToSize(img, 20000, { format: "jpeg" });
    expect(out.byteLength).toBeLessThanOrEqual(20000);
    expect(api.getImageInfo(out).format).toBe("jpeg");
  });

  it("compressToSize: same target via targetBytes option", async () => {
    const api = await loadApi();
    const img = makePng("noise", 128, 96, 7);
    const out = api.compressImage(img, { format: "jpeg", targetBytes: 20000 });
    expect(out.byteLength).toBeLessThanOrEqual(20000);
  });

  it("compressWithReport: maths add up", async () => {
    const api = await loadApi();
    const { checkReportMaths } = await import("./parse");
    const img = makePng("noise", 128, 96, 7);
    const report = api.compressWithReport(img, { format: "jpeg", quality: 80 });
    expect(report.originalSize).toBe(img.byteLength);
    expect(report.outputSize).toBe(report.output.byteLength);
    expect(report.format).toBe("jpeg");
    expect(report.width).toBe(128);
    expect(report.height).toBe(96);
    expect(report.qualityUsed).toBe(80);
    expect(report.resized).toBe(false);
    expect(checkReportMaths(report)).toEqual([]);
    expect(compressOptionsSchema.safeParse({ quality: 80 }).success).toBe(true);
  });

  it("compressWithReport: maxWidth downscales and sets resized", async () => {
    const api = await loadApi();
    const img = makePng("gradient", 100, 80, 2);
    const report = api.compressWithReport(img, { maxWidth: 50 });
    expect(report.resized).toBe(true);
    expect(report.width).toBe(50);
    expect(report.height).toBe(40);
  });

  it("getImageInfo: fields for a gradient PNG", async () => {
    const api = await loadApi();
    const img = makePng("gradient", 64, 48, 4);
    const info = api.getImageInfo(img);
    expect(info.format).toBe("png");
    expect(info.width).toBe(64);
    expect(info.height).toBe(48);
    expect(info.hasAlpha).toBe(false);
    expect(info.colourType).toBe("rgb8");
    expect(info.fileSize).toBe(img.byteLength);
  });

  it("bulkCompress: order kept, one bad input fails only its own item", async () => {
    const api = await loadApi();
    const good = makePng("gradient", 64, 48, 4);
    const bad = new TextEncoder().encode("not an image at all");
    const results = api.bulkCompress([good, bad, good], {
      format: "jpeg",
      quality: 80,
    });
    expect(results.map((r) => r.index)).toEqual([0, 1, 2]);
    expect(results[0].ok).toBe(true);
    expect(results[0].report?.format).toBe("jpeg");
    expect(results[1].ok).toBe(false);
    expect(results[1].error?.code).toBe("INVALID_IMAGE");
    expect(results[2].ok).toBe(true);
  });

  it("bulkCompress: empty list returns []", async () => {
    const api = await loadApi();
    expect(api.bulkCompress([])).toEqual([]);
  });

  it("errors: invalid bytes are INVALID_IMAGE, empty are EMPTY_INPUT", async () => {
    const api = await loadApi();
    try {
      api.compressImage(new TextEncoder().encode("this is not an image"));
      expect.unreachable();
    } catch (e) {
      const err = e as InstanceType<typeof api.DusimagezError>;
      expect(err.code).toBe("INVALID_IMAGE");
      expect(err.messageEn.length).toBeGreaterThan(0);
      expect(err.messageHi.length).toBeGreaterThan(0);
    }
    try {
      api.compressImage(new Uint8Array(0));
      expect.unreachable();
    } catch (e) {
      expect((e as InstanceType<typeof api.DusimagezError>).code).toBe("EMPTY_INPUT");
    }
  });

  it("errors: non-Uint8Array input is INVALID_TYPE", async () => {
    const api = await loadApi();
    try {
      api.compressImage("nope");
      expect.unreachable();
    } catch (e) {
      expect((e as InstanceType<typeof api.DusimagezError>).code).toBe("INVALID_TYPE");
    }
  });

  it("errors: quality out of range is INVALID_QUALITY", async () => {
    const api = await loadApi();
    const img = makePng("gradient", 64, 48, 4);
    for (const bad of [0, 101]) {
      try {
        api.compressImage(img, { quality: bad });
        expect.unreachable();
      } catch (e) {
        expect((e as InstanceType<typeof api.DusimagezError>).code).toBe("INVALID_QUALITY");
      }
    }
  });

  it("errors: unreachable target size is TARGET_SIZE_UNREACHABLE", async () => {
    const api = await loadApi();
    const img = makePng("noise", 128, 128, 7);
    try {
      api.compressToSize(img, 10);
      expect.unreachable();
    } catch (e) {
      expect((e as InstanceType<typeof api.DusimagezError>).code).toBe(
        "TARGET_SIZE_UNREACHABLE",
      );
    }
  });

  it("preset web and subsampling spellings work", async () => {
    const api = await loadApi();
    const img = makePng("noise", 128, 96, 7);
    const out = api.compressImage(img, {
      format: "jpeg",
      preset: "web",
      subsampling: "s444",
      stripMetadata: "keep_exif",
    });
    expect(api.getImageInfo(out).format).toBe("jpeg");
    expect(out.byteLength).toBeGreaterThan(0);
  });
});
