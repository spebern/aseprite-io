use aseprite::AsepriteFile;

fn round_trip(path: &str) {
    let original = std::fs::read(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    let file = AsepriteFile::from_reader(&original[..])
        .unwrap_or_else(|e| panic!("failed to parse {path}: {e}"));
    let mut output = Vec::new();
    file.write_to(&mut output)
        .unwrap_or_else(|e| panic!("failed to write {path}: {e}"));

    if original != output {
        let pos = original.iter().zip(output.iter()).position(|(a, b)| a != b);
        let len_diff = if original.len() != output.len() {
            format!(", length: {} vs {}", original.len(), output.len())
        } else {
            String::new()
        };
        panic!(
            "round-trip failed for {path}: first diff at byte {}{len_diff}",
            pos.map_or_else(|| original.len().min(output.len()), |p| p),
        );
    }
}

#[test]
fn round_trip_1empty3() {
    round_trip("tests/fixtures/1empty3.aseprite");
}

#[test]
fn round_trip_2f_index_3x3() {
    round_trip("tests/fixtures/2f-index-3x3.aseprite");
}

#[test]
fn round_trip_abcd() {
    round_trip("tests/fixtures/abcd.aseprite");
}

#[test]
fn round_trip_bg_index_3() {
    round_trip("tests/fixtures/bg-index-3.aseprite");
}

#[test]
fn round_trip_groups2() {
    round_trip("tests/fixtures/groups2.aseprite");
}

#[test]
fn round_trip_groups3abc() {
    round_trip("tests/fixtures/groups3abc.aseprite");
}

#[test]
fn round_trip_link() {
    round_trip("tests/fixtures/link.aseprite");
}

#[test]
fn round_trip_tags3() {
    round_trip("tests/fixtures/tags3.aseprite");
}

#[test]
fn round_trip_slices() {
    round_trip("tests/fixtures/slices.aseprite");
}

#[test]
fn round_trip_slices_moving() {
    round_trip("tests/fixtures/slices-moving.aseprite");
}

#[test]
fn round_trip_file_tests_props() {
    round_trip("tests/fixtures/file-tests-props.aseprite");
}

#[test]
fn round_trip_2x2tilemap2x2tile() {
    round_trip("tests/fixtures/2x2tilemap2x2tile.aseprite");
}

#[test]
fn round_trip_2x3tilemap_indexed() {
    round_trip("tests/fixtures/2x3tilemap-indexed.aseprite");
}

#[test]
fn round_trip_3x2tilemap_grayscale() {
    round_trip("tests/fixtures/3x2tilemap-grayscale.aseprite");
}

#[test]
fn round_trip_4f_index_4x4() {
    round_trip("tests/fixtures/4f-index-4x4.aseprite");
}

#[test]
fn round_trip_cut_paste() {
    round_trip("tests/fixtures/cut_paste.aseprite");
}

#[test]
fn round_trip_point2frames() {
    round_trip("tests/fixtures/point2frames.aseprite");
}

#[test]
fn round_trip_point4frames() {
    round_trip("tests/fixtures/point4frames.aseprite");
}

#[test]
fn round_trip_tags3x123reps() {
    round_trip("tests/fixtures/tags3x123reps.aseprite");
}
