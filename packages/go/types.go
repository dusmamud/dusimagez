package dusimagez

import "strings"

// Types — the pro control layer (SPEC section 2), mirroring
// crates/dusimagez/src/types.rs field for field. Start from
// DefaultCompressOptions() and change what you need; a nil
// *CompressOptions means "all defaults". Option fields that are
// pointers (Format, Preset, TargetBytes, MaxWidth, MaxHeight, DPI)
// are nil when unset; plain scalars always carry a value (their
// defaults come from DefaultCompressOptions).

// ImageFormat is an image format. The empty value "" means "keep the
// input format" (only meaningful in CompressOptions.Format).
type ImageFormat string

const (
	FormatJPEG ImageFormat = "jpeg"
	FormatPNG  ImageFormat = "png"
	FormatWebP ImageFormat = "webp"
	FormatAVIF ImageFormat = "avif"
	FormatGIF  ImageFormat = "gif"
	FormatBMP  ImageFormat = "bmp"
	FormatTIFF ImageFormat = "tiff"
)

// String returns the lowercase format name ("" for keep).
func (f ImageFormat) String() string { return string(f) }

// Subsampling is JPEG chroma subsampling. The empty value "" means
// "core default" (S420 for quality < 90, S444 otherwise).
type Subsampling string

const (
	Subsampling444 Subsampling = "444"
	Subsampling422 Subsampling = "422"
	Subsampling420 Subsampling = "420"
)

// String returns the subsampling digits ("" for default).
func (s Subsampling) String() string { return string(s) }

// Preset is a named bundle of quality/effort/subsampling defaults.
// Explicit option fields override the preset.
type Preset string

const (
	PresetWeb     Preset = "web"
	PresetSocial  Preset = "social"
	PresetPrint   Preset = "print"
	PresetArchive Preset = "archive"
)

// String returns the lowercase preset name.
func (p Preset) String() string { return string(p) }

// StripMetadata selects which metadata survives compression. The empty
// value "" means All (strip everything).
type StripMetadata string

const (
	StripAll      StripMetadata = "all"
	StripKeepExif StripMetadata = "keep-exif"
	StripKeepIcc  StripMetadata = "keep-icc"
	StripKeepAll  StripMetadata = "keep-all"
)

// String returns the strip-metadata name ("" for All).
func (s StripMetadata) String() string { return string(s) }

// CompressOptions are the options for Compress / CompressToSize /
// CompressWithReport / BulkCompress.
type CompressOptions struct {
	// Quality 1-100 (default 80). JPEG/WebP/AVIF use it directly;
	// PNG maps it to a compression level.
	Quality uint8
	// Output format; nil keeps the input format.
	Format *ImageFormat
	// Progressive JPEG / interlaced PNG (default false).
	Progressive bool
	// Lossless output (WebP/PNG only; default false).
	Lossless bool
	// Encoding effort 0-10 (default 6; WebP 0-6, AVIF speed 0-10).
	Effort uint8
	// Chroma subsampling; "" means the core default (S420 for
	// quality < 90, S444 otherwise).
	Subsampling Subsampling
	// PNG compression level 0-9 (default 6).
	PNGCompressionLevel uint8
	// Quantise PNG output to a palette (default false).
	Palette bool
	// Preset (Web / Social / Print / Archive); nil means none.
	// Explicit fields override the preset.
	Preset *Preset
	// Target output size in bytes; nil means unset. When set,
	// Compress behaves like CompressToSize (SPEC section 2).
	TargetBytes *uint64
	// Downscale-only max width in px; nil means unset (never upscale).
	MaxWidth *uint32
	// Downscale-only max height in px; nil means unset (never upscale).
	MaxHeight *uint32
	// Which metadata to keep; "" means All (strip everything).
	StripMetadata StripMetadata
	// Apply EXIF orientation on load (default true).
	AutoOrient bool
	// DPI to write on JPEG/PNG output, where the encoder supports
	// it ([x, y]); nil means unset.
	DPI *[2]uint32
	// Refuse images with more pixels than this (default 120,000,000).
	MaxPixels uint64
	// Refuse images with a side longer than this (default 16000).
	MaxDimension uint32
}

// DefaultCompressOptions returns the Rust CompressOptions defaults.
func DefaultCompressOptions() CompressOptions {
	return CompressOptions{
		Quality:             80,
		Format:              nil,
		Progressive:         false,
		Lossless:            false,
		Effort:              6,
		Subsampling:         "",
		PNGCompressionLevel: 6,
		Palette:             false,
		Preset:              nil,
		TargetBytes:         nil,
		MaxWidth:            nil,
		MaxHeight:           nil,
		StripMetadata:       "",
		AutoOrient:          true,
		DPI:                 nil,
		MaxPixels:           120_000_000,
		MaxDimension:        16000,
	}
}

// resolveOptions returns the effective options: defaults when opts is
// nil, otherwise opts as given, validated.
func resolveOptions(opts *CompressOptions) (CompressOptions, error) {
	if opts == nil {
		return DefaultCompressOptions(), nil
	}
	if err := opts.validate(); err != nil {
		return CompressOptions{}, err
	}
	return *opts, nil
}

func (o CompressOptions) validate() error {
	if o.Quality < 1 || o.Quality > 100 {
		return invalidQualityError(int(o.Quality))
	}
	if o.Effort > 10 {
		return invalidArgumentError(
			"Effort is invalid. Effort must be between 0 and 10.",
			"Effort galat hai. Effort 0 se 10 ke beech hona chahiye.")
	}
	if o.PNGCompressionLevel > 9 {
		return invalidArgumentError(
			"PNG compression level is invalid. It must be between 0 and 9.",
			"PNG compression level galat hai. Ye 0 se 9 ke beech hona chahiye.")
	}
	if o.TargetBytes != nil && *o.TargetBytes == 0 {
		return invalidArgumentError(
			"target_bytes must not be zero.",
			"target_bytes zero nahi ho sakta.")
	}
	if o.MaxWidth != nil && *o.MaxWidth == 0 {
		return invalidDimensionsError("max_width must not be zero.", "max_width zero nahi ho sakta.")
	}
	if o.MaxHeight != nil && *o.MaxHeight == 0 {
		return invalidDimensionsError("max_height must not be zero.", "max_height zero nahi ho sakta.")
	}
	if o.MaxPixels == 0 {
		return invalidArgumentError(
			"max_pixels must not be zero.",
			"max_pixels zero nahi ho sakta.")
	}
	if o.MaxDimension == 0 {
		return invalidArgumentError(
			"max_dimension must not be zero.",
			"max_dimension zero nahi ho sakta.")
	}
	if o.DPI != nil && (o.DPI[0] == 0 || o.DPI[1] == 0) {
		return invalidArgumentError(
			"dpi needs both axes greater than zero.",
			"dpi ke liye dono axes zero se bade hone chahiye.")
	}
	return nil
}

// CompressReport is the before/after report from CompressWithReport
// (SPEC section 1).
type CompressReport struct {
	// Compressed image bytes.
	Output []byte
	// Input size in bytes.
	OriginalSize uint64
	// Output size in bytes.
	OutputSize uint64
	// OriginalSize - OutputSize.
	SavedBytes uint64
	// SavedBytes / OriginalSize * 100.
	SavedPercent float32
	// Output format.
	Format ImageFormat
	// Output dimensions in px.
	Width  uint32
	Height uint32
	// Quality the encoder actually used.
	QualityUsed uint8
	// Whether the image was resized (downscaled) to hit the target.
	Resized bool
}

// ImageInfo is the summary from GetInfo (SPEC section 1).
type ImageInfo struct {
	// Detected format.
	Format ImageFormat
	// Dimensions in px.
	Width  uint32
	Height uint32
	// Whether the image has an alpha channel.
	HasAlpha bool
	// Lowercase image-crate colour type ("rgb8", "rgba8", "la8",
	// "l8", ...).
	ColourType string
	// Input size in bytes.
	FileSize uint64
}

// BulkResult is one item of BulkCompress: either Report (with Output)
// or Err — never both. One bad input fails only its own item.
type BulkResult struct {
	Report *CompressReport
	Err    error
}

// --- enum code mapping (C codes in crates/dusimagez-ffi/dusimagez.h) ---

func formatCode(f ImageFormat) (int32, error) {
	switch ImageFormat(strings.ToLower(string(f))) {
	case "":
		return 0, nil
	case FormatJPEG, "jpg":
		return 1, nil
	case FormatPNG:
		return 2, nil
	case FormatWebP:
		return 3, nil
	case FormatAVIF:
		return 4, nil
	case FormatGIF:
		return 5, nil
	case FormatBMP:
		return 6, nil
	case FormatTIFF:
		return 7, nil
	default:
		return 0, invalidArgumentError(
			"Unknown image format \""+string(f)+"\". Use jpeg, png, webp, avif, gif, bmp or tiff.",
			"Galat image format hai.")
	}
}

func formatFromCode(code int32) (ImageFormat, error) {
	switch code {
	case 0:
		return "", nil
	case 1:
		return FormatJPEG, nil
	case 2:
		return FormatPNG, nil
	case 3:
		return FormatWebP, nil
	case 4:
		return FormatAVIF, nil
	case 5:
		return FormatGIF, nil
	case 6:
		return FormatBMP, nil
	case 7:
		return FormatTIFF, nil
	default:
		return "", invalidArgumentError(
			"Unknown image format code.",
			"Galat image format code hai.")
	}
}

func subsamplingCode(s Subsampling) (int32, error) {
	switch strings.ToLower(strings.TrimPrefix(strings.ToLower(string(s)), "s")) {
	case "":
		return 0, nil
	case "444":
		return 1, nil
	case "422":
		return 2, nil
	case "420":
		return 3, nil
	default:
		return 0, invalidArgumentError(
			"Unknown subsampling \""+string(s)+"\". Use 444, 422 or 420.",
			"Galat subsampling hai.")
	}
}

func presetCode(p Preset) (int32, error) {
	switch Preset(strings.ToLower(string(p))) {
	case "":
		return 0, nil
	case PresetWeb:
		return 1, nil
	case PresetSocial:
		return 2, nil
	case PresetPrint:
		return 3, nil
	case PresetArchive:
		return 4, nil
	default:
		return 0, invalidArgumentError(
			"Unknown preset \""+string(p)+"\". Use web, social, print or archive.",
			"Galat preset hai.")
	}
}

func stripCode(s StripMetadata) (int32, error) {
	switch StripMetadata(strings.ToLower(strings.ReplaceAll(string(s), "_", "-"))) {
	case "", StripAll:
		return 0, nil
	case StripKeepExif:
		return 1, nil
	case StripKeepIcc:
		return 2, nil
	case StripKeepAll:
		return 3, nil
	default:
		return 0, invalidArgumentError(
			"Unknown strip_metadata \""+string(s)+"\". Use all, keep-exif, keep-icc or keep-all.",
			"Galat strip_metadata hai.")
	}
}
