//! Read and write Aseprite `.ase`/`.aseprite` files.
//!
//! This crate provides full support for the [Aseprite binary file format](https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md),
//! including reading, writing, and byte-perfect round-trip preservation. All color
//! modes (RGBA, Grayscale, Indexed), layer types (normal, group, tilemap), animation
//! tags, slices, user data with typed properties, and tilesets are supported.
//!
//! # Reading a file
//!
//! ```no_run
//! use aseprite::AsepriteFile;
//!
//! let data = std::fs::read("sprite.aseprite").unwrap();
//! let file = AsepriteFile::from_reader(&data[..]).unwrap();
//!
//! println!("{}x{}, {} frames", file.width(), file.height(), file.frames().len());
//! for layer in file.layers() {
//!     println!("  layer: {}", layer.name);
//! }
//! ```
//!
//! # Creating a file from scratch
//!
//! ```
//! use aseprite::*;
//!
//! let mut file = AsepriteFile::new(16, 16, ColorMode::Rgba);
//! let layer = file.add_layer("Background");
//! let frame = file.add_frame(100);
//! let pixels = Pixels::new(vec![0u8; 16 * 16 * 4], 16, 16, ColorMode::Rgba).unwrap();
//! file.set_cel(layer, frame, pixels, 0, 0).unwrap();
//!
//! let mut output = Vec::new();
//! file.write_to(&mut output).unwrap();
//! ```
//!
//! # Feature flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `image` | Enables conversions between [`Pixels`] and `image::RgbaImage` |
//! | `tiny-skia` | Enables conversions between [`Pixels`] and `tiny_skia::Pixmap` (handles premultiplied alpha) |

mod error;
#[cfg(feature = "image")]
mod image_conv;
#[cfg(feature = "tiny-skia")]
mod tiny_skia_conv;
mod reader;
mod types;
mod writer;

pub use error::AsepriteError;
pub use types::*;

impl AsepriteFile {
    /// Parses an Aseprite file from any reader.
    pub fn from_reader<R: std::io::Read>(r: R) -> Result<Self, AsepriteError> {
        reader::from_reader(r)
    }

    /// Writes the file in Aseprite binary format to any writer.
    pub fn write_to<W: std::io::Write>(&self, w: W) -> Result<(), AsepriteError> {
        writer::write_to(self, w)
    }
}
