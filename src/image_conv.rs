use image::RgbaImage;

use crate::error::AsepriteError;
use crate::types::Pixels;

/// Converts an [`image::RgbaImage`] into [`Pixels`] (zero-copy, takes ownership of the buffer).
impl From<RgbaImage> for Pixels {
    fn from(img: RgbaImage) -> Self {
        let width = img.width() as u16;
        let height = img.height() as u16;
        Self {
            data: img.into_raw(),
            width,
            height,
        }
    }
}

/// Converts [`Pixels`] into an [`image::RgbaImage`].
///
/// Returns [`AsepriteError::PixelSizeMismatch`] if the buffer size is invalid.
impl TryFrom<Pixels> for RgbaImage {
    type Error = AsepriteError;

    fn try_from(pixels: Pixels) -> Result<Self, Self::Error> {
        RgbaImage::from_raw(pixels.width as u32, pixels.height as u32, pixels.data)
            .ok_or(AsepriteError::PixelSizeMismatch {
                expected: pixels.width as usize * pixels.height as usize * 4,
                actual: 0,
            })
    }
}
