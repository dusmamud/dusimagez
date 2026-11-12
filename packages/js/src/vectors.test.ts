/**
 * dusimagez JS tests — shared vectors (`shared/test-vectors.json`).
 *
 * Every language runs this file and must agree (SPEC §5–6). Cases
 * assert format / dimensions / `max_output_bytes` /
 * `min_saved_percent` / report fields / stable error codes — never
 * exact output hashes (encoder-dependent).
 *
 * Fixtures resolve three ways: a committed file under
 * `shared/fixtures/`, inline pseudo-fixtures (`empty` = 0 bytes,
 * `garbage` = plain-text bytes), or a `dusimagez mock ...` command
 * — translated to the real CLI flags (`--out`, format follows the
 * extension, fixed seed) and run with the CLI binary from
 * `target/debug/dusimagez` (built by W3's crate; if it is not
 * built yet, those cases skip, never fake-pass) with output into
 * a temp dir, never into `shared/`.
 *
 * The whole suite is skipped when `wasm-pkg/` is not built yet.
 */
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// Vitest runs with cwd = packages/js, so the repo root is two
// levels up (import.meta.url is unusable: the package compiles as
// CommonJS).
const repoRoot = join(process.cwd(), "..", "..");
const wasmBuilt = existsSync(join(process.cwd(), "wasm-pkg", "dusimagez_wasm.js"));
const vectorsPath = join(repoRoot, "shared", "test-vectors.json");
const vectors = existsSync(vectorsPath)
  ? (JSON.parse(readFileSync(vectorsPath, "utf8")) as {
      cases: Array<Record<string, unknown>>;
      fixtures: Array<Record<string, unknown>>;
    })
  : null;

// The static import of `./index` would pull in `../wasm-pkg` at
// collection time, so the API is loaded lazily inside each test.
async function loadApi(): Promise<typeof import("./index")> {
  return import("./index");
}

const GARBAGE_TEXT = new TextEncoder().encode(
  "this is not an image at all, just plain text bytes 1234567890",
);

function cliBinary(): string | null {
  for (const candidate of [
    join(repoRoot, "target", "debug", "dusimagez"),
    join(repoRoot, "target", "release", "dusimagez"),
  ]) {
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

/** Resolve fixture name -> bytes (missing mock fixtures are left out). */
function resolveFixtures(): Map<string, Uint8Array> {
  const out = new Map<string, Uint8Array>();
  if (!vectors) return out;
  const cli = cliBinary();
  const tmp = mkdtempSync(join(tmpdir(), "dusimagez-vectors-"));
  for (const fixture of vectors.fixtures) {
    const name = fixture["name"] as string;
    const kind = fixture["kind"] as string;
    if (kind === "empty") {
      out.set(name, new Uint8Array(0));
    } else if (kind === "garbage") {
      out.set(name, GARBAGE_TEXT);
    } else if (typeof fixture["file"] === "string") {
      const path = join(repoRoot, fixture["file"] as string);
      if (existsSync(path)) out.set(name, new Uint8Array(readFileSync(path)));
    } else if (
      typeof fixture["generate"] === "string" &&
      (fixture["generate"] as string).startsWith("dusimagez mock")
    ) {
      if (cli === null) continue; // unresolved: cases using it skip
      // Generate strings use the real mock CLI flags: --kind /
      // --width / --height / --out (format follows the extension,
      // fixed seed). Parse --out (fall back to legacy --output).
      const parts = (fixture["generate"] as string).split(" ");
      const flag = (name: string): string => parts[parts.indexOf(name) + 1];
      const outName = parts.includes("--out") ? flag("--out") : flag("--output");
      const ext = outName.includes(".") ? outName.slice(outName.lastIndexOf(".")) : ".png";
      const target = join(tmp, `${name}${ext}`);
      execFileSync(
        cli,
        [
          "mock",
          "--kind", flag("--kind"),
          "--width", flag("--width"),
          "--height", flag("--height"),
          "--out", target,
        ],
        { stdio: "pipe", timeout: 120_000 },
      );
      out.set(name, new Uint8Array(readFileSync(target)));
    }
  }
  return out;
}

const fixtures = resolveFixtures();

/** Translate vector options (Rust snake_case, canonical enums) to
 *  the TS camelCase options the wrapper takes. */
function toTsOptions(options: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  const snakeToCamel = (s: string): string =>
    s.replace(/_([a-z])/g, (_, c: string) => c.toUpperCase());
  const lowerEnum = (s: string): string =>
    s
      .replace(/([A-Z])/g, "_$1")
      .toLowerCase()
      .replace(/^_/, "");
  for (const [key, value] of Object.entries(options)) {
    const camel = snakeToCamel(key);
    if (
      (key === "subsampling" || key === "preset" || key === "strip_metadata") &&
      typeof value === "string"
    ) {
      out[camel] = lowerEnum(value);
    } else if (key === "format" && typeof value === "string") {
      out[camel] = value.toLowerCase();
    } else {
      out[camel] = value;
    }
  }
  return out;
}

function checkExpect(
  api: Awaited<ReturnType<typeof loadApi>>,
  caseId: string,
  data: Uint8Array,
  options: Record<string, unknown>,
  exp: Record<string, unknown>,
  out: Uint8Array,
  report?: { qualityUsed: number; resized: boolean },
): void {
  if (typeof exp["error_code"] === "string") {
    throw new Error(`${caseId}: expected error ${exp["error_code"]} but op succeeded`);
  }
  const r =
    report ??
    (typeof exp["quality_used"] === "number" || typeof exp["resized"] === "boolean"
      ? api.compressWithReport(data, options as never)
      : undefined);
  if (r !== undefined) {
    if (typeof exp["quality_used"] === "number") {
      expect(r.qualityUsed).toBe(exp["quality_used"]);
    }
    if (typeof exp["resized"] === "boolean") {
      expect(r.resized).toBe(exp["resized"]);
    }
  }
  if (typeof exp["max_output_bytes"] === "number") {
    expect(out.byteLength).toBeLessThanOrEqual(exp["max_output_bytes"] as number);
  }
  if (typeof exp["min_saved_percent"] === "number") {
    const saved = data.byteLength === 0 ? 0 : (100 * (data.byteLength - out.byteLength)) / data.byteLength;
    expect(saved).toBeGreaterThanOrEqual(exp["min_saved_percent"] as number);
  }
  if (
    typeof exp["format"] === "string" ||
    typeof exp["width"] === "number" ||
    typeof exp["height"] === "number"
  ) {
    const info = api.getImageInfo(out);
    if (typeof exp["format"] === "string") expect(info.format).toBe(exp["format"]);
    if (typeof exp["width"] === "number") expect(info.width).toBe(exp["width"]);
    if (typeof exp["height"] === "number") expect(info.height).toBe(exp["height"]);
  }
}

const suite = wasmBuilt && vectors !== null ? describe : describe.skip;

suite("shared vectors", () => {
  // Note: vitest executes the suite body even for a skipped suite,
  // so the loop must tolerate `vectors === null` (zero cases then).
  for (const c of vectors?.cases ?? []) {
    const caseId = c["id"] as string;
    const op = c["op"] as string;
    const options = toTsOptions((c["options"] as Record<string, unknown>) ?? {});
    const exp = (c["expect"] as Record<string, unknown>) ?? {};
    const names: string[] =
      op === "bulk"
        ? (c["fixtures"] as string[])
        : [c["fixture"] as string];
    const resolvable = names.every((n) => fixtures.has(n));

    it.skipIf(!resolvable)(caseId, async () => {
      const api = await loadApi();
      if (op === "bulk") {
        const inputs = names.map((n) => fixtures.get(n)!);
        const results = api.bulkCompress(inputs, options as never);
        const wanted = exp["results"] as Array<Record<string, unknown>>;
        expect(results).toHaveLength(wanted.length);
        for (let i = 0; i < wanted.length; i++) {
          const want = wanted[i];
          const item = results[i];
          if (typeof want["error_code"] === "string") {
            expect(item.ok).toBe(false);
            expect(item.error?.code).toBe(want["error_code"]);
          } else {
            expect(item.ok).toBe(true);
            checkExpect(api, caseId, inputs[i], options, want, item.report!.output, {
              qualityUsed: item.report!.qualityUsed,
              resized: item.report!.resized,
            });
          }
        }
        return;
      }
      const data = fixtures.get(names[0])!;
      try {
        if (op === "compress") {
          const out = api.compressImage(data, options as never);
          checkExpect(api, caseId, data, options, exp, out);
        } else if (op === "compress_to_size") {
          const target = options["targetBytes"] as number;
          const { targetBytes: _tb, ...rest } = options;
          const out = api.compressToSize(data, target, rest as never);
          checkExpect(api, caseId, data, options, exp, out);
        } else if (op === "compress_with_report") {
          const report = api.compressWithReport(data, options as never);
          checkExpect(api, caseId, data, options, exp, report.output, report);
        } else if (op === "get_info") {
          const info = api.getImageInfo(data);
          if (typeof exp["format"] === "string") expect(info.format).toBe(exp["format"]);
          if (typeof exp["width"] === "number") expect(info.width).toBe(exp["width"]);
          if (typeof exp["height"] === "number") expect(info.height).toBe(exp["height"]);
        } else {
          throw new Error(`unknown op ${op}`);
        }
      } catch (e) {
        if (typeof exp["error_code"] !== "string") throw e;
        const err = e as InstanceType<typeof api.DusimagezError>;
        expect(err.code).toBe(exp["error_code"] as string);
      }
    });
  }
});
