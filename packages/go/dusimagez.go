package dusimagez

import (
	"encoding/base64"
	"strings"
)

// decodeBase64 decodes the base64 output of a bulk item. The FFI
// layer uses standard base64 with padding.
func decodeBase64(s string) ([]byte, error) {
	if s == "" {
		return []byte{}, nil
	}
	return base64.StdEncoding.DecodeString(s)
}

// FormatFromString parses a format name ("jpeg", "jpg", "png", ...,
// case-insensitive) into an ImageFormat, for callers that read the
// format from config or user input.
func FormatFromString(s string) (ImageFormat, error) {
	switch ImageFormat(strings.ToLower(strings.TrimSpace(s))) {
	case "":
		return "", invalidArgumentError("Format name is empty.", "Format naam khaali hai.")
	case FormatJPEG, "jpg":
		return FormatJPEG, nil
	case FormatPNG:
		return FormatPNG, nil
	case FormatWebP:
		return FormatWebP, nil
	case FormatAVIF:
		return FormatAVIF, nil
	case FormatGIF:
		return FormatGIF, nil
	case FormatBMP:
		return FormatBMP, nil
	case FormatTIFF:
		return FormatTIFF, nil
	default:
		return "", invalidArgumentError(
			"Unknown image format \""+s+"\". Use jpeg, png, webp, avif, gif, bmp or tiff.",
			"Galat image format hai.")
	}
}
