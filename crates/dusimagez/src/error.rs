//! Stable error codes (SPEC §4). Every error carries the same code plus
//! an English and a Hindi message, so every binding can surface identical
//! text.

use std::fmt;

/// Stable error code. The string form is the wire-stable identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    InvalidImage,
    UnsupportedFormat,
    ImageTooLarge,
    TargetSizeUnreachable,
    InvalidQuality,
    InvalidDimensions,
    InvalidArgument,
    EmptyInput,
}

impl ErrorCode {
    /// Wire-stable code string (SPEC §4).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidImage => "INVALID_IMAGE",
            Self::UnsupportedFormat => "UNSUPPORTED_FORMAT",
            Self::ImageTooLarge => "IMAGE_TOO_LARGE",
            Self::TargetSizeUnreachable => "TARGET_SIZE_UNREACHABLE",
            Self::InvalidQuality => "INVALID_QUALITY",
            Self::InvalidDimensions => "INVALID_DIMENSIONS",
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::EmptyInput => "EMPTY_INPUT",
        }
    }
}

/// Error returned by every public function in this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DusimagezError {
    code: ErrorCode,
    message_en: String,
    message_hi: String,
}

impl DusimagezError {
    fn new(code: ErrorCode, message_en: String, message_hi: String) -> Self {
        Self {
            code,
            message_en,
            message_hi,
        }
    }

    /// Stable code string, e.g. `"INVALID_IMAGE"`.
    #[must_use]
    pub fn code(&self) -> &'static str {
        self.code.as_str()
    }

    /// The [`ErrorCode`] enum value.
    #[must_use]
    pub const fn error_code(&self) -> ErrorCode {
        self.code
    }

    /// English message.
    #[must_use]
    pub fn message_en(&self) -> &str {
        &self.message_en
    }

    /// Hindi (Devanagari) message.
    #[must_use]
    pub fn message_hi(&self) -> &str {
        &self.message_hi
    }

    pub(crate) fn empty_input(what: &str) -> Self {
        Self::new(
            ErrorCode::EmptyInput,
            format!("No input: {what} is empty."),
            format!("कोई इनपुट नहीं है: {what} खाली है।"),
        )
    }

    pub(crate) fn invalid_image(detail: &str) -> Self {
        Self::new(
            ErrorCode::InvalidImage,
            format!("The input is not a valid image: {detail}."),
            format!("इनपुट एक मान्य इमेज नहीं है: {detail}।"),
        )
    }

    pub(crate) fn unsupported_format(format_name: &str) -> Self {
        Self::new(
            ErrorCode::UnsupportedFormat,
            format!(
                "Format '{format_name}' is not supported in v0.1.0. \
                 Supported inputs: JPEG, PNG, WebP, GIF, BMP, TIFF. \
                 AVIF input needs a dav1d decoder, which is not bundled."
            ),
            format!(
                "'{format_name}' फ़ॉर्मैट v0.1.0 में समर्थित नहीं है। \
                 समर्थित इनपुट: JPEG, PNG, WebP, GIF, BMP, TIFF। \
                 AVIF इनपुट के लिए dav1d डिकोडर चाहिए, जो शामिल नहीं है।"
            ),
        )
    }

    pub(crate) fn image_too_large(
        width: u32,
        height: u32,
        max_pixels: u64,
        max_dimension: u32,
    ) -> Self {
        Self::new(
            ErrorCode::ImageTooLarge,
            format!(
                "Image is {width}x{height}, which exceeds the limits \
                 (max {max_pixels} pixels, max {max_dimension}px per side)."
            ),
            format!(
                "इमेज {width}x{height} है, जो सीमा से बड़ी है \
                 (अधिकतम {max_pixels} पिक्सेल, प्रति तरफ़ अधिकतम {max_dimension}px)।"
            ),
        )
    }

    pub(crate) fn target_size_unreachable(target_bytes: u64) -> Self {
        Self::new(
            ErrorCode::TargetSizeUnreachable,
            format!(
                "Cannot reach {target_bytes} bytes: even the lowest quality \
                 at 10% of the original dimensions is larger."
            ),
            format!(
                "{target_bytes} बाइट तक नहीं पहुँचा जा सकता: सबसे कम क्वालिटी पर \
                 मूल आकार के 10% पर भी फ़ाइल बड़ी है।"
            ),
        )
    }

    pub(crate) fn invalid_quality(quality: u8) -> Self {
        Self::new(
            ErrorCode::InvalidQuality,
            format!("Quality must be 1-100, got {quality}."),
            format!("क्वालिटी 1-100 के बीच होनी चाहिए, मिली {quality}।"),
        )
    }

    pub(crate) fn invalid_dimensions(detail: &str) -> Self {
        Self::new(
            ErrorCode::InvalidDimensions,
            format!("Invalid dimensions: {detail}."),
            format!("अमान्य आयाम: {detail}।"),
        )
    }

    pub(crate) fn invalid_argument(detail: &str) -> Self {
        Self::new(
            ErrorCode::InvalidArgument,
            format!("Invalid argument: {detail}."),
            format!("अमान्य आर्ग्युमेंट: {detail}।"),
        )
    }
}

impl fmt::Display for DusimagezError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message_en)
    }
}

impl std::error::Error for DusimagezError {}

/// Convenience alias used across the crate.
pub(crate) type Result<T> = std::result::Result<T, DusimagezError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_en_and_hi_messages() {
        let errs = [
            DusimagezError::empty_input("image bytes"),
            DusimagezError::invalid_image("bad header"),
            DusimagezError::unsupported_format("avif"),
            DusimagezError::image_too_large(20_000, 20_000, 120_000_000, 16_000),
            DusimagezError::target_size_unreachable(100),
            DusimagezError::invalid_quality(0),
            DusimagezError::invalid_dimensions("width is 0"),
            DusimagezError::invalid_argument("effort is 99"),
        ];
        let codes = [
            "EMPTY_INPUT",
            "INVALID_IMAGE",
            "UNSUPPORTED_FORMAT",
            "IMAGE_TOO_LARGE",
            "TARGET_SIZE_UNREACHABLE",
            "INVALID_QUALITY",
            "INVALID_DIMENSIONS",
            "INVALID_ARGUMENT",
        ];
        for (err, code) in errs.iter().zip(codes.iter()) {
            assert_eq!(err.code(), *code);
            assert!(!err.message_en().is_empty(), "{code} has no EN message");
            assert!(!err.message_hi().is_empty(), "{code} has no HI message");
            // Hindi messages are Devanagari: at least one char in U+0900..U+097F.
            assert!(
                err.message_hi()
                    .chars()
                    .any(|c| ('\u{0900}'..='\u{097F}').contains(&c)),
                "{code} HI message has no Devanagari"
            );
        }
    }
}
