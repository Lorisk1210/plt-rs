//! Image files: PNG, JPEG, TIFF, GIF and WebP files (the seed corpus of
//! decoder variants and what pltrs' own encoders write) corrupted and
//! read with `imread`, the images decoded shown with `imshow` and saved;
//! AVIF files encoded from pixels of every size, kind and quality; and
//! corrupted time zone files.

use std::path::Path;

use pltrs::axes::ImshowOptions;
use pltrs::dates::TzFile;
use pltrs::figure::Figure;
use pltrs::image::{BitmapFormat, ImsaveOptions, imread_bytes, imsave_bytes};
use pltrs::numeric::Grid;
use pltrs_codec::av1::ChromaSampling;
use pltrs_codec::avif::{AvifOptions, encode_rgba8};

use crate::harness::{Gen, fuzz};
use crate::support::{format, render};

/// The seed corpus: every file of `corpus/` but its generator, then the
/// files pltrs writes for a small gradient in every bitmap format.
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

/// The kinds of pixels the AVIF encoder is fed.
#[derive(Clone, Copy, Debug)]
enum Pixels {
    /// Each byte random.
    Noise,
    /// One colour, opaque.
    Constant,
    /// Everything fully transparent.
    Transparent,
    /// Smooth ramps in every channel.
    Gradient,
    /// White with a few random pixels, like thin lines on paper.
    Sparse,
}

/// `width × height` RGBA pixels of `kind`, drawn from `seed`.
fn avif_pixels(width: usize, height: usize, kind: Pixels, seed: u64) -> Vec<u8> {
    let mut g = Gen::new(seed);
    let count = width * height;
    let byte = |g: &mut Gen| g.below(256) as u8;
    match kind {
        Pixels::Noise => (0..count * 4).map(|_| byte(&mut g)).collect(),
        Pixels::Constant => [byte(&mut g), byte(&mut g), 7, 255].repeat(count),
        Pixels::Transparent => vec![0; count * 4],
        Pixels::Gradient => (0..count)
            .flat_map(|i| {
                let (x, y) = (i % width, i / width);
                let ramp = |v: usize, n: usize| (v * 255 / n) as u8;
                [
                    ramp(x, width),
                    ramp(y, height),
                    ((x + y) % 256) as u8,
                    (x % 256) as u8,
                ]
            })
            .collect(),
        Pixels::Sparse => {
            let mut pixels = vec![255; count * 4];
            for _ in 0..g.below(count / 4 + 1) {
                let at = g.below(count) * 4;
                pixels[at] = byte(&mut g);
                pixels[at + 1..at + 3].fill(0);
            }
            pixels
        }
    }
}

#[test]
fn avif_encoding() {
    let kinds = [
        Pixels::Noise,
        Pixels::Constant,
        Pixels::Transparent,
        Pixels::Gradient,
        Pixels::Sparse,
    ];
    let samplings = [
        ChromaSampling::Yuv420,
        ChromaSampling::Yuv444,
        ChromaSampling::Monochrome,
    ];
    fuzz(
        "avif encoding",
        150,
        |g| {
            let (width, height) = match g.below(8) {
                0 => (1 + g.below(3), 1 + g.below(3)),
                1..=4 => (1 + g.below(80), 1 + g.below(80)),
                5 => (1 + g.below(300), 1 + g.below(12)),
                6 => (1 + g.below(12), 1 + g.below(300)),
                // Wider than one tile.
                _ => (4097 + g.below(200), 1 + g.below(9)),
            };
            let quality = if g.chance(0.2) {
                100
            } else {
                g.below(101) as u8
            };
            let seed = g.below(1 << 30) as u64;
            (
                width,
                height,
                g.pick(&kinds),
                quality,
                g.pick(&samplings),
                seed,
            )
        },
        |&(width, height, kind, quality, subsampling, seed)| {
            let pixels = avif_pixels(width, height, kind, seed);
            let options = AvifOptions {
                quality,
                subsampling,
            };
            let file = encode_rgba8(width as u32, height as u32, &pixels, &options)
                .expect("valid pixels encode");
            assert_eq!(&file[4..12], b"ftypavif");
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
                let tz = pltrs::dates::TimeZone::Zone(std::sync::Arc::new(tz));
                let _ = pltrs::dates::num2date(*x, &tz);
            }
        },
    );
}
