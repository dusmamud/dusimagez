// Package dusimagez is the Go binding for dusimagez — the offline
// image compressor. One Rust core does the image work, reached here
// through cgo over the C API in crates/dusimagez-ffi (see ffi.go);
// this package only marshals options and results.
//
// Start from DefaultCompressOptions and change what you need:
//
//	opts := dusimagez.DefaultCompressOptions()
//	opts.Quality = 70
//	out, err := dusimagez.Compress(input, &opts)
//
// A nil *CompressOptions means "all defaults". Every failure is a
// *DusimagezError carrying the stable error code (INVALID_IMAGE,
// UNSUPPORTED_FORMAT, IMAGE_TOO_LARGE, TARGET_SIZE_UNREACHABLE,
// INVALID_QUALITY, INVALID_DIMENSIONS, INVALID_ARGUMENT, EMPTY_INPUT,
// UNKNOWN) plus English and Hindi messages from the Rust core —
// use errors.Is(err, dusimagez.ErrInvalidImage) to match by code.
//
// The native library is NOT bundled: build it first with
// scripts/build-ffi.sh (see README.md). The Go package links the
// static library (target/release/libdusimagez_ffi.a) at build time.
package dusimagez
