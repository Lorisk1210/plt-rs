//! AVIF files decoded by Pillow (through libavif and dav1d), the reader
//! matplotlib's users open them with: lossless files must give back the
//! exact pixels, lossy ones a PSNR near what libavif reaches at the same
//! quality. Each test passes when no Python with Pillow's AVIF support is
//! found (`PLTRS_PYTHON`, the workspace's `.venv`, `.venv-pltrs`, or
//! `python3`), and says so.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use pltrs_codec::av1::ChromaSampling;
use pltrs_codec::avif::{AvifOptions, encode_rgba8};

/// Decodes an AVIF file from stdin and writes its mode, size and pixels.
const DECODE: &str = "
import io, sys
from PIL import Image
im = Image.open(io.BytesIO(sys.stdin.buffer.read()))
im.load()
sys.stdout.buffer.write(f'{im.mode} {im.width} {im.height}\\n'.encode() + im.tobytes())
";

/// Encodes raw RGBA from stdin with Pillow at a quality, then decodes it.
const REFERENCE: &str = "
import io, sys
from PIL import Image
width, height, quality = map(int, sys.argv[1:4])
im = Image.frombytes('RGBA', (width, height), sys.stdin.buffer.read())
buffer = io.BytesIO()
im.save(buffer, 'AVIF', quality=quality)
out = Image.open(io.BytesIO(buffer.getvalue())).convert('RGBA')
sys.stdout.buffer.write(f'RGBA {width} {height}\\n'.encode() + out.tobytes())
";

/// A Python whose Pillow reads and writes AVIF; without one, the checks
/// say they are skipped.
#[expect(
    clippy::print_stderr,
    reason = "the notice that the checks are skipped"
)]
fn python() -> Option<PathBuf> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let candidates = std::env::var_os("PLTRS_PYTHON")
        .map(PathBuf::from)
        .into_iter()
        .chain([".venv", ".venv-pltrs"].map(|venv| workspace.join(venv).join("bin/python")))
        .chain([PathBuf::from("python3")]);
    let found = candidates.into_iter().find(|python| {
        Command::new(python)
            .args([
                "-I",
                "-c",
                "from PIL import features; assert features.check('avif')",
            ])
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    });
    if found.is_none() {
        eprintln!("no Pillow with AVIF support: the AVIF decoding checks are skipped");
    }
    found
}

/// Runs a script with `input` on stdin and parses the decoded image.
fn run(
    python: &PathBuf,
    script: &str,
    args: &[String],
    input: &[u8],
) -> (String, u32, u32, Vec<u8>) {
    let mut child = Command::new(python)
        .args(["-I", "-c", script])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Python starts");
    child
        .stdin
        .take()
        .expect("a pipe")
        .write_all(input)
        .expect("Python reads");
    let output = child.wait_with_output().expect("Python finishes");
    assert!(output.status.success(), "Pillow failed");
    let split = output
        .stdout
        .iter()
        .position(|&b| b == b'\n')
        .expect("a header");
    let header = String::from_utf8_lossy(&output.stdout[..split]).into_owned();
    let fields: Vec<&str> = header.split(' ').collect();
    let width = fields[1].parse().expect("a width");
    let height = fields[2].parse().expect("a height");
    (
        fields[0].to_owned(),
        width,
        height,
        output.stdout[split + 1..].to_vec(),
    )
}

/// The RGBA pixels of a decoded image (grey, RGB or RGBA).
fn rgba(mode: &str, pixels: &[u8]) -> Vec<u8> {
    match mode {
        "RGBA" => pixels.to_vec(),
        "RGB" => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        "L" => pixels.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        other => panic!("unexpected mode {other}"),
    }
}

/// The PSNR of `b` against `a` over the RGB channels.
fn psnr(a: &[u8], b: &[u8]) -> f64 {
    let (sum, count) = a
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0.iter())
        .flat_map(|(p, q)| (0..3).map(move |c| f64::from(p[c]) - f64::from(q[c])))
        .fold((0.0, 0usize), |(sum, count), d| (sum + d * d, count + 1));
    let mse = sum / count as f64;
    if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (255.0 * 255.0 / mse).log10()
    }
}

/// A test picture: gradients, hard edges, thin dark strokes like text,
/// and a translucent band when `alpha` is set.
fn picture(width: u32, height: u32, alpha: bool) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let gradient = [
                (x * 255 / width.max(1)) as u8,
                (y * 255 / height.max(1)) as u8,
                128,
            ];
            let edge = x * 3 > width && y * 2 < height;
            let stroke = (x % 11 == 3 && y % 7 < 5) || (y % 9 == 4 && x % 13 < 6);
            let rgb = if stroke {
                [20, 20, 30]
            } else if edge {
                [31, 119, 180]
            } else {
                gradient
            };
            let a = if alpha && y * 4 > height {
                ((x * 7 + y * 3) % 256) as u8
            } else {
                255
            };
            pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], a]);
        }
    }
    pixels
}

/// Decodes our file of `pixels` and returns the decoded RGBA.
fn decoded(
    python: &PathBuf,
    width: u32,
    height: u32,
    pixels: &[u8],
    options: AvifOptions,
) -> Vec<u8> {
    let file = encode_rgba8(width, height, pixels, &options).expect("the picture encodes");
    let (mode, w, h, out) = run(python, DECODE, &[], &file);
    assert_eq!((w, h), (width, height));
    let translucent = pixels.as_chunks::<4>().0.iter().any(|p| p[3] != 255);
    let grey = options.subsampling == ChromaSampling::Monochrome && options.quality < 100;
    let expected = match (translucent, grey) {
        (true, _) => "RGBA",
        (false, true) => "L",
        (false, false) => "RGB",
    };
    assert_eq!(mode, expected);
    rgba(&mode, &out)
}

#[test]
fn lossless_files_decode_to_the_exact_pixels() {
    let Some(python) = python() else { return };
    let options = AvifOptions {
        quality: 100,
        ..AvifOptions::default()
    };
    // The widest picture needs two tiles.
    let sizes = [
        (1, 1, false),
        (3, 5, true),
        (8, 8, false),
        (65, 33, true),
        (130, 70, false),
        (4200, 10, true),
    ];
    for &(width, height, alpha) in &sizes {
        let pixels = picture(width, height, alpha);
        assert_eq!(
            decoded(&python, width, height, &pixels, options),
            pixels,
            "{width}×{height}"
        );
    }
}

#[test]
fn lossy_files_reach_libavif_quality() {
    let Some(python) = python() else { return };
    for &(width, height, quality) in &[(97, 61, 75), (256, 200, 75), (64, 64, 50), (200, 150, 90)] {
        let pixels = picture(width, height, false);
        let options = AvifOptions {
            quality,
            ..AvifOptions::default()
        };
        let ours = psnr(&pixels, &decoded(&python, width, height, &pixels, options));
        let args = [width, height, u32::from(quality)].map(|v| v.to_string());
        let (_, _, _, reference) = run(&python, REFERENCE, &args, &pixels);
        let theirs = psnr(&pixels, &reference);
        eprintln!("{width}×{height} at quality {quality}: {ours:.2} dB, libavif {theirs:.2} dB");
        assert!(
            ours > theirs - 1.5,
            "{width}×{height} at {quality}: {ours:.2} dB against {theirs:.2} dB"
        );
    }
}

#[test]
fn every_layout_decodes() {
    let Some(python) = python() else { return };
    for sampling in [
        ChromaSampling::Yuv420,
        ChromaSampling::Yuv444,
        ChromaSampling::Monochrome,
    ] {
        for &(width, height) in &[(1, 1), (2, 3), (17, 9), (71, 129)] {
            let pixels = picture(width, height, true);
            let options = AvifOptions {
                quality: 60,
                subsampling: sampling,
            };
            let out = decoded(&python, width, height, &pixels, options);
            // Alpha is coded too, at the colour's quality.
            let alpha_error = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .zip(out.as_chunks::<4>().0.iter())
                .map(|(p, q)| p[3].abs_diff(q[3]))
                .max();
            assert!(
                alpha_error < Some(64),
                "{sampling:?} {width}×{height}: {alpha_error:?}"
            );
        }
    }
}
