/**
 * dusimagez — error type (pure module, no WASM import).
 *
 * Every failure from this package surfaces as a `DusimagezError`
 * with a stable machine-readable `code` plus `messageEn` /
 * `messageHi` from the Rust core — never a raw TypeError, never a
 * raw JSON string.
 */

/** Stable error codes — verbatim from SPEC §4. */
export type DusimagezErrorCode =
  | "INVALID_IMAGE"
  | "UNSUPPORTED_FORMAT"
  | "IMAGE_TOO_LARGE"
  | "TARGET_SIZE_UNREACHABLE"
  | "INVALID_QUALITY"
  | "INVALID_DIMENSIONS"
  | "INVALID_ARGUMENT"
  | "EMPTY_INPUT"
  | "INVALID_TYPE"
  | "OPERATION_FAILED";

export class DusimagezError extends Error {
  readonly code: DusimagezErrorCode;
  readonly messageEn: string;
  readonly messageHi: string;
  /** Alias of `messageEn` (the wasm JSON also carries `reason_*` in some builds). */
  readonly reasonEn: string;
  /** Alias of `messageHi`. */
  readonly reasonHi: string;

  constructor(code: DusimagezErrorCode, messageEn: string, messageHi: string) {
    super(`${code}: ${messageEn}`);
    this.name = "DusimagezError";
    this.code = code;
    this.messageEn = messageEn;
    this.messageHi = messageHi;
    this.reasonEn = messageEn;
    this.reasonHi = messageHi;
  }
}

export function invalidType(what: string): DusimagezError {
  return new DusimagezError(
    "INVALID_TYPE",
    `${what} must be a Uint8Array.`,
    `${what} Uint8Array hona chahiye.`,
  );
}

export function invalidArgument(messageEn: string, messageHi: string): DusimagezError {
  return new DusimagezError("INVALID_ARGUMENT", messageEn, messageHi);
}

/** Turn whatever wasm threw into a DusimagezError (pass-through if
 *  it already is one, e.g. from the client-side checks below). The
 *  wasm layer throws a JSON string:
 *  `{"ok":false,"code":"…","message_en":"…","message_hi":"…"}`. */
export function toDusimagezError(e: unknown): DusimagezError {
  if (e instanceof DusimagezError) return e;
  if (typeof e === "string") {
    try {
      const parsed = JSON.parse(e) as {
        code?: unknown;
        message_en?: unknown;
        message_hi?: unknown;
        reason_en?: unknown;
        reason_hi?: unknown;
      };
      if (parsed && typeof parsed.code === "string") {
        const en =
          typeof parsed.message_en === "string"
            ? parsed.message_en
            : typeof parsed.reason_en === "string"
              ? parsed.reason_en
              : "Operation failed.";
        const hi =
          typeof parsed.message_hi === "string"
            ? parsed.message_hi
            : typeof parsed.reason_hi === "string"
              ? parsed.reason_hi
              : "Operation fail ho gaya.";
        return new DusimagezError(parsed.code as DusimagezErrorCode, en, hi);
      }
    } catch {
      // Not JSON — fall through to the plain-string case below.
    }
    return new DusimagezError("OPERATION_FAILED", e, e);
  }
  if (e instanceof Error) {
    return new DusimagezError("OPERATION_FAILED", e.message, e.message);
  }
  return new DusimagezError(
    "OPERATION_FAILED",
    "Unknown error.",
    "Unknown error hai.",
  );
}

/** Run a wasm call, converting any throw into a DusimagezError. */
export function call<T>(fn: () => T): T {
  try {
    return fn();
  } catch (e) {
    throw toDusimagezError(e);
  }
}
