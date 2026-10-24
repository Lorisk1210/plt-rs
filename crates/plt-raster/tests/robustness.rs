//! Robustness: hostile geometry and styles (NaN, infinities, huge and tiny
//! coordinates, degenerate paths, zero/negative/NaN widths, extreme dash
//! patterns and approximation scales, empty canvases, clip boxes outside
//! the canvas, degenerate resampling transforms) must never panic, hang or
//! allocate without bound.

use plt_core::rng::SplitMix64;
use plt_core::{Affine2D, Path, PathCode, Point, Rgba};
use plt_raster::resample::{Grid, Interpolation, PixelTransform, ResampleParams, resample};
use plt_raster::{
    Antialias, Clip, ClipBox, DashPattern, FillRule, InnerJoin, LineCap, LineJoin, Mask, Painter,
    Pixmap, Rasterizer, Rgba8, Scanline, StrokeStyle, draw_coverage, draw_image,
};

/// `SplitMix64`, for deterministic hostile inputs.
struct Rng(SplitMix64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn below(&mut self, bound: usize) -> usize {
        self.0.below(bound)
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }

    fn flip(&mut self) -> bool {
        self.next() & 1 == 1
    }

    /// Ordinary on-canvas values three times out of four, nasty otherwise.
    fn coordinate(&mut self) -> f64 {
        if self.below(4) == 0 {
            self.pick(&NASTY)
        } else {
            (self.next() % 4000) as f64 / 100.0 - 5.0
        }
    }

    fn point(&mut self) -> Point {
        Point::new(self.coordinate(), self.coordinate())
    }

    fn int(&mut self) -> i32 {
        if self.below(4) == 0 {
            self.pick(&[i32::MIN, i32::MAX, -1, 0, 1 << 30, -(1 << 30)])
        } else {
            self.below(40) as i32 - 5
        }
    }
}

const NASTY: [f64; 13] = [
    0.0,
    -0.0,
    0.5,
    -1.0,
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
    1e300,
    -1e300,
    1e-300,
    f64::MAX,
    1e9,
    -1e9,
];

fn path(rng: &mut Rng) -> Path {
    let len = rng.below(10);
    let vertices: Vec<Point> = (0..len).map(|_| rng.point()).collect();
    match rng.below(3) {
        0 => Path::polyline(vertices),
        1 => Path::closed(vertices),
        _ => {
            let codes = (0..len)
                .map(|i| match (i, rng.below(5)) {
                    (0, _) | (_, 0) => PathCode::MoveTo,
                    (_, 1) => PathCode::LineTo,
                    (_, 2) => PathCode::Curve3,
                    (_, 3) => PathCode::Curve4,
                    _ => PathCode::ClosePoly,
                })
                .collect();
            Path::new(vertices, codes).expect("codes start with a move")
        }
    }
}

/// Dash patterns, valid or not. Patterns far finer than a pixel are left to
/// the dasher's unit tests: they are bounded, but take millions of steps.
fn dashes(rng: &mut Rng) -> Option<DashPattern> {
    let lengths: Vec<f64> = (0..rng.below(5))
        .map(|_| rng.pick(&[0.0, 0.5, 3.0, 1e300, -1.0, f64::NAN, f64::INFINITY]))
        .collect();
    let offset = rng.pick(&[0.0, -2.5, 7.25, 1e18, -1e300, f64::MAX, f64::NAN]);
    DashPattern::from_lengths(&lengths, offset).ok()
}

fn stroke_style(rng: &mut Rng) -> StrokeStyle {
    let width = rng.pick(&[1.0, 0.0, -3.0, 1e-300, 1e9, 1e300, f64::NAN, f64::INFINITY]);
    let mut style = StrokeStyle::new(width)
        .with_cap(rng.pick(&[LineCap::Butt, LineCap::Round, LineCap::Square]))
        .with_join(rng.pick(&[
            LineJoin::Miter,
            LineJoin::MiterRevert,
            LineJoin::MiterRound,
            LineJoin::Round,
            LineJoin::Bevel,
        ]))
        .with_miter_limit(rng.pick(&[4.0, 0.0, -1.0, 1e300, f64::NAN]))
        .with_dashes(dashes(rng));
    style.inner_join = rng.pick(&[
        InnerJoin::Bevel,
        InnerJoin::Miter,
        InnerJoin::Jag,
        InnerJoin::Round,
    ]);
    style.inner_miter_limit = rng.pick(&[1.01, 0.0, f64::NAN, 1e300]);
    style.approximation_scale = rng.pick(&[1.0, 0.0, -1.0, 1e-300, 1e300, f64::NAN]);
    style
}

fn clip_box(rng: &mut Rng) -> Option<ClipBox> {
    rng.flip()
        .then(|| ClipBox::new(rng.int(), rng.int(), rng.int(), rng.int()))
}

fn color(rng: &mut Rng) -> Rgba {
    Rgba::new(
        0.2,
        rng.coordinate(),
        0.7,
        rng.pick(&[1.0, 0.5, f64::NAN, -1.0]),
    )
}

/// Runs every drawing operation on a small canvas.
fn exercise(rng: &mut Rng, painter: &mut Painter) {
    let (width, height) = (rng.pick(&[0, 1, 2, 17]), rng.pick(&[0, 1, 3, 13]));
    let mut target = Pixmap::new(width, height).expect("small canvas");
    let mask =
        Mask::from_vec(width, height, vec![128; (width * height) as usize]).expect("matching mask");
    let wrong_mask = Mask::new(width + 1, height).expect("small mask");
    let masks = [None, Some(&mask), Some(&wrong_mask)];
    let clip = Clip {
        rect: clip_box(rng),
        mask: masks[rng.below(masks.len())],
    };
    let path = path(rng);
    let rule = rng.pick(&[FillRule::NonZero, FillRule::EvenOdd]);
    let aa = rng.pick(&[Antialias::On, Antialias::Off]);
    let _ = painter.fill_path(&mut target, &path, color(rng), rule, aa, &clip);
    let style = stroke_style(rng);
    let _ = painter.stroke_path(&mut target, &path, color(rng), &style, aa, &clip);
    let tile = Pixmap::filled(
        rng.pick(&[0, 1, 3]),
        rng.pick(&[0, 1, 2]),
        Rgba8::new(0, 0, 0, 255),
    )
    .expect("small tile");
    let _ = painter.fill_path_with_pattern(&mut target, &path, &tile, rule, &clip);
    let _ = painter.hatch_tile(
        rng.pick(&[0, 1, 9]),
        &path,
        color(rng),
        rng.coordinate(),
        Rgba8::TRANSPARENT_WHITE,
    );
    let _ = painter.gouraud_triangle(
        &mut target,
        [rng.point(), rng.point(), rng.point()],
        [color(rng), color(rng), color(rng)],
        &clip,
    );
    if let Ok(stamp) = painter.marker_stamp(&path, rng.flip(), rng.flip().then_some(&style)) {
        let _ = stamp.bounds();
        let positions = [rng.point(), rng.point()];
        let _ = stamp.draw(&mut target, &positions, Some(color(rng)), None, &clip);
    }
    if let Ok(coverage) = painter.coverage_mask(&path, width, height, rule, aa, clip_box(rng)) {
        let transform = Affine2D::from_values(
            rng.coordinate(),
            rng.coordinate(),
            rng.coordinate(),
            rng.coordinate(),
            rng.coordinate(),
            rng.coordinate(),
        );
        let _ = painter.draw_coverage_transformed(
            &mut target,
            &coverage,
            &transform,
            color(rng),
            &clip,
        );
        let _ = draw_coverage(
            &mut target,
            &coverage,
            rng.int(),
            rng.int(),
            color(rng),
            &clip,
        );
    }
    let image = target.clone();
    let _ = draw_image(
        &mut target,
        &image,
        rng.int(),
        rng.int(),
        rng.coordinate(),
        &clip,
    );
    let _ = target.copy_region(rng.int(), rng.int(), width, height);
    target.paste(&image, rng.int(), rng.int());
}

#[test]
fn hostile_drawing_does_not_panic() {
    let mut rng = Rng(SplitMix64::new(0x0BAD_5EED));
    // A small cell limit makes runaway geometry fail fast instead of
    // allocating matplotlib's two-gigabyte budget.
    let mut painter = Painter::with_cell_limit(1 << 12);
    for _ in 0..250 {
        exercise(&mut rng, &mut painter);
    }
}

#[test]
fn hostile_rasterizer_input_does_not_panic() {
    let mut rng = Rng(SplitMix64::new(5));
    let mut rasterizer = Rasterizer::with_cell_limit(1 << 12);
    let mut scanline = Scanline::new();
    for _ in 0..400 {
        if rng.flip() {
            rasterizer.set_clip_box(
                rng.coordinate(),
                rng.coordinate(),
                rng.coordinate(),
                rng.coordinate(),
            );
        } else {
            rasterizer.reset_clipping();
        }
        rasterizer.add_path(&path(&mut rng));
        let _ = rasterizer.bounds();
        let _ = rasterizer.sweep(&mut scanline, |sl| {
            for span in sl.spans() {
                let _ = (
                    span.cover_at(0),
                    span.cover_at(span.len().saturating_sub(1)),
                );
            }
        });
        rasterizer.reset();
    }
}

#[test]
fn hostile_clip_rectangles_do_not_panic() {
    let mut rng = Rng(SplitMix64::new(11));
    for _ in 0..400 {
        let b = ClipBox::from_matplotlib_cliprect(
            rng.coordinate(),
            rng.coordinate(),
            rng.coordinate(),
            rng.coordinate(),
            rng.pick(&[0, 1, u32::MAX]),
            rng.pick(&[0, 1, u32::MAX]),
        );
        let _ = b.inclusive();
    }
    assert!(Pixmap::new(u32::MAX, 1).is_err());
    assert!(Mask::new(1, 1 << 23).is_err());
}

#[test]
fn hostile_resampling_does_not_panic() {
    let mut rng = Rng(SplitMix64::new(23));
    for _ in 0..300 {
        let (sw, sh) = (rng.below(4), rng.below(4));
        let values: Vec<f64> = (0..sw * sh).map(|_| rng.coordinate()).collect();
        let src = Grid::from_vec(sw, sh, values).expect("matching size");
        let (dw, dh) = (rng.below(6), rng.below(6));
        let mut dst = Grid::filled(dw, dh, 0.0).expect("small grid");
        let transform = if rng.flip() {
            PixelTransform::Affine(Affine2D::from_values(
                rng.coordinate(),
                rng.coordinate(),
                rng.coordinate(),
                rng.coordinate(),
                rng.coordinate(),
                rng.coordinate(),
            ))
        } else {
            let mesh: Vec<[f64; 2]> = (0..dw * dh)
                .map(|_| [rng.coordinate(), rng.coordinate()])
                .collect();
            PixelTransform::Mesh(Grid::from_vec(dw, dh, mesh).expect("matching size"))
        };
        let params = ResampleParams::new(transform)
            .with_interpolation(rng.pick(&Interpolation::ALL))
            .with_resample(rng.flip())
            .with_alpha(rng.pick(&[1.0, 0.5, 0.0, -1.0, f64::NAN]))
            .with_norm(rng.flip())
            .with_radius(rng.pick(&[1.0, 4.0, 0.0, 1e-300, 255.9, 1e9, f64::NAN]));
        let _ = resample(&src, &mut dst, &params);
    }
}
