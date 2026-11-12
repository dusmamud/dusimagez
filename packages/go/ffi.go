package dusimagez

/*
#cgo linux LDFLAGS: ${SRCDIR}/../../target/release/libdusimagez_ffi.a -ldl -lpthread -lm
#cgo darwin LDFLAGS: ${SRCDIR}/../../target/release/libdusimagez_ffi.a -lpthread -lm
#cgo windows LDFLAGS: ${SRCDIR}/../../target/release/dusimagez_ffi.lib -lws2_32 -luserenv -lbcrypt
#include <stdint.h>
#include <stdlib.h>
#include <stddef.h>

// Declarations copied from crates/dusimagez-ffi/dusimagez.h —
// this file is the ONLY place cgo declarations live, so if the C API
// changes, this is the one file to fix. Do NOT #include the header:
// the Go package must build from the checked-in declarations alone.
//
// The static library is libdusimagez_ffi.a (the FFI crate's lib name
// is `dusimagez_ffi`, not `dusimagez`); build it first with
// scripts/build-ffi.sh. The struct field order below matches the
// header exactly — do not reorder fields.
typedef struct {
    int32_t quality;
    int32_t format;
    int32_t progressive;
    int32_t lossless;
    int32_t effort;
    int32_t subsampling;
    int32_t png_compression_level;
    int32_t palette;
    int32_t preset;
    int64_t target_bytes;
    int64_t max_width;
    int64_t max_height;
    int32_t strip_metadata;
    int32_t auto_orient;
    int64_t dpi_x;
    int64_t dpi_y;
    int64_t max_pixels;
    int64_t max_dimension;
} DusimagezOptions;
typedef struct {
    uint8_t *output;
    size_t output_len;
    uint64_t original_size;
    uint64_t output_size;
    uint64_t saved_bytes;
    float saved_percent;
    int32_t format;
    uint32_t width;
    uint32_t height;
    uint8_t quality_used;
    int32_t resized;
} DusimagezReport;
typedef struct {
    int32_t format;
    uint32_t width;
    uint32_t height;
    int32_t has_alpha;
    char colour_type[24];
    uint64_t file_size;
} DusimagezInfo;
int32_t dusimagez_compress(const uint8_t *input, size_t input_len,
                           const DusimagezOptions *options,
                           uint8_t **out, size_t *out_len);
int32_t dusimagez_compress_to_size(const uint8_t *input, size_t input_len,
                                   uint64_t target_bytes,
                                   const DusimagezOptions *options,
                                   uint8_t **out, size_t *out_len);
int32_t dusimagez_compress_with_report(const uint8_t *input, size_t input_len,
                                       const DusimagezOptions *options,
                                       DusimagezReport *report);
int32_t dusimagez_get_info(const uint8_t *input, size_t input_len,
                           DusimagezInfo *info);
void dusimagez_free(uint8_t *ptr, size_t len);
const char *dusimagez_last_error(void);
const char *dusimagez_version(void);
int32_t dusimagez_bulk_compress(const uint8_t *const *inputs,
                                const size_t *input_lens, size_t count,
                                const DusimagezOptions *options,
                                uint8_t **out_json, size_t *out_len);
*/
import "C"

import (
	"encoding/json"
	"unsafe"
)

// cInput returns a C pointer to the first byte of b, or nil for an
// empty slice. The FFI layer accepts a NULL pointer with length 0
// (the core then reports EMPTY_INPUT).
func cInput(b []byte) *C.uint8_t {
	if len(b) == 0 {
		return nil
	}
	return (*C.uint8_t)(unsafe.Pointer(&b[0]))
}

// cOptions translates resolved Go options into the C struct. Numeric
// fields use -1 for "unset / core default", matching the header.
func cOptions(opts CompressOptions) (C.DusimagezOptions, error) {
	var c C.DusimagezOptions
	c.quality = C.int32_t(opts.Quality)
	if opts.Format != nil {
		code, err := formatCode(*opts.Format)
		if err != nil {
			return c, err
		}
		c.format = C.int32_t(code)
	} else {
		c.format = 0
	}
	c.progressive = boolCode(opts.Progressive)
	c.lossless = boolCode(opts.Lossless)
	c.effort = C.int32_t(opts.Effort)
	sub, err := subsamplingCode(opts.Subsampling)
	if err != nil {
		return c, err
	}
	c.subsampling = C.int32_t(sub)
	c.png_compression_level = C.int32_t(opts.PNGCompressionLevel)
	c.palette = boolCode(opts.Palette)
	if opts.Preset != nil {
		code, err := presetCode(*opts.Preset)
		if err != nil {
			return c, err
		}
		c.preset = C.int32_t(code)
	} else {
		c.preset = 0
	}
	c.target_bytes = -1
	if opts.TargetBytes != nil {
		c.target_bytes = C.int64_t(*opts.TargetBytes)
	}
	c.max_width = -1
	if opts.MaxWidth != nil {
		c.max_width = C.int64_t(*opts.MaxWidth)
	}
	c.max_height = -1
	if opts.MaxHeight != nil {
		c.max_height = C.int64_t(*opts.MaxHeight)
	}
	strip, err := stripCode(opts.StripMetadata)
	if err != nil {
		return c, err
	}
	c.strip_metadata = C.int32_t(strip)
	c.auto_orient = boolCode(opts.AutoOrient)
	c.dpi_x = -1
	c.dpi_y = -1
	if opts.DPI != nil {
		c.dpi_x = C.int64_t(opts.DPI[0])
		c.dpi_y = C.int64_t(opts.DPI[1])
	}
	c.max_pixels = C.int64_t(opts.MaxPixels)
	c.max_dimension = C.int64_t(opts.MaxDimension)
	return c, nil
}

// boolCode maps a Go bool to the C bool field (0 = false, 1 = true).
// (The C struct also accepts -1 for "default"; this package always
// sends an explicit value because resolveOptions starts from the
// defaults.)
func boolCode(b bool) C.int32_t {
	if b {
		return 1
	}
	return 0
}

// lastError reads dusimagez_last_error() (NULL when the last call
// succeeded) and parses its JSON into a DusimagezError.
func lastError() *DusimagezError {
	ptr := C.dusimagez_last_error()
	if ptr == nil {
		return nil
	}
	return parseErrorJSON(C.GoString(ptr))
}

// errFromStatus builds the error for a non-zero FFI status: prefer
// the thread-local last error, fall back to synthesising from the
// status code.
func errFromStatus(status C.int32_t) error {
	if err := lastError(); err != nil {
		return err
	}
	code := statusCode(int32(status))
	return &DusimagezError{Code: code, MessageEn: "Operation failed.", MessageHi: "Operation fail ho gaya."}
}

// readBytes copies `length` bytes out of a Rust-owned buffer, then
// frees the buffer. The copy happens before the free, so the returned
// slice is fully Go-owned.
func readBytes(ptr *C.uint8_t, length C.size_t) []byte {
	if ptr == nil || length == 0 {
		return []byte{}
	}
	out := C.GoBytes(unsafe.Pointer(ptr), C.int(length))
	C.dusimagez_free(ptr, length)
	return out
}

// readJSON is readBytes for a JSON output buffer.
func readJSON(ptr *C.uint8_t, length C.size_t) []byte {
	return readBytes(ptr, length)
}

// reportFromC converts a filled DusimagezReport (whose output buffer
// this function frees after copying) into a Go CompressReport.
func reportFromC(cRep *C.DusimagezReport) (*CompressReport, error) {
	format, err := formatFromCode(int32(cRep.format))
	if err != nil {
		return nil, err
	}
	rep := &CompressReport{
		Output:       readBytes(cRep.output, cRep.output_len),
		OriginalSize: uint64(cRep.original_size),
		OutputSize:   uint64(cRep.output_size),
		SavedBytes:   uint64(cRep.saved_bytes),
		SavedPercent: float32(cRep.saved_percent),
		Format:       format,
		Width:        uint32(cRep.width),
		Height:       uint32(cRep.height),
		QualityUsed:  uint8(cRep.quality_used),
		Resized:      cRep.resized != 0,
	}
	return rep, nil
}

// infoFromC converts a filled DusimagezInfo into a Go ImageInfo.
func infoFromC(cInfo *C.DusimagezInfo) (*ImageInfo, error) {
	format, err := formatFromCode(int32(cInfo.format))
	if err != nil {
		return nil, err
	}
	return &ImageInfo{
		Format:     format,
		Width:      uint32(cInfo.width),
		Height:     uint32(cInfo.height),
		HasAlpha:   cInfo.has_alpha != 0,
		ColourType: C.GoString(&cInfo.colour_type[0]),
		FileSize:   uint64(cInfo.file_size),
	}, nil
}

// cBulkInputs builds C-malloc'd arrays of input pointers and lengths
// for dusimagez_bulk_compress. The arrays are C memory (the elements
// are Go pointers into the caller's byte slices, which stay alive for
// the synchronous call); free must be called exactly once afterwards.
func cBulkInputs(inputs [][]byte) (cInputs **C.uint8_t, cLens *C.size_t, free func()) {
	count := len(inputs)
	if count == 0 {
		return nil, nil, func() {}
	}
	cInputs = (**C.uint8_t)(C.malloc(C.size_t(count) * C.size_t(unsafe.Sizeof(uintptr(0)))))
	cLens = (*C.size_t)(C.malloc(C.size_t(count) * C.size_t(unsafe.Sizeof(C.size_t(0)))))
	ptrs := unsafe.Slice(cInputs, count)
	lens := unsafe.Slice(cLens, count)
	for i, b := range inputs {
		ptrs[i] = cInput(b)
		lens[i] = C.size_t(len(b))
	}
	return cInputs, cLens, func() {
		C.free(unsafe.Pointer(cInputs))
		C.free(unsafe.Pointer(cLens))
	}
}

// bulkItemJSON is one item of the dusimagez_bulk_compress JSON array.
type bulkItemJSON struct {
	Index        int             `json:"index"`
	OK           bool            `json:"ok"`
	Output       string          `json:"output"`
	OriginalSize uint64          `json:"original_size"`
	OutputSize   uint64          `json:"output_size"`
	SavedBytes   uint64          `json:"saved_bytes"`
	SavedPercent float32         `json:"saved_percent"`
	Format       string          `json:"format"`
	Width        uint32          `json:"width"`
	Height       uint32          `json:"height"`
	QualityUsed  uint8           `json:"quality_used"`
	Resized      bool            `json:"resized"`
	Error        json.RawMessage `json:"error"`
}

// parseBulkJSON decodes the bulk JSON array into []BulkResult, in
// index order. A malformed array is an OPERATION_FAILED error, never
// a silent empty result.
func parseBulkJSON(data []byte, count int) ([]BulkResult, error) {
	var items []bulkItemJSON
	if err := json.Unmarshal(data, &items); err != nil {
		return nil, &DusimagezError{Code: CodeOperationFailed, MessageEn: "Could not parse bulk results.", MessageHi: "Bulk results parse nahi ho sake."}
	}
	results := make([]BulkResult, count)
	for i := range results {
		results[i] = BulkResult{Err: &DusimagezError{Code: CodeOperationFailed, MessageEn: "Missing bulk result.", MessageHi: "Bulk result missing hai."}}
	}
	for _, item := range items {
		if item.Index < 0 || item.Index >= count {
			continue
		}
		if !item.OK {
			var parsed errorJSON
			if err := json.Unmarshal(item.Error, &parsed); err != nil || parsed.Code == "" {
				results[item.Index] = BulkResult{Err: &DusimagezError{Code: CodeOperationFailed, MessageEn: "Operation failed.", MessageHi: "Operation fail ho gaya."}}
			} else {
				results[item.Index] = BulkResult{Err: &DusimagezError{Code: parsed.Code, MessageEn: parsed.MessageEn, MessageHi: parsed.MessageHi}}
			}
			continue
		}
		output, err := decodeBase64(item.Output)
		if err != nil {
			results[item.Index] = BulkResult{Err: &DusimagezError{Code: CodeOperationFailed, MessageEn: "Could not decode bulk output.", MessageHi: "Bulk output decode nahi ho saka."}}
			continue
		}
		results[item.Index] = BulkResult{Report: &CompressReport{
			Output:       output,
			OriginalSize: item.OriginalSize,
			OutputSize:   item.OutputSize,
			SavedBytes:   item.SavedBytes,
			SavedPercent: item.SavedPercent,
			Format:       ImageFormat(item.Format),
			Width:        item.Width,
			Height:       item.Height,
			QualityUsed:  item.QualityUsed,
			Resized:      item.Resized,
		}}
	}
	return results, nil
}

// ---------------------------------------------------------------------------
// Public API (must live in this file: cgo declarations are only visible
// in the file whose preamble contains them)
// ---------------------------------------------------------------------------

// Version returns the native library version, e.g. "0.1.0"
// (lockstep with the core crate).
func Version() string {
	ptr := C.dusimagez_version()
	if ptr == nil {
		return ""
	}
	return C.GoString(ptr)
}

// Compress re-encodes input at a fixed quality (and Format, or the
// input format when unset). opts is nil for all defaults; start from
// DefaultCompressOptions() and change what you need. When
// opts.TargetBytes is set, Compress behaves like CompressToSize
// (SPEC section 2).
func Compress(input []byte, opts *CompressOptions) ([]byte, error) {
	resolved, err := resolveOptions(opts)
	if err != nil {
		return nil, err
	}
	cOpts, err := cOptions(resolved)
	if err != nil {
		return nil, err
	}
	var out *C.uint8_t
	var outLen C.size_t
	status := C.dusimagez_compress(cInput(input), C.size_t(len(input)), &cOpts, &out, &outLen)
	if status != statusOK {
		return nil, errFromStatus(status)
	}
	return readBytes(out, outLen), nil
}

// CompressToSize re-encodes input so the result is at most
// targetBytes bytes (iterative quality search, then downscale; see
// SPEC section 1). It fails with ErrTargetSizeUnreachable when even
// the lowest quality exceeds the target.
func CompressToSize(input []byte, targetBytes uint64, opts *CompressOptions) ([]byte, error) {
	if targetBytes == 0 {
		return nil, invalidArgumentError(
			"target_bytes must not be zero.",
			"target_bytes zero nahi ho sakta.")
	}
	resolved, err := resolveOptions(opts)
	if err != nil {
		return nil, err
	}
	cOpts, err := cOptions(resolved)
	if err != nil {
		return nil, err
	}
	var out *C.uint8_t
	var outLen C.size_t
	status := C.dusimagez_compress_to_size(cInput(input), C.size_t(len(input)), C.uint64_t(targetBytes), &cOpts, &out, &outLen)
	if status != statusOK {
		return nil, errFromStatus(status)
	}
	return readBytes(out, outLen), nil
}

// CompressWithReport re-encodes input and returns the before/after
// report (sizes, saved bytes/percent, format, dimensions, quality
// actually used, whether it was resized). The compressed bytes are
// in Report.Output.
func CompressWithReport(input []byte, opts *CompressOptions) (*CompressReport, error) {
	resolved, err := resolveOptions(opts)
	if err != nil {
		return nil, err
	}
	cOpts, err := cOptions(resolved)
	if err != nil {
		return nil, err
	}
	var cRep C.DusimagezReport
	status := C.dusimagez_compress_with_report(cInput(input), C.size_t(len(input)), &cOpts, &cRep)
	if status != statusOK {
		return nil, errFromStatus(status)
	}
	return reportFromC(&cRep)
}

// GetInfo returns the image summary: format, dimensions, alpha,
// colour type and file size.
func GetInfo(input []byte) (*ImageInfo, error) {
	var cInfo C.DusimagezInfo
	status := C.dusimagez_get_info(cInput(input), C.size_t(len(input)), &cInfo)
	if status != statusOK {
		return nil, errFromStatus(status)
	}
	return infoFromC(&cInfo)
}

// BulkCompress runs Compress over every input; one BulkResult per
// input, same order. A bad input fails only its own item (its Err is
// set, Report is nil) and never aborts the batch. An empty input
// list returns an empty (non-nil) slice with a nil error.
func BulkCompress(inputs [][]byte, opts *CompressOptions) ([]BulkResult, error) {
	resolved, err := resolveOptions(opts)
	if err != nil {
		return nil, err
	}
	cOpts, err := cOptions(resolved)
	if err != nil {
		return nil, err
	}
	cInputs, cLens, free := cBulkInputs(inputs)
	defer free()
	var out *C.uint8_t
	var outLen C.size_t
	status := C.dusimagez_bulk_compress(cInputs, cLens, C.size_t(len(inputs)), &cOpts, &out, &outLen)
	if status != statusOK {
		return nil, errFromStatus(status)
	}
	return parseBulkJSON(readJSON(out, outLen), len(inputs))
}
