use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

// Fixtures that both crates can parse (excludes tilemap files that aseprite-loader rejects)
const COMPARABLE_FIXTURES: &[&str] = &[
    "1empty3",
    "2f-index-3x3",
    "4f-index-4x4",
    "abcd",
    "bg-index-3",
    "cut_paste",
    "groups2",
    "groups3abc",
    "link",
    "point2frames",
    "point4frames",
    "slices",
    "slices-moving",
    "tags3",
    "tags3x123reps",
];

// Fixtures only our crate can parse (tilemap files)
const OUR_ONLY_FIXTURES: &[&str] = &[
    "2x2tilemap2x2tile",
    "2x3tilemap-indexed",
    "3x2tilemap-grayscale",
    "file-tests-props",
];

fn fixture_path(name: &str) -> String {
    format!("tests/fixtures/{name}.aseprite")
}

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    for &name in COMPARABLE_FIXTURES {
        let data = std::fs::read(fixture_path(name)).unwrap();

        group.bench_with_input(BenchmarkId::new("aseprite", name), &data, |b, data| {
            b.iter(|| aseprite::AsepriteFile::from_reader(&data[..]).unwrap());
        });

        group.bench_with_input(
            BenchmarkId::new("aseprite-loader", name),
            &data,
            |b, data| {
                b.iter(|| aseprite_loader::loader::AsepriteFile::load(data).unwrap());
            },
        );
    }

    // Tilemap fixtures — only our crate
    for &name in OUR_ONLY_FIXTURES {
        let data = std::fs::read(fixture_path(name)).unwrap();

        group.bench_with_input(BenchmarkId::new("aseprite", name), &data, |b, data| {
            b.iter(|| aseprite::AsepriteFile::from_reader(&data[..]).unwrap());
        });
    }

    group.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
