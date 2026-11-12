package dusimagez

import (
	"encoding/json"
	"fmt"
)

// Stable error codes — verbatim from crates/dusimagez/src/error.rs
// (SPEC section 4) and the C header (crates/dusimagez-ffi/dusimagez.h).
const (
	CodeInvalidImage          = "INVALID_IMAGE"
	CodeUnsupportedFormat     = "UNSUPPORTED_FORMAT"
	CodeImageTooLarge         = "IMAGE_TOO_LARGE"
	CodeTargetSizeUnreachable = "TARGET_SIZE_UNREACHABLE"
	CodeInvalidQuality        = "INVALID_QUALITY"
	CodeInvalidDimensions     = "INVALID_DIMENSIONS"
	CodeInvalidArgument       = "INVALID_ARGUMENT"
	CodeEmptyInput            = "EMPTY_INPUT"
	// CodeUnknown is the code the FFI layer itself reports for status 99.
	CodeUnknown = "UNKNOWN"
	// CodeOperationFailed is used only when the native layer reports a
	// failure with no stable code (an unparsable last-error string).
	CodeOperationFailed = "OPERATION_FAILED"
)

// FFI status codes — the int32 return of every C function
// (crates/dusimagez-ffi/dusimagez.h). Untyped on purpose: C functions
// return C.int32_t, and an untyped constant compares cleanly with it.
const (
	statusOK                    = 0
	statusInvalidImage          = 1
	statusUnsupportedFormat     = 2
	statusImageTooLarge         = 3
	statusTargetSizeUnreachable = 4
	statusInvalidQuality        = 5
	statusInvalidDimensions     = 6
	statusInvalidArgument       = 7
	statusEmptyInput            = 8
	statusUnknown               = 99
)

// statusCode maps an FFI status int to its stable error code string.
func statusCode(status int32) string {
	switch status {
	case statusInvalidImage:
		return CodeInvalidImage
	case statusUnsupportedFormat:
		return CodeUnsupportedFormat
	case statusImageTooLarge:
		return CodeImageTooLarge
	case statusTargetSizeUnreachable:
		return CodeTargetSizeUnreachable
	case statusInvalidQuality:
		return CodeInvalidQuality
	case statusInvalidDimensions:
		return CodeInvalidDimensions
	case statusInvalidArgument:
		return CodeInvalidArgument
	case statusEmptyInput:
		return CodeEmptyInput
	case statusUnknown:
		return CodeUnknown
	default:
		return CodeOperationFailed
	}
}

// DusimagezError is every dusimagez failure: a stable machine-readable
// Code plus human-readable messages in English and Hindi, taken from
// the Rust core via dusimagez_last_error().
type DusimagezError struct {
	Code      string
	MessageEn string
	MessageHi string
}

// Error implements error. Format matches the other bindings:
// "CODE: english message".
func (e *DusimagezError) Error() string {
	return fmt.Sprintf("%s: %s", e.Code, e.MessageEn)
}

// ReasonEn is an alias of MessageEn.
func (e *DusimagezError) ReasonEn() string { return e.MessageEn }

// ReasonHi is an alias of MessageHi.
func (e *DusimagezError) ReasonHi() string { return e.MessageHi }

// Is reports whether target is a *DusimagezError with the same Code,
// so errors.Is(err, ErrInvalidImage) matches any INVALID_IMAGE
// failure regardless of its message.
func (e *DusimagezError) Is(target error) bool {
	t, ok := target.(*DusimagezError)
	return ok && t != nil && e != nil && t.Code == e.Code
}

// Sentinels for errors.Is matching by stable code. They carry a
// generic message; real failures carry the core's message instead.
var (
	ErrInvalidImage          = &DusimagezError{Code: CodeInvalidImage, MessageEn: "Invalid image.", MessageHi: "Galat image hai."}
	ErrUnsupportedFormat     = &DusimagezError{Code: CodeUnsupportedFormat, MessageEn: "Unsupported image format.", MessageHi: "Ye image format support nahi hai."}
	ErrImageTooLarge         = &DusimagezError{Code: CodeImageTooLarge, MessageEn: "Image is too large.", MessageHi: "Image bahut badi hai."}
	ErrTargetSizeUnreachable = &DusimagezError{Code: CodeTargetSizeUnreachable, MessageEn: "Target size is unreachable.", MessageHi: "Target size possible nahi hai."}
	ErrInvalidQuality        = &DusimagezError{Code: CodeInvalidQuality, MessageEn: "Invalid quality.", MessageHi: "Quality galat hai."}
	ErrInvalidDimensions     = &DusimagezError{Code: CodeInvalidDimensions, MessageEn: "Invalid dimensions.", MessageHi: "Galat dimensions hain."}
	ErrInvalidArgument       = &DusimagezError{Code: CodeInvalidArgument, MessageEn: "Invalid argument.", MessageHi: "Galat argument hai."}
	ErrEmptyInput            = &DusimagezError{Code: CodeEmptyInput, MessageEn: "Input is empty.", MessageHi: "Input khaali hai."}
	ErrUnknown               = &DusimagezError{Code: CodeUnknown, MessageEn: "An unexpected internal error occurred.", MessageHi: "Ek anpekkshit internal error hua."}
)

// errorJSON is the shape of dusimagez_last_error():
// {"code": "...", "message_en": "...", "message_hi": "..."}.
type errorJSON struct {
	Code      string `json:"code"`
	MessageEn string `json:"message_en"`
	MessageHi string `json:"message_hi"`
	ReasonEn  string `json:"reason_en"`
	ReasonHi  string `json:"reason_hi"`
}

// parseErrorJSON turns a last-error JSON string into a DusimagezError.
// It returns nil when the string is empty or not a recognisable error
// object, so the caller can fall back to synthesising from the status.
func parseErrorJSON(s string) *DusimagezError {
	if s == "" {
		return nil
	}
	var parsed errorJSON
	if err := json.Unmarshal([]byte(s), &parsed); err != nil {
		return nil
	}
	if parsed.Code == "" {
		return nil
	}
	en := parsed.MessageEn
	if en == "" {
		en = parsed.ReasonEn
	}
	hi := parsed.MessageHi
	if hi == "" {
		hi = parsed.ReasonHi
	}
	return &DusimagezError{Code: parsed.Code, MessageEn: en, MessageHi: hi}
}

// newError builds a client-side DusimagezError (for the cheap checks
// this package does before calling the native layer).
func newError(code, messageEn, messageHi string) *DusimagezError {
	return &DusimagezError{Code: code, MessageEn: messageEn, MessageHi: messageHi}
}

func invalidQualityError(quality int) *DusimagezError {
	return newError(CodeInvalidQuality,
		fmt.Sprintf("Quality %d is invalid. Quality must be between 1 and 100.", quality),
		fmt.Sprintf("Quality %d galat hai. Quality 1 se 100 ke beech honi chahiye.", quality))
}

func invalidArgumentError(messageEn, messageHi string) *DusimagezError {
	return newError(CodeInvalidArgument, messageEn, messageHi)
}

func invalidDimensionsError(detailEn, detailHi string) *DusimagezError {
	return newError(CodeInvalidDimensions,
		fmt.Sprintf("Invalid dimensions: %s", detailEn),
		fmt.Sprintf("Galat dimensions hain: %s", detailHi))
}
