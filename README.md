# aseprite-io

[![CI](https://github.com/spebern/aseprite-io/actions/workflows/ci.yml/badge.svg)](https://github.com/spebern/aseprite-io/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/spebern/aseprite-io/graph/badge.svg)](https://codecov.io/gh/spebern/aseprite-io)
[![Crates.io](https://img.shields.io/crates/v/aseprite-io.svg)](https://crates.io/crates/aseprite-io)
[![docs.rs](https://docs.rs/aseprite-io/badge.svg)](https://docs.rs/aseprite-io)

Read and write [Aseprite](https://www.aseprite.org/) `.ase`/`.aseprite` files in Rust.

The only Rust crate that supports **both reading and writing** the Aseprite binary format, with byte-perfect round-trip fidelity.

## Features

- Full [Aseprite file format](https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md) support: RGBA, Grayscale, and Indexed color modes
- All layer types: normal, group, and tilemap
- Animation tags, slices (with nine-patch and pivot), user data with typed properties
- Tileset support (embedded and external)
- Linked cels, cel extras, legacy mask chunks
- Byte-perfect round-trip: read a file and write it back to get identical bytes

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
aseprite-io = "0.1"
```

### Reading a file

```rust
use aseprite::AsepriteFile;

let data = std::fs::read("sprite.aseprite")?;
let file = AsepriteFile::from_reader(&data[..])?;

println!("{}x{}, {} frames", file.width(), file.height(), file.frames().len());
for layer in file.layers() {
    println!("  layer: {}", layer.name);
}
```

### Creating a file from scratch

```rust
use aseprite::*;

let mut file = AsepriteFile::new(16, 16, ColorMode::Rgba);
let layer = file.add_layer("Background");
let frame = file.add_frame(100);
let pixels = Pixels::new(vec![0u8; 16 * 16 * 4], 16, 16, ColorMode::Rgba)?;
file.set_cel(layer, frame, pixels, 0, 0)?;

std::fs::write("output.aseprite", {
    let mut buf = Vec::new();
    file.write_to(&mut buf)?;
    buf
})?;
```

## Feature flags

| Feature | Description |
|---------|-------------|
| `image` | Conversions between `Pixels` and `image::RgbaImage` |
| `tiny-skia` | Conversions between `Pixels` and `tiny_skia::Pixmap` (handles premultiplied alpha) |

## Alternatives

| Crate | Read | Write | Tilemaps | User data |
|-------|:----:|:-----:|:--------:|:---------:|
| **aseprite-io** | Yes | **Yes** | Yes | Yes |
| asefile | Yes | No | Yes | Partial |
| aseprite-loader | Yes | No | No | No |

## License

MIT OR Apache-2.0
