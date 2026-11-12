<?php

declare(strict_types=1);

namespace Dusimagez;

/**
 * Every dusimagez failure, with a stable machine-readable code.
 *
 * Mirrors the Rust `DusimagezError { code, message_en, message_hi }`
 * (crates/dusimagez/src/error.rs) and the Go wrapper:
 * `$code` is one of the stable codes (`INVALID_IMAGE`,
 * `UNSUPPORTED_FORMAT`, `IMAGE_TOO_LARGE`, `TARGET_SIZE_UNREACHABLE`,
 * `INVALID_QUALITY`, `INVALID_DIMENSIONS`, `INVALID_ARGUMENT`,
 * `EMPTY_INPUT`, plus `UNKNOWN` for an unrecognised FFI status);
 * `$messageEn` / `$messageHi` are the human-readable messages from
 * the Rust core (Roman-script Hinglish, like indycheck).
 */
class DusimagezError extends \Exception
{
    /**
     * Stable, machine-readable error code, e.g. `"INVALID_IMAGE"`.
     *
     * Untyped on purpose: `\Exception` already declares an untyped
     * `$code`, and redeclaring it with a type is a PHP fatal error
     * at class load.
     *
     * @var string
     */
    public $code;

    /** English message from the Rust core. */
    public string $messageEn;

    /** Hindi (Roman-script) message from the Rust core. */
    public string $messageHi;

    /** Alias of `$messageEn` (the Rust JSON also carries `reason_*`). */
    public string $reasonEn;

    /** Alias of `$messageHi`. */
    public string $reasonHi;

    public function __construct(string $code, string $messageEn = '', string $messageHi = '')
    {
        parent::__construct("{$code}: {$messageEn}");
        $this->code = $code;
        $this->messageEn = $messageEn;
        $this->messageHi = $messageHi;
        $this->reasonEn = $messageEn;
        $this->reasonHi = $messageHi;
    }
}
