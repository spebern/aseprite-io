use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use crate::error::AsepriteError;

// --- Enums ---

/// The color depth mode of an Aseprite file.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Rgba,
    Grayscale,
    Indexed,
}

impl ColorMode {
    /// Returns the number of bytes per pixel for this mode.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba => 4,
            Self::Grayscale => 2,
            Self::Indexed => 1,
        }
    }

    pub(crate) fn from_depth(depth: u16) -> Result<Self, AsepriteError> {
        match depth {
            32 => Ok(Self::Rgba),
            16 => Ok(Self::Grayscale),
            8 => Ok(Self::Indexed),
            d => Err(AsepriteError::UnsupportedColorDepth(d)),
        }
    }

    pub(crate) fn to_depth(self) -> u16 {
        match self {
            Self::Rgba => 32,
            Self::Grayscale => 16,
            Self::Indexed => 8,
        }
    }
}

/// The type of a layer.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LayerKind {
    Normal,
    Group,
    /// A tilemap layer referencing a tileset by index.
    Tilemap { tileset_index: u32 },
}

/// Layer blend mode, matching Aseprite's blend mode list.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    Addition,
    Subtract,
    Divide,
}

impl BlendMode {
    pub(crate) fn from_u16(v: u16) -> Self {
        match v {
            0 => Self::Normal, 1 => Self::Multiply, 2 => Self::Screen,
            3 => Self::Overlay, 4 => Self::Darken, 5 => Self::Lighten,
            6 => Self::ColorDodge, 7 => Self::ColorBurn, 8 => Self::HardLight,
            9 => Self::SoftLight, 10 => Self::Difference, 11 => Self::Exclusion,
            12 => Self::Hue, 13 => Self::Saturation, 14 => Self::Color,
            15 => Self::Luminosity, 16 => Self::Addition, 17 => Self::Subtract,
            18 => Self::Divide, _ => Self::Normal,
        }
    }

    pub(crate) fn to_u16(self) -> u16 {
        match self {
            Self::Normal => 0, Self::Multiply => 1, Self::Screen => 2,
            Self::Overlay => 3, Self::Darken => 4, Self::Lighten => 5,
            Self::ColorDodge => 6, Self::ColorBurn => 7, Self::HardLight => 8,
            Self::SoftLight => 9, Self::Difference => 10, Self::Exclusion => 11,
            Self::Hue => 12, Self::Saturation => 13, Self::Color => 14,
            Self::Luminosity => 15, Self::Addition => 16, Self::Subtract => 17,
            Self::Divide => 18,
        }
    }
}

/// Animation loop direction for tags.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoopDirection {
    Forward,
    Reverse,
    PingPong,
    PingPongReverse,
}

impl LoopDirection {
    pub(crate) fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Forward, 1 => Self::Reverse,
            2 => Self::PingPong, 3 => Self::PingPongReverse,
            _ => Self::Forward,
        }
    }

    pub(crate) fn to_u8(self) -> u8 {
        match self {
            Self::Forward => 0, Self::Reverse => 1,
            Self::PingPong => 2, Self::PingPongReverse => 3,
        }
    }
}

/// Color profile embedded in the file.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ColorProfile {
    None,
    SRgb { flags: u16, gamma: u32 },
    Icc { flags: u16, gamma: u32, data: Vec<u8> },
}

// --- Handles ---

/// Handle to a non-group layer. Obtained from [`AsepriteFile::add_layer`] or [`AsepriteFile::layer_ref`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LayerRef(pub(crate) usize);

/// Handle to a group layer. Obtained from [`AsepriteFile::add_group`] or [`AsepriteFile::group_ref`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GroupRef(pub(crate) usize);

impl LayerRef {
    /// Returns the underlying layer index.
    pub fn index(&self) -> usize { self.0 }
}

impl GroupRef {
    /// Returns the underlying layer index.
    pub fn index(&self) -> usize { self.0 }
}

// --- Data structs ---

/// Raw pixel data buffer. Format depends on the file's [`ColorMode`]: RGBA = 4 bytes/pixel, Grayscale = 2 bytes/pixel, Indexed = 1 byte/pixel.
#[derive(Clone, Debug, PartialEq)]
pub struct Pixels {
    pub data: Vec<u8>,
    pub width: u16,
    pub height: u16,
}

impl Pixels {
    /// Creates a new pixel buffer, validating that `data.len()` matches `width * height * color_mode.bytes_per_pixel()`.
    pub fn new(data: Vec<u8>, width: u16, height: u16, color_mode: ColorMode) -> Result<Self, AsepriteError> {
        let expected = width as usize * height as usize * color_mode.bytes_per_pixel();
        if data.len() != expected {
            return Err(AsepriteError::PixelSizeMismatch { expected, actual: data.len() });
        }
        Ok(Self { data, width, height })
    }
}

/// An RGBA color with an optional name (used in palettes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
    pub name: Option<String>,
}

/// Grid overlay settings.
#[derive(Clone, Debug, PartialEq)]
pub struct GridInfo {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Default for GridInfo {
    fn default() -> Self {
        Self { x: 0, y: 0, width: 16, height: 16 }
    }
}

#[derive(Clone)]
pub(crate) struct UnknownChunk {
    pub frame_index: usize,
    pub chunk_type: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub(crate) struct ChunkOrderEntry {
    pub frame_index: usize,
    pub chunk_type: u16,
    /// For cel chunks (0x2005), stores the layer index to identify which cel
    pub layer_index: Option<usize>,
}

// --- Layer ---

/// A layer in the file (normal, group, or tilemap).
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: String,
    pub kind: LayerKind,
    pub parent: Option<usize>,
    pub opacity: u8,
    pub blend_mode: BlendMode,
    pub visible: bool,
    pub editable: bool,
    pub lock_movement: bool,
    pub background: bool,
    pub prefer_linked_cels: bool,
    pub collapsed: bool,
    pub reference_layer: bool,
    pub user_data: Option<UserData>,
}

/// Options for creating a new layer.
pub struct LayerOptions {
    pub opacity: u8,
    pub blend_mode: BlendMode,
    pub visible: bool,
    pub editable: bool,
    pub lock_movement: bool,
    pub background: bool,
    pub collapsed: bool,
    pub prefer_linked_cels: bool,
    pub reference_layer: bool,
}

impl Default for LayerOptions {
    fn default() -> Self {
        Self {
            opacity: 255, blend_mode: BlendMode::Normal,
            visible: true, editable: true, lock_movement: false,
            background: false, collapsed: false,
            prefer_linked_cels: false, reference_layer: false,
        }
    }
}

// --- Frame ---

/// A single animation frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub duration_ms: u16,
}

// --- Tag ---

/// An animation tag spanning a range of frames.
#[derive(Clone, Debug, PartialEq)]
pub struct Tag {
    pub name: String,
    pub from_frame: usize,
    pub to_frame: usize,
    pub direction: LoopDirection,
    pub repeat: u16,
    pub user_data: Option<UserData>,
}

// --- Cel ---

/// A cel (the content of one layer in one frame).
#[derive(Clone, Debug, PartialEq)]
pub struct Cel {
    pub kind: CelKind,
    pub opacity: u8,
    pub z_index: i16,
    pub user_data: Option<UserData>,
    pub extra: Option<CelExtra>,
}

/// The type and data of a cel.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum CelKind {
    /// Uncompressed pixel data.
    Raw { pixels: Pixels, x: i16, y: i16 },
    /// Zlib-compressed pixel data. `original_compressed` preserves the original bytes for round-trip fidelity; `None` for programmatically created cels.
    Compressed { pixels: Pixels, x: i16, y: i16, original_compressed: Option<Vec<u8>> },
    /// References another frame's cel on the same layer. Use [`AsepriteFile::resolve_cel`] to follow the link.
    Linked { source_frame: usize, x: i16, y: i16 },
    /// Tilemap data referencing tiles by ID.
    Tilemap {
        width: u16,
        height: u16,
        bits_per_tile: u16,
        tile_id_bitmask: u32,
        x_flip_bitmask: u32,
        y_flip_bitmask: u32,
        d_flip_bitmask: u32,
        tiles: Vec<u32>,
        x: i16,
        y: i16,
        original_compressed: Option<Vec<u8>>,
    },
}

/// Options for creating a cel with non-default opacity, position, or z-index.
pub struct CelOptions {
    pub pixels: Pixels,
    pub x: i16,
    pub y: i16,
    pub opacity: u8,
    pub z_index: i16,
}

impl Default for CelOptions {
    fn default() -> Self {
        Self {
            pixels: Pixels { data: vec![], width: 0, height: 0 },
            x: 0, y: 0, opacity: 255, z_index: 0,
        }
    }
}

// --- User Data ---

/// User-defined metadata attached to layers, cels, tags, slices, or the sprite itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserData {
    pub text: Option<String>,
    pub color: Option<Color>,
    pub properties: Vec<PropertiesMap>,
}

/// A named map of typed properties within user data.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertiesMap {
    pub key: u32,
    pub entries: Vec<(String, PropertyValue)>,
}

/// A typed value within a properties map.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    Bool(bool), Int8(i8), UInt8(u8), Int16(i16), UInt16(u16),
    Int32(i32), UInt32(u32), Int64(i64), UInt64(u64),
    Fixed(u32), Float(f32), Double(f64), String(String),
    Point(i32, i32), Size(i32, i32), Rect(i32, i32, i32, i32),
    Vector(Vec<PropertyValue>), Properties(Vec<(String, PropertyValue)>),
    Uuid([u8; 16]),
}

// --- Slice ---

/// A named rectangular region, optionally with nine-patch and pivot data.
#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
    pub name: String,
    pub keys: Vec<SliceKey>,
    pub has_nine_patch: bool,
    pub has_pivot: bool,
    pub user_data: Option<UserData>,
}

/// A slice's bounds at a specific frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceKey {
    pub frame: u32, pub x: i32, pub y: i32,
    pub width: u32, pub height: u32,
    pub nine_patch: Option<NinePatch>,
    pub pivot: Option<(i32, i32)>,
}

/// Nine-patch (9-slice) center region within a slice.
#[derive(Clone, Debug, PartialEq)]
pub struct NinePatch {
    pub center_x: i32, pub center_y: i32,
    pub center_width: u32, pub center_height: u32,
}

// --- Cel Extra ---

/// Extra precision bounds for a cel.
#[derive(Clone, Debug, PartialEq)]
pub struct CelExtra {
    pub precise_x: u32, pub precise_y: u32,
    pub width: u32, pub height: u32,
}

// --- Tileset ---

/// Bitflags for tileset properties.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TilesetFlags(pub u32);

impl TilesetFlags {
    /// Returns whether the tileset references an external file.
    pub fn has_external_link(self) -> bool { self.0 & 1 != 0 }
    /// Returns whether the tileset has embedded tile pixel data.
    pub fn has_embedded_tiles(self) -> bool { self.0 & 2 != 0 }
    /// Returns whether tile ID 0 represents an empty tile.
    pub fn empty_tile_is_zero(self) -> bool { self.0 & 4 != 0 }
}

/// A tileset definition.
#[derive(Clone, Debug, PartialEq)]
pub struct Tileset {
    pub id: u32,
    pub flags: TilesetFlags,
    pub name: String,
    pub tile_count: u32,
    pub tile_width: u16,
    pub tile_height: u16,
    pub base_index: i16,
    pub data: TilesetData,
    pub user_data: Option<UserData>,
    pub tile_user_data: Vec<Option<UserData>>,
}

/// The pixel data source for a tileset.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum TilesetData {
    Embedded { pixels: Vec<u8>, original_compressed: Option<Vec<u8>> },
    External { external_file_id: u32, tileset_id_in_external: u32 },
    Empty,
}

// --- External File ---

/// A reference to an external file.
#[derive(Clone, Debug, PartialEq)]
pub struct ExternalFile {
    pub id: u32,
    pub file_type: ExternalFileType,
    pub name: String,
}

/// The type of an external file reference.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExternalFileType {
    Palette,
    Tileset,
    ExtensionProps,
    ExtensionTileMgmt,
}

impl ExternalFileType {
    pub(crate) fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Palette,
            1 => Self::Tileset,
            2 => Self::ExtensionProps,
            3 => Self::ExtensionTileMgmt,
            _ => Self::Palette,
        }
    }

    pub(crate) fn to_u8(self) -> u8 {
        match self {
            Self::Palette => 0,
            Self::Tileset => 1,
            Self::ExtensionProps => 2,
            Self::ExtensionTileMgmt => 3,
        }
    }
}

// --- Legacy types (read-only) ---

/// A legacy mask chunk (deprecated in modern Aseprite, preserved for round-trip fidelity).
#[derive(Clone, Debug, PartialEq)]
pub struct LegacyMask {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub name: String,
    pub bitmap: Vec<u8>,
}

// --- The main file struct ---

/// An Aseprite `.ase`/`.aseprite` file.
///
/// # Creating a file from scratch
///
/// ```
/// use aseprite::*;
///
/// let mut file = AsepriteFile::new(32, 32, ColorMode::Rgba);
/// let layer = file.add_layer("Background");
/// let frame = file.add_frame(100);
/// let pixels = Pixels::new(vec![0u8; 32 * 32 * 4], 32, 32, ColorMode::Rgba).unwrap();
/// file.set_cel(layer, frame, pixels, 0, 0).unwrap();
/// ```
///
/// # Reading and writing
///
/// ```no_run
/// use aseprite::AsepriteFile;
///
/// let data = std::fs::read("sprite.aseprite").unwrap();
/// let file = AsepriteFile::from_reader(&data[..]).unwrap();
/// let mut output = Vec::new();
/// file.write_to(&mut output).unwrap();
/// ```
#[derive(Clone)]
pub struct AsepriteFile {
    width: u16,
    height: u16,
    color_mode: ColorMode,
    flags: u32,
    deprecated_speed: u16,
    num_colors: u16,
    transparent_index: u8,
    pixel_ratio: (u8, u8),
    grid: GridInfo,
    color_profile: Option<ColorProfile>,
    palette: Vec<Color>,
    layers: Vec<Layer>,
    frames: Vec<Frame>,
    tags: Vec<Tag>,
    slices: Vec<Slice>,
    sprite_user_data: Option<UserData>,
    cels: BTreeMap<(usize, usize), Cel>,
    tilesets: Vec<Tileset>,
    external_files: Vec<ExternalFile>,
    legacy_masks: Vec<LegacyMask>,
    pub(crate) unknown_chunks: Vec<UnknownChunk>,
    pub(crate) chunk_order: Vec<ChunkOrderEntry>,
}

impl std::fmt::Debug for AsepriteFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsepriteFile")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("color_mode", &self.color_mode)
            .field("layers", &self.layers.len())
            .field("frames", &self.frames.len())
            .field("tags", &self.tags.len())
            .field("cels", &self.cels.len())
            .finish()
    }
}

impl AsepriteFile {
    /// Creates an empty file with the given canvas dimensions and color mode.
    pub fn new(width: u16, height: u16, color_mode: ColorMode) -> Self {
        Self {
            width, height, color_mode,
            flags: 1, deprecated_speed: 0, num_colors: 0,
            transparent_index: 0,
            pixel_ratio: (1, 1), grid: GridInfo::default(),
            color_profile: None, palette: Vec::new(),
            layers: Vec::new(), frames: Vec::new(),
            tags: Vec::new(), slices: Vec::new(),
            sprite_user_data: None,
            cels: BTreeMap::new(),
            tilesets: Vec::new(),
            external_files: Vec::new(),
            legacy_masks: Vec::new(),
            unknown_chunks: Vec::new(),
            chunk_order: Vec::new(),
        }
    }

    // --- Accessors ---
    /// Returns the canvas width in pixels.
    pub fn width(&self) -> u16 { self.width }
    /// Returns the canvas height in pixels.
    pub fn height(&self) -> u16 { self.height }
    /// Returns the color mode.
    pub fn color_mode(&self) -> ColorMode { self.color_mode }
    /// Returns the raw header flags.
    pub fn flags(&self) -> u32 { self.flags }
    pub(crate) fn deprecated_speed(&self) -> u16 { self.deprecated_speed }
    pub(crate) fn num_colors(&self) -> u16 { self.num_colors }
    /// Returns the pixel aspect ratio as (width, height).
    pub fn pixel_ratio(&self) -> (u8, u8) { self.pixel_ratio }
    /// Returns the grid overlay settings.
    pub fn grid(&self) -> &GridInfo { &self.grid }
    /// Returns the color profile, if set.
    pub fn color_profile(&self) -> &Option<ColorProfile> { &self.color_profile }
    /// Returns the color palette (empty if no palette chunk was present).
    pub fn palette(&self) -> &[Color] { &self.palette }
    /// Returns all layers in document order.
    pub fn layers(&self) -> &[Layer] { &self.layers }
    /// Returns all frames.
    pub fn frames(&self) -> &[Frame] { &self.frames }
    /// Returns all animation tags.
    pub fn tags(&self) -> &[Tag] { &self.tags }
    /// Returns all slices.
    pub fn slices(&self) -> &[Slice] { &self.slices }
    /// Returns the sprite-level user data, if set.
    pub fn sprite_user_data(&self) -> &Option<UserData> { &self.sprite_user_data }
    /// Returns the transparent palette index (only meaningful for indexed color mode).
    pub fn transparent_index(&self) -> u8 { self.transparent_index }
    /// Returns legacy mask chunks (deprecated in modern Aseprite, preserved for round-trip fidelity).
    pub fn legacy_masks(&self) -> &[LegacyMask] { &self.legacy_masks }
    /// Returns all tilesets.
    pub fn tilesets(&self) -> &[Tileset] { &self.tilesets }
    /// Returns all external file references.
    pub fn external_files(&self) -> &[ExternalFile] { &self.external_files }

    /// Returns the raw cel at the given layer and frame, or `None`. May return [`CelKind::Linked`]; use [`resolve_cel`](Self::resolve_cel) to follow links.
    pub fn cel(&self, layer: LayerRef, frame: usize) -> Option<&Cel> {
        self.cels.get(&(layer.0, frame))
    }

    /// Returns the cel at the given layer and frame, following linked cels.
    ///
    /// Unlike [`cel()`](Self::cel), this never returns [`CelKind::Linked`].
    /// Returns `None` if the cel doesn't exist or the link target is missing.
    pub fn resolve_cel(&self, layer: LayerRef, frame: usize) -> Option<&Cel> {
        let cel = self.cels.get(&(layer.0, frame))?;
        match &cel.kind {
            CelKind::Linked { source_frame, .. } => self.cels.get(&(layer.0, *source_frame)),
            _ => Some(cel),
        }
    }

    // --- Handle construction from parsed data ---
    /// Returns a [`LayerRef`] handle for the given index, or `None` if the index is out of bounds or points to a group.
    pub fn layer_ref(&self, index: usize) -> Option<LayerRef> {
        self.layers.get(index).and_then(|l| {
            match l.kind {
                LayerKind::Normal | LayerKind::Tilemap { .. } => Some(LayerRef(index)),
                LayerKind::Group => None,
            }
        })
    }

    /// Returns a [`GroupRef`] handle for the given index, or `None` if the index is out of bounds or is not a group.
    pub fn group_ref(&self, index: usize) -> Option<GroupRef> {
        self.layers.get(index).and_then(|l| {
            if l.kind == LayerKind::Group { Some(GroupRef(index)) } else { None }
        })
    }

    // --- Layers ---
    fn push_layer(&mut self, name: &str, kind: LayerKind, parent: Option<usize>, opts: &LayerOptions) -> usize {
        let index = self.layers.len();
        self.layers.push(Layer {
            name: name.to_string(), kind, parent,
            opacity: opts.opacity, blend_mode: opts.blend_mode,
            visible: opts.visible, editable: opts.editable,
            lock_movement: opts.lock_movement, background: opts.background,
            prefer_linked_cels: opts.prefer_linked_cels,
            collapsed: opts.collapsed, reference_layer: opts.reference_layer,
            user_data: None,
        });
        index
    }

    /// Adds a normal layer at the top level.
    pub fn add_layer(&mut self, name: &str) -> LayerRef {
        LayerRef(self.push_layer(name, LayerKind::Normal, None, &LayerOptions::default()))
    }
    /// Adds a normal layer at the top level with custom options.
    pub fn add_layer_with(&mut self, name: &str, opts: LayerOptions) -> LayerRef {
        LayerRef(self.push_layer(name, LayerKind::Normal, None, &opts))
    }
    /// Adds a group layer at the top level.
    pub fn add_group(&mut self, name: &str) -> GroupRef {
        GroupRef(self.push_layer(name, LayerKind::Group, None, &LayerOptions::default()))
    }
    /// Adds a group layer at the top level with custom options.
    pub fn add_group_with(&mut self, name: &str, opts: LayerOptions) -> GroupRef {
        GroupRef(self.push_layer(name, LayerKind::Group, None, &opts))
    }
    /// Adds a normal layer inside a group.
    pub fn add_layer_in(&mut self, name: &str, parent: GroupRef) -> LayerRef {
        LayerRef(self.push_layer(name, LayerKind::Normal, Some(parent.0), &LayerOptions::default()))
    }
    /// Adds a normal layer inside a group with custom options.
    pub fn add_layer_in_with(&mut self, name: &str, parent: GroupRef, opts: LayerOptions) -> LayerRef {
        LayerRef(self.push_layer(name, LayerKind::Normal, Some(parent.0), &opts))
    }
    /// Adds a nested group inside a parent group.
    pub fn add_group_in(&mut self, name: &str, parent: GroupRef) -> GroupRef {
        GroupRef(self.push_layer(name, LayerKind::Group, Some(parent.0), &LayerOptions::default()))
    }
    /// Adds a nested group inside a parent group with custom options.
    pub fn add_group_in_with(&mut self, name: &str, parent: GroupRef, opts: LayerOptions) -> GroupRef {
        GroupRef(self.push_layer(name, LayerKind::Group, Some(parent.0), &opts))
    }

    /// Adds a tilemap layer referencing the given tileset index.
    pub fn add_tilemap_layer(&mut self, name: &str, tileset_index: u32) -> LayerRef {
        let index = self.layers.len();
        self.layers.push(Layer {
            name: name.to_string(),
            kind: LayerKind::Tilemap { tileset_index },
            parent: None,
            opacity: 255,
            blend_mode: BlendMode::Normal,
            visible: true,
            editable: true,
            lock_movement: false,
            background: false,
            prefer_linked_cels: false,
            collapsed: false,
            reference_layer: false,
            user_data: None,
        });
        LayerRef(index)
    }

    /// Sets tilemap data for a tilemap layer/frame.
    #[allow(clippy::too_many_arguments)]
    pub fn set_tilemap_cel(
        &mut self, layer: LayerRef, frame: usize,
        tiles: Vec<u32>, width: u16, height: u16, x: i16, y: i16,
    ) -> Result<(), AsepriteError> {
        if frame >= self.frames.len() { return Err(AsepriteError::FrameOutOfBounds(frame)); }
        self.cels.insert((layer.0, frame), Cel {
            kind: CelKind::Tilemap {
                width, height, bits_per_tile: 32,
                tile_id_bitmask: 0x1fff_ffff, x_flip_bitmask: 0x2000_0000,
                y_flip_bitmask: 0x4000_0000, d_flip_bitmask: 0x8000_0000,
                tiles, x, y, original_compressed: None,
            },
            opacity: 255, z_index: 0, user_data: None, extra: None,
        });
        Ok(())
    }

    // --- Frames ---
    /// Adds a frame with the given duration in milliseconds. Returns the frame index.
    pub fn add_frame(&mut self, duration_ms: u16) -> usize {
        let index = self.frames.len();
        self.frames.push(Frame { duration_ms });
        index
    }

    // --- Cels ---
    /// Sets compressed pixel data for a layer/frame. Returns an error if the frame index is out of bounds.
    pub fn set_cel(&mut self, layer: LayerRef, frame: usize, pixels: Pixels, x: i16, y: i16) -> Result<(), AsepriteError> {
        if frame >= self.frames.len() { return Err(AsepriteError::FrameOutOfBounds(frame)); }
        self.cels.insert((layer.0, frame), Cel {
            kind: CelKind::Compressed { pixels, x, y, original_compressed: None },
            opacity: 255, z_index: 0, user_data: None, extra: None,
        });
        Ok(())
    }

    /// Sets pixel data for a layer/frame with custom opacity, position, and z-index.
    pub fn set_cel_with(&mut self, layer: LayerRef, frame: usize, opts: CelOptions) -> Result<(), AsepriteError> {
        if frame >= self.frames.len() { return Err(AsepriteError::FrameOutOfBounds(frame)); }
        self.cels.insert((layer.0, frame), Cel {
            kind: CelKind::Compressed { pixels: opts.pixels, x: opts.x, y: opts.y, original_compressed: None },
            opacity: opts.opacity, z_index: opts.z_index, user_data: None, extra: None,
        });
        Ok(())
    }

    /// Sets uncompressed pixel data for a layer/frame.
    pub fn set_raw_cel(&mut self, layer: LayerRef, frame: usize, pixels: Pixels, x: i16, y: i16) -> Result<(), AsepriteError> {
        if frame >= self.frames.len() { return Err(AsepriteError::FrameOutOfBounds(frame)); }
        self.cels.insert((layer.0, frame), Cel {
            kind: CelKind::Raw { pixels, x, y }, opacity: 255, z_index: 0,
            user_data: None, extra: None,
        });
        Ok(())
    }

    /// Sets a linked cel pointing to another frame's cel. Returns an error if either frame index is out of bounds.
    pub fn set_linked_cel(&mut self, layer: LayerRef, frame: usize, source_frame: usize) -> Result<(), AsepriteError> {
        if frame >= self.frames.len() { return Err(AsepriteError::FrameOutOfBounds(frame)); }
        if source_frame >= self.frames.len() { return Err(AsepriteError::FrameOutOfBounds(source_frame)); }
        self.cels.insert((layer.0, frame), Cel {
            kind: CelKind::Linked { source_frame, x: 0, y: 0 }, opacity: 255, z_index: 0,
            user_data: None, extra: None,
        });
        Ok(())
    }

    // --- Tags ---
    /// Adds an animation tag spanning the given frame range.
    pub fn add_tag(&mut self, name: &str, frames: RangeInclusive<usize>, direction: LoopDirection) -> Result<usize, AsepriteError> {
        self.add_tag_with(name, frames, direction, 0)
    }

    /// Adds an animation tag with a custom repeat count.
    pub fn add_tag_with(&mut self, name: &str, frames: RangeInclusive<usize>, direction: LoopDirection, repeat: u16) -> Result<usize, AsepriteError> {
        let from = *frames.start();
        let to = *frames.end();
        if !self.frames.is_empty() && to >= self.frames.len() {
            return Err(AsepriteError::InvalidFrameRange);
        }
        let index = self.tags.len();
        self.tags.push(Tag { name: name.to_string(), from_frame: from, to_frame: to, direction, repeat, user_data: None });
        Ok(index)
    }

    // --- Palette ---
    /// Sets the color palette. Returns an error if more than 256 entries.
    pub fn set_palette(&mut self, colors: &[Color]) -> Result<(), AsepriteError> {
        if colors.len() > 256 {
            return Err(AsepriteError::FormatLimitExceeded { field: "palette", value: colors.len(), max: 256 });
        }
        self.palette = colors.to_vec();
        Ok(())
    }

    /// Sets the transparent palette index.
    pub fn set_transparent_index(&mut self, index: u8) { self.transparent_index = index; }
    /// Sets the color profile.
    pub fn set_color_profile(&mut self, profile: ColorProfile) { self.color_profile = Some(profile); }

    /// Adds a slice.
    pub fn add_slice(&mut self, slice: Slice) { self.slices.push(slice); }
    /// Sets the sprite-level user data.
    pub fn set_sprite_user_data(&mut self, ud: UserData) { self.sprite_user_data = Some(ud); }
    /// Adds a tileset definition.
    pub fn add_tileset(&mut self, tileset: Tileset) { self.tilesets.push(tileset); }
    /// Adds an external file reference.
    pub fn add_external_file(&mut self, ef: ExternalFile) { self.external_files.push(ef); }

    /// Sets user data on a layer.
    pub fn set_layer_user_data(&mut self, layer: LayerRef, ud: UserData) {
        if let Some(l) = self.layers.get_mut(layer.0) { l.user_data = Some(ud); }
    }
    /// Sets user data on a group layer.
    pub fn set_group_user_data(&mut self, group: GroupRef, ud: UserData) {
        if let Some(l) = self.layers.get_mut(group.0) { l.user_data = Some(ud); }
    }
    /// Sets user data on a cel.
    pub fn set_cel_user_data(&mut self, layer: LayerRef, frame: usize, ud: UserData) {
        if let Some(cel) = self.cels.get_mut(&(layer.0, frame)) { cel.user_data = Some(ud); }
    }
    /// Sets extra precision bounds on a cel.
    pub fn set_cel_extra(&mut self, layer: LayerRef, frame: usize, extra: CelExtra) {
        if let Some(cel) = self.cels.get_mut(&(layer.0, frame)) { cel.extra = Some(extra); }
    }
    /// Sets user data on a tag.
    pub fn set_tag_user_data(&mut self, tag_index: usize, ud: UserData) {
        if let Some(tag) = self.tags.get_mut(tag_index) { tag.user_data = Some(ud); }
    }

    // --- Internal setters for reader ---
    pub(crate) fn set_flags(&mut self, flags: u32) { self.flags = flags; }
    pub(crate) fn set_deprecated_speed(&mut self, speed: u16) { self.deprecated_speed = speed; }
    pub(crate) fn set_num_colors(&mut self, n: u16) { self.num_colors = n; }
    pub(crate) fn set_pixel_ratio(&mut self, ratio: (u8, u8)) { self.pixel_ratio = ratio; }
    pub(crate) fn set_grid(&mut self, grid: GridInfo) { self.grid = grid; }
    pub(crate) fn push_legacy_mask(&mut self, mask: LegacyMask) { self.legacy_masks.push(mask); }
    pub(crate) fn push_unknown_chunk(&mut self, frame_index: usize, chunk_type: u16, data: Vec<u8>) {
        self.unknown_chunks.push(UnknownChunk { frame_index, chunk_type, data });
    }
    pub(crate) fn push_tileset(&mut self, tileset: Tileset) { self.tilesets.push(tileset); }
    pub(crate) fn push_external_file(&mut self, ef: ExternalFile) { self.external_files.push(ef); }
    pub(crate) fn tilesets_mut(&mut self) -> &mut Vec<Tileset> { &mut self.tilesets }
    pub(crate) fn push_layer_raw(&mut self, layer: Layer) { self.layers.push(layer); }
    pub(crate) fn insert_cel(&mut self, layer_index: usize, frame_index: usize, cel: Cel) {
        self.cels.insert((layer_index, frame_index), cel);
    }
    pub(crate) fn push_tag(&mut self, tag: Tag) { self.tags.push(tag); }
    pub(crate) fn push_slice(&mut self, slice: Slice) { self.slices.push(slice); }
    pub(crate) fn set_sprite_user_data_raw(&mut self, ud: UserData) { self.sprite_user_data = Some(ud); }
    pub(crate) fn layers_mut(&mut self) -> &mut Vec<Layer> { &mut self.layers }
    pub(crate) fn tags_mut(&mut self) -> &mut Vec<Tag> { &mut self.tags }
    pub(crate) fn slices_mut(&mut self) -> &mut Vec<Slice> { &mut self.slices }
    pub(crate) fn cel_mut(&mut self, layer_index: usize, frame_index: usize) -> Option<&mut Cel> {
        self.cels.get_mut(&(layer_index, frame_index))
    }
    pub(crate) fn set_palette_entry(&mut self, index: usize, color: Color) {
        if index >= self.palette.len() {
            self.palette.resize(index + 1, Color { r: 0, g: 0, b: 0, a: 255, name: None });
        }
        self.palette[index] = color;
    }
    pub(crate) fn cels_iter(&self) -> impl Iterator<Item = (&(usize, usize), &Cel)> { self.cels.iter() }
    pub(crate) fn unknown_chunks_for_frame(&self, frame_index: usize) -> impl Iterator<Item = &UnknownChunk> {
        self.unknown_chunks.iter().filter(move |uc| uc.frame_index == frame_index)
    }

    pub(crate) fn push_chunk_order(&mut self, frame_index: usize, chunk_type: u16, layer_index: Option<usize>) {
        self.chunk_order.push(ChunkOrderEntry { frame_index, chunk_type, layer_index });
    }

    pub(crate) fn chunk_order_for_frame(&self, frame_index: usize) -> impl Iterator<Item = &ChunkOrderEntry> {
        self.chunk_order.iter().filter(move |e| e.frame_index == frame_index)
    }
}
