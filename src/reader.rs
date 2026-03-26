use std::io::Read;
use crate::error::AsepriteError;
use crate::types::*;

// --- Binary read helpers ---

fn read_byte<R: Read>(r: &mut R) -> Result<u8, AsepriteError> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_word<R: Read>(r: &mut R) -> Result<u16, AsepriteError> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_short<R: Read>(r: &mut R) -> Result<i16, AsepriteError> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

fn read_dword<R: Read>(r: &mut R) -> Result<u32, AsepriteError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_string<R: Read>(r: &mut R) -> Result<String, AsepriteError> {
    let len = read_word(r)? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    // Try zero-copy path first; only copy on invalid UTF-8
    Ok(String::from_utf8(buf).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
}

fn read_bytes<R: Read>(r: &mut R, n: usize) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

fn read_long<R: Read>(r: &mut R) -> Result<i32, AsepriteError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_long64<R: Read>(r: &mut R) -> Result<i64, AsepriteError> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(i64::from_le_bytes(buf))
}

fn read_qword<R: Read>(r: &mut R) -> Result<u64, AsepriteError> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_float<R: Read>(r: &mut R) -> Result<f32, AsepriteError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_double<R: Read>(r: &mut R) -> Result<f64, AsepriteError> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

fn skip<R: Read>(r: &mut R, n: usize) -> Result<(), AsepriteError> {
    if n == 0 { return Ok(()); }
    let mut buf = [0u8; 128];
    let mut remaining = n;
    while remaining > 0 {
        let chunk = remaining.min(128);
        r.read_exact(&mut buf[..chunk])?;
        remaining -= chunk;
    }
    Ok(())
}

// --- User Data last-entity tracking ---

#[derive(Clone, Debug)]
enum LastEntity {
    None,
    Layer(usize),
    Cel(usize, usize),
    TagSequence(usize),
    Palette,
    Slice(usize),
    Tileset(usize),
    TileSequence(usize, usize),
}

// --- Main reader ---

pub fn from_reader<R: Read>(mut reader: R) -> Result<AsepriteFile, AsepriteError> {
    // Header (128 bytes)
    let _file_size = read_dword(&mut reader)?;
    let magic = read_word(&mut reader)?;
    if magic != 0xA5E0 {
        return Err(AsepriteError::InvalidMagic);
    }
    let num_frames = read_word(&mut reader)?;
    let width = read_word(&mut reader)?;
    let height = read_word(&mut reader)?;
    let color_depth = read_word(&mut reader)?;
    let color_mode = ColorMode::from_depth(color_depth)?;
    let flags = read_dword(&mut reader)?;
    let deprecated_speed = read_word(&mut reader)?;
    skip(&mut reader, 8)?; // two reserved DWORDs
    let transparent_index = read_byte(&mut reader)?;
    skip(&mut reader, 3)?;
    let num_colors = read_word(&mut reader)?;
    let pixel_width = read_byte(&mut reader)?;
    let pixel_height = read_byte(&mut reader)?;
    let grid_x = read_short(&mut reader)?;
    let grid_y = read_short(&mut reader)?;
    let grid_w = read_word(&mut reader)?;
    let grid_h = read_word(&mut reader)?;
    skip(&mut reader, 84)?;

    let pixel_ratio = (
        if pixel_width == 0 { 1 } else { pixel_width },
        if pixel_height == 0 { 1 } else { pixel_height },
    );

    let mut file = AsepriteFile::new(width, height, color_mode);
    file.set_flags(flags);
    file.set_deprecated_speed(deprecated_speed);
    file.set_num_colors(num_colors);
    file.set_transparent_index(transparent_index);
    file.set_pixel_ratio(pixel_ratio);
    file.set_grid(GridInfo { x: grid_x, y: grid_y, width: grid_w, height: grid_h });

    let mut last_entity = LastEntity::None;

    // Frames
    for frame_index in 0..num_frames as usize {
        let _frame_bytes = read_dword(&mut reader)?;
        let frame_magic = read_word(&mut reader)?;
        if frame_magic != 0xF1FA {
            return Err(AsepriteError::InvalidMagic);
        }
        let old_num_chunks = read_word(&mut reader)?;
        let duration = read_word(&mut reader)?;
        skip(&mut reader, 2)?;
        let new_num_chunks = read_dword(&mut reader)?;

        file.add_frame(duration);

        let num_chunks = if new_num_chunks != 0 {
            new_num_chunks as usize
        } else {
            old_num_chunks as usize
        };

        for _ in 0..num_chunks {
            let chunk_size = read_dword(&mut reader)? as usize;
            let chunk_type = read_word(&mut reader)?;
            let data_size = chunk_size - 6;

            match chunk_type {
                0x2004 => {
                    read_layer_chunk(&mut reader, &mut file, data_size)?;
                    last_entity = LastEntity::Layer(file.layers().len() - 1);
                    file.push_chunk_order(frame_index, 0x2004, None);
                }
                0x2005 => {
                    let layer_idx = read_cel_chunk(&mut reader, &mut file, frame_index, color_mode, data_size)?;
                    if let Some(li) = layer_idx {
                        last_entity = LastEntity::Cel(li, frame_index);
                    }
                    file.push_chunk_order(frame_index, 0x2005, layer_idx);
                }
                0x2007 => {
                    read_color_profile_chunk(&mut reader, &mut file, data_size)?;
                    file.push_chunk_order(frame_index, 0x2007, None);
                }
                0x2018 => {
                    read_tags_chunk(&mut reader, &mut file, data_size)?;
                    last_entity = LastEntity::TagSequence(0);
                    file.push_chunk_order(frame_index, 0x2018, None);
                }
                0x2019 => {
                    read_palette_chunk(&mut reader, &mut file, data_size)?;
                    if frame_index == 0 {
                        last_entity = LastEntity::Palette;
                    }
                    file.push_chunk_order(frame_index, 0x2019, None);
                }
                0x2020 => {
                    let ud = read_user_data_chunk(&mut reader, data_size)?;
                    apply_user_data(&mut file, &mut last_entity, ud);
                    file.push_chunk_order(frame_index, 0x2020, None);
                }
                0x2022 => {
                    read_slice_chunk(&mut reader, &mut file, data_size)?;
                    last_entity = LastEntity::Slice(file.slices().len() - 1);
                    file.push_chunk_order(frame_index, 0x2022, None);
                }
                0x2006 => {
                    let extra = read_cel_extra_chunk(&mut reader, data_size)?;
                    if let LastEntity::Cel(li, fi) = &last_entity
                        && let Some(cel) = file.cel_mut(*li, *fi)
                    {
                        cel.extra = Some(extra);
                    }
                    file.push_chunk_order(frame_index, 0x2006, None);
                }
                0x2008 => {
                    read_external_files_chunk(&mut reader, &mut file, data_size)?;
                    file.push_chunk_order(frame_index, 0x2008, None);
                }
                0x2023 => {
                    read_tileset_chunk(&mut reader, &mut file, data_size)?;
                    last_entity = LastEntity::Tileset(file.tilesets().len() - 1);
                    file.push_chunk_order(frame_index, 0x2023, None);
                }
                0x0004 => {
                    // Old palette chunk — parse to populate palette, but store
                    // raw bytes as unknown for round-trip fidelity.
                    let data = read_bytes(&mut reader, data_size)?;
                    parse_old_palette_0004(&data, &mut file)?;
                    file.push_unknown_chunk(frame_index, chunk_type, data);
                    file.push_chunk_order(frame_index, chunk_type, None);
                    if frame_index == 0 {
                        last_entity = LastEntity::Palette;
                    }
                }
                0x0011 => {
                    // Old palette chunk (VGA 6-bit) — same treatment as 0x0004.
                    let data = read_bytes(&mut reader, data_size)?;
                    parse_old_palette_0011(&data, &mut file)?;
                    file.push_unknown_chunk(frame_index, chunk_type, data);
                    file.push_chunk_order(frame_index, chunk_type, None);
                    if frame_index == 0 {
                        last_entity = LastEntity::Palette;
                    }
                }
                0x2016 => {
                    // Legacy mask chunk — parse into typed data, keep raw bytes.
                    let data = read_bytes(&mut reader, data_size)?;
                    parse_mask_chunk(&data, &mut file);
                    file.push_unknown_chunk(frame_index, chunk_type, data);
                    file.push_chunk_order(frame_index, chunk_type, None);
                }
                _ => {
                    let data = read_bytes(&mut reader, data_size)?;
                    file.push_unknown_chunk(frame_index, chunk_type, data);
                    file.push_chunk_order(frame_index, chunk_type, None);
                }
            }
        }
    }

    Ok(file)
}

// --- Chunk readers ---

fn read_layer_chunk<R: Read>(r: &mut R, file: &mut AsepriteFile, data_size: usize) -> Result<(), AsepriteError> {
    let mut consumed = 0usize;

    let flags_raw = read_word(r)?; consumed += 2;
    let layer_type = read_word(r)?; consumed += 2;
    let child_level = read_word(r)?; consumed += 2;
    let _default_width = read_word(r)?; consumed += 2;
    let _default_height = read_word(r)?; consumed += 2;
    let blend_mode = BlendMode::from_u16(read_word(r)?); consumed += 2;
    let opacity = read_byte(r)?; consumed += 1;
    skip(r, 3)?; consumed += 3;
    let name = read_string(r)?; consumed += 2 + name.len();

    let tileset_index = if layer_type == 2 {
        let idx = read_dword(r)?; consumed += 4;
        Some(idx)
    } else {
        None
    };

    if consumed < data_size {
        skip(r, data_size - consumed)?;
    }

    let kind = match layer_type {
        1 => LayerKind::Group,
        2 => LayerKind::Tilemap { tileset_index: tileset_index.unwrap_or(0) },
        _ => LayerKind::Normal,
    };

    let parent = if child_level == 0 {
        None
    } else {
        find_parent_for_child_level(file, child_level)
    };

    file.push_layer_raw(Layer {
        name, kind, parent, opacity, blend_mode,
        visible: flags_raw & 1 != 0,
        editable: flags_raw & 2 != 0,
        lock_movement: flags_raw & 4 != 0,
        background: flags_raw & 8 != 0,
        prefer_linked_cels: flags_raw & 16 != 0,
        collapsed: flags_raw & 32 != 0,
        reference_layer: flags_raw & 64 != 0,
        user_data: None,
    });

    Ok(())
}

fn find_parent_for_child_level(file: &AsepriteFile, child_level: u16) -> Option<usize> {
    if child_level == 0 { return None; }
    let layers = file.layers();
    for i in (0..layers.len()).rev() {
        let layer_level = compute_child_level(layers, i);
        if layer_level == child_level - 1 && layers[i].kind == LayerKind::Group {
            return Some(i);
        }
    }
    None
}

fn compute_child_level(layers: &[Layer], index: usize) -> u16 {
    let mut level = 0u16;
    let mut current = index;
    while let Some(parent) = layers[current].parent {
        level += 1;
        current = parent;
    }
    level
}

fn read_cel_chunk<R: Read>(
    r: &mut R, file: &mut AsepriteFile, frame_index: usize,
    color_mode: ColorMode, data_size: usize,
) -> Result<Option<usize>, AsepriteError> {
    let layer_index = read_word(r)? as usize;
    let x = read_short(r)?;
    let y = read_short(r)?;
    let opacity = read_byte(r)?;
    let cel_type = read_word(r)?;
    let z_index = read_short(r)?;
    skip(r, 5)?;

    let remaining = data_size - 16; // 2+2+2+1+2+2+5 = 16

    let kind = match cel_type {
        0 => {
            let w = read_word(r)?;
            let h = read_word(r)?;
            let pixel_data_size = w as usize * h as usize * color_mode.bytes_per_pixel();
            let data = read_bytes(r, pixel_data_size)?;
            let extra = remaining - 4 - pixel_data_size;
            if extra > 0 { skip(r, extra)?; }
            CelKind::Raw { pixels: Pixels { data, width: w, height: h }, x, y }
        }
        1 => {
            let source_frame = read_word(r)? as usize;
            if remaining > 2 { skip(r, remaining - 2)?; }
            CelKind::Linked { source_frame, x, y }
        }
        2 => {
            let w = read_word(r)?;
            let h = read_word(r)?;
            let compressed_size = remaining - 4;
            let compressed = read_bytes(r, compressed_size)?;
            let mut decoder = flate2::read::ZlibDecoder::new(&compressed[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            CelKind::Compressed {
                pixels: Pixels { data: decompressed, width: w, height: h },
                x, y,
                original_compressed: Some(compressed),
            }
        }
        3 => {
            let w = read_word(r)?;
            let h = read_word(r)?;
            let bits_per_tile = read_word(r)?;
            let tile_id_bitmask = read_dword(r)?;
            let x_flip_bitmask = read_dword(r)?;
            let y_flip_bitmask = read_dword(r)?;
            let d_flip_bitmask = read_dword(r)?;
            skip(r, 10)?;

            let compressed_size = remaining - 32;
            let compressed = read_bytes(r, compressed_size)?;

            let mut decoder = flate2::read::ZlibDecoder::new(&compressed[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;

            let num_tiles = w as usize * h as usize;
            let tiles: Vec<u32> = decompressed.chunks_exact(4)
                .take(num_tiles)
                .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();

            CelKind::Tilemap {
                width: w, height: h, bits_per_tile,
                tile_id_bitmask, x_flip_bitmask, y_flip_bitmask, d_flip_bitmask,
                tiles, x, y,
                original_compressed: Some(compressed),
            }
        }
        _ => {
            skip(r, remaining)?;
            return Ok(None);
        }
    };

    file.insert_cel(layer_index, frame_index, Cel { kind, opacity, z_index, user_data: None, extra: None });
    Ok(Some(layer_index))
}

fn read_color_profile_chunk<R: Read>(r: &mut R, file: &mut AsepriteFile, data_size: usize) -> Result<(), AsepriteError> {
    let profile_type = read_word(r)?;
    let flags = read_word(r)?;
    let gamma = read_dword(r)?;
    skip(r, 8)?;

    let remaining = data_size - 16;

    let profile = match profile_type {
        1 => {
            if remaining > 0 { skip(r, remaining)?; }
            ColorProfile::SRgb { flags, gamma }
        }
        2 => {
            let icc_length = read_dword(r)? as usize;
            let icc_data = read_bytes(r, icc_length)?;
            let extra = remaining - 4 - icc_length;
            if extra > 0 { skip(r, extra)?; }
            ColorProfile::Icc { flags, gamma, data: icc_data }
        }
        _ => {
            if remaining > 0 { skip(r, remaining)?; }
            ColorProfile::None
        }
    };

    file.set_color_profile(profile);
    Ok(())
}

fn read_tags_chunk<R: Read>(r: &mut R, file: &mut AsepriteFile, data_size: usize) -> Result<(), AsepriteError> {
    let mut consumed = 0usize;
    let num_tags = read_word(r)?; consumed += 2;
    skip(r, 8)?; consumed += 8;

    for _ in 0..num_tags {
        let from_frame = read_word(r)? as usize; consumed += 2;
        let to_frame = read_word(r)? as usize; consumed += 2;
        let direction = LoopDirection::from_u8(read_byte(r)?); consumed += 1;
        let repeat = read_word(r)?; consumed += 2;
        skip(r, 6)?; consumed += 6;
        skip(r, 3)?; consumed += 3;
        skip(r, 1)?; consumed += 1;
        let name = read_string(r)?; consumed += 2 + name.len();

        file.push_tag(Tag { name, from_frame, to_frame, direction, repeat, user_data: None });
    }

    if consumed < data_size { skip(r, data_size - consumed)?; }
    Ok(())
}

fn read_palette_chunk<R: Read>(r: &mut R, file: &mut AsepriteFile, data_size: usize) -> Result<(), AsepriteError> {
    let mut consumed = 0usize;
    let new_size = read_dword(r)? as usize; consumed += 4;
    let first = read_dword(r)? as usize; consumed += 4;
    let last = read_dword(r)? as usize; consumed += 4;
    skip(r, 8)?; consumed += 8;

    let needed = new_size.max(last + 1);
    if needed > file.palette().len() {
        let mut palette = file.palette().to_vec();
        palette.resize(needed, Color { r: 0, g: 0, b: 0, a: 255, name: None });
        let _ = file.set_palette(&palette);
    }

    for i in first..=last {
        let entry_flags = read_word(r)?; consumed += 2;
        let red = read_byte(r)?; consumed += 1;
        let green = read_byte(r)?; consumed += 1;
        let blue = read_byte(r)?; consumed += 1;
        let alpha = read_byte(r)?; consumed += 1;

        let name = if entry_flags & 1 != 0 {
            let n = read_string(r)?;
            consumed += 2 + n.len();
            Some(n)
        } else {
            None
        };

        file.set_palette_entry(i, Color { r: red, g: green, b: blue, a: alpha, name });
    }

    if consumed < data_size { skip(r, data_size - consumed)?; }
    Ok(())
}

fn apply_user_data(file: &mut AsepriteFile, last_entity: &mut LastEntity, ud: UserData) {
    match last_entity {
        LastEntity::Layer(idx) => {
            if let Some(layer) = file.layers_mut().get_mut(*idx) {
                layer.user_data = Some(ud);
            }
        }
        LastEntity::Cel(layer_idx, frame_idx) => {
            if let Some(cel) = file.cel_mut(*layer_idx, *frame_idx) {
                cel.user_data = Some(ud);
            }
        }
        LastEntity::TagSequence(next_tag) => {
            let tag_idx = *next_tag;
            if let Some(tag) = file.tags_mut().get_mut(tag_idx) {
                tag.user_data = Some(ud);
            }
            *next_tag = tag_idx + 1;
        }
        LastEntity::Palette => {
            file.set_sprite_user_data_raw(ud);
        }
        LastEntity::Slice(idx) => {
            if let Some(slice) = file.slices_mut().get_mut(*idx) {
                slice.user_data = Some(ud);
            }
        }
        LastEntity::Tileset(idx) => {
            let ts_idx = *idx;
            if let Some(ts) = file.tilesets_mut().get_mut(ts_idx) {
                ts.user_data = Some(ud);
            }
            *last_entity = LastEntity::TileSequence(ts_idx, 0);
        }
        LastEntity::TileSequence(ts_idx, tile_idx) => {
            let ti = *tile_idx;
            let tsi = *ts_idx;
            if let Some(ts) = file.tilesets_mut().get_mut(tsi)
                && ti < ts.tile_user_data.len()
            {
                ts.tile_user_data[ti] = Some(ud);
            }
            *last_entity = LastEntity::TileSequence(tsi, ti + 1);
        }
        LastEntity::None => {}
    }
}

fn read_user_data_chunk<R: Read>(r: &mut R, data_size: usize) -> Result<UserData, AsepriteError> {
    let mut consumed = 0usize;
    let flags = read_dword(r)?; consumed += 4;

    let text = if flags & 1 != 0 {
        let s = read_string(r)?;
        consumed += 2 + s.len();
        Some(s)
    } else {
        None
    };

    let color = if flags & 2 != 0 {
        let cr = read_byte(r)?;
        let cg = read_byte(r)?;
        let cb = read_byte(r)?;
        let ca = read_byte(r)?;
        consumed += 4;
        Some(Color { r: cr, g: cg, b: cb, a: ca, name: None })
    } else {
        None
    };

    let properties = if flags & 4 != 0 {
        let prop_size = read_dword(r)? as usize; consumed += 4;
        let num_maps = read_dword(r)? as usize; consumed += 4;
        let mut maps = Vec::with_capacity(num_maps);
        let mut prop_consumed = 8usize; // the size and num_maps DWORDs count toward prop_size
        for _ in 0..num_maps {
            let (map, bytes) = read_properties_map(r)?;
            prop_consumed += bytes;
            maps.push(map);
        }
        // Skip any remaining bytes in the properties block
        if prop_consumed < prop_size {
            skip(r, prop_size - prop_consumed)?;
        }
        consumed += prop_size - 8; // we already counted the 8 bytes for size+num_maps
        maps
    } else {
        Vec::new()
    };

    if consumed < data_size {
        skip(r, data_size - consumed)?;
    }

    Ok(UserData { text, color, properties })
}

fn read_properties_map<R: Read>(r: &mut R) -> Result<(PropertiesMap, usize), AsepriteError> {
    let mut consumed = 0usize;
    let key = read_dword(r)?; consumed += 4;
    let num_entries = read_dword(r)? as usize; consumed += 4;
    let mut entries = Vec::with_capacity(num_entries);
    for _ in 0..num_entries {
        let name = read_string(r)?; consumed += 2 + name.len();
        let type_id = read_word(r)?; consumed += 2;
        let (value, bytes) = read_property_value(r, type_id)?;
        consumed += bytes;
        entries.push((name, value));
    }
    Ok((PropertiesMap { key, entries }, consumed))
}

fn read_property_value<R: Read>(r: &mut R, type_id: u16) -> Result<(PropertyValue, usize), AsepriteError> {
    match type_id {
        0x0001 => { // Bool
            let v = read_byte(r)?;
            Ok((PropertyValue::Bool(v != 0), 1))
        }
        0x0002 => { // Int8
            let v = read_byte(r)? as i8;
            Ok((PropertyValue::Int8(v), 1))
        }
        0x0003 => { // UInt8
            let v = read_byte(r)?;
            Ok((PropertyValue::UInt8(v), 1))
        }
        0x0004 => { // Int16
            let v = read_short(r)?;
            Ok((PropertyValue::Int16(v), 2))
        }
        0x0005 => { // UInt16
            let v = read_word(r)?;
            Ok((PropertyValue::UInt16(v), 2))
        }
        0x0006 => { // Int32
            let v = read_long(r)?;
            Ok((PropertyValue::Int32(v), 4))
        }
        0x0007 => { // UInt32
            let v = read_dword(r)?;
            Ok((PropertyValue::UInt32(v), 4))
        }
        0x0008 => { // Int64
            let v = read_long64(r)?;
            Ok((PropertyValue::Int64(v), 8))
        }
        0x0009 => { // UInt64
            let v = read_qword(r)?;
            Ok((PropertyValue::UInt64(v), 8))
        }
        0x000A => { // Fixed (16.16)
            let v = read_dword(r)?;
            Ok((PropertyValue::Fixed(v), 4))
        }
        0x000B => { // Float
            let v = read_float(r)?;
            Ok((PropertyValue::Float(v), 4))
        }
        0x000C => { // Double
            let v = read_double(r)?;
            Ok((PropertyValue::Double(v), 8))
        }
        0x000D => { // String
            let s = read_string(r)?;
            let bytes = 2 + s.len();
            Ok((PropertyValue::String(s), bytes))
        }
        0x000E => { // Point
            let x = read_long(r)?;
            let y = read_long(r)?;
            Ok((PropertyValue::Point(x, y), 8))
        }
        0x000F => { // Size
            let w = read_long(r)?;
            let h = read_long(r)?;
            Ok((PropertyValue::Size(w, h), 8))
        }
        0x0010 => { // Rect
            let x = read_long(r)?;
            let y = read_long(r)?;
            let w = read_long(r)?;
            let h = read_long(r)?;
            Ok((PropertyValue::Rect(x, y, w, h), 16))
        }
        0x0011 => { // Vector
            let mut consumed = 0usize;
            let count = read_dword(r)? as usize; consumed += 4;
            let elem_type = read_word(r)?; consumed += 2;
            let mut elements = Vec::with_capacity(count);
            for _ in 0..count {
                let (val, bytes) = if elem_type == 0 {
                    // Heterogeneous: each element has its own type prefix
                    let et = read_word(r)?;
                    let (v, b) = read_property_value(r, et)?;
                    (v, b + 2) // +2 for the type prefix
                } else {
                    // Homogeneous: all elements share the same type
                    read_property_value(r, elem_type)?
                };
                consumed += bytes;
                elements.push(val);
            }
            Ok((PropertyValue::Vector(elements), consumed))
        }
        0x0012 => { // Properties (nested map)
            let mut consumed = 0usize;
            let num_entries = read_dword(r)? as usize; consumed += 4;
            let mut entries = Vec::with_capacity(num_entries);
            for _ in 0..num_entries {
                let name = read_string(r)?; consumed += 2 + name.len();
                let tid = read_word(r)?; consumed += 2;
                let (value, bytes) = read_property_value(r, tid)?;
                consumed += bytes;
                entries.push((name, value));
            }
            Ok((PropertyValue::Properties(entries), consumed))
        }
        0x0013 => { // UUID
            let mut uuid = [0u8; 16];
            r.read_exact(&mut uuid)?;
            Ok((PropertyValue::Uuid(uuid), 16))
        }
        _ => Err(AsepriteError::UnsupportedChunkType(type_id)),
    }
}

fn read_slice_chunk<R: Read>(r: &mut R, file: &mut AsepriteFile, data_size: usize) -> Result<(), AsepriteError> {
    let mut consumed = 0usize;
    let num_keys = read_dword(r)?; consumed += 4;
    let flags = read_dword(r)?; consumed += 4;
    let _reserved = read_dword(r)?; consumed += 4;
    let name = read_string(r)?; consumed += 2 + name.len();

    let has_nine_patch = flags & 1 != 0;
    let has_pivot = flags & 2 != 0;

    let mut keys = Vec::new();
    for _ in 0..num_keys {
        let frame = read_dword(r)?; consumed += 4;
        let x = read_long(r)?; consumed += 4;
        let y = read_long(r)?; consumed += 4;
        let width = read_dword(r)?; consumed += 4;
        let height = read_dword(r)?; consumed += 4;

        let nine_patch = if has_nine_patch {
            let cx = read_long(r)?; consumed += 4;
            let cy = read_long(r)?; consumed += 4;
            let cw = read_dword(r)?; consumed += 4;
            let ch = read_dword(r)?; consumed += 4;
            Some(NinePatch { center_x: cx, center_y: cy, center_width: cw, center_height: ch })
        } else { None };

        let pivot = if has_pivot {
            let px = read_long(r)?; consumed += 4;
            let py = read_long(r)?; consumed += 4;
            Some((px, py))
        } else { None };

        keys.push(SliceKey { frame, x, y, width, height, nine_patch, pivot });
    }

    if consumed < data_size { skip(r, data_size - consumed)?; }
    file.push_slice(Slice { name, keys, has_nine_patch, has_pivot, user_data: None });
    Ok(())
}

fn read_cel_extra_chunk<R: Read>(r: &mut R, data_size: usize) -> Result<CelExtra, AsepriteError> {
    let mut consumed = 0usize;
    let _flags = read_dword(r)?; consumed += 4;
    let precise_x = read_dword(r)?; consumed += 4;
    let precise_y = read_dword(r)?; consumed += 4;
    let width = read_dword(r)?; consumed += 4;
    let height = read_dword(r)?; consumed += 4;
    skip(r, 16)?; consumed += 16;
    if consumed < data_size { skip(r, data_size - consumed)?; }
    Ok(CelExtra { precise_x, precise_y, width, height })
}

fn read_external_files_chunk<R: Read>(r: &mut R, file: &mut AsepriteFile, data_size: usize) -> Result<(), AsepriteError> {
    let mut consumed = 0usize;
    let num_entries = read_dword(r)?; consumed += 4;
    skip(r, 8)?; consumed += 8;
    for _ in 0..num_entries {
        let id = read_dword(r)?; consumed += 4;
        let file_type = ExternalFileType::from_u8(read_byte(r)?); consumed += 1;
        skip(r, 7)?; consumed += 7;
        let name = read_string(r)?; consumed += 2 + name.len();
        file.push_external_file(ExternalFile { id, file_type, name });
    }
    if consumed < data_size { skip(r, data_size - consumed)?; }
    Ok(())
}

fn read_tileset_chunk<R: Read>(r: &mut R, file: &mut AsepriteFile, data_size: usize) -> Result<(), AsepriteError> {
    let mut consumed = 0usize;

    let id = read_dword(r)?; consumed += 4;
    let flags_raw = read_dword(r)?; consumed += 4;
    let flags = TilesetFlags(flags_raw);
    let tile_count = read_dword(r)?; consumed += 4;
    let tile_width = read_word(r)?; consumed += 2;
    let tile_height = read_word(r)?; consumed += 2;
    let base_index = read_short(r)?; consumed += 2;
    skip(r, 14)?; consumed += 14;
    let name = read_string(r)?; consumed += 2 + name.len();

    let mut data = TilesetData::Empty;

    if flags.has_external_link() {
        let external_file_id = read_dword(r)?; consumed += 4;
        let tileset_id_in_external = read_dword(r)?; consumed += 4;
        data = TilesetData::External { external_file_id, tileset_id_in_external };
    }

    if flags.has_embedded_tiles() {
        let compressed_len = read_dword(r)? as usize; consumed += 4;
        let compressed = read_bytes(r, compressed_len)?; consumed += compressed_len;
        let mut decoder = flate2::read::ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        data = TilesetData::Embedded {
            pixels: decompressed,
            original_compressed: Some(compressed),
        };
    }

    if consumed < data_size { skip(r, data_size - consumed)?; }

    file.push_tileset(Tileset {
        id, flags, name, tile_count, tile_width, tile_height, base_index, data,
        user_data: None,
        tile_user_data: vec![None; tile_count as usize],
    });

    Ok(())
}

fn parse_old_palette_0004(data: &[u8], file: &mut AsepriteFile) -> Result<(), AsepriteError> {
    let mut r = data;
    let num_packets = read_word(&mut r)? as usize;
    let mut index = 0usize;
    for _ in 0..num_packets {
        let skip_count = read_byte(&mut r)? as usize;
        index += skip_count;
        let raw_count = read_byte(&mut r)? as usize;
        let count = if raw_count == 0 { 256 } else { raw_count };
        for _ in 0..count {
            let red = read_byte(&mut r)?;
            let green = read_byte(&mut r)?;
            let blue = read_byte(&mut r)?;
            file.set_palette_entry(index, Color { r: red, g: green, b: blue, a: 255, name: None });
            index += 1;
        }
    }
    Ok(())
}

fn parse_old_palette_0011(data: &[u8], file: &mut AsepriteFile) -> Result<(), AsepriteError> {
    let mut r = data;
    let num_packets = read_word(&mut r)? as usize;
    let mut index = 0usize;
    for _ in 0..num_packets {
        let skip_count = read_byte(&mut r)? as usize;
        index += skip_count;
        let raw_count = read_byte(&mut r)? as usize;
        let count = if raw_count == 0 { 256 } else { raw_count };
        for _ in 0..count {
            // VGA 6-bit range (0-63): scale to 0-255 via (v << 2) | (v >> 4)
            let r6 = read_byte(&mut r)?;
            let g6 = read_byte(&mut r)?;
            let b6 = read_byte(&mut r)?;
            let red = (r6 << 2) | (r6 >> 4);
            let green = (g6 << 2) | (g6 >> 4);
            let blue = (b6 << 2) | (b6 >> 4);
            file.set_palette_entry(index, Color { r: red, g: green, b: blue, a: 255, name: None });
            index += 1;
        }
    }
    Ok(())
}

fn parse_mask_chunk(data: &[u8], file: &mut AsepriteFile) {
    if data.len() < 16 { return; }
    let x = i16::from_le_bytes([data[0], data[1]]);
    let y = i16::from_le_bytes([data[2], data[3]]);
    let width = u16::from_le_bytes([data[4], data[5]]);
    let height = u16::from_le_bytes([data[6], data[7]]);
    // data[8..16] reserved

    let mut pos = 16;
    if pos + 2 > data.len() { return; }
    let name_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    let name = if pos + name_len <= data.len() {
        String::from_utf8_lossy(&data[pos..pos + name_len]).into_owned()
    } else {
        return;
    };
    pos += name_len;

    let bitmap = data[pos..].to_vec();

    file.push_legacy_mask(LegacyMask { x, y, width, height, name, bitmap });
}
