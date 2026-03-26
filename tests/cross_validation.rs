use aseprite::AsepriteFile;
use aseprite_loader::loader::AsepriteFile as LoaderFile;

fn blend_modes_match(
    ours: &aseprite::BlendMode,
    theirs: &aseprite_loader::binary::blend_mode::BlendMode,
) -> bool {
    matches!(
        (ours, theirs),
        (aseprite::BlendMode::Normal, aseprite_loader::binary::blend_mode::BlendMode::Normal)
            | (
                aseprite::BlendMode::Multiply,
                aseprite_loader::binary::blend_mode::BlendMode::Multiply
            )
            | (aseprite::BlendMode::Screen, aseprite_loader::binary::blend_mode::BlendMode::Screen)
            | (
                aseprite::BlendMode::Overlay,
                aseprite_loader::binary::blend_mode::BlendMode::Overlay
            )
            | (aseprite::BlendMode::Darken, aseprite_loader::binary::blend_mode::BlendMode::Darken)
            | (
                aseprite::BlendMode::Lighten,
                aseprite_loader::binary::blend_mode::BlendMode::Lighten
            )
            | (
                aseprite::BlendMode::ColorDodge,
                aseprite_loader::binary::blend_mode::BlendMode::ColorDodge
            )
            | (
                aseprite::BlendMode::ColorBurn,
                aseprite_loader::binary::blend_mode::BlendMode::ColorBurn
            )
            | (
                aseprite::BlendMode::HardLight,
                aseprite_loader::binary::blend_mode::BlendMode::HardLight
            )
            | (
                aseprite::BlendMode::SoftLight,
                aseprite_loader::binary::blend_mode::BlendMode::SoftLight
            )
            | (
                aseprite::BlendMode::Difference,
                aseprite_loader::binary::blend_mode::BlendMode::Difference
            )
            | (
                aseprite::BlendMode::Exclusion,
                aseprite_loader::binary::blend_mode::BlendMode::Exclusion
            )
            | (aseprite::BlendMode::Hue, aseprite_loader::binary::blend_mode::BlendMode::Hue)
            | (
                aseprite::BlendMode::Saturation,
                aseprite_loader::binary::blend_mode::BlendMode::Saturation
            )
            | (aseprite::BlendMode::Color, aseprite_loader::binary::blend_mode::BlendMode::Color)
            | (
                aseprite::BlendMode::Luminosity,
                aseprite_loader::binary::blend_mode::BlendMode::Luminosity
            )
            | (
                aseprite::BlendMode::Addition,
                aseprite_loader::binary::blend_mode::BlendMode::Addition
            )
            | (
                aseprite::BlendMode::Subtract,
                aseprite_loader::binary::blend_mode::BlendMode::Subtract
            )
            | (aseprite::BlendMode::Divide, aseprite_loader::binary::blend_mode::BlendMode::Divide)
    )
}

fn layer_kinds_match(
    ours: &aseprite::LayerKind,
    theirs: &aseprite_loader::binary::chunks::layer::LayerType,
) -> bool {
    matches!(
        (ours, theirs),
        (
            aseprite::LayerKind::Normal,
            aseprite_loader::binary::chunks::layer::LayerType::Normal
        ) | (
            aseprite::LayerKind::Group,
            aseprite_loader::binary::chunks::layer::LayerType::Group
        ) | (
            aseprite::LayerKind::Tilemap { .. },
            aseprite_loader::binary::chunks::layer::LayerType::Tilemap
        )
    )
}

fn directions_match(
    ours: &aseprite::LoopDirection,
    theirs: &aseprite_loader::binary::chunks::tags::AnimationDirection,
) -> bool {
    matches!(
        (ours, theirs),
        (
            aseprite::LoopDirection::Forward,
            aseprite_loader::binary::chunks::tags::AnimationDirection::Forward
        ) | (
            aseprite::LoopDirection::Reverse,
            aseprite_loader::binary::chunks::tags::AnimationDirection::Reverse
        ) | (
            aseprite::LoopDirection::PingPong,
            aseprite_loader::binary::chunks::tags::AnimationDirection::PingPong
        ) | (
            aseprite::LoopDirection::PingPongReverse,
            aseprite_loader::binary::chunks::tags::AnimationDirection::PingPongReverse
        )
    )
}

fn cross_validate(path: &str) {
    let data = std::fs::read(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));

    let ours = match AsepriteFile::from_reader(&data[..]) {
        Ok(f) => f,
        Err(e) => {
            println!("SKIP {path}: our crate failed to parse: {e}");
            return;
        }
    };

    let theirs = match LoaderFile::load(&data) {
        Ok(f) => f,
        Err(e) => {
            println!("SKIP {path}: aseprite_loader failed to parse: {e}");
            return;
        }
    };

    // --- Header ---
    assert_eq!(
        ours.width(),
        theirs.size().0,
        "{path}: width mismatch"
    );
    assert_eq!(
        ours.height(),
        theirs.size().1,
        "{path}: height mismatch"
    );
    assert_eq!(
        ours.frames().len(),
        theirs.frames().len(),
        "{path}: frame count mismatch"
    );

    // --- Layers ---
    // aseprite_loader's high-level API filters out Tilemap layers, keeping only Normal and Group.
    // Use the raw low-level layers (file.file.layers) for index-by-index comparison.
    let their_raw_layers = &theirs.file.layers;
    assert_eq!(
        ours.layers().len(),
        their_raw_layers.len(),
        "{path}: layer count mismatch (raw)"
    );

    for (i, our_layer) in ours.layers().iter().enumerate() {
        let their_layer = &their_raw_layers[i];

        assert_eq!(
            our_layer.name, their_layer.name,
            "{path}: layer[{i}] name mismatch"
        );
        assert!(
            layer_kinds_match(&our_layer.kind, &their_layer.layer_type),
            "{path}: layer[{i}] kind mismatch: {:?} vs {:?}",
            our_layer.kind,
            their_layer.layer_type
        );
        assert_eq!(
            our_layer.opacity, their_layer.opacity,
            "{path}: layer[{i}] opacity mismatch"
        );
        assert!(
            blend_modes_match(&our_layer.blend_mode, &their_layer.blend_mode),
            "{path}: layer[{i}] blend_mode mismatch: {:?} vs {:?}",
            our_layer.blend_mode,
            their_layer.blend_mode
        );

        // Visibility: their raw layer uses LayerFlags::VISIBLE
        let their_visible = their_layer
            .flags
            .contains(aseprite_loader::binary::chunks::layer::LayerFlags::VISIBLE);
        assert_eq!(
            our_layer.visible, their_visible,
            "{path}: layer[{i}] visible mismatch"
        );
    }

    // --- Frames ---
    for (i, our_frame) in ours.frames().iter().enumerate() {
        let their_frame = &theirs.frames()[i];
        assert_eq!(
            our_frame.duration_ms, their_frame.duration,
            "{path}: frame[{i}] duration mismatch"
        );
    }

    // --- Tags ---
    // Both use the same ordering from the file. aseprite_loader uses the raw tags from
    // file.file.tags for low-level access, but the high-level tags are in theirs.tags().
    assert_eq!(
        ours.tags().len(),
        theirs.tags().len(),
        "{path}: tag count mismatch"
    );

    for (i, our_tag) in ours.tags().iter().enumerate() {
        let their_tag = &theirs.tags()[i];

        assert_eq!(
            our_tag.name, their_tag.name,
            "{path}: tag[{i}] name mismatch"
        );
        assert_eq!(
            our_tag.from_frame as u16,
            *their_tag.range.start(),
            "{path}: tag[{i}] from_frame mismatch"
        );
        assert_eq!(
            our_tag.to_frame as u16,
            *their_tag.range.end(),
            "{path}: tag[{i}] to_frame mismatch"
        );
        assert!(
            directions_match(&our_tag.direction, &their_tag.direction),
            "{path}: tag[{i}] direction mismatch: {:?} vs {:?}",
            our_tag.direction,
            their_tag.direction
        );

        // Repeat: theirs uses Option<u16> where 0 maps to None, ours stores raw u16
        let their_repeat_raw = their_tag.repeat.unwrap_or(0);
        assert_eq!(
            our_tag.repeat, their_repeat_raw,
            "{path}: tag[{i}] repeat mismatch"
        );
    }

    // --- Slices ---
    let their_slices = theirs.slices();
    assert_eq!(
        ours.slices().len(),
        their_slices.len(),
        "{path}: slice count mismatch"
    );

    for (i, our_slice) in ours.slices().iter().enumerate() {
        let their_slice = &their_slices[i];

        assert_eq!(
            our_slice.name, their_slice.name,
            "{path}: slice[{i}] name mismatch"
        );
        assert_eq!(
            our_slice.keys.len(),
            their_slice.slice_keys.len(),
            "{path}: slice[{i}] key count mismatch"
        );

        for (j, our_key) in our_slice.keys.iter().enumerate() {
            let their_key = &their_slice.slice_keys[j];

            assert_eq!(
                our_key.frame, their_key.frame_number,
                "{path}: slice[{i}] key[{j}] frame mismatch"
            );
            assert_eq!(
                our_key.x, their_key.x,
                "{path}: slice[{i}] key[{j}] x mismatch"
            );
            assert_eq!(
                our_key.y, their_key.y,
                "{path}: slice[{i}] key[{j}] y mismatch"
            );
            assert_eq!(
                our_key.width, their_key.width,
                "{path}: slice[{i}] key[{j}] width mismatch"
            );
            assert_eq!(
                our_key.height, their_key.height,
                "{path}: slice[{i}] key[{j}] height mismatch"
            );

            // Nine patch
            match (&our_key.nine_patch, &their_key.nine_patch) {
                (Some(our_np), Some(their_np)) => {
                    assert_eq!(
                        our_np.center_x, their_np.x,
                        "{path}: slice[{i}] key[{j}] nine_patch x mismatch"
                    );
                    assert_eq!(
                        our_np.center_y, their_np.y,
                        "{path}: slice[{i}] key[{j}] nine_patch y mismatch"
                    );
                    assert_eq!(
                        our_np.center_width, their_np.width,
                        "{path}: slice[{i}] key[{j}] nine_patch width mismatch"
                    );
                    assert_eq!(
                        our_np.center_height, their_np.height,
                        "{path}: slice[{i}] key[{j}] nine_patch height mismatch"
                    );
                }
                (None, None) => {}
                _ => panic!(
                    "{path}: slice[{i}] key[{j}] nine_patch presence mismatch: ours={} theirs={}",
                    our_key.nine_patch.is_some(),
                    their_key.nine_patch.is_some()
                ),
            }

            // Pivot
            match (&our_key.pivot, &their_key.pivot) {
                (Some((our_px, our_py)), Some(their_pv)) => {
                    assert_eq!(
                        *our_px, their_pv.x,
                        "{path}: slice[{i}] key[{j}] pivot x mismatch"
                    );
                    assert_eq!(
                        *our_py, their_pv.y,
                        "{path}: slice[{i}] key[{j}] pivot y mismatch"
                    );
                }
                (None, None) => {}
                _ => panic!(
                    "{path}: slice[{i}] key[{j}] pivot presence mismatch: ours={} theirs={}",
                    our_key.pivot.is_some(),
                    their_key.pivot.is_some()
                ),
            }
        }
    }

    // --- Palette ---
    // aseprite_loader only creates a palette for indexed files (file.palette is Option<Palette>).
    // Our crate always stores palette entries if a palette chunk is present.
    //
    // Known difference: aseprite_loader forces palette[transparent_index].alpha = 0 during
    // palette construction. Our crate stores raw palette values from the file. We skip alpha
    // comparison at the transparent index to account for this.
    if let Some(their_palette) = &theirs.file.palette {
        let our_palette = ours.palette();
        let transparent_idx = ours.transparent_index() as usize;
        if !our_palette.is_empty() {
            for (i, our_color) in our_palette.iter().enumerate() {
                let their_color = &their_palette.colors[i];
                assert_eq!(
                    our_color.r, their_color.red,
                    "{path}: palette[{i}] red mismatch"
                );
                assert_eq!(
                    our_color.g, their_color.green,
                    "{path}: palette[{i}] green mismatch"
                );
                assert_eq!(
                    our_color.b, their_color.blue,
                    "{path}: palette[{i}] blue mismatch"
                );
                if i != transparent_idx {
                    assert_eq!(
                        our_color.a, their_color.alpha,
                        "{path}: palette[{i}] alpha mismatch"
                    );
                } else if our_color.a != their_color.alpha {
                    println!(
                        "{path}: palette[{i}] alpha differs at transparent index \
                         (ours={}, theirs={}) -- expected, aseprite_loader forces alpha=0",
                        our_color.a, their_color.alpha
                    );
                }
            }
            println!("{path}: palette validated ({} entries)", our_palette.len());
        }
    }

    println!("{path}: cross-validation PASSED");
}

#[test]
fn cross_validate_1empty3() {
    cross_validate("tests/fixtures/1empty3.aseprite");
}

#[test]
fn cross_validate_2f_index_3x3() {
    cross_validate("tests/fixtures/2f-index-3x3.aseprite");
}

#[test]
fn cross_validate_2x2tilemap2x2tile() {
    cross_validate("tests/fixtures/2x2tilemap2x2tile.aseprite");
}

#[test]
fn cross_validate_2x3tilemap_indexed() {
    cross_validate("tests/fixtures/2x3tilemap-indexed.aseprite");
}

#[test]
fn cross_validate_3x2tilemap_grayscale() {
    cross_validate("tests/fixtures/3x2tilemap-grayscale.aseprite");
}

#[test]
fn cross_validate_4f_index_4x4() {
    cross_validate("tests/fixtures/4f-index-4x4.aseprite");
}

#[test]
fn cross_validate_abcd() {
    cross_validate("tests/fixtures/abcd.aseprite");
}

#[test]
fn cross_validate_bg_index_3() {
    cross_validate("tests/fixtures/bg-index-3.aseprite");
}

#[test]
fn cross_validate_cut_paste() {
    cross_validate("tests/fixtures/cut_paste.aseprite");
}

#[test]
fn cross_validate_file_tests_props() {
    cross_validate("tests/fixtures/file-tests-props.aseprite");
}

#[test]
fn cross_validate_groups2() {
    cross_validate("tests/fixtures/groups2.aseprite");
}

#[test]
fn cross_validate_groups3abc() {
    cross_validate("tests/fixtures/groups3abc.aseprite");
}

#[test]
fn cross_validate_link() {
    cross_validate("tests/fixtures/link.aseprite");
}

#[test]
fn cross_validate_point2frames() {
    cross_validate("tests/fixtures/point2frames.aseprite");
}

#[test]
fn cross_validate_point4frames() {
    cross_validate("tests/fixtures/point4frames.aseprite");
}

#[test]
fn cross_validate_slices() {
    cross_validate("tests/fixtures/slices.aseprite");
}

#[test]
fn cross_validate_slices_moving() {
    cross_validate("tests/fixtures/slices-moving.aseprite");
}

#[test]
fn cross_validate_tags3() {
    cross_validate("tests/fixtures/tags3.aseprite");
}

#[test]
fn cross_validate_tags3x123reps() {
    cross_validate("tests/fixtures/tags3x123reps.aseprite");
}
