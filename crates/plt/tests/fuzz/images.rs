//! Image files: PNG, JPEG, TIFF, GIF and WebP files (the seed corpus of
//! decoder variants and what plt-rs' own encoders write) corrupted and
//! read with `imread`, the images decoded shown with `imshow` and saved;
//! and corrupted time zone files.

use std::path::Path;

use plt::axes::ImshowOptions;
use plt::dates::TzFile;
use plt::figure::Figure;
use plt::image::{BitmapFormat, ImsaveOptions, imread_bytes, imsave_bytes};
use plt::numeric::Grid;

use crate::harness::{Gen, fuzz};
use crate::support::{format, render};

/// The seed corpus: every file of `corpus/` but its generator, then the
/// files plt-rs writes for a small gradient in every bitmap format.
fn corpus() -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fuzz/corpus");
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir)
        .expect("the corpus directory")
        .map(|entry| entry.expect("a corpus entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext != "py"))
        .map(|path| {
            let name = path
                .file_name()
                .expect("a file")
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read(&path).expect("a readable corpus file"))
        })
        .collect();
    files.sort();
    let gradient = Grid::from_fn(7, 9, |i, j| [i as f64 / 7.0, j as f64 / 9.0, 0.5, 0.75]);
    for format in [
        BitmapFormat::Png,
        BitmapFormat::Jpeg,
        BitmapFormat::Tiff,
        BitmapFormat::Gif,
        BitmapFormat::Webp,
    ] {
        let data = gradient.clone();
        let file =
            imsave_bytes(data, format, ImsaveOptions::default()).expect("an encodable image");
        files.push((format!("{format:?}"), file));
    }
    files
}

#[test]
fn corrupted_images() {
    let corpus = corpus();
    fuzz(
        "images",
        10_000,
        |g| {
            let file = g.below(corpus.len());
            (corpus[file].0.clone(), file, g.corrupt(&corpus[file].1))
        },
        |(_, file, corruption)| {
            let _ = imread_bytes(&corruption.apply(&corpus[*file].1));
        },
    );
}

#[test]
fn showing_corrupted_images() {
    let corpus = corpus();
    fuzz(
        "image showing",
        400,
        |g| {
            let file = g.below(corpus.len());
            (
                corpus[file].0.clone(),
                file,
                g.corrupt(&corpus[file].1),
                format(g),
            )
        },
        |(_, file, corruption, format)| {
            if let Ok(image) = imread_bytes(&corruption.apply(&corpus[*file].1)) {
                let fig = Figure::with_size(1.5, 1.5, 30.0);
                if fig
                    .subplot()
                    .imshow(image, ImshowOptions::default())
                    .is_ok()
                {
                    render(&fig, *format);
                }
            }
        },
    );
}

/// A compiled zone of the system's zoneinfo, if there is one.
fn zone_file() -> Option<Vec<u8>> {
    ["Europe/Paris", "America/New_York", "UTC"]
        .iter()
        .find_map(|zone| std::fs::read(Path::new("/usr/share/zoneinfo").join(zone)).ok())
}

#[test]
fn corrupted_time_zone_files() {
    let Some(zone) = zone_file() else { return };
    fuzz(
        "tzfile",
        10_000,
        |g: &mut Gen| (g.corrupt(&zone), g.number()),
        |(corruption, x)| {
            if let Ok(tz) = TzFile::parse(&corruption.apply(&zone), "fuzzed") {
                let tz = plt::dates::TimeZone::Zone(std::sync::Arc::new(tz));
                let _ = plt::dates::num2date(*x, &tz);
            }
        },
    );
}
