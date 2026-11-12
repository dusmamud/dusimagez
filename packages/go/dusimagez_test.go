package dusimagez

import (
	"bytes"
	"encoding/base64"
	"errors"
	"image/png"
	"testing"
)

// Embedded fixtures (no binary blobs committed): a 1x1 greyscale+alpha
// PNG (68 bytes, colour type "la8" — core lowercases it) and garbage bytes.
const png1x1Base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR4nGNgYAAAAAMAAbitOmMAAAAASUVORK5CYII="

func png1x1(t *testing.T) []byte {
	t.Helper()
	b, err := base64.StdEncoding.DecodeString(png1x1Base64)
	if err != nil {
		t.Fatal(err)
	}
	return b
}

var garbageBytes = []byte("this is not an image at all, just plain text bytes 1234567890")

func TestDefaultOptions(t *testing.T) {
	o := DefaultCompressOptions()
	if o.Quality != 80 || o.Format != nil || o.Progressive || o.Lossless ||
		o.Effort != 6 || o.Subsampling != "" || o.PNGCompressionLevel != 6 ||
		o.Palette || o.Preset != nil || o.TargetBytes != nil ||
		o.MaxWidth != nil || o.MaxHeight != nil || o.StripMetadata != "" ||
		!o.AutoOrient || o.DPI != nil || o.MaxPixels != 120_000_000 || o.MaxDimension != 16000 {
		t.Fatalf("DefaultCompressOptions = %+v", o)
	}
}

func TestValidateOptions(t *testing.T) {
	o := DefaultCompressOptions()
	o.Quality = 0
	if err := o.validate(); !errors.Is(err, ErrInvalidQuality) {
		t.Fatalf("quality 0: err = %v, want INVALID_QUALITY", err)
	}
	o = DefaultCompressOptions()
	o.Quality = 101
	if err := o.validate(); !errors.Is(err, ErrInvalidQuality) {
		t.Fatalf("quality 101: err = %v, want INVALID_QUALITY", err)
	}
	o = DefaultCompressOptions()
	o.Effort = 11
	if err := o.validate(); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("effort 11: err = %v, want INVALID_ARGUMENT", err)
	}
	o = DefaultCompressOptions()
	o.PNGCompressionLevel = 10
	if err := o.validate(); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("png level 10: err = %v, want INVALID_ARGUMENT", err)
	}
	o = DefaultCompressOptions()
	zero := uint64(0)
	o.TargetBytes = &zero
	if err := o.validate(); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("target 0: err = %v, want INVALID_ARGUMENT", err)
	}
}

func TestErrorIsMatching(t *testing.T) {
	err := &DusimagezError{Code: CodeInvalidImage, MessageEn: "x", MessageHi: "y"}
	if !errors.Is(err, ErrInvalidImage) {
		t.Fatalf("errors.Is did not match INVALID_IMAGE")
	}
	if errors.Is(err, ErrEmptyInput) {
		t.Fatalf("errors.Is matched the wrong code")
	}
	if got := err.Error(); got != "INVALID_IMAGE: x" {
		t.Fatalf("Error() = %q", got)
	}
}

func TestParseErrorJSON(t *testing.T) {
	err := parseErrorJSON(`{"code":"EMPTY_INPUT","message_en":"Input is empty.","message_hi":"Input khaali hai."}`)
	if err == nil || err.Code != CodeEmptyInput || err.MessageEn != "Input is empty." || err.MessageHi != "Input khaali hai." {
		t.Fatalf("parseErrorJSON = %+v", err)
	}
	if parseErrorJSON("") != nil || parseErrorJSON("not json") != nil || parseErrorJSON(`{"no_code":1}`) != nil {
		t.Fatalf("parseErrorJSON should return nil for unparsable input")
	}
	// reason_* spellings are accepted too.
	err = parseErrorJSON(`{"code":"INVALID_IMAGE","reason_en":"bad","reason_hi":"galat"}`)
	if err == nil || err.MessageEn != "bad" || err.MessageHi != "galat" {
		t.Fatalf("parseErrorJSON reason_* = %+v", err)
	}
}

func TestFormatCodes(t *testing.T) {
	for _, tc := range []struct {
		format ImageFormat
		code   int32
	}{
		{"", 0},
		{FormatJPEG, 1}, {"jpg", 1},
		{FormatPNG, 2},
		{FormatWebP, 3},
		{FormatAVIF, 4},
		{FormatGIF, 5},
		{FormatBMP, 6},
		{FormatTIFF, 7},
	} {
		code, err := formatCode(tc.format)
		if err != nil || code != tc.code {
			t.Fatalf("formatCode(%q) = %d, %v; want %d", tc.format, code, err, tc.code)
		}
		back, err := formatFromCode(tc.code)
		if err != nil {
			t.Fatalf("formatFromCode(%d): %v", tc.code, err)
		}
		if tc.code == 0 {
			if back != "" {
				t.Fatalf("formatFromCode(0) = %q, want empty", back)
			}
		} else if ImageFormat(back) != tc.format && !(tc.format == "jpg" && back == FormatJPEG) {
			t.Fatalf("formatFromCode(%d) = %q, want %q", tc.code, back, tc.format)
		}
	}
	if _, err := formatCode("heic"); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("formatCode(heic): err = %v, want INVALID_ARGUMENT", err)
	}
	if f, err := FormatFromString("JPG"); err != nil || f != FormatJPEG {
		t.Fatalf("FormatFromString(JPG) = %q, %v", f, err)
	}
}

func TestEnumCodes(t *testing.T) {
	if c, _ := subsamplingCode("444"); c != 1 {
		t.Fatalf("subsampling 444 = %d", c)
	}
	if c, _ := subsamplingCode("S420"); c != 3 {
		t.Fatalf("subsampling S420 = %d", c)
	}
	if c, _ := presetCode("social"); c != 2 {
		t.Fatalf("preset social = %d", c)
	}
	if c, _ := stripCode("keep_exif"); c != 1 {
		t.Fatalf("strip keep_exif = %d", c)
	}
	if _, err := presetCode("nope"); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("preset nope: err = %v", err)
	}
}

// --- Native tests (need the built static lib: scripts/build-ffi.sh) ---

func TestCompressPNG(t *testing.T) {
	out, err := Compress(png1x1(t), nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(out) == 0 || !bytes.HasPrefix(out, []byte("\x89PNG\r\n\x1a\n")) {
		t.Fatalf("Compress output is not a PNG (%d bytes)", len(out))
	}
	// Output must still decode as a 1x1 PNG.
	cfg, err := png.DecodeConfig(bytes.NewReader(out))
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Width != 1 || cfg.Height != 1 {
		t.Fatalf("decoded size = %dx%d, want 1x1", cfg.Width, cfg.Height)
	}
}

func TestCompressWithReport(t *testing.T) {
	png := png1x1(t)
	rep, err := CompressWithReport(png, nil)
	if err != nil {
		t.Fatal(err)
	}
	if rep.OriginalSize != uint64(len(png)) {
		t.Fatalf("OriginalSize = %d, want %d", rep.OriginalSize, len(png))
	}
	if rep.OutputSize != uint64(len(rep.Output)) {
		t.Fatalf("OutputSize = %d, len(Output) = %d", rep.OutputSize, len(rep.Output))
	}
	if rep.Format != FormatPNG || rep.Width != 1 || rep.Height != 1 || rep.QualityUsed != 80 {
		t.Fatalf("report = %+v", rep)
	}
	if rep.SavedBytes != rep.OriginalSize-rep.OutputSize {
		t.Fatalf("SavedBytes = %d, want %d", rep.SavedBytes, rep.OriginalSize-rep.OutputSize)
	}
}

func TestCompressToSize(t *testing.T) {
	out, err := CompressToSize(png1x1(t), 10, nil)
	if err == nil {
		t.Fatalf("CompressToSize(10) succeeded with %d bytes, want TARGET_SIZE_UNREACHABLE", len(out))
	}
	if !errors.Is(err, ErrTargetSizeUnreachable) {
		t.Fatalf("CompressToSize(10): err = %v, want TARGET_SIZE_UNREACHABLE", err)
	}
}

func TestGetInfoPNG(t *testing.T) {
	info, err := GetInfo(png1x1(t))
	if err != nil {
		t.Fatal(err)
	}
	if info.Format != FormatPNG || info.Width != 1 || info.Height != 1 ||
		!info.HasAlpha || info.ColourType != "la8" || info.FileSize != uint64(len(png1x1(t))) {
		t.Fatalf("GetInfo = %+v", info)
	}
}

func TestCompressErrors(t *testing.T) {
	if _, err := Compress(nil, nil); !errors.Is(err, ErrEmptyInput) {
		t.Fatalf("Compress(nil): err = %v, want EMPTY_INPUT", err)
	}
	if _, err := Compress([]byte{}, nil); !errors.Is(err, ErrEmptyInput) {
		t.Fatalf("Compress(empty): err = %v, want EMPTY_INPUT", err)
	}
	if _, err := Compress(garbageBytes, nil); !errors.Is(err, ErrInvalidImage) {
		t.Fatalf("Compress(garbage): err = %v, want INVALID_IMAGE", err)
	}
	if _, err := GetInfo(garbageBytes); !errors.Is(err, ErrInvalidImage) {
		t.Fatalf("GetInfo(garbage): err = %v, want INVALID_IMAGE", err)
	}
	opts := DefaultCompressOptions()
	opts.Quality = 0
	if _, err := Compress(png1x1(t), &opts); !errors.Is(err, ErrInvalidQuality) {
		t.Fatalf("Compress(quality 0): err = %v, want INVALID_QUALITY", err)
	}
}

func TestBulkCompressMixed(t *testing.T) {
	results, err := BulkCompress([][]byte{png1x1(t), garbageBytes}, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(results) != 2 {
		t.Fatalf("len(results) = %d, want 2", len(results))
	}
	if results[0].Err != nil || results[0].Report == nil || len(results[0].Report.Output) == 0 {
		t.Fatalf("results[0] = %+v", results[0])
	}
	if !errors.Is(results[1].Err, ErrInvalidImage) || results[1].Report != nil {
		t.Fatalf("results[1] = %+v, want INVALID_IMAGE", results[1])
	}
	empty, err := BulkCompress(nil, nil)
	if err != nil || len(empty) != 0 {
		t.Fatalf("BulkCompress(nil) = %v, %v", empty, err)
	}
}

func TestVersion(t *testing.T) {
	if v := Version(); v != "0.1.0" {
		t.Fatalf("Version() = %q, want 0.1.0", v)
	}
}
