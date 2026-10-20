//! Corrupted PNG, GIF, WebP and zlib data: decoding returns errors (or
//! some image) but never panics. Valid files (the fixtures and files the
//! encoders write) get bits flipped, bytes replaced and tails cut.

use std::fs;
use std::path::Path;

use plt_codec::gif::{GifEncoder, GifFrame};
use plt_codec::png::{self, ImageRef};
use plt_codec::{webp, zlib};

/// `SplitMix64`.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..bound` (`bound > 0`).
    fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).expect("a usize fits in u64");
        usize::try_from(self.next() % bound).expect("below a usize bound")
    }
}

/// `data` with 1 to 4 bits, bytes or lengths changed.
fn mutate(data: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut out = data.to_vec();
    for _ in 0..=rng.below(4) {
        if out.is_empty() {
            break;
        }
        let at = rng.below(out.len());
        match rng.below(4) {
            0 => out[at] ^= 1 << rng.below(8),
            1 => out[at] = [0, 0xFF, 0x7F, 0x80][rng.below(4)],
            2 => out[at] = rng.next().to_le_bytes()[0],
            _ => out.truncate(at),
        }
    }
    out
}

/// The files with `extension` in the fixture directory `name`.
fn fixtures(name: &str, extension: &str) -> Vec<Vec<u8>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let mut paths: Vec<_> = fs::read_dir(dir)
        .expect("fixture directory exists")
        .map(|entry| entry.expect("readable directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == extension))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| fs::read(path).expect("readable fixture"))
        .collect()
}

/// An RGBA gradient of `width × height` pixels with partial alpha.
fn gradient(width: u32, height: u32) -> Vec<u8> {
    (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                let byte = |v: u32| u8::try_from(v % 256).expect("below 256");
                [byte(x * 13), byte(y * 7), byte(x * y), byte(x + 3 * y)]
            })
        })
        .collect()
}

/// Feeds `rounds` corruptions of the `corpus` files to `decode`.
fn corrupt_all(corpus: &[Vec<u8>], rounds: usize, seed: u64, decode: impl Fn(&[u8])) {
    let mut rng = Rng(seed);
    for round in 0..rounds {
        decode(&mutate(&corpus[round % corpus.len()], &mut rng));
    }
}

#[test]
fn corrupted_png_files_never_panic() {
    let mut corpus = fixtures("png", "png");
    let pixels = gradient(9, 7);
    let image = ImageRef::rgba8(9, 7, &pixels).expect("sized pixels");
    corpus.push(png::encode(image).expect("encodable image"));
    corrupt_all(&corpus, 30_000, 1, |data| {
        let _ = png::decode(data);
    });
}

#[test]
fn corrupted_zlib_streams_never_panic() {
    let corpus = fixtures("zlib", "zlib");
    corrupt_all(&corpus, 20_000, 2, |data| {
        let _ = zlib::decompress_with_limit(data, 1 << 20);
    });
}

#[test]
fn corrupted_gif_files_never_panic() {
    let pixels = gradient(12, 10);
    let shifted: Vec<u8> = pixels.iter().map(|v| v.wrapping_add(40)).collect();
    let frames = [
        GifFrame {
            pixels: &pixels,
            delay: 10,
        },
        GifFrame {
            pixels: &shifted,
            delay: 20,
        },
    ];
    let corpus = vec![
        GifEncoder::new()
            .encode(12, 10, &frames[..1])
            .expect("encodable frame"),
        GifEncoder::new()
            .plays(2)
            .encode(12, 10, &frames)
            .expect("encodable frames"),
    ];
    corrupt_all(&corpus, 20_000, 3, |data| {
        let _ = plt_codec::gif::decode(data);
    });
}

#[test]
fn corrupted_webp_files_never_panic() {
    let mut corpus = fixtures("webp", "webp");
    corpus.push(webp::encode_rgba8(11, 9, &gradient(11, 9)).expect("encodable image"));
    corrupt_all(&corpus, 30_000, 4, |data| {
        let _ = webp::decode(data);
    });
}
