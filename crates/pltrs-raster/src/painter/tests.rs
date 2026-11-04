//! Tests of the painter's operations.

use pltrs_core::{Affine2D, Path, Point, Rgba};

use super::Painter;
use crate::clip::{Clip, ClipBox};
use crate::color::Rgba8;
use crate::error::RasterError;
use crate::geometry::{feed_path, is_flat};
use crate::mask::Mask;
use crate::pixmap::Pixmap;
use crate::raster::{Rasterizer, Scanline};
use crate::style::{Antialias, DashPattern, FillRule, LineCap, LineJoin, StrokeStyle};

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn alpha_sum(p: &Pixmap) -> u32 {
    p.data().chunks(4).map(|c| u32::from(c[3])).sum()
}

#[test]
fn fills_and_clips_a_rectangle() {
    let mut target = Pixmap::new(8, 8).unwrap();
    let mut painter = Painter::new();
    let rect = Path::polygon([p(1.0, 1.0), p(7.0, 1.0), p(7.0, 7.0), p(1.0, 7.0)]);
    let clip = Clip::rect(ClipBox::new(0, 0, 4, 8));
    painter
        .fill_path(
            &mut target,
            &rect,
            Rgba::BLACK,
            FillRule::NonZero,
            Antialias::On,
            &clip,
        )
        .unwrap();
    assert_eq!(alpha_sum(&target), 3 * 6 * 255);
    assert_eq!(target.pixel(3, 3).unwrap().a, 255);
    assert_eq!(target.pixel(4, 3).unwrap().a, 0);
}

#[test]
fn flat_paths_cover_nothing_when_rasterized() {
    // What the fill shortcut relies on, clipped or not: vertical and
    // horizontal outlines (rectangles snapped to a line) and lines
    // reaching outside the clip box leave no scanline.
    let flats = [
        Path::polygon([p(2.3, 1.0), p(2.3, 1.0), p(2.3, 7.5), p(2.3, 7.5)]),
        Path::polygon([p(1.0, 3.0), p(6.5, 3.0), p(6.5, 3.0), p(1.0, 3.0)]),
        Path::polyline(vec![p(-4.0, 2.0), p(-4.0, 9.0), p(-4.0, 4.0)]),
        Path::polyline(vec![p(3.0, -2.0), p(3.0, 12.0)]),
    ];
    for path in &flats {
        assert!(is_flat(path));
        for clip in [None, Some((0.0, 0.0, 8.0, 8.0))] {
            let mut rasterizer = Rasterizer::new();
            if let Some((x0, y0, x1, y1)) = clip {
                rasterizer.set_clip_box(x0, y0, x1, y1);
            }
            feed_path(path, &mut rasterizer);
            let mut covered = false;
            rasterizer
                .sweep(&mut Scanline::default(), |_| covered = true)
                .unwrap();
            assert!(!covered, "{path:?} clipped to {clip:?}");
        }
    }
    let triangle = Path::polygon([p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)]);
    assert!(!is_flat(&triangle));
    let curve = Path::builder()
        .move_to(p(1.0, 0.0))
        .quad_to(p(1.0, 1.0), p(1.0, 2.0))
        .build();
    assert!(!is_flat(&curve), "curves are left to the rasterizer");
    let mut target = Pixmap::new(8, 8).unwrap();
    Painter::new()
        .fill_path(
            &mut target,
            &flats[0],
            Rgba::BLACK,
            FillRule::NonZero,
            Antialias::On,
            &Clip::NONE,
        )
        .unwrap();
    assert_eq!(alpha_sum(&target), 0);
}

#[test]
fn strokes_with_and_without_dashes() {
    let mut painter = Painter::new();
    let line = Path::polyline(vec![p(0.0, 2.0), p(20.0, 2.0)]);
    let mut solid = Pixmap::new(20, 4).unwrap();
    let style = StrokeStyle::new(2.0).with_join(LineJoin::Round);
    painter
        .stroke_path(
            &mut solid,
            &line,
            Rgba::BLACK,
            &style,
            Antialias::On,
            &Clip::NONE,
        )
        .unwrap();
    assert_eq!(alpha_sum(&solid), 40 * 255);
    let mut dashed = Pixmap::new(20, 4).unwrap();
    let style = style.with_dashes(Some(DashPattern::new(vec![(5.0, 5.0)], 0.0).unwrap()));
    painter
        .stroke_path(
            &mut dashed,
            &line,
            Rgba::BLACK,
            &style,
            Antialias::On,
            &Clip::NONE,
        )
        .unwrap();
    assert_eq!(alpha_sum(&dashed), 20 * 255);
}

#[test]
fn pattern_fill_repeats_the_tile() {
    let mut painter = Painter::new();
    let mut tile = Pixmap::new(2, 1).unwrap();
    tile.set_pixel(0, 0, Rgba8::new(255, 0, 0, 255));
    let mut target = Pixmap::new(4, 1).unwrap();
    let rect = Path::polygon([p(0.0, 0.0), p(4.0, 0.0), p(4.0, 1.0), p(0.0, 1.0)]);
    painter
        .fill_path_with_pattern(&mut target, &rect, &tile, FillRule::NonZero, &Clip::NONE)
        .unwrap();
    let alphas: Vec<u8> = target.data().chunks(4).map(|c| c[3]).collect();
    assert_eq!(alphas, vec![255, 0, 255, 0]);
}

#[test]
fn coverage_mask_and_markers() {
    let mut painter = Painter::new();
    let square = Path::polygon([p(1.0, 1.0), p(3.0, 1.0), p(3.0, 3.0), p(1.0, 3.0)]);
    let mask = painter
        .coverage_mask(&square, 4, 4, FillRule::NonZero, Antialias::On, None)
        .unwrap();
    assert_eq!(
        mask.data().iter().map(|&v| u32::from(v)).sum::<u32>(),
        4 * 255
    );
    let stamp = painter
        .marker_stamp(&square, true, Some(&StrokeStyle::new(1.0)))
        .unwrap();
    assert_eq!(stamp.bounds(), Some((0, 0, 3, 3)));
    let mut target = Pixmap::new(10, 10).unwrap();
    stamp
        .draw(
            &mut target,
            &[p(5.0, 5.0)],
            Some(Rgba::BLACK),
            Some(Rgba::BLACK),
            &Clip::NONE,
        )
        .unwrap();
    assert_eq!(target.pixel(7, 7).unwrap().a, 255);
    assert_eq!(target.pixel(2, 2).unwrap().a, 0);
}

#[test]
fn gouraud_and_transformed_coverage_draw_something() {
    let mut painter = Painter::new();
    let mut target = Pixmap::new(16, 16).unwrap();
    let colors = [Rgba::BLACK, Rgba::WHITE, Rgba::opaque(1.0, 0.0, 0.0)];
    painter
        .gouraud_triangle(
            &mut target,
            [p(1.0, 1.0), p(14.0, 2.0), p(3.0, 14.0)],
            colors,
            &Clip::NONE,
        )
        .unwrap();
    assert!(alpha_sum(&target) > 50 * 255);
    let before = target.clone();
    painter
        .gouraud_triangle(
            &mut target,
            [p(f64::NAN, 1.0), p(14.0, 2.0), p(3.0, 14.0)],
            colors,
            &Clip::NONE,
        )
        .unwrap();
    assert_eq!(target, before);

    let mut target = Pixmap::new(16, 16).unwrap();
    let glyph = Mask::from_vec(4, 4, vec![255; 16]).unwrap();
    let t = Affine2D::identity().rotate_deg(30.0).translate(8.0, 4.0);
    painter
        .draw_coverage_transformed(&mut target, &glyph, &t, Rgba::BLACK, &Clip::NONE)
        .unwrap();
    assert!(alpha_sum(&target) > 8 * 254);
}

#[test]
fn hatch_tile_and_errors() {
    let mut painter = Painter::new();
    let diagonal = Path::polyline(vec![p(0.0, 8.0), p(8.0, 0.0)]);
    let tile = painter
        .hatch_tile(8, &diagonal, Rgba::BLACK, 1.0, Rgba8::TRANSPARENT_WHITE)
        .unwrap();
    assert!(alpha_sum(&tile) > 0);
    assert_eq!(tile.pixel(0, 0), Some(Rgba8::TRANSPARENT_WHITE));

    let mut small = Painter::with_cell_limit(2);
    let mut target = Pixmap::new(8, 8).unwrap();
    let big = Path::polygon([p(0.5, 0.5), p(7.5, 0.5), p(7.5, 7.5), p(0.5, 7.5)]);
    assert!(
        small
            .fill_path(
                &mut target,
                &big,
                Rgba::BLACK,
                FillRule::NonZero,
                Antialias::On,
                &Clip::NONE
            )
            .is_err()
    );
    let wrong = Mask::new(2, 2).unwrap();
    assert!(
        painter
            .fill_path(
                &mut target,
                &big,
                Rgba::BLACK,
                FillRule::NonZero,
                Antialias::On,
                &Clip::NONE.with_mask(&wrong)
            )
            .is_err()
    );
}

#[test]
fn hostile_strokes_fail_fast() {
    // 1e15 dashes, each capped by thousands of points, all clipped away:
    // the edge count trips the cell limit and stroking stops early.
    let mut painter = Painter::with_cell_limit(1 << 16);
    let mut target = Pixmap::new(8, 8).unwrap();
    let line = Path::polyline(vec![p(0.0, 4.0), p(1e6, 4.0)]);
    let dashes = DashPattern::new(vec![(1e-9, 1e-9)], 0.0).unwrap();
    let style = StrokeStyle::new(1e9)
        .with_cap(LineCap::Round)
        .with_dashes(Some(dashes));
    let clip = Clip::rect(ClipBox::new(100, 100, 108, 108));
    let result = painter.stroke_path(
        &mut target,
        &line,
        Rgba::BLACK,
        &style,
        Antialias::On,
        &clip,
    );
    assert_eq!(
        result,
        Err(RasterError::CellLimitExceeded { limit: 1 << 16 })
    );
}

#[test]
fn shapes_wholly_inside_blend_as_clipped_ones_do() {
    // A mask of full coverage leaves every cover as it is, but takes the
    // span-by-span path: the unclipped one must agree with it.
    let (width, height) = (24, 20);
    let full = Mask::from_vec(width, height, vec![255; (width * height) as usize]).unwrap();
    let ring = Path::circle(p(11.3, 9.6), 6.2);
    let flat: Vec<Point> = (0..40)
        .map(|i| {
            let a = f64::from(i) * std::f64::consts::TAU / 40.0;
            p(11.3 + 6.2 * a.cos(), 9.6 + 4.1 * a.sin())
        })
        .collect();
    let shapes = [Path::polygon(flat), ring];
    let style = StrokeStyle::new(2.08).with_join(LineJoin::Round);
    let mut drawn = Vec::new();
    for mask in [None, Some(&full)] {
        let mut target = Pixmap::new(width, height).unwrap();
        target.fill(Rgba8::new(40, 90, 200, 120));
        let mut painter = Painter::new();
        let clip = Clip { rect: None, mask };
        for (i, shape) in shapes.iter().enumerate() {
            let color = Rgba::new(0.9, 0.2, 0.1, if i == 0 { 1.0 } else { 0.6 });
            painter
                .fill_path(
                    &mut target,
                    shape,
                    color,
                    FillRule::NonZero,
                    Antialias::On,
                    &clip,
                )
                .unwrap();
            painter
                .stroke_path(&mut target, shape, color, &style, Antialias::On, &clip)
                .unwrap();
        }
        drawn.push(target);
    }
    assert_eq!(drawn[0], drawn[1]);
}
