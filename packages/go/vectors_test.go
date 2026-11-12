package dusimagez

// Shared test vectors (SPEC section 6): Go runs shared/test-vectors.json
// and must agree with the Rust, Python, JS, PHP, Kotlin and Swift
// runners. The file's shape (W5, v0.1.0):
//
//	{
//	  "version": "0.1.0",
//	  "fixtures": [{"name", "kind", "format", "width", "height",
//	                "seed"?, "file"?, "generate"?}, ...],
//	  "cases": [{"id", "op", "fixture" | "fixtures", "options",
//	             "expect", "notes"?}, ...]
//	}
//
// Ops: "compress", "compress_with_report", "compress_to_size",
// "get_info", "bulk". A case's "fixture" / "fixtures" are fixture
// NAMES resolved against the top-level "fixtures" list. "expect"
// keys: format, width, height, max_output_bytes, min_saved_percent,
// quality_used, resized, error_code; bulk cases expect a "results"
// array with one expect object per item.
//
// Fixtures:
//   - A descriptor with a "file" is loaded from shared/<file> (exact).
//   - Otherwise `solid` / `gradient` / `noise` come from the
//     `dusimagez mock` CLI, with the exact flags the vectors file
//     records: `mock --kind <kind> --width <w> --height <h>
//     --out <file>` (format follows the extension, fixed seed).
//     The CLI is
//     located via $DUSIMAGEZ_CLI, else target/release/dusimagez in a
//     repo checkout, else PATH. If no CLI is found, fixtures are
//     generated in-test with image/png instead — those fixtures are
//     marked inexact, and size-bound assertions (max_output_bytes /
//     min_saved_percent) are skipped for them with a t.Log, because
//     exact byte sizes depend on the CLI's pixels. Dimension / format
//     / error assertions still run on every case.
//   - Derived fixtures (jpeg, tiff, garbage, empty) are built here
//     with the standard library plus a minimal TIFF writer — no
//     binary blobs are committed.
//
// Unknown fixture kinds/names, ops or expect keys fail the case by
// name instead of silently skipping it. Pixel checks decode PNG
// output with the standard library.

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"image"
	"image/color"
	"image/jpeg"
	"image/png"
	"math/rand"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"testing"
)

// ---------------------------------------------------------------------------
// Loading the vectors
// ---------------------------------------------------------------------------

type fixtureDef struct {
	Name     string `json:"name"`
	Kind     string `json:"kind"`
	Format   string `json:"format"`
	Width    int    `json:"width"`
	Height   int    `json:"height"`
	Seed     uint64 `json:"seed"`
	File     string `json:"file"`
	Generate string `json:"generate"`
}

type vectorFile struct {
	Version  string       `json:"version"`
	Fixtures []fixtureDef `json:"fixtures"`
	Cases    []vectorCase `json:"cases"`
}

type vectorCase struct {
	ID       string                     `json:"id"`
	Op       string                     `json:"op"`
	Fixture  string                     `json:"fixture"`
	Fixtures []string                   `json:"fixtures"`
	Options  json.RawMessage            `json:"options"`
	Expect   map[string]json.RawMessage `json:"expect"`
}

func loadVectors(t *testing.T) (*vectorFile, string) {
	t.Helper()
	path := os.Getenv("DUSIMAGEZ_VECTORS")
	if path == "" {
		_, thisFile, _, ok := runtime.Caller(0)
		if ok {
			for _, c := range []string{
				filepath.Join(filepath.Dir(thisFile), "..", "..", "shared", "test-vectors.json"),
				filepath.Join("..", "..", "shared", "test-vectors.json"),
				filepath.Join("shared", "test-vectors.json"),
			} {
				if _, err := os.Stat(c); err == nil {
					path = c
					break
				}
			}
		}
	}
	if path == "" {
		return nil, ""
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var vf vectorFile
	if err := json.Unmarshal(raw, &vf); err != nil {
		t.Fatal(err)
	}
	return &vf, path
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

var (
	fixtureCache   = map[string]cachedFixture{}
	fixtureCacheMu sync.Mutex
)

type cachedFixture struct {
	bytes []byte
	exact bool
}

// cliPath locates the `dusimagez` CLI binary used for mock fixtures:
// $DUSIMAGEZ_CLI, else the release binary next to this repo, else
// PATH. It returns "" when nothing is found — the caller then falls
// back to in-test PNG generation (inexact fixtures).
func cliPath() string {
	if p := os.Getenv("DUSIMAGEZ_CLI"); p != "" {
		return p
	}
	_, thisFile, _, ok := runtime.Caller(0)
	if ok {
		p := filepath.Join(filepath.Dir(thisFile), "..", "..", "target", "release", "dusimagez")
		if _, err := os.Stat(p); err == nil {
			return p
		}
	}
	if p, err := exec.LookPath("dusimagez"); err == nil {
		return p
	}
	return ""
}

// mockViaCLI runs `dusimagez mock` into a temp file and returns the bytes.
// Real CLI flags only: --kind/--width/--height/--out; the format
// follows the output extension (fixed seed in the CLI itself).
func mockViaCLI(cli, kind, format string, width, height int) ([]byte, error) {
	dir, err := os.MkdirTemp("", "dusimagez-mock-*")
	if err != nil {
		return nil, err
	}
	defer os.RemoveAll(dir)
	out := filepath.Join(dir, "mock."+format)
	cmd := exec.Command(cli, "mock",
		"--kind", kind,
		"--width", fmt.Sprint(width),
		"--height", fmt.Sprint(height),
		"--out", out)
	if err := cmd.Run(); err != nil {
		return nil, fmt.Errorf("dusimagez mock %s: %w", kind, err)
	}
	return os.ReadFile(out)
}

// pngBytes encodes an RGBA image as PNG.
func pngBytes(img image.Image) []byte {
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		panic(err)
	}
	return buf.Bytes()
}

// solidPNG builds a solid-colour PNG in-test (inexact fixture).
func solidPNG(width, height int, rgb [3]int) []byte {
	img := image.NewRGBA(image.Rect(0, 0, width, height))
	c := color.RGBA{uint8(rgb[0]), uint8(rgb[1]), uint8(rgb[2]), 255}
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			img.Set(x, y, c)
		}
	}
	return pngBytes(img)
}

// gradientPNG builds a horizontal black-to-white gradient PNG in-test.
func gradientPNG(width, height int) []byte {
	img := image.NewRGBA(image.Rect(0, 0, width, height))
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			v := uint8(0)
			if width > 1 {
				v = uint8(x * 255 / (width - 1))
			}
			img.Set(x, y, color.RGBA{v, v, v, 255})
		}
	}
	return pngBytes(img)
}

// noisePNG builds a deterministic pseudo-random PNG in-test (seeded).
func noisePNG(width, height int, seed uint64) []byte {
	img := image.NewRGBA(image.Rect(0, 0, width, height))
	rng := rand.New(rand.NewSource(int64(seed)))
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			img.Set(x, y, color.RGBA{uint8(rng.Intn(256)), uint8(rng.Intn(256)), uint8(rng.Intn(256)), 255})
		}
	}
	return pngBytes(img)
}

// jpegBytes encodes a solid-colour JPEG in-test (inexact fixture).
func jpegBytes(width, height int, rgb [3]int) []byte {
	img := image.NewRGBA(image.Rect(0, 0, width, height))
	c := color.RGBA{uint8(rgb[0]), uint8(rgb[1]), uint8(rgb[2]), 255}
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			img.Set(x, y, c)
		}
	}
	return encodeJPEG(img)
}

// encodeJPEG encodes img as a JPEG (quality 90) in-test.
func encodeJPEG(img image.Image) []byte {
	var buf bytes.Buffer
	if err := jpeg.Encode(&buf, img, &jpeg.Options{Quality: 90}); err != nil {
		panic(err)
	}
	return buf.Bytes()
}

// gradientImage builds a horizontal black-to-white gradient image.
func gradientImage(width, height int) image.Image {
	img := image.NewRGBA(image.Rect(0, 0, width, height))
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			v := uint8(0)
			if width > 1 {
				v = uint8(x * 255 / (width - 1))
			}
			img.Set(x, y, color.RGBA{v, v, v, 255})
		}
	}
	return img
}

// noiseImage builds a deterministic pseudo-random image (seeded).
func noiseImage(width, height int, seed uint64) image.Image {
	img := image.NewRGBA(image.Rect(0, 0, width, height))
	rng := rand.New(rand.NewSource(int64(seed)))
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			img.Set(x, y, color.RGBA{uint8(rng.Intn(256)), uint8(rng.Intn(256)), uint8(rng.Intn(256)), 255})
		}
	}
	return img
}

// tiffBytes writes a minimal uncompressed little-endian TIFF (8-bit
// RGB, one strip) for one solid colour.
func tiffBytes(width, height int, rgb [3]int) []byte {
	var buf bytes.Buffer
	pixels := make([]byte, 0, width*height*3)
	for i := 0; i < width*height; i++ {
		pixels = append(pixels, byte(rgb[0]), byte(rgb[1]), byte(rgb[2]))
	}
	// Header (8) + IFD: count (2) + 10 entries * 12 + next (4) = 126,
	// then BitsPerSample array (6), then pixel data.
	const ifdEntries = 10
	headerSize := 8
	ifdSize := 2 + ifdEntries*12 + 4
	bitsOffset := headerSize + ifdSize
	dataOffset := bitsOffset + 6
	type entry struct {
		tag, typ uint16
		count    uint32
		value    uint32
	}
	entries := []entry{
		{256, 4, 1, uint32(width)},       // ImageWidth, LONG
		{257, 4, 1, uint32(height)},      // ImageLength, LONG
		{258, 3, 3, uint32(bitsOffset)},  // BitsPerSample, SHORT x3 (offset)
		{259, 3, 1, 1},                   // Compression: none
		{262, 3, 1, 2},                   // Photometric: RGB
		{273, 4, 1, uint32(dataOffset)},  // StripOffsets, LONG
		{277, 3, 1, 3},                   // SamplesPerPixel
		{278, 4, 1, uint32(height)},      // RowsPerStrip
		{279, 4, 1, uint32(len(pixels))}, // StripByteCounts
		{284, 3, 1, 1},                   // PlanarConfiguration: chunky
	}
	buf.Write([]byte("II*\x00"))
	_ = binary.Write(&buf, binary.LittleEndian, uint32(headerSize))
	_ = binary.Write(&buf, binary.LittleEndian, uint16(ifdEntries))
	for _, e := range entries {
		_ = binary.Write(&buf, binary.LittleEndian, e.tag)
		_ = binary.Write(&buf, binary.LittleEndian, e.typ)
		_ = binary.Write(&buf, binary.LittleEndian, e.count)
		_ = binary.Write(&buf, binary.LittleEndian, e.value)
	}
	_ = binary.Write(&buf, binary.LittleEndian, uint32(0))
	_ = binary.Write(&buf, binary.LittleEndian, uint16(8))
	_ = binary.Write(&buf, binary.LittleEndian, uint16(8))
	_ = binary.Write(&buf, binary.LittleEndian, uint16(8))
	buf.Write(pixels)
	return buf.Bytes()
}

// fixture resolves a fixture NAME against the vectors file's fixture
// list and builds its bytes. The second return value is true when
// the bytes are exactly the shared-vector fixture (committed file,
// CLI mock, or a deterministic derived encoding), false for in-test
// approximations (size-bound assertions are skipped for those).
func fixture(t *testing.T, vf *vectorFile, vectorsPath, name string) ([]byte, bool) {
	t.Helper()
	fixtureCacheMu.Lock()
	if c, ok := fixtureCache[name]; ok {
		fixtureCacheMu.Unlock()
		return c.bytes, c.exact
	}
	fixtureCacheMu.Unlock()

	var def *fixtureDef
	for i := range vf.Fixtures {
		if vf.Fixtures[i].Name == name {
			def = &vf.Fixtures[i]
			break
		}
	}
	if def == nil {
		t.Fatalf("unknown fixture %q (not in shared/test-vectors.json fixtures)", name)
	}

	var out []byte
	exact := true
	switch {
	case def.File != "":
		// Committed file, relative to the repo root (shared/...).
		candidates := []string{
			filepath.Join(filepath.Dir(vectorsPath), def.File[strings.Index(def.File, "/")+1:]),
			filepath.Join(filepath.Dir(vectorsPath), "..", def.File),
			def.File,
		}
		var readErr error
		for _, c := range candidates {
			if b, err := os.ReadFile(c); err == nil {
				out = b
				break
			} else {
				readErr = err
			}
		}
		if out == nil {
			t.Fatalf("fixture %q: cannot read %s: %v", name, def.File, readErr)
		}
	case def.Kind == "empty":
		out = []byte{}
	case def.Kind == "garbage":
		out = []byte("this is not an image at all, just plain text bytes 1234567890")
	case def.Kind == "solid" || def.Kind == "gradient" || def.Kind == "noise":
		if def.Width <= 0 || def.Height <= 0 {
			t.Fatalf("fixture %q: width/height required", name)
		}
		format := def.Format
		if format == "" {
			format = "png"
		}
		if cli := cliPath(); cli != "" {
			b, err := mockViaCLI(cli, def.Kind, format, def.Width, def.Height)
			if err != nil {
				t.Fatalf("fixture %q: %v", name, err)
			}
			out = b
		} else {
			exact = false
			rgb := [3]int{255, 0, 0} // fixtures are named solid-red-*
			switch def.Kind {
			case "solid":
				if format == "jpeg" {
					out = jpegBytes(def.Width, def.Height, rgb)
				} else {
					out = solidPNG(def.Width, def.Height, rgb)
				}
			case "gradient":
				if format == "jpeg" {
					out = encodeJPEG(gradientImage(def.Width, def.Height))
				} else {
					out = gradientPNG(def.Width, def.Height)
				}
			case "noise":
				if format == "jpeg" {
					out = encodeJPEG(noiseImage(def.Width, def.Height, def.Seed))
				} else {
					out = noisePNG(def.Width, def.Height, def.Seed)
				}
			}
		}
	default:
		t.Fatalf("unknown fixture kind %q (fixture %q)", def.Kind, name)
	}

	fixtureCacheMu.Lock()
	fixtureCache[name] = cachedFixture{bytes: out, exact: exact}
	fixtureCacheMu.Unlock()
	return out, exact
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

type vectorOptions struct {
	Quality             *int       `json:"quality"`
	Format              *string    `json:"format"`
	Progressive         *bool      `json:"progressive"`
	Lossless            *bool      `json:"lossless"`
	Effort              *int       `json:"effort"`
	Subsampling         *string    `json:"subsampling"`
	PNGCompressionLevel *int       `json:"png_compression_level"`
	Palette             *bool      `json:"palette"`
	Preset              *string    `json:"preset"`
	TargetBytes         *uint64    `json:"target_bytes"`
	MaxWidth            *uint32    `json:"max_width"`
	MaxHeight           *uint32    `json:"max_height"`
	StripMetadata       *string    `json:"strip_metadata"`
	AutoOrient          *bool      `json:"auto_orient"`
	DPI                 *[2]uint32 `json:"dpi"`
	MaxPixels           *uint64    `json:"max_pixels"`
	MaxDimension        *uint32    `json:"max_dimension"`
}

func optionsFromVector(raw json.RawMessage) *CompressOptions {
	if len(raw) == 0 || string(raw) == "null" {
		return nil
	}
	var v vectorOptions
	if err := json.Unmarshal(raw, &v); err != nil {
		return nil
	}
	opts := DefaultCompressOptions()
	if v.Quality != nil {
		opts.Quality = uint8(*v.Quality)
	}
	if v.Format != nil {
		f := ImageFormat(strings.ToLower(*v.Format))
		opts.Format = &f
	}
	if v.Progressive != nil {
		opts.Progressive = *v.Progressive
	}
	if v.Lossless != nil {
		opts.Lossless = *v.Lossless
	}
	if v.Effort != nil {
		opts.Effort = uint8(*v.Effort)
	}
	if v.Subsampling != nil {
		opts.Subsampling = Subsampling(*v.Subsampling)
	}
	if v.PNGCompressionLevel != nil {
		opts.PNGCompressionLevel = uint8(*v.PNGCompressionLevel)
	}
	if v.Palette != nil {
		opts.Palette = *v.Palette
	}
	if v.Preset != nil {
		p := Preset(strings.ToLower(*v.Preset))
		opts.Preset = &p
	}
	if v.TargetBytes != nil {
		opts.TargetBytes = v.TargetBytes
	}
	if v.MaxWidth != nil {
		opts.MaxWidth = v.MaxWidth
	}
	if v.MaxHeight != nil {
		opts.MaxHeight = v.MaxHeight
	}
	if v.StripMetadata != nil {
		opts.StripMetadata = StripMetadata(strings.ToLower(*v.StripMetadata))
	}
	if v.AutoOrient != nil {
		opts.AutoOrient = *v.AutoOrient
	}
	if v.DPI != nil {
		opts.DPI = v.DPI
	}
	if v.MaxPixels != nil {
		opts.MaxPixels = *v.MaxPixels
	}
	if v.MaxDimension != nil {
		opts.MaxDimension = *v.MaxDimension
	}
	return &opts
}

// ---------------------------------------------------------------------------
// Expect checks
// ---------------------------------------------------------------------------

func expectString(t *testing.T, id, key string, expect map[string]json.RawMessage, got string) {
	t.Helper()
	raw, ok := expect[key]
	if !ok {
		return
	}
	var want string
	if err := json.Unmarshal(raw, &want); err != nil {
		t.Errorf("%s: expect.%s is not a string: %s", id, key, raw)
		return
	}
	if got != want {
		t.Errorf("%s: %s = %q, want %q", id, key, got, want)
	}
}

func expectUint(t *testing.T, id, key string, expect map[string]json.RawMessage, got uint64) {
	t.Helper()
	raw, ok := expect[key]
	if !ok {
		return
	}
	var want uint64
	if err := json.Unmarshal(raw, &want); err != nil {
		t.Errorf("%s: expect.%s is not a number: %s", id, key, raw)
		return
	}
	if got != want {
		t.Errorf("%s: %s = %d, want %d", id, key, got, want)
	}
}

func expectBool(t *testing.T, id, key string, expect map[string]json.RawMessage, got bool) {
	t.Helper()
	raw, ok := expect[key]
	if !ok {
		return
	}
	var want bool
	if err := json.Unmarshal(raw, &want); err != nil {
		t.Errorf("%s: expect.%s is not a bool: %s", id, key, raw)
		return
	}
	if got != want {
		t.Errorf("%s: %s = %v, want %v", id, key, got, want)
	}
}

// checkExpect runs the assertions a case supports. Size bounds are
// only checked when exact is true (see the file header). Unknown
// expect keys fail the case by name.
func checkExpect(t *testing.T, id string, expect map[string]json.RawMessage, info *ImageInfo, rep *CompressReport, out []byte, exact bool) {
	t.Helper()
	known := map[string]bool{
		"format": true, "width": true, "height": true, "has_alpha": true,
		"colour_type": true, "file_size": true, "error_code": true,
		"max_output_bytes": true, "min_output_bytes": true,
		"min_saved_percent": true, "quality_used": true, "resized": true,
		"output_smaller": true,
	}
	for key := range expect {
		if !known[key] {
			t.Errorf("%s: unknown expect key %q", id, key)
		}
	}
	if info != nil {
		expectString(t, id, "format", expect, string(info.Format))
		expectUint(t, id, "width", expect, uint64(info.Width))
		expectUint(t, id, "height", expect, uint64(info.Height))
		expectBool(t, id, "has_alpha", expect, info.HasAlpha)
		expectString(t, id, "colour_type", expect, info.ColourType)
		expectUint(t, id, "file_size", expect, info.FileSize)
	}
	if rep != nil {
		expectString(t, id, "format", expect, string(rep.Format))
		expectUint(t, id, "width", expect, uint64(rep.Width))
		expectUint(t, id, "height", expect, uint64(rep.Height))
		expectUint(t, id, "quality_used", expect, uint64(rep.QualityUsed))
		expectBool(t, id, "resized", expect, rep.Resized)
		if raw, ok := expect["output_smaller"]; ok {
			var want bool
			if err := json.Unmarshal(raw, &want); err == nil && want && rep.OutputSize >= rep.OriginalSize {
				t.Errorf("%s: output_size %d is not smaller than original %d", id, rep.OutputSize, rep.OriginalSize)
			}
		}
	}
	if out != nil {
		if exact {
			if raw, ok := expect["max_output_bytes"]; ok {
				var max uint64
				if err := json.Unmarshal(raw, &max); err == nil && uint64(len(out)) > max {
					t.Errorf("%s: output size %d exceeds max %d", id, len(out), max)
				}
			}
			if raw, ok := expect["min_output_bytes"]; ok {
				var min uint64
				if err := json.Unmarshal(raw, &min); err == nil && uint64(len(out)) < min {
					t.Errorf("%s: output size %d is below min %d", id, len(out), min)
				}
			}
		} else if _, ok := expect["max_output_bytes"]; ok {
			t.Logf("%s: skipping max_output_bytes (inexact fixture, no CLI)", id)
		}
	}
	if rep != nil && exact {
		if raw, ok := expect["min_saved_percent"]; ok {
			var min float64
			if err := json.Unmarshal(raw, &min); err == nil && float64(rep.SavedPercent) < min {
				t.Errorf("%s: saved_percent %v is below min %v", id, rep.SavedPercent, min)
			}
		}
	} else if rep != nil && !exact {
		if _, ok := expect["min_saved_percent"]; ok {
			t.Logf("%s: skipping min_saved_percent (inexact fixture, no CLI)", id)
		}
	}
}

// expectErrorCode asserts the case expects an error with the given
// code and fails the case if the call succeeded or failed
// differently. It returns true when the caller should stop.
func expectErrorCode(t *testing.T, id string, expect map[string]json.RawMessage, err error) bool {
	t.Helper()
	raw, ok := expect["error_code"]
	if !ok {
		if err != nil {
			t.Errorf("%s: unexpected error: %v", id, err)
		}
		return err != nil
	}
	var want string
	if err := json.Unmarshal(raw, &want); err != nil {
		t.Errorf("%s: expect.error_code is not a string: %s", id, raw)
		return true
	}
	if err == nil {
		t.Errorf("%s: expected error %s, got success", id, want)
		return true
	}
	dErr, ok := err.(*DusimagezError)
	if !ok || dErr.Code != want {
		t.Errorf("%s: error = %v, want code %s", id, err, want)
		return true
	}
	return true
}

// checkBulkItem checks one bulk result against its per-item expect
// object (format/width/height/max_output_bytes, or error_code).
func checkBulkItem(t *testing.T, id string, index int, expect map[string]json.RawMessage, result BulkResult, exact bool) {
	t.Helper()
	if raw, ok := expect["error_code"]; ok {
		var want string
		if err := json.Unmarshal(raw, &want); err != nil {
			t.Errorf("%s: item %d: expect.error_code is not a string", id, index)
			return
		}
		dErr, isD := result.Err.(*DusimagezError)
		if result.Report != nil || !isD || dErr.Code != want {
			t.Errorf("%s: item %d err = %v, want code %s", id, index, result.Err, want)
		}
		return
	}
	if result.Err != nil {
		t.Errorf("%s: item %d: unexpected error: %v", id, index, result.Err)
		return
	}
	checkExpect(t, fmt.Sprintf("%s item %d", id, index), expect, nil, result.Report, result.Report.Output, exact)
}

// ---------------------------------------------------------------------------
// The test
// ---------------------------------------------------------------------------

func TestSharedVectors(t *testing.T) {
	vf, vectorsPath := loadVectors(t)
	if vf == nil {
		t.Log("shared/test-vectors.json not found (W5 has not created it yet) — running embedded-fixture fallback cases instead")
		out, err := Compress(png1x1(t), nil)
		if err != nil {
			t.Fatalf("fallback compress: %v", err)
		}
		if len(out) == 0 {
			t.Fatalf("fallback compress returned empty output")
		}
		if _, err := Compress(garbageBytes, nil); err == nil {
			t.Fatalf("fallback compress(garbage) should fail")
		}
		return
	}
	if len(vf.Cases) == 0 {
		t.Fatal("shared/test-vectors.json has no cases")
	}
	for _, tc := range vf.Cases {
		t.Run(tc.ID, func(t *testing.T) {
			opts := optionsFromVector(tc.Options)
			switch tc.Op {
			case "get_info":
				input, _ := fixture(t, vf, vectorsPath, tc.Fixture)
				info, err := GetInfo(input)
				if expectErrorCode(t, tc.ID, tc.Expect, err) {
					return
				}
				checkExpect(t, tc.ID, tc.Expect, info, nil, nil, true)
			case "compress":
				input, exact := fixture(t, vf, vectorsPath, tc.Fixture)
				out, err := Compress(input, opts)
				if expectErrorCode(t, tc.ID, tc.Expect, err) {
					return
				}
				// Verify the output's own format/dimensions, not
				// just its size: decode the summary from the output.
				outInfo, infoErr := GetInfo(out)
				if infoErr != nil {
					t.Fatalf("%s: GetInfo(output): %v", tc.ID, infoErr)
				}
				checkExpect(t, tc.ID, tc.Expect, outInfo, nil, out, exact)
				// A compress case may also assert report fields
				// (quality_used, resized): check them via a report
				// call on the same input when the expect object asks.
				if _, ok := tc.Expect["quality_used"]; ok {
					rep, repErr := CompressWithReport(input, opts)
					if repErr != nil {
						t.Fatalf("%s: CompressWithReport: %v", tc.ID, repErr)
					}
					checkExpect(t, tc.ID, tc.Expect, nil, rep, out, exact)
				} else if _, ok := tc.Expect["resized"]; ok {
					rep, repErr := CompressWithReport(input, opts)
					if repErr != nil {
						t.Fatalf("%s: CompressWithReport: %v", tc.ID, repErr)
					}
					checkExpect(t, tc.ID, tc.Expect, nil, rep, out, exact)
				}
			case "compress_to_size":
				input, exact := fixture(t, vf, vectorsPath, tc.Fixture)
				target := uint64(0)
				if opts != nil && opts.TargetBytes != nil {
					target = *opts.TargetBytes
				}
				if target == 0 {
					t.Fatalf("%s: compress_to_size needs target_bytes in options", tc.ID)
				}
				out, err := CompressToSize(input, target, opts)
				if expectErrorCode(t, tc.ID, tc.Expect, err) {
					return
				}
				if uint64(len(out)) > target {
					t.Errorf("%s: output size %d exceeds target %d", tc.ID, len(out), target)
				}
				outInfo, infoErr := GetInfo(out)
				if infoErr != nil {
					t.Fatalf("%s: GetInfo(output): %v", tc.ID, infoErr)
				}
				checkExpect(t, tc.ID, tc.Expect, outInfo, nil, out, exact)
			case "compress_with_report":
				input, exact := fixture(t, vf, vectorsPath, tc.Fixture)
				rep, err := CompressWithReport(input, opts)
				if expectErrorCode(t, tc.ID, tc.Expect, err) {
					return
				}
				checkExpect(t, tc.ID, tc.Expect, nil, rep, rep.Output, exact)
			case "bulk":
				if len(tc.Fixtures) == 0 {
					t.Fatalf("%s: bulk needs fixtures[]", tc.ID)
				}
				inputs := make([][]byte, 0, len(tc.Fixtures))
				allExact := true
				for _, name := range tc.Fixtures {
					b, exact := fixture(t, vf, vectorsPath, name)
					inputs = append(inputs, b)
					allExact = allExact && exact
				}
				results, err := BulkCompress(inputs, opts)
				if err != nil {
					t.Fatalf("%s: BulkCompress: %v", tc.ID, err)
				}
				if len(results) != len(inputs) {
					t.Fatalf("%s: %d results for %d inputs", tc.ID, len(results), len(inputs))
				}
				var itemExpects []map[string]json.RawMessage
				if raw, ok := tc.Expect["results"]; ok {
					if err := json.Unmarshal(raw, &itemExpects); err != nil {
						t.Fatalf("%s: expect.results is not an array: %v", tc.ID, err)
					}
					if len(itemExpects) != len(results) {
						t.Fatalf("%s: %d item expects for %d results", tc.ID, len(itemExpects), len(results))
					}
					for i, itemExpect := range itemExpects {
						checkBulkItem(t, tc.ID, i, itemExpect, results[i], allExact)
					}
				}
			default:
				t.Fatalf("%s: unknown op %q", tc.ID, tc.Op)
			}
		})
	}
}
