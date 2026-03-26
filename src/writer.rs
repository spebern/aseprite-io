use std::io::Write;
use crate::error::AsepriteError;
use crate::types::*;

// --- Binary write helpers ---

fn write_byte<W: Write>(w: &mut W, v: u8) -> Result<(), AsepriteError> {
    w.write_all(&[v])?;
    Ok(())
}

fn write_word<W: Write>(w: &mut W, v: u16) -> Result<(), AsepriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_short<W: Write>(w: &mut W, v: i16) -> Result<(), AsepriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_dword<W: Write>(w: &mut W, v: u32) -> Result<(), AsepriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_string<W: Write>(w: &mut W, s: &str) -> Result<(), AsepriteError> {
    write_word(w, s.len() as u16)?;
    w.write_all(s.as_bytes())?;
    Ok(())
}

fn write_long<W: Write>(w: &mut W, v: i32) -> Result<(), AsepriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_long64<W: Write>(w: &mut W, v: i64) -> Result<(), AsepriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_qword<W: Write>(w: &mut W, v: u64) -> Result<(), AsepriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_float<W: Write>(w: &mut W, v: f32) -> Result<(), AsepriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_double<W: Write>(w: &mut W, v: f64) -> Result<(), AsepriteError> {
    w.write_all(&v.to_le_bytes())?;
    Ok(())
}

fn write_zeros<W: Write>(w: &mut W, n: usize) -> Result<(), AsepriteError> {
    let buf = [0u8; 128];
    let mut remaining = n;
    while remaining > 0 {
        let chunk = remaining.min(128);
        w.write_all(&buf[..chunk])?;
        remaining -= chunk;
    }
    Ok(())
}

fn wrap_chunk(chunk_type: u16, data: &[u8]) -> Vec<u8> {
    let size = (data.len() + 6) as u32;
    let mut buf = Vec::with_capacity(size as usize);
    buf.extend_from_slice(&size.to_le_bytes());
    buf.extend_from_slice(&chunk_type.to_le_bytes());
    buf.extend_from_slice(data);
    buf
}

// --- Writer last-entity tracking for recorded-order path ---

enum WriterLastEntity {
    None,
    Layer(usize),
    Cel(usize, usize),
    TagSequence(usize),
    Palette,
    Slice(usize),
    Tileset(usize),
    TileSequence(usize, usize),
}

fn writer_resolve_user_data(file: &AsepriteFile, last: &mut WriterLastEntity) -> UserData {
    match last {
        WriterLastEntity::Layer(idx) => {
            file.layers().get(*idx)
                .and_then(|l| l.user_data.clone())
                .unwrap_or_default()
        }
        WriterLastEntity::Cel(layer_idx, frame_idx) => {
            file.cels_iter()
                .find(|&(&(l, f), _)| l == *layer_idx && f == *frame_idx)
                .and_then(|(_, c)| c.user_data.clone())
                .unwrap_or_default()
        }
        WriterLastEntity::TagSequence(next_tag) => {
            let tag_idx = *next_tag;
            *next_tag = tag_idx + 1;
            file.tags().get(tag_idx)
                .and_then(|t| t.user_data.clone())
                .unwrap_or_default()
        }
        WriterLastEntity::Palette => {
            file.sprite_user_data().clone().unwrap_or_default()
        }
        WriterLastEntity::Slice(idx) => {
            file.slices().get(*idx)
                .and_then(|s| s.user_data.clone())
                .unwrap_or_default()
        }
        WriterLastEntity::Tileset(idx) => {
            let ts_idx = *idx;
            let ud = file.tilesets().get(ts_idx)
                .and_then(|ts| ts.user_data.clone())
                .unwrap_or_default();
            *last = WriterLastEntity::TileSequence(ts_idx, 0);
            ud
        }
        WriterLastEntity::TileSequence(ts_idx, tile_idx) => {
            let ti = *tile_idx;
            let tsi = *ts_idx;
            let ud = file.tilesets().get(tsi)
                .and_then(|ts| ts.tile_user_data.get(ti))
                .and_then(|u| u.clone())
                .unwrap_or_default();
            *last = WriterLastEntity::TileSequence(tsi, ti + 1);
            ud
        }
        WriterLastEntity::None => UserData::default(),
    }
}

// --- Main writer ---

pub fn write_to<W: Write>(file: &AsepriteFile, mut writer: W) -> Result<(), AsepriteError> {
    if file.frames().len() > u16::MAX as usize {
        return Err(AsepriteError::FormatLimitExceeded { field: "frames", value: file.frames().len(), max: u16::MAX as usize });
    }
    if file.layers().len() > u16::MAX as usize {
        return Err(AsepriteError::FormatLimitExceeded { field: "layers", value: file.layers().len(), max: u16::MAX as usize });
    }
    if file.color_mode() == ColorMode::Indexed && file.palette().is_empty() {
        return Err(AsepriteError::MissingPalette);
    }

    let mut frame_buffers: Vec<Vec<u8>> = Vec::new();

    for frame_index in 0..file.frames().len() {
        let mut chunks: Vec<Vec<u8>> = Vec::new();

        let chunk_order: Vec<ChunkOrderEntry> = file.chunk_order_for_frame(frame_index).cloned().collect();

        if chunk_order.is_empty() {
            // Programmatically created file -- use default order
            if frame_index == 0 {
                if !file.external_files().is_empty() {
                    chunks.push(wrap_chunk(0x2008, &encode_external_files_chunk(file.external_files())?));
                }
                for tileset in file.tilesets() {
                    chunks.push(wrap_chunk(0x2023, &encode_tileset_chunk(tileset)?));
                    let has_tile_ud = tileset.tile_user_data.iter().any(|ud| ud.is_some());
                    if tileset.user_data.is_some() || has_tile_ud {
                        let ud = tileset.user_data.as_ref().cloned().unwrap_or_default();
                        chunks.push(wrap_chunk(0x2020, &encode_user_data_chunk(&ud)?));
                    }
                    if has_tile_ud {
                        for i in 0..tileset.tile_count as usize {
                            let ud = tileset.tile_user_data.get(i).and_then(|u| u.as_ref()).cloned().unwrap_or_default();
                            chunks.push(wrap_chunk(0x2020, &encode_user_data_chunk(&ud)?));
                        }
                    }
                }
                for layer in file.layers() {
                    chunks.push(wrap_chunk(0x2004, &encode_layer_chunk(file, layer)?));
                    emit_user_data_chunk(&mut chunks, &layer.user_data)?;
                }
                if let Some(profile) = file.color_profile() {
                    chunks.push(wrap_chunk(0x2007, &encode_color_profile_chunk(profile)?));
                }
                let has_sprite_ud = is_non_empty_user_data(file.sprite_user_data());
                if !file.palette().is_empty() {
                    chunks.push(wrap_chunk(0x2019, &encode_palette_chunk(file.palette())?));
                    emit_user_data_chunk(&mut chunks, file.sprite_user_data())?;
                } else if has_sprite_ud {
                    let default_palette = [Color { r: 0, g: 0, b: 0, a: 255, name: None }];
                    chunks.push(wrap_chunk(0x2019, &encode_palette_chunk(&default_palette)?));
                    emit_user_data_chunk(&mut chunks, file.sprite_user_data())?;
                }
                if !file.tags().is_empty() {
                    chunks.push(wrap_chunk(0x2018, &encode_tags_chunk(file.tags())?));
                    // After tags chunk, emit one user_data per tag if any tag has user data
                    let any_tag_ud = file.tags().iter().any(|t| is_non_empty_user_data(&t.user_data));
                    if any_tag_ud {
                        for tag in file.tags() {
                            let ud = tag.user_data.as_ref().cloned().unwrap_or_default();
                            chunks.push(wrap_chunk(0x2020, &encode_user_data_chunk(&ud)?));
                        }
                    }
                }
                for slice in file.slices() {
                    chunks.push(wrap_chunk(0x2022, &encode_slice_chunk(slice)?));
                    emit_user_data_chunk(&mut chunks, &slice.user_data)?;
                }
            }
            for (&(layer_idx, frame_idx), cel) in file.cels_iter() {
                if frame_idx == frame_index {
                    chunks.push(wrap_chunk(0x2005, &encode_cel_chunk(cel, layer_idx)?));
                    if let Some(extra) = &cel.extra {
                        chunks.push(wrap_chunk(0x2006, &encode_cel_extra_chunk(extra)?));
                    }
                    emit_user_data_chunk(&mut chunks, &cel.user_data)?;
                }
            }
            for uc in file.unknown_chunks_for_frame(frame_index) {
                chunks.push(wrap_chunk(uc.chunk_type, &uc.data));
            }
        } else {
            // Replay recorded chunk order for round-trip fidelity
            let mut layer_emit_counter = 0usize;
            let mut slice_emit_counter = 0usize;
            let mut tileset_emit_counter = 0usize;
            let mut unknown_emit_counter = 0usize;
            let mut writer_last_entity = WriterLastEntity::None;
            let unknown_for_frame: Vec<_> = file.unknown_chunks_for_frame(frame_index).collect();

            for entry in &chunk_order {
                match entry.chunk_type {
                    0x2004 => {
                        if layer_emit_counter < file.layers().len() {
                            chunks.push(wrap_chunk(0x2004, &encode_layer_chunk(file, &file.layers()[layer_emit_counter])?));
                            writer_last_entity = WriterLastEntity::Layer(layer_emit_counter);
                            layer_emit_counter += 1;
                        }
                    }
                    0x2005 => {
                        if let Some(li) = entry.layer_index
                            && let Some(cel) = file.cels_iter()
                                .find(|&(&(l, f), _)| l == li && f == frame_index)
                                .map(|(_, c)| c)
                        {
                            chunks.push(wrap_chunk(0x2005, &encode_cel_chunk(cel, li)?));
                            writer_last_entity = WriterLastEntity::Cel(li, frame_index);
                        }
                    }
                    0x2006 => {
                        if let WriterLastEntity::Cel(li, fi) = &writer_last_entity
                            && let Some(cel) = file.cels_iter()
                                .find(|&(&(l, f), _)| l == *li && f == *fi)
                                .map(|(_, c)| c)
                            && let Some(extra) = &cel.extra
                        {
                            chunks.push(wrap_chunk(0x2006, &encode_cel_extra_chunk(extra)?));
                        }
                    }
                    0x2007 => {
                        if let Some(profile) = file.color_profile() {
                            chunks.push(wrap_chunk(0x2007, &encode_color_profile_chunk(profile)?));
                        }
                    }
                    0x2018 => {
                        if !file.tags().is_empty() {
                            chunks.push(wrap_chunk(0x2018, &encode_tags_chunk(file.tags())?));
                            writer_last_entity = WriterLastEntity::TagSequence(0);
                        }
                    }
                    0x2019 => {
                        if !file.palette().is_empty() {
                            chunks.push(wrap_chunk(0x2019, &encode_palette_chunk(file.palette())?));
                            if frame_index == 0 {
                                writer_last_entity = WriterLastEntity::Palette;
                            }
                        }
                    }
                    0x2020 => {
                        let ud = writer_resolve_user_data(file, &mut writer_last_entity);
                        chunks.push(wrap_chunk(0x2020, &encode_user_data_chunk(&ud)?));
                    }
                    0x2008 => {
                        if !file.external_files().is_empty() {
                            chunks.push(wrap_chunk(0x2008, &encode_external_files_chunk(file.external_files())?));
                        }
                    }
                    0x2023 => {
                        if tileset_emit_counter < file.tilesets().len() {
                            chunks.push(wrap_chunk(0x2023, &encode_tileset_chunk(&file.tilesets()[tileset_emit_counter])?));
                            writer_last_entity = WriterLastEntity::Tileset(tileset_emit_counter);
                            tileset_emit_counter += 1;
                        }
                    }
                    0x2022 => {
                        if slice_emit_counter < file.slices().len() {
                            chunks.push(wrap_chunk(0x2022, &encode_slice_chunk(&file.slices()[slice_emit_counter])?));
                            writer_last_entity = WriterLastEntity::Slice(slice_emit_counter);
                            slice_emit_counter += 1;
                        }
                    }
                    0x0004 => {
                        if unknown_emit_counter < unknown_for_frame.len() {
                            let uc = unknown_for_frame[unknown_emit_counter];
                            chunks.push(wrap_chunk(uc.chunk_type, &uc.data));
                            unknown_emit_counter += 1;
                        }
                        if frame_index == 0 {
                            writer_last_entity = WriterLastEntity::Palette;
                        }
                    }
                    _ => {
                        if unknown_emit_counter < unknown_for_frame.len() {
                            let uc = unknown_for_frame[unknown_emit_counter];
                            chunks.push(wrap_chunk(uc.chunk_type, &uc.data));
                            unknown_emit_counter += 1;
                        }
                    }
                }
            }
        }

        let num_chunks = chunks.len();
        let chunks_data: Vec<u8> = chunks.into_iter().flatten().collect();
        let frame_size = 16 + chunks_data.len();

        let mut frame_buf = Vec::with_capacity(frame_size);
        write_dword(&mut frame_buf, frame_size as u32)?;
        write_word(&mut frame_buf, 0xF1FA)?;
        let old_chunks = if num_chunks > 0xFFFE { 0xFFFF } else { num_chunks as u16 };
        write_word(&mut frame_buf, old_chunks)?;
        write_word(&mut frame_buf, file.frames()[frame_index].duration_ms)?;
        write_zeros(&mut frame_buf, 2)?;
        write_dword(&mut frame_buf, num_chunks as u32)?;
        frame_buf.extend_from_slice(&chunks_data);

        frame_buffers.push(frame_buf);
    }

    let total_size: usize = 128 + frame_buffers.iter().map(|f| f.len()).sum::<usize>();

    // Header (128 bytes)
    write_dword(&mut writer, total_size as u32)?;
    write_word(&mut writer, 0xA5E0)?;
    write_word(&mut writer, file.frames().len() as u16)?;
    write_word(&mut writer, file.width())?;
    write_word(&mut writer, file.height())?;
    write_word(&mut writer, file.color_mode().to_depth())?;
    write_dword(&mut writer, file.flags())?;
    write_word(&mut writer, file.deprecated_speed())?;
    write_dword(&mut writer, 0)?; // reserved
    write_dword(&mut writer, 0)?; // reserved
    write_byte(&mut writer, file.transparent_index())?;
    write_zeros(&mut writer, 3)?;
    write_word(&mut writer, file.num_colors())?;
    let (pw, ph) = file.pixel_ratio();
    write_byte(&mut writer, pw)?;
    write_byte(&mut writer, ph)?;
    let grid = file.grid();
    write_short(&mut writer, grid.x)?;
    write_short(&mut writer, grid.y)?;
    write_word(&mut writer, grid.width)?;
    write_word(&mut writer, grid.height)?;
    write_zeros(&mut writer, 84)?;

    for frame_buf in &frame_buffers {
        writer.write_all(frame_buf)?;
    }

    Ok(())
}

// --- Chunk encoders ---

fn encode_layer_chunk(file: &AsepriteFile, layer: &Layer) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = Vec::new();

    let mut flags: u16 = 0;
    if layer.visible { flags |= 1; }
    if layer.editable { flags |= 2; }
    if layer.lock_movement { flags |= 4; }
    if layer.background { flags |= 8; }
    if layer.prefer_linked_cels { flags |= 16; }
    if layer.collapsed { flags |= 32; }
    if layer.reference_layer { flags |= 64; }
    write_word(&mut buf, flags)?;

    let layer_type: u16 = match layer.kind {
        LayerKind::Normal => 0,
        LayerKind::Group => 1,
        LayerKind::Tilemap { .. } => 2,
    };
    write_word(&mut buf, layer_type)?;

    let child_level = compute_child_level(file.layers(), layer);
    write_word(&mut buf, child_level)?;

    write_word(&mut buf, 0)?; // default width
    write_word(&mut buf, 0)?; // default height
    write_word(&mut buf, layer.blend_mode.to_u16())?;
    write_byte(&mut buf, layer.opacity)?;
    write_zeros(&mut buf, 3)?;
    write_string(&mut buf, &layer.name)?;

    if let LayerKind::Tilemap { tileset_index } = layer.kind {
        write_dword(&mut buf, tileset_index)?;
    }

    Ok(buf)
}

fn compute_child_level(layers: &[Layer], layer: &Layer) -> u16 {
    let mut level = 0u16;
    let mut current_parent = layer.parent;
    while let Some(parent_idx) = current_parent {
        level += 1;
        current_parent = layers[parent_idx].parent;
    }
    level
}

fn encode_cel_chunk(cel: &Cel, layer_index: usize) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = Vec::new();

    write_word(&mut buf, layer_index as u16)?;

    let (x, y) = match &cel.kind {
        CelKind::Raw { x, y, .. } | CelKind::Compressed { x, y, .. } => (*x, *y),
        CelKind::Linked { x, y, .. } => (*x, *y),
        CelKind::Tilemap { x, y, .. } => (*x, *y),
    };
    write_short(&mut buf, x)?;
    write_short(&mut buf, y)?;
    write_byte(&mut buf, cel.opacity)?;

    let cel_type: u16 = match &cel.kind {
        CelKind::Raw { .. } => 0,
        CelKind::Compressed { .. } => 2,
        CelKind::Linked { .. } => 1,
        CelKind::Tilemap { .. } => 3,
    };
    write_word(&mut buf, cel_type)?;
    write_short(&mut buf, cel.z_index)?;
    write_zeros(&mut buf, 5)?;

    match &cel.kind {
        CelKind::Raw { pixels, .. } => {
            write_word(&mut buf, pixels.width)?;
            write_word(&mut buf, pixels.height)?;
            buf.extend_from_slice(&pixels.data);
        }
        CelKind::Compressed { pixels, original_compressed, .. } => {
            write_word(&mut buf, pixels.width)?;
            write_word(&mut buf, pixels.height)?;
            if let Some(raw) = original_compressed {
                buf.extend_from_slice(raw);
            } else {
                let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(&pixels.data)?;
                let compressed = encoder.finish()?;
                buf.extend_from_slice(&compressed);
            }
        }
        CelKind::Linked { source_frame, .. } => {
            write_word(&mut buf, *source_frame as u16)?;
        }
        CelKind::Tilemap {
            width, height, bits_per_tile,
            tile_id_bitmask, x_flip_bitmask, y_flip_bitmask, d_flip_bitmask,
            tiles, original_compressed, ..
        } => {
            write_word(&mut buf, *width)?;
            write_word(&mut buf, *height)?;
            write_word(&mut buf, *bits_per_tile)?;
            write_dword(&mut buf, *tile_id_bitmask)?;
            write_dword(&mut buf, *x_flip_bitmask)?;
            write_dword(&mut buf, *y_flip_bitmask)?;
            write_dword(&mut buf, *d_flip_bitmask)?;
            write_zeros(&mut buf, 10)?;

            if let Some(raw) = original_compressed {
                buf.extend_from_slice(raw);
            } else {
                let mut tile_bytes = Vec::with_capacity(tiles.len() * 4);
                for tile in tiles {
                    tile_bytes.extend_from_slice(&tile.to_le_bytes());
                }
                let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(&tile_bytes)?;
                let compressed = encoder.finish()?;
                buf.extend_from_slice(&compressed);
            }
        }
    }

    Ok(buf)
}

fn encode_color_profile_chunk(profile: &ColorProfile) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = Vec::new();
    match profile {
        ColorProfile::None => {
            write_word(&mut buf, 0)?;
            write_word(&mut buf, 0)?;
            write_dword(&mut buf, 0)?;
            write_zeros(&mut buf, 8)?;
        }
        ColorProfile::SRgb { flags, gamma } => {
            write_word(&mut buf, 1)?;
            write_word(&mut buf, *flags)?;
            write_dword(&mut buf, *gamma)?;
            write_zeros(&mut buf, 8)?;
        }
        ColorProfile::Icc { flags, gamma, data } => {
            write_word(&mut buf, 2)?;
            write_word(&mut buf, *flags)?;
            write_dword(&mut buf, *gamma)?;
            write_zeros(&mut buf, 8)?;
            write_dword(&mut buf, data.len() as u32)?;
            buf.extend_from_slice(data);
        }
    }
    Ok(buf)
}

fn encode_palette_chunk(palette: &[Color]) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = Vec::new();
    let count = palette.len() as u32;
    write_dword(&mut buf, count)?;
    write_dword(&mut buf, 0)?;        // first
    write_dword(&mut buf, count - 1)?; // last
    write_zeros(&mut buf, 8)?;

    for color in palette {
        let has_name = color.name.is_some();
        write_word(&mut buf, if has_name { 1 } else { 0 })?;
        write_byte(&mut buf, color.r)?;
        write_byte(&mut buf, color.g)?;
        write_byte(&mut buf, color.b)?;
        write_byte(&mut buf, color.a)?;
        if let Some(name) = &color.name {
            write_string(&mut buf, name)?;
        }
    }

    Ok(buf)
}

fn encode_slice_chunk(slice: &Slice) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = Vec::new();
    write_dword(&mut buf, slice.keys.len() as u32)?;
    let mut flags: u32 = 0;
    if slice.has_nine_patch { flags |= 1; }
    if slice.has_pivot { flags |= 2; }
    write_dword(&mut buf, flags)?;
    write_dword(&mut buf, 0)?; // reserved
    write_string(&mut buf, &slice.name)?;

    for key in &slice.keys {
        write_dword(&mut buf, key.frame)?;
        write_long(&mut buf, key.x)?;
        write_long(&mut buf, key.y)?;
        write_dword(&mut buf, key.width)?;
        write_dword(&mut buf, key.height)?;
        if let Some(np) = &key.nine_patch {
            write_long(&mut buf, np.center_x)?;
            write_long(&mut buf, np.center_y)?;
            write_dword(&mut buf, np.center_width)?;
            write_dword(&mut buf, np.center_height)?;
        }
        if let Some((px, py)) = key.pivot {
            write_long(&mut buf, px)?;
            write_long(&mut buf, py)?;
        }
    }
    Ok(buf)
}

fn encode_external_files_chunk(files: &[ExternalFile]) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = Vec::new();
    write_dword(&mut buf, files.len() as u32)?;
    write_zeros(&mut buf, 8)?;
    for ef in files {
        write_dword(&mut buf, ef.id)?;
        write_byte(&mut buf, ef.file_type.to_u8())?;
        write_zeros(&mut buf, 7)?;
        write_string(&mut buf, &ef.name)?;
    }
    Ok(buf)
}

fn encode_tileset_chunk(tileset: &Tileset) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = Vec::new();
    write_dword(&mut buf, tileset.id)?;
    write_dword(&mut buf, tileset.flags.0)?;
    write_dword(&mut buf, tileset.tile_count)?;
    write_word(&mut buf, tileset.tile_width)?;
    write_word(&mut buf, tileset.tile_height)?;
    write_short(&mut buf, tileset.base_index)?;
    write_zeros(&mut buf, 14)?;
    write_string(&mut buf, &tileset.name)?;

    if tileset.flags.has_external_link()
        && let TilesetData::External { external_file_id, tileset_id_in_external } = &tileset.data
    {
        write_dword(&mut buf, *external_file_id)?;
        write_dword(&mut buf, *tileset_id_in_external)?;
    }

    if tileset.flags.has_embedded_tiles()
        && let TilesetData::Embedded { pixels, original_compressed } = &tileset.data
    {
        if let Some(raw) = original_compressed {
            write_dword(&mut buf, raw.len() as u32)?;
            buf.extend_from_slice(raw);
        } else {
            let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(pixels)?;
            let compressed = encoder.finish()?;
            write_dword(&mut buf, compressed.len() as u32)?;
            buf.extend_from_slice(&compressed);
        }
    }

    Ok(buf)
}

fn encode_cel_extra_chunk(extra: &CelExtra) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = Vec::new();
    write_dword(&mut buf, 1)?; // flags: precise bounds set
    write_dword(&mut buf, extra.precise_x)?;
    write_dword(&mut buf, extra.precise_y)?;
    write_dword(&mut buf, extra.width)?;
    write_dword(&mut buf, extra.height)?;
    write_zeros(&mut buf, 16)?;
    Ok(buf)
}

fn encode_tags_chunk(tags: &[Tag]) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = Vec::new();
    write_word(&mut buf, tags.len() as u16)?;
    write_zeros(&mut buf, 8)?;

    for tag in tags {
        write_word(&mut buf, tag.from_frame as u16)?;
        write_word(&mut buf, tag.to_frame as u16)?;
        write_byte(&mut buf, tag.direction.to_u8())?;
        write_word(&mut buf, tag.repeat)?;
        write_zeros(&mut buf, 6)?;
        write_zeros(&mut buf, 3)?; // deprecated RGB
        write_byte(&mut buf, 0)?;  // extra
        write_string(&mut buf, &tag.name)?;
    }

    Ok(buf)
}

fn encode_user_data_chunk(ud: &UserData) -> Result<Vec<u8>, AsepriteError> {
    let mut buf = Vec::new();
    let mut flags: u32 = 0;
    if ud.text.is_some() { flags |= 1; }
    if ud.color.is_some() { flags |= 2; }
    if !ud.properties.is_empty() { flags |= 4; }
    write_dword(&mut buf, flags)?;

    if let Some(ref text) = ud.text {
        write_string(&mut buf, text)?;
    }
    if let Some(ref color) = ud.color {
        write_byte(&mut buf, color.r)?;
        write_byte(&mut buf, color.g)?;
        write_byte(&mut buf, color.b)?;
        write_byte(&mut buf, color.a)?;
    }
    if !ud.properties.is_empty() {
        // Encode properties into a temp buffer to compute size
        let mut props_buf = Vec::new();
        write_dword(&mut props_buf, ud.properties.len() as u32)?;
        for map in &ud.properties {
            encode_properties_map(&mut props_buf, map)?;
        }
        // prop_size includes itself (4 bytes) + props_buf (which already contains num_maps DWORD + maps data)
        let prop_size = 4 + props_buf.len();
        write_dword(&mut buf, prop_size as u32)?;
        buf.extend_from_slice(&props_buf);
    }
    Ok(buf)
}

fn encode_properties_map(buf: &mut Vec<u8>, map: &PropertiesMap) -> Result<(), AsepriteError> {
    write_dword(buf, map.key)?;
    write_dword(buf, map.entries.len() as u32)?;
    for (name, value) in &map.entries {
        write_string(buf, name)?;
        let type_id = property_value_type_id(value);
        write_word(buf, type_id)?;
        encode_property_value(buf, value)?;
    }
    Ok(())
}

fn property_value_type_id(value: &PropertyValue) -> u16 {
    match value {
        PropertyValue::Bool(_) => 0x0001,
        PropertyValue::Int8(_) => 0x0002,
        PropertyValue::UInt8(_) => 0x0003,
        PropertyValue::Int16(_) => 0x0004,
        PropertyValue::UInt16(_) => 0x0005,
        PropertyValue::Int32(_) => 0x0006,
        PropertyValue::UInt32(_) => 0x0007,
        PropertyValue::Int64(_) => 0x0008,
        PropertyValue::UInt64(_) => 0x0009,
        PropertyValue::Fixed(_) => 0x000A,
        PropertyValue::Float(_) => 0x000B,
        PropertyValue::Double(_) => 0x000C,
        PropertyValue::String(_) => 0x000D,
        PropertyValue::Point(_, _) => 0x000E,
        PropertyValue::Size(_, _) => 0x000F,
        PropertyValue::Rect(_, _, _, _) => 0x0010,
        PropertyValue::Vector(_) => 0x0011,
        PropertyValue::Properties(_) => 0x0012,
        PropertyValue::Uuid(_) => 0x0013,
    }
}

fn encode_property_value(buf: &mut Vec<u8>, value: &PropertyValue) -> Result<(), AsepriteError> {
    match value {
        PropertyValue::Bool(v) => write_byte(buf, if *v { 1 } else { 0 })?,
        PropertyValue::Int8(v) => write_byte(buf, *v as u8)?,
        PropertyValue::UInt8(v) => write_byte(buf, *v)?,
        PropertyValue::Int16(v) => write_short(buf, *v)?,
        PropertyValue::UInt16(v) => write_word(buf, *v)?,
        PropertyValue::Int32(v) => write_long(buf, *v)?,
        PropertyValue::UInt32(v) => write_dword(buf, *v)?,
        PropertyValue::Int64(v) => write_long64(buf, *v)?,
        PropertyValue::UInt64(v) => write_qword(buf, *v)?,
        PropertyValue::Fixed(v) => write_dword(buf, *v)?,
        PropertyValue::Float(v) => write_float(buf, *v)?,
        PropertyValue::Double(v) => write_double(buf, *v)?,
        PropertyValue::String(s) => write_string(buf, s)?,
        PropertyValue::Point(x, y) => { write_long(buf, *x)?; write_long(buf, *y)?; }
        PropertyValue::Size(w, h) => { write_long(buf, *w)?; write_long(buf, *h)?; }
        PropertyValue::Rect(x, y, w, h) => {
            write_long(buf, *x)?; write_long(buf, *y)?;
            write_long(buf, *w)?; write_long(buf, *h)?;
        }
        PropertyValue::Vector(elements) => {
            write_dword(buf, elements.len() as u32)?;
            let homogeneous_type = if elements.is_empty() {
                None
            } else {
                let first_type = property_value_type_id(&elements[0]);
                if elements.iter().all(|e| property_value_type_id(e) == first_type) {
                    Some(first_type)
                } else {
                    None
                }
            };
            if let Some(elem_type) = homogeneous_type {
                write_word(buf, elem_type)?;
                for elem in elements {
                    encode_property_value(buf, elem)?;
                }
            } else {
                write_word(buf, 0)?;
                for elem in elements {
                    write_word(buf, property_value_type_id(elem))?;
                    encode_property_value(buf, elem)?;
                }
            }
        }
        PropertyValue::Properties(entries) => {
            write_dword(buf, entries.len() as u32)?;
            for (name, value) in entries {
                write_string(buf, name)?;
                write_word(buf, property_value_type_id(value))?;
                encode_property_value(buf, value)?;
            }
        }
        PropertyValue::Uuid(bytes) => { buf.extend_from_slice(bytes); }
    }
    Ok(())
}

fn is_non_empty_user_data(ud: &Option<UserData>) -> bool {
    match ud {
        Some(ud) => ud.text.is_some() || ud.color.is_some() || !ud.properties.is_empty(),
        None => false,
    }
}

fn emit_user_data_chunk(chunks: &mut Vec<Vec<u8>>, ud: &Option<UserData>) -> Result<(), AsepriteError> {
    if let Some(ud) = ud {
        chunks.push(wrap_chunk(0x2020, &encode_user_data_chunk(ud)?));
    }
    Ok(())
}
