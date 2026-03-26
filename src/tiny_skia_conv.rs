use tiny_skia::Pixmap;

use crate::error::AsepriteError;
use crate::types::Pixels;

/// Convert premultiplied-alpha RGBA to straight-alpha RGBA (in place).
fn premultiplied_to_straight(data: &mut [u8]) {
    for chunk in data.chunks_exact_mut(4) {
        let a = chunk[3] as u16;
        if a == 0 {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
        } else if a < 255 {
            chunk[0] = ((chunk[0] as u16 * 255 + a / 2) / a).min(255) as u8;
            chunk[1] = ((chunk[1] as u16 * 255 + a / 2) / a).min(255) as u8;
            chunk[2] = ((chunk[2] as u16 * 255 + a / 2) / a).min(255) as u8;
        }
    }
}

/// Convert straight-alpha RGBA to premultiplied-alpha RGBA (in place).
fn straight_to_premultiplied(data: &mut [u8]) {
    for chunk in data.chunks_exact_mut(4) {
        let a = chunk[3] as u16;
        if a == 0 {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
        } else if a < 255 {
            chunk[0] = ((chunk[0] as u16 * a + 127) / 255) as u8;
            chunk[1] = ((chunk[1] as u16 * a + 127) / 255) as u8;
            chunk[2] = ((chunk[2] as u16 * a + 127) / 255) as u8;
        }
    }
}

/// Converts a [`tiny_skia::Pixmap`] (premultiplied alpha) into [`Pixels`] (straight alpha).
impl From<Pixmap> for Pixels {
    fn from(pixmap: Pixmap) -> Self {
        let width = pixmap.width() as u16;
        let height = pixmap.height() as u16;
        let mut data = pixmap.take();
        premultiplied_to_straight(&mut data);
        Self { data, width, height }
    }
}

/// Converts a [`tiny_skia::Pixmap`] reference into [`Pixels`] (straight alpha), copying the data.
impl From<&Pixmap> for Pixels {
    fn from(pixmap: &Pixmap) -> Self {
        let width = pixmap.width() as u16;
        let height = pixmap.height() as u16;
        let mut data = pixmap.data().to_vec();
        premultiplied_to_straight(&mut data);
        Self { data, width, height }
    }
}

/// Converts [`Pixels`] (straight alpha) into a [`tiny_skia::Pixmap`] (premultiplied alpha).
///
/// Returns [`AsepriteError::PixelSizeMismatch`] if the dimensions are invalid.
impl TryFrom<Pixels> for Pixmap {
    type Error = AsepriteError;

    fn try_from(pixels: Pixels) -> Result<Self, Self::Error> {
        let mut data = pixels.data;
        straight_to_premultiplied(&mut data);
        Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(pixels.width as u32, pixels.height as u32)
            .ok_or(AsepriteError::PixelSizeMismatch {
                expected: pixels.width as usize * pixels.height as usize * 4,
                actual: 0,
            })?)
        .ok_or(AsepriteError::PixelSizeMismatch {
            expected: pixels.width as usize * pixels.height as usize * 4,
            actual: 0,
        })
    }
}
