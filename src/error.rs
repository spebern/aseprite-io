use std::fmt;
use std::io;

/// Errors that can occur when reading or writing Aseprite files.
#[derive(Debug)]
pub enum AsepriteError {
    /// An I/O error occurred during reading or writing.
    Io(io::Error),
    /// The file does not start with the Aseprite magic number `0xA5E0`.
    InvalidMagic,
    /// The file uses a color depth that is not 8, 16, or 32 bits.
    UnsupportedColorDepth(u16),
    /// A frame index is out of bounds.
    FrameOutOfBounds(usize),
    /// Pixel data buffer size does not match the expected size for the given dimensions and color mode.
    PixelSizeMismatch { expected: usize, actual: usize },
    /// A tag's frame range extends beyond the number of frames in the file.
    InvalidFrameRange,
    /// Indexed color mode requires a palette, but none was set.
    MissingPalette,
    /// A linked cel references a source frame that does not contain a cel on the same layer.
    LinkedCelNotFound { layer: usize, source_frame: usize },
    /// A chunk's declared size is invalid.
    InvalidChunkSize,
    /// A chunk or property type ID is not recognized.
    UnsupportedChunkType(u16),
    /// A value exceeds the format's limit (e.g., more than 256 palette entries).
    FormatLimitExceeded { field: &'static str, value: usize, max: usize },
}

impl fmt::Display for AsepriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::InvalidMagic => write!(f, "invalid magic number (expected 0xA5E0)"),
            Self::UnsupportedColorDepth(d) => write!(f, "unsupported color depth: {d}"),
            Self::FrameOutOfBounds(i) => write!(f, "frame index {i} out of bounds"),
            Self::PixelSizeMismatch { expected, actual } => {
                write!(f, "pixel data size mismatch: expected {expected}, got {actual}")
            }
            Self::InvalidFrameRange => write!(f, "invalid frame range"),
            Self::MissingPalette => write!(f, "indexed color mode requires a palette"),
            Self::LinkedCelNotFound { layer, source_frame } => {
                write!(f, "linked cel not found: layer {layer}, source frame {source_frame}")
            }
            Self::InvalidChunkSize => write!(f, "invalid chunk size"),
            Self::UnsupportedChunkType(t) => write!(f, "unsupported chunk type: 0x{t:04X}"),
            Self::FormatLimitExceeded { field, value, max } => {
                write!(f, "format limit exceeded for {field}: {value} > {max}")
            }
        }
    }
}

impl std::error::Error for AsepriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for AsepriteError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
