//! Throughput benchmark: stroking a 100 000-segment polyline into a
//! 640×480 canvas, like a dense matplotlib line plot. Ignored by default;
//! run it in release mode:
//!
//! ```sh
//! cargo test -p plt-raster --release --test stroke_bench -- --ignored --nocapture
//! ```

use std::time::Instant;

use plt_core::{Path, Point, Rgba};
use plt_raster::{
    Antialias, Clip, FillRule, LineCap, LineJoin, Painter, Pixmap, Rgba8, StrokeStyle,
};

/// A deterministic signal across the canvas, optionally with `SplitMix64`
/// noise (each noisy segment then spans ~70 rows: a worst case).
fn polyline(segments: usize, width: f64, height: f64, noise_amplitude: f64) -> Path {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut noise = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) as f64 / u64::MAX as f64 - 0.5
    };
    let points = (0..=segments)
        .map(|i| {
            let t = i as f64 / segments as f64;
            let y = 0.5 + 0.3 * (t * 40.0).sin() + noise_amplitude * noise();
            Point::new(20.0 + t * (width - 40.0), y * height)
        })
        .collect();
    Path::polyline(points)
}

/// Mean milliseconds per run of `f` (after one warm-up run).
fn time<F: FnMut()>(runs: u32, mut f: F) -> f64 {
    f();
    let start = Instant::now();
    for _ in 0..runs {
        f();
    }
    start.elapsed().as_secs_f64() * 1000.0 / f64::from(runs)
}

#[test]
#[ignore = "benchmark; run in release mode with --ignored"]
fn stroke_100k_segments() {
    let noisy = polyline(100_000, 640.0, 480.0, 0.15);
    let smooth = polyline(100_000, 640.0, 480.0, 0.0);
    let mut painter = Painter::new();
    let mut canvas = Pixmap::filled(640, 480, Rgba8::TRANSPARENT_WHITE).expect("canvas");
    let color = Rgba::new(0.12, 0.47, 0.71, 1.0);
    for (label, path, join, width) in [
        (
            "noisy 1e5 segments, round joins, width 2.08",
            &noisy,
            LineJoin::Round,
            2.08,
        ),
        (
            "noisy 1e5 segments, miter joins, width 1",
            &noisy,
            LineJoin::MiterRevert,
            1.0,
        ),
        (
            "smooth 1e5 segments, round joins, width 2.08",
            &smooth,
            LineJoin::Round,
            2.08,
        ),
    ] {
        let style = StrokeStyle::new(width)
            .with_cap(LineCap::Square)
            .with_join(join)
            .with_miter_limit(width);
        let ms = time(5, || {
            painter
                .stroke_path(&mut canvas, path, color, &style, Antialias::On, &Clip::NONE)
                .expect("stroke");
        });
        println!("{label:<46} {ms:8.2} ms");
    }
    let ms = time(5, || {
        painter
            .fill_path(
                &mut canvas,
                &noisy,
                color.with_alpha(0.3),
                FillRule::NonZero,
                Antialias::On,
                &Clip::NONE,
            )
            .expect("fill");
    });
    println!("{:<46} {ms:8.2} ms", "noisy 1e5-vertex polygon fill");
}
