use aseprite::*;

fn write_and_read(file: &AsepriteFile) -> AsepriteFile {
    let mut buf = Vec::new();
    file.write_to(&mut buf).unwrap();
    AsepriteFile::from_reader(&buf[..]).unwrap()
}

// --- Write-then-read integration tests ---

#[test]
fn single_layer_single_frame() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let layer = file.add_layer("Background");
    let frame = file.add_frame(100);
    let pixels = Pixels::new(vec![255u8; 4 * 4 * 4], 4, 4, ColorMode::Rgba).unwrap();
    file.set_cel(layer, frame, pixels, 0, 0).unwrap();

    let out = write_and_read(&file);
    assert_eq!(out.width(), 4);
    assert_eq!(out.height(), 4);
    assert_eq!(out.color_mode(), ColorMode::Rgba);
    assert_eq!(out.layers().len(), 1);
    assert_eq!(out.layers()[0].name, "Background");
    assert_eq!(out.frames().len(), 1);
    assert_eq!(out.frames()[0].duration_ms, 100);
    let layer_ref = out.layer_ref(0).unwrap();
    let cel = out.cel(layer_ref, 0).unwrap();
    match &cel.kind {
        CelKind::Compressed { pixels, x, y, .. } => {
            assert_eq!(*x, 0);
            assert_eq!(*y, 0);
            assert_eq!(pixels.width, 4);
            assert_eq!(pixels.height, 4);
            assert_eq!(pixels.data.len(), 4 * 4 * 4);
            assert!(pixels.data.iter().all(|&b| b == 255));
        }
        other => panic!("expected Compressed cel, got {:?}", other),
    }
}

#[test]
fn nested_groups() {
    let mut file = AsepriteFile::new(8, 8, ColorMode::Rgba);
    let _bg = file.add_layer("Background");
    let character = file.add_group("Character");
    let _body = file.add_layer_in("Body", character);
    let accessories = file.add_group_in("Accessories", character);
    let _hat = file.add_layer_in("Hat", accessories);
    file.add_frame(100);

    let out = write_and_read(&file);
    assert_eq!(out.layers().len(), 5);

    assert_eq!(out.layers()[0].name, "Background");
    assert_eq!(out.layers()[0].parent, None);
    assert_eq!(out.layers()[0].kind, LayerKind::Normal);

    assert_eq!(out.layers()[1].name, "Character");
    assert_eq!(out.layers()[1].parent, None);
    assert_eq!(out.layers()[1].kind, LayerKind::Group);

    assert_eq!(out.layers()[2].name, "Body");
    assert_eq!(out.layers()[2].parent, Some(1));
    assert_eq!(out.layers()[2].kind, LayerKind::Normal);

    assert_eq!(out.layers()[3].name, "Accessories");
    assert_eq!(out.layers()[3].parent, Some(1));
    assert_eq!(out.layers()[3].kind, LayerKind::Group);

    assert_eq!(out.layers()[4].name, "Hat");
    assert_eq!(out.layers()[4].parent, Some(3));
    assert_eq!(out.layers()[4].kind, LayerKind::Normal);
}

#[test]
fn multiple_frames_and_durations() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);
    file.add_frame(200);
    file.add_frame(50);

    let out = write_and_read(&file);
    assert_eq!(out.frames().len(), 3);
    assert_eq!(out.frames()[0].duration_ms, 100);
    assert_eq!(out.frames()[1].duration_ms, 200);
    assert_eq!(out.frames()[2].duration_ms, 50);
}

#[test]
fn linked_cels() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    let layer = file.add_layer("Layer");
    file.add_frame(100);
    file.add_frame(100);

    let pixels = Pixels::new(vec![128u8; 2 * 2 * 4], 2, 2, ColorMode::Rgba).unwrap();
    file.set_cel(layer, 0, pixels, 0, 0).unwrap();
    file.set_linked_cel(layer, 1, 0).unwrap();

    let out = write_and_read(&file);
    let layer_ref = out.layer_ref(0).unwrap();

    let cel0 = out.cel(layer_ref, 0).unwrap();
    assert!(matches!(&cel0.kind, CelKind::Compressed { .. }));

    let cel1 = out.cel(layer_ref, 1).unwrap();
    match &cel1.kind {
        CelKind::Linked { source_frame, .. } => assert_eq!(*source_frame, 0),
        other => panic!("expected Linked cel, got {:?}", other),
    }
}

#[test]
fn tags_with_directions() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);
    file.add_frame(100);
    file.add_frame(100);

    file.add_tag("idle", 0..=0, LoopDirection::Forward).unwrap();
    file.add_tag_with("walk", 1..=2, LoopDirection::PingPong, 3).unwrap();

    let out = write_and_read(&file);
    assert_eq!(out.tags().len(), 2);

    assert_eq!(out.tags()[0].name, "idle");
    assert_eq!(out.tags()[0].from_frame, 0);
    assert_eq!(out.tags()[0].to_frame, 0);
    assert_eq!(out.tags()[0].direction, LoopDirection::Forward);
    assert_eq!(out.tags()[0].repeat, 0);

    assert_eq!(out.tags()[1].name, "walk");
    assert_eq!(out.tags()[1].from_frame, 1);
    assert_eq!(out.tags()[1].to_frame, 2);
    assert_eq!(out.tags()[1].direction, LoopDirection::PingPong);
    assert_eq!(out.tags()[1].repeat, 3);
}

#[test]
fn indexed_mode_with_palette() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Indexed);
    file.set_transparent_index(0);
    let palette = vec![
        Color { r: 0, g: 0, b: 0, a: 0, name: None },
        Color { r: 255, g: 0, b: 0, a: 255, name: None },
        Color { r: 0, g: 255, b: 0, a: 255, name: None },
    ];
    file.set_palette(&palette).unwrap();

    let layer = file.add_layer("Layer");
    file.add_frame(100);
    let pixels = Pixels::new(vec![0, 1, 2, 1], 2, 2, ColorMode::Indexed).unwrap();
    file.set_cel(layer, 0, pixels, 0, 0).unwrap();

    let out = write_and_read(&file);
    assert_eq!(out.color_mode(), ColorMode::Indexed);
    assert_eq!(out.transparent_index(), 0);
    assert_eq!(out.palette().len(), 3);
    assert_eq!(out.palette()[0], Color { r: 0, g: 0, b: 0, a: 0, name: None });
    assert_eq!(out.palette()[1], Color { r: 255, g: 0, b: 0, a: 255, name: None });
    assert_eq!(out.palette()[2], Color { r: 0, g: 255, b: 0, a: 255, name: None });
}

#[test]
fn grayscale_mode() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Grayscale);
    let layer = file.add_layer("Layer");
    file.add_frame(100);
    // Grayscale: 2 bytes per pixel (gray + alpha)
    let pixels = Pixels::new(vec![128, 255, 64, 255, 0, 255, 200, 128], 2, 2, ColorMode::Grayscale).unwrap();
    file.set_cel(layer, 0, pixels, 0, 0).unwrap();

    let out = write_and_read(&file);
    assert_eq!(out.color_mode(), ColorMode::Grayscale);
    let layer_ref = out.layer_ref(0).unwrap();
    let cel = out.cel(layer_ref, 0).unwrap();
    match &cel.kind {
        CelKind::Compressed { pixels, .. } => {
            assert_eq!(pixels.data, vec![128, 255, 64, 255, 0, 255, 200, 128]);
        }
        other => panic!("expected Compressed cel, got {:?}", other),
    }
}

#[test]
fn cel_options() {
    let mut file = AsepriteFile::new(8, 8, ColorMode::Rgba);
    let layer = file.add_layer("Layer");
    file.add_frame(100);
    let pixels = Pixels::new(vec![0u8; 2 * 2 * 4], 2, 2, ColorMode::Rgba).unwrap();
    file.set_cel_with(layer, 0, CelOptions {
        pixels,
        x: 5,
        y: -3,
        opacity: 128,
        z_index: 2,
    }).unwrap();

    let out = write_and_read(&file);
    let layer_ref = out.layer_ref(0).unwrap();
    let cel = out.cel(layer_ref, 0).unwrap();
    assert_eq!(cel.opacity, 128);
    assert_eq!(cel.z_index, 2);
    match &cel.kind {
        CelKind::Compressed { x, y, .. } => {
            assert_eq!(*x, 5);
            assert_eq!(*y, -3);
        }
        other => panic!("expected Compressed cel, got {:?}", other),
    }
}

#[test]
fn layer_options() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    file.add_layer_with("Hidden", LayerOptions {
        visible: false,
        opacity: 128,
        blend_mode: BlendMode::Multiply,
        ..LayerOptions::default()
    });
    file.add_frame(100);

    let out = write_and_read(&file);
    assert_eq!(out.layers().len(), 1);
    let layer = &out.layers()[0];
    assert_eq!(layer.name, "Hidden");
    assert!(!layer.visible);
    assert_eq!(layer.opacity, 128);
    assert_eq!(layer.blend_mode, BlendMode::Multiply);
}

#[test]
fn color_profile_srgb() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    file.set_color_profile(ColorProfile::SRgb { flags: 0, gamma: 0 });
    file.add_layer("Layer");
    file.add_frame(100);

    let out = write_and_read(&file);
    match out.color_profile() {
        Some(ColorProfile::SRgb { flags, gamma }) => {
            assert_eq!(*flags, 0);
            assert_eq!(*gamma, 0);
        }
        other => panic!("expected SRgb profile, got {:?}", other),
    }
}

#[test]
fn empty_frame_no_cels() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let layer = file.add_layer("Layer");
    file.add_frame(100);

    let out = write_and_read(&file);
    let layer_ref = out.layer_ref(0).unwrap();
    assert!(out.cel(layer_ref, 0).is_none());
    // Ensure the layer itself still exists
    assert_eq!(out.layers().len(), 1);
    assert_eq!(out.layers()[0].name, "Layer");
    let _ = layer; // suppress unused warning
}

// --- Edge case tests ---

#[test]
fn negative_cel_offset() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let layer = file.add_layer("Layer");
    file.add_frame(100);
    let pixels = Pixels::new(vec![0u8; 2 * 2 * 4], 2, 2, ColorMode::Rgba).unwrap();
    file.set_cel(layer, 0, pixels, -1, -2).unwrap();

    let out = write_and_read(&file);
    let layer_ref = out.layer_ref(0).unwrap();
    let cel = out.cel(layer_ref, 0).unwrap();
    match &cel.kind {
        CelKind::Compressed { x, y, .. } => {
            assert_eq!(*x, -1);
            assert_eq!(*y, -2);
        }
        other => panic!("expected Compressed cel, got {:?}", other),
    }
}

#[test]
fn max_palette_size() {
    let mut file = AsepriteFile::new(1, 1, ColorMode::Indexed);
    let palette: Vec<Color> = (0..=255u8)
        .map(|i| Color { r: i, g: i, b: i, a: 255, name: None })
        .collect();
    assert_eq!(palette.len(), 256);
    file.set_palette(&palette).unwrap();

    let layer = file.add_layer("Layer");
    file.add_frame(100);
    let pixels = Pixels::new(vec![0], 1, 1, ColorMode::Indexed).unwrap();
    file.set_cel(layer, 0, pixels, 0, 0).unwrap();

    let out = write_and_read(&file);
    assert_eq!(out.palette().len(), 256);
    for (i, color) in out.palette().iter().enumerate() {
        let v = i as u8;
        assert_eq!(color.r, v, "palette[{i}].r mismatch");
        assert_eq!(color.g, v, "palette[{i}].g mismatch");
        assert_eq!(color.b, v, "palette[{i}].b mismatch");
        assert_eq!(color.a, 255, "palette[{i}].a mismatch");
    }
}

#[test]
fn zero_duration_frame() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(0);

    let out = write_and_read(&file);
    assert_eq!(out.frames().len(), 1);
    assert_eq!(out.frames()[0].duration_ms, 0);
}

#[test]
fn pixels_rejects_wrong_buffer_size() {
    let result = Pixels::new(vec![0u8; 10], 4, 4, ColorMode::Rgba);
    match result {
        Err(AsepriteError::PixelSizeMismatch { expected: 64, actual: 10 }) => {}
        other => panic!("expected PixelSizeMismatch(64, 10), got {:?}", other),
    }
}

#[test]
fn frame_out_of_bounds() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    let layer = file.add_layer("Layer");
    file.add_frame(100);

    let pixels = Pixels::new(vec![0u8; 2 * 2 * 4], 2, 2, ColorMode::Rgba).unwrap();
    let result = file.set_cel(layer, 5, pixels, 0, 0);
    match result {
        Err(AsepriteError::FrameOutOfBounds(5)) => {}
        other => panic!("expected FrameOutOfBounds(5), got {:?}", other),
    }
}

#[test]
fn palette_too_large() {
    let mut file = AsepriteFile::new(1, 1, ColorMode::Indexed);
    let palette: Vec<Color> = (0..257)
        .map(|_| Color { r: 0, g: 0, b: 0, a: 255, name: None })
        .collect();
    let result = file.set_palette(&palette);
    match result {
        Err(AsepriteError::FormatLimitExceeded { field: "palette", value: 257, max: 256 }) => {}
        other => panic!("expected FormatLimitExceeded, got {:?}", other),
    }
}

#[test]
fn invalid_tag_range() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);

    let result = file.add_tag("bad", 0..=5, LoopDirection::Forward);
    match result {
        Err(AsepriteError::InvalidFrameRange) => {}
        other => panic!("expected InvalidFrameRange, got {:?}", other),
    }
}

#[cfg(feature = "image")]
#[test]
fn image_crate_conversion() {
    use image::RgbaImage;
    let img = RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
    let pixels: Pixels = img.into();
    assert_eq!(pixels.width, 4);
    assert_eq!(pixels.height, 4);
    assert_eq!(pixels.data.len(), 64);

    let back: RgbaImage = pixels.try_into().unwrap();
    assert_eq!(back.width(), 4);
    assert_eq!(back.get_pixel(0, 0), &image::Rgba([255, 0, 0, 255]));
}

#[cfg(feature = "tiny-skia")]
#[test]
fn tiny_skia_conversion() {
    use tiny_skia::Pixmap;

    let mut pixmap = Pixmap::new(4, 4).unwrap();
    for pixel in pixmap.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(128, 0, 0, 255).unwrap();
    }

    let pixels: Pixels = pixmap.into();
    assert_eq!(pixels.width, 4);
    assert_eq!(pixels.height, 4);
    assert_eq!(pixels.data[0], 128);
    assert_eq!(pixels.data[3], 255);

    let back: Pixmap = pixels.try_into().unwrap();
    assert_eq!(back.width(), 4);
    assert_eq!(back.pixel(0, 0).unwrap().red(), 128);
}

#[cfg(feature = "tiny-skia")]
#[test]
fn tiny_skia_premultiplied_alpha_round_trip() {
    use tiny_skia::Pixmap;

    let mut pixmap = Pixmap::new(1, 1).unwrap();
    *pixmap.pixels_mut().first_mut().unwrap() =
        tiny_skia::PremultipliedColorU8::from_rgba(64, 0, 0, 128).unwrap();

    let pixels: Pixels = pixmap.into();
    assert_eq!(pixels.data[0], 128); // straight: 64*255/128 ≈ 128
    assert_eq!(pixels.data[3], 128);

    let back: Pixmap = pixels.try_into().unwrap();
    assert_eq!(back.pixel(0, 0).unwrap().red(), 64); // back to premultiplied
}

#[cfg(feature = "tiny-skia")]
#[test]
fn tiny_skia_ref_conversion() {
    use tiny_skia::Pixmap;

    let pixmap = Pixmap::new(2, 2).unwrap();
    let pixels: Pixels = (&pixmap).into();
    assert_eq!(pixels.width, 2);
    assert_eq!(pixels.height, 2);
    assert_eq!(pixels.data.len(), 2 * 2 * 4);
}

// --- v0.2 feature tests: User Data ---

#[test]
fn user_data_on_layer() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    let layer = file.add_layer("MyLayer");
    file.add_frame(100);
    file.set_layer_user_data(layer, UserData {
        text: Some("hello".to_string()),
        color: None,
        properties: vec![],
    });

    let out = write_and_read(&file);
    let ud = out.layers()[0].user_data.as_ref().expect("layer should have user data");
    assert_eq!(ud.text.as_deref(), Some("hello"));
}

#[test]
fn user_data_text_and_color() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);
    file.set_sprite_user_data(UserData {
        text: Some("sprite info".to_string()),
        color: Some(Color { r: 255, g: 0, b: 0, a: 255, name: None }),
        properties: vec![],
    });

    let out = write_and_read(&file);
    let ud = out.sprite_user_data().as_ref().expect("sprite should have user data");
    assert_eq!(ud.text.as_deref(), Some("sprite info"));
    let c = ud.color.as_ref().expect("should have color");
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 255);
}

#[test]
fn user_data_with_properties() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);

    let props = PropertiesMap {
        key: 0,
        entries: vec![
            ("name".to_string(), PropertyValue::String("hero".to_string())),
            ("health".to_string(), PropertyValue::Int32(100)),
            ("alive".to_string(), PropertyValue::Bool(true)),
            ("speed".to_string(), PropertyValue::Float(3.5)),
            ("score".to_string(), PropertyValue::Int64(999999)),
            ("origin".to_string(), PropertyValue::Point(10, 20)),
            ("dimensions".to_string(), PropertyValue::Size(64, 64)),
            ("bounds".to_string(), PropertyValue::Rect(0, 0, 100, 200)),
        ],
    };
    file.set_sprite_user_data(UserData {
        text: None,
        color: None,
        properties: vec![props],
    });

    let out = write_and_read(&file);
    let ud = out.sprite_user_data().as_ref().expect("sprite should have user data");
    assert_eq!(ud.properties.len(), 1);
    let map = &ud.properties[0];
    assert_eq!(map.key, 0);
    assert_eq!(map.entries.len(), 8);

    let find = |name: &str| -> &PropertyValue {
        &map.entries.iter().find(|(k, _)| k == name).unwrap().1
    };
    assert_eq!(*find("name"), PropertyValue::String("hero".to_string()));
    assert_eq!(*find("health"), PropertyValue::Int32(100));
    assert_eq!(*find("alive"), PropertyValue::Bool(true));
    assert_eq!(*find("speed"), PropertyValue::Float(3.5));
    assert_eq!(*find("score"), PropertyValue::Int64(999999));
    assert_eq!(*find("origin"), PropertyValue::Point(10, 20));
    assert_eq!(*find("dimensions"), PropertyValue::Size(64, 64));
    assert_eq!(*find("bounds"), PropertyValue::Rect(0, 0, 100, 200));
}

#[test]
fn tag_user_data() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);
    file.add_frame(100);
    let tag_idx = file.add_tag("attack", 0..=1, LoopDirection::Forward).unwrap();
    file.set_tag_user_data(tag_idx, UserData {
        text: Some("melee".to_string()),
        color: None,
        properties: vec![],
    });

    let out = write_and_read(&file);
    let ud = out.tags()[0].user_data.as_ref().expect("tag should have user data");
    assert_eq!(ud.text.as_deref(), Some("melee"));
}

#[test]
fn tag_user_data_sequential_with_gap() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);
    file.add_frame(100);
    file.add_frame(100);
    file.add_tag("idle", 0..=0, LoopDirection::Forward).unwrap();
    let tag1 = file.add_tag("run", 1..=2, LoopDirection::Forward).unwrap();
    file.set_tag_user_data(tag1, UserData {
        text: Some("running animation".to_string()),
        color: None,
        properties: vec![],
    });

    let out = write_and_read(&file);
    assert_eq!(out.tags().len(), 2);
    assert!(out.tags()[0].user_data.is_none() ||
        out.tags()[0].user_data.as_ref().map_or(true, |ud| ud.text.is_none() && ud.color.is_none() && ud.properties.is_empty()));
    let ud = out.tags()[1].user_data.as_ref().expect("tag 1 should have user data");
    assert_eq!(ud.text.as_deref(), Some("running animation"));
}

#[test]
fn cel_user_data() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let layer = file.add_layer("Layer");
    file.add_frame(100);
    let pixels = Pixels::new(vec![0u8; 4 * 4 * 4], 4, 4, ColorMode::Rgba).unwrap();
    file.set_cel(layer, 0, pixels, 0, 0).unwrap();
    file.set_cel_user_data(layer, 0, UserData {
        text: Some("cel note".to_string()),
        color: Some(Color { r: 0, g: 255, b: 0, a: 255, name: None }),
        properties: vec![],
    });

    let out = write_and_read(&file);
    let layer_ref = out.layer_ref(0).unwrap();
    let cel = out.cel(layer_ref, 0).unwrap();
    let ud = cel.user_data.as_ref().expect("cel should have user data");
    assert_eq!(ud.text.as_deref(), Some("cel note"));
    let c = ud.color.as_ref().expect("should have color");
    assert_eq!(c.r, 0);
    assert_eq!(c.g, 255);
    assert_eq!(c.b, 0);
    assert_eq!(c.a, 255);
}

// --- v0.2 feature tests: Slices ---

#[test]
fn basic_slice() {
    let mut file = AsepriteFile::new(32, 32, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);
    file.add_slice(Slice {
        name: "hitbox".to_string(),
        keys: vec![SliceKey {
            frame: 0, x: 4, y: 4, width: 24, height: 12,
            nine_patch: None, pivot: None,
        }],
        has_nine_patch: false,
        has_pivot: false,
        user_data: None,
    });

    let out = write_and_read(&file);
    assert_eq!(out.slices().len(), 1);
    let s = &out.slices()[0];
    assert_eq!(s.name, "hitbox");
    assert_eq!(s.keys.len(), 1);
    assert_eq!(s.keys[0].frame, 0);
    assert_eq!(s.keys[0].x, 4);
    assert_eq!(s.keys[0].y, 4);
    assert_eq!(s.keys[0].width, 24);
    assert_eq!(s.keys[0].height, 12);
}

#[test]
fn nine_patch_slice() {
    let mut file = AsepriteFile::new(32, 32, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);
    file.add_slice(Slice {
        name: "button".to_string(),
        keys: vec![SliceKey {
            frame: 0, x: 0, y: 0, width: 32, height: 32,
            nine_patch: Some(NinePatch {
                center_x: 4, center_y: 4, center_width: 24, center_height: 24,
            }),
            pivot: None,
        }],
        has_nine_patch: true,
        has_pivot: false,
        user_data: None,
    });

    let out = write_and_read(&file);
    assert_eq!(out.slices().len(), 1);
    let s = &out.slices()[0];
    assert!(s.has_nine_patch);
    let np = s.keys[0].nine_patch.as_ref().expect("should have nine patch");
    assert_eq!(np.center_x, 4);
    assert_eq!(np.center_width, 24);
}

#[test]
fn animated_slice_with_pivot() {
    let mut file = AsepriteFile::new(32, 32, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);
    file.add_frame(100);
    file.add_slice(Slice {
        name: "weapon".to_string(),
        keys: vec![
            SliceKey {
                frame: 0, x: 2, y: 3, width: 16, height: 16,
                nine_patch: None, pivot: Some((8, 12)),
            },
            SliceKey {
                frame: 1, x: 4, y: 5, width: 20, height: 20,
                nine_patch: None, pivot: Some((10, 15)),
            },
        ],
        has_nine_patch: false,
        has_pivot: true,
        user_data: None,
    });

    let out = write_and_read(&file);
    assert_eq!(out.slices().len(), 1);
    let s = &out.slices()[0];
    assert!(s.has_pivot);
    assert_eq!(s.keys.len(), 2);
    assert_eq!(s.keys[0].frame, 0);
    assert_eq!(s.keys[0].x, 2);
    assert_eq!(s.keys[0].y, 3);
    assert_eq!(s.keys[0].pivot, Some((8, 12)));
    assert_eq!(s.keys[1].frame, 1);
    assert_eq!(s.keys[1].x, 4);
    assert_eq!(s.keys[1].y, 5);
    assert_eq!(s.keys[1].pivot, Some((10, 15)));
}

#[test]
fn slice_with_user_data() {
    let mut file = AsepriteFile::new(32, 32, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);
    file.add_slice(Slice {
        name: "region".to_string(),
        keys: vec![SliceKey {
            frame: 0, x: 0, y: 0, width: 32, height: 32,
            nine_patch: None, pivot: None,
        }],
        has_nine_patch: false,
        has_pivot: false,
        user_data: Some(UserData {
            text: Some("spawn point".to_string()),
            color: None,
            properties: vec![],
        }),
    });

    let out = write_and_read(&file);
    assert_eq!(out.slices().len(), 1);
    let ud = out.slices()[0].user_data.as_ref().expect("slice should have user data");
    assert_eq!(ud.text.as_deref(), Some("spawn point"));
}

// --- v0.2 feature tests: Cel Extra ---

#[test]
fn cel_extra_round_trip() {
    let mut file = AsepriteFile::new(8, 8, ColorMode::Rgba);
    let layer = file.add_layer("Layer");
    file.add_frame(100);
    let pixels = Pixels::new(vec![0u8; 4 * 4 * 4], 4, 4, ColorMode::Rgba).unwrap();
    file.set_cel(layer, 0, pixels, 0, 0).unwrap();
    file.set_cel_extra(layer, 0, CelExtra {
        precise_x: 65536,
        precise_y: 131072,
        width: 262144,
        height: 524288,
    });

    let out = write_and_read(&file);
    let layer_ref = out.layer_ref(0).unwrap();
    let cel = out.cel(layer_ref, 0).unwrap();
    let extra = cel.extra.as_ref().expect("cel should have extra");
    assert_eq!(extra.precise_x, 65536);
    assert_eq!(extra.precise_y, 131072);
    assert_eq!(extra.width, 262144);
    assert_eq!(extra.height, 524288);
}

// --- v0.2 feature tests: Palette Entry Names ---

#[test]
fn palette_entry_names() {
    let mut file = AsepriteFile::new(1, 1, ColorMode::Indexed);
    file.set_transparent_index(0);
    let palette = vec![
        Color { r: 0, g: 0, b: 0, a: 255, name: Some("Black".to_string()) },
        Color { r: 128, g: 128, b: 128, a: 255, name: None },
        Color { r: 0, g: 255, b: 0, a: 255, name: Some("Green".to_string()) },
    ];
    file.set_palette(&palette).unwrap();
    let layer = file.add_layer("Layer");
    file.add_frame(100);
    let pixels = Pixels::new(vec![0], 1, 1, ColorMode::Indexed).unwrap();
    file.set_cel(layer, 0, pixels, 0, 0).unwrap();

    let out = write_and_read(&file);
    assert_eq!(out.palette().len(), 3);
    assert_eq!(out.palette()[0].name.as_deref(), Some("Black"));
    assert_eq!(out.palette()[1].name, None);
    assert_eq!(out.palette()[2].name.as_deref(), Some("Green"));
}

// --- v0.3 feature tests: Tilesets, Tilemaps, External Files ---

#[test]
fn embedded_tileset() {
    let mut file = AsepriteFile::new(16, 16, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);

    // 2 tiles, each 8x8 RGBA = 8*8*4 = 256 bytes per tile
    let tile0: Vec<u8> = vec![255; 8 * 8 * 4];
    let tile1: Vec<u8> = vec![128; 8 * 8 * 4];
    let mut pixels = tile0.clone();
    pixels.extend_from_slice(&tile1);

    file.add_tileset(Tileset {
        id: 0,
        flags: TilesetFlags(2), // has_embedded_tiles
        name: "terrain".to_string(),
        tile_count: 2,
        tile_width: 8,
        tile_height: 8,
        base_index: 1,
        data: TilesetData::Embedded { pixels: pixels.clone(), original_compressed: None },
        user_data: None,
        tile_user_data: vec![],
    });

    let out = write_and_read(&file);
    assert_eq!(out.tilesets().len(), 1);
    let ts = &out.tilesets()[0];
    assert_eq!(ts.tile_count, 2);
    assert_eq!(ts.tile_width, 8);
    assert_eq!(ts.tile_height, 8);
    assert_eq!(ts.name, "terrain");
    match &ts.data {
        TilesetData::Embedded { pixels: out_pixels, .. } => {
            assert_eq!(out_pixels.len(), pixels.len());
            assert_eq!(*out_pixels, pixels);
        }
        other => panic!("expected Embedded tileset data, got {:?}", other),
    }
}

#[test]
fn external_tileset() {
    let mut file = AsepriteFile::new(16, 16, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);

    file.add_external_file(ExternalFile {
        id: 1,
        file_type: ExternalFileType::Tileset,
        name: "shared_tiles.aseprite".to_string(),
    });

    file.add_tileset(Tileset {
        id: 0,
        flags: TilesetFlags(1), // has_external_link
        name: "ext_terrain".to_string(),
        tile_count: 4,
        tile_width: 16,
        tile_height: 16,
        base_index: 1,
        data: TilesetData::External { external_file_id: 1, tileset_id_in_external: 42 },
        user_data: None,
        tile_user_data: vec![],
    });

    let out = write_and_read(&file);
    assert_eq!(out.tilesets().len(), 1);
    let ts = &out.tilesets()[0];
    assert_eq!(ts.name, "ext_terrain");
    match &ts.data {
        TilesetData::External { external_file_id, tileset_id_in_external } => {
            assert_eq!(*external_file_id, 1);
            assert_eq!(*tileset_id_in_external, 42);
        }
        other => panic!("expected External tileset data, got {:?}", other),
    }

    assert_eq!(out.external_files().len(), 1);
    assert_eq!(out.external_files()[0].id, 1);
    assert_eq!(out.external_files()[0].file_type, ExternalFileType::Tileset);
    assert_eq!(out.external_files()[0].name, "shared_tiles.aseprite");
}

#[test]
fn tilemap_cel() {
    let mut file = AsepriteFile::new(32, 32, ColorMode::Rgba);

    // Embedded tileset with 1 tile (needed for the tilemap layer reference)
    let tile_pixels = vec![0u8; 8 * 8 * 4];
    file.add_tileset(Tileset {
        id: 0,
        flags: TilesetFlags(2),
        name: "tiles".to_string(),
        tile_count: 1,
        tile_width: 8,
        tile_height: 8,
        base_index: 1,
        data: TilesetData::Embedded { pixels: tile_pixels, original_compressed: None },
        user_data: None,
        tile_user_data: vec![],
    });

    let layer = file.add_tilemap_layer("tilemap", 0);
    file.add_frame(100);

    // 4x4 tilemap grid
    let tiles: Vec<u32> = (0..16).collect();
    file.set_tilemap_cel(layer, 0, tiles.clone(), 4, 4, 0, 0).unwrap();

    let out = write_and_read(&file);
    let layer_ref = out.layer_ref(0).unwrap();
    let cel = out.cel(layer_ref, 0).unwrap();
    match &cel.kind {
        CelKind::Tilemap { width, height, tiles: out_tiles, .. } => {
            assert_eq!(*width, 4);
            assert_eq!(*height, 4);
            assert_eq!(out_tiles.len(), 16);
            assert_eq!(*out_tiles, tiles);
        }
        other => panic!("expected Tilemap cel, got {:?}", other),
    }
}

#[test]
fn tilemap_layer_kind() {
    let mut file = AsepriteFile::new(16, 16, ColorMode::Rgba);

    file.add_tileset(Tileset {
        id: 0,
        flags: TilesetFlags(2),
        name: "tiles".to_string(),
        tile_count: 1,
        tile_width: 8,
        tile_height: 8,
        base_index: 1,
        data: TilesetData::Embedded { pixels: vec![0u8; 8 * 8 * 4], original_compressed: None },
        user_data: None,
        tile_user_data: vec![],
    });

    file.add_tilemap_layer("my_tilemap", 0);
    file.add_frame(100);

    let out = write_and_read(&file);
    assert_eq!(out.layers().len(), 1);
    assert_eq!(out.layers()[0].name, "my_tilemap");
    match out.layers()[0].kind {
        LayerKind::Tilemap { tileset_index } => {
            assert_eq!(tileset_index, 0);
        }
        other => panic!("expected Tilemap layer kind, got {:?}", other),
    }
}

#[test]
fn tileset_user_data() {
    let mut file = AsepriteFile::new(16, 16, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);

    file.add_tileset(Tileset {
        id: 0,
        flags: TilesetFlags(2),
        name: "ground".to_string(),
        tile_count: 1,
        tile_width: 8,
        tile_height: 8,
        base_index: 1,
        data: TilesetData::Embedded { pixels: vec![0u8; 8 * 8 * 4], original_compressed: None },
        user_data: Some(UserData {
            text: Some("ground tiles".to_string()),
            color: None,
            properties: vec![],
        }),
        tile_user_data: vec![],
    });

    let out = write_and_read(&file);
    assert_eq!(out.tilesets().len(), 1);
    let ud = out.tilesets()[0].user_data.as_ref().expect("tileset should have user data");
    assert_eq!(ud.text.as_deref(), Some("ground tiles"));
}

#[test]
fn external_files_round_trip() {
    let mut file = AsepriteFile::new(16, 16, ColorMode::Rgba);
    file.add_layer("Layer");
    file.add_frame(100);

    file.add_external_file(ExternalFile {
        id: 1,
        file_type: ExternalFileType::Palette,
        name: "shared_palette.aseprite".to_string(),
    });
    file.add_external_file(ExternalFile {
        id: 2,
        file_type: ExternalFileType::ExtensionProps,
        name: "my_extension".to_string(),
    });

    let out = write_and_read(&file);
    assert_eq!(out.external_files().len(), 2);

    assert_eq!(out.external_files()[0].id, 1);
    assert_eq!(out.external_files()[0].file_type, ExternalFileType::Palette);
    assert_eq!(out.external_files()[0].name, "shared_palette.aseprite");

    assert_eq!(out.external_files()[1].id, 2);
    assert_eq!(out.external_files()[1].file_type, ExternalFileType::ExtensionProps);
    assert_eq!(out.external_files()[1].name, "my_extension");
}

#[test]
fn legacy_masks_empty_by_default() {
    let file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    assert!(file.legacy_masks().is_empty());
}

#[test]
fn legacy_old_palette_0004_round_trip() {
    // Build a file with an old palette chunk by writing an indexed file,
    // then verify palette data survives a write-read cycle.
    let mut file = AsepriteFile::new(2, 2, ColorMode::Indexed);
    let layer = file.add_layer("Layer");
    file.add_frame(100);
    let pixels = Pixels::new(vec![0u8; 4], 2, 2, ColorMode::Indexed).unwrap();
    file.set_cel(layer, 0, pixels, 0, 0).unwrap();

    let colors: Vec<Color> = (0..4).map(|i| Color {
        r: i * 60, g: i * 40, b: i * 20, a: 255, name: None,
    }).collect();
    file.set_palette(&colors).unwrap();

    let out = write_and_read(&file);
    assert_eq!(out.palette().len(), 4);
    assert_eq!(out.palette()[0].r, 0);
    assert_eq!(out.palette()[1].r, 60);
}

#[test]
fn legacy_mask_struct_fields() {
    let mask = LegacyMask {
        x: 10, y: 20, width: 32, height: 16,
        name: "test mask".to_string(),
        bitmap: vec![0xFF; 4 * 16],
    };
    assert_eq!(mask.x, 10);
    assert_eq!(mask.y, 20);
    assert_eq!(mask.width, 32);
    assert_eq!(mask.height, 16);
    assert_eq!(mask.name, "test mask");
    assert_eq!(mask.bitmap.len(), 64);
}

// --- Error path tests ---

#[test]
fn rejects_bad_magic() {
    let data = vec![0u8; 128];
    assert!(matches!(AsepriteFile::from_reader(&data[..]), Err(AsepriteError::InvalidMagic)));
}

#[test]
fn rejects_truncated_header() {
    let data = vec![0u8; 10];
    assert!(AsepriteFile::from_reader(&data[..]).is_err());
}

#[test]
fn error_display() {
    let e = AsepriteError::InvalidMagic;
    assert!(format!("{e}").contains("magic"));
    let e = AsepriteError::FrameOutOfBounds(5);
    assert!(format!("{e}").contains("5"));
    let e = AsepriteError::PixelSizeMismatch { expected: 64, actual: 10 };
    assert!(format!("{e}").contains("64"));
    let e = AsepriteError::FormatLimitExceeded { field: "palette", value: 300, max: 256 };
    assert!(format!("{e}").contains("palette"));
}

#[test]
fn error_source() {
    use std::error::Error;
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");
    let e = AsepriteError::from(io_err);
    assert!(e.source().is_some());
    assert!(AsepriteError::InvalidMagic.source().is_none());
}

// --- Unused public API method tests ---

#[test]
fn group_ref_and_layer_ref() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let _layer = file.add_layer("L");
    let _group = file.add_group("G");

    assert!(file.layer_ref(0).is_some());
    assert!(file.layer_ref(1).is_none());
    assert!(file.group_ref(1).is_some());
    assert!(file.group_ref(0).is_none());
    assert!(file.group_ref(99).is_none());
}

#[test]
fn add_group_with_options() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let group = file.add_group_with("G", LayerOptions { opacity: 128, visible: false, ..Default::default() });
    let _child = file.add_layer_in("Child", group);
    file.add_frame(100);

    let read = write_and_read(&file);
    assert_eq!(read.layers()[0].opacity, 128);
    assert!(!read.layers()[0].visible);
    assert_eq!(read.layers()[1].parent, Some(0));
}

#[test]
fn add_layer_in_with_options() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let group = file.add_group("G");
    let _child = file.add_layer_in_with("Child", group, LayerOptions { opacity: 64, ..Default::default() });
    file.add_frame(100);

    let read = write_and_read(&file);
    assert_eq!(read.layers()[1].opacity, 64);
}

#[test]
fn add_group_in_with_options() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let outer = file.add_group("Outer");
    let inner = file.add_group_in_with("Inner", outer, LayerOptions { visible: false, ..Default::default() });
    let _leaf = file.add_layer_in("Leaf", inner);
    file.add_frame(100);

    let read = write_and_read(&file);
    assert!(!read.layers()[1].visible);
    assert_eq!(read.layers()[2].parent, Some(1));
}

#[test]
fn set_raw_cel() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let layer = file.add_layer("L");
    let f0 = file.add_frame(100);
    let pixels = Pixels::new(vec![0u8; 4 * 4 * 4], 4, 4, ColorMode::Rgba).unwrap();
    file.set_raw_cel(layer, f0, pixels, 0, 0).unwrap();

    let read = write_and_read(&file);
    let lr = read.layer_ref(0).unwrap();
    let cel = read.cel(lr, 0).unwrap();
    assert!(matches!(cel.kind, CelKind::Raw { .. }));
}

#[test]
fn set_group_user_data() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let group = file.add_group("G");
    file.set_group_user_data(group, UserData { text: Some("group note".into()), ..Default::default() });
    file.add_layer_in("Child", group);
    file.add_frame(100);

    let read = write_and_read(&file);
    assert_eq!(read.layers()[0].user_data.as_ref().unwrap().text.as_deref(), Some("group note"));
}

// --- Structural assertions on existing fixtures ---

#[test]
fn parse_tags3_structure() {
    let data = std::fs::read("tests/fixtures/tags3.aseprite").unwrap();
    let file = AsepriteFile::from_reader(&data[..]).unwrap();
    assert_eq!(file.tags().len(), 3);
    assert!(!file.tags()[0].name.is_empty());
    assert!(!file.tags()[1].name.is_empty());
    assert!(!file.tags()[2].name.is_empty());
}

#[test]
fn parse_groups3abc_structure() {
    let data = std::fs::read("tests/fixtures/groups3abc.aseprite").unwrap();
    let file = AsepriteFile::from_reader(&data[..]).unwrap();
    assert!(file.layers().len() >= 3);
    assert!(file.layers().iter().any(|l| l.kind == LayerKind::Group));
}

#[test]
fn parse_slices_structure() {
    let data = std::fs::read("tests/fixtures/slices.aseprite").unwrap();
    let file = AsepriteFile::from_reader(&data[..]).unwrap();
    assert!(!file.slices().is_empty());
    assert!(!file.slices()[0].name.is_empty());
    assert!(!file.slices()[0].keys.is_empty());
}

#[test]
fn parse_2x2tilemap_structure() {
    let data = std::fs::read("tests/fixtures/2x2tilemap2x2tile.aseprite").unwrap();
    let file = AsepriteFile::from_reader(&data[..]).unwrap();
    assert!(!file.tilesets().is_empty());
    assert!(file.layers().iter().any(|l| matches!(l.kind, LayerKind::Tilemap { .. })));
}

#[test]
fn resolve_cel_follows_link() {
    let mut file = AsepriteFile::new(2, 2, ColorMode::Rgba);
    let layer = file.add_layer("Layer");
    file.add_frame(100);
    file.add_frame(100);

    let pixels = Pixels::new(vec![42u8; 2 * 2 * 4], 2, 2, ColorMode::Rgba).unwrap();
    file.set_cel(layer, 0, pixels, 5, 10).unwrap();
    file.set_linked_cel(layer, 1, 0).unwrap();

    // resolve_cel on a normal cel returns the cel itself
    let cel0 = file.resolve_cel(layer, 0).unwrap();
    assert!(matches!(&cel0.kind, CelKind::Compressed { x: 5, y: 10, .. }));

    // resolve_cel on a linked cel returns the source cel
    let cel1 = file.resolve_cel(layer, 1).unwrap();
    assert!(matches!(&cel1.kind, CelKind::Compressed { x: 5, y: 10, .. }));

    // resolve_cel on a missing cel returns None
    assert!(file.resolve_cel(layer, 99).is_none());
}

#[test]
fn layer_ref_and_group_ref_index() {
    let mut file = AsepriteFile::new(4, 4, ColorMode::Rgba);
    let layer = file.add_layer("Layer");
    let group = file.add_group("Group");
    let nested = file.add_layer_in("Nested", group);

    assert_eq!(layer.index(), 0);
    assert_eq!(group.index(), 1);
    assert_eq!(nested.index(), 2);
}
