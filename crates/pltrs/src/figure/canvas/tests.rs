use std::cell::Cell;
use std::rc::Rc;

use pltrs_core::{Bbox, Point};

use super::*;
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::EventKind;
use crate::lines::{Line2D, LineProps};

/// A 50 × 50 pixel figure with an axes over its middle, showing [0, 1]²,
/// and a thick horizontal line at `y`, animated.
fn figure_with_line(y: f64) -> (Figure, Axes, Line2D) {
    let fig = Figure::with_size(1.0, 1.0, 50.0);
    let ax = fig
        .add_axes([0.2, 0.2, 0.6, 0.6])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 1.0)
        .and_then(|ax| ax.set_ylim(0.0, 1.0))
        .expect("finite limits");
    let props = LineProps::default().linewidth(8.0);
    let line = ax.plot_with(vec![-1.0, 2.0], vec![y, y], props).unwrap();
    line.set_animated(true);
    (fig, ax, line)
}

/// The RGBA bytes of the canvas.
fn pixels(fig: &Figure) -> Vec<u8> {
    fig.canvas().buffer_rgba().unwrap().into_data()
}

/// The pixel at `(x, y)` of the canvas, from its top-left corner.
fn pixel(fig: &Figure, x: usize, y: usize) -> [u8; 4] {
    fig.canvas().buffer_rgba().unwrap().pixel(x, y).unwrap()
}

const WHITE: [u8; 4] = [255; 4];

#[test]
fn the_canvas_holds_the_last_draw() {
    let (fig, _, _) = figure_with_line(0.5);
    let canvas = fig.canvas();
    assert!(matches!(canvas.buffer_rgba(), Err(CanvasError::NotDrawn)));
    let draws = Rc::new(Cell::new(0));
    let counted = Rc::clone(&draws);
    fig.connect(EventKind::Draw, move |_| counted.set(counted.get() + 1));
    canvas.draw().unwrap();
    assert_eq!(draws.get(), 1);
    let image = canvas.buffer_rgba().unwrap();
    assert_eq!((image.width(), image.height()), (50, 50));
    assert_eq!(
        image.pixel(25, 25),
        Some(WHITE),
        "the animated line is left out"
    );
    assert_eq!(fig.bbox(), Bbox::from_extents(0.0, 0.0, 50.0, 50.0));
}

#[test]
fn artists_drawn_over_a_restored_background() {
    let (fig, ax, line) = figure_with_line(0.5);
    let canvas = fig.canvas();
    canvas.draw().unwrap();
    let clean = pixels(&fig);
    let background = canvas.copy_from_bbox(&fig.bbox()).unwrap();
    assert_eq!(background.extents(), PixelBox::new(0, 0, 50, 50));
    ax.draw_artist(&line).unwrap();
    assert_ne!(pixel(&fig, 25, 25), WHITE, "drawn");
    assert_eq!(
        pixel(&fig, 5, 25),
        WHITE,
        "clipped to the axes, which start at x = 10"
    );
    canvas.restore_region(&background).unwrap();
    assert_eq!(pixels(&fig), clean);
    line.set_clip_on(false);
    fig.draw_artist(&line).unwrap();
    assert_ne!(pixel(&fig, 5, 25), WHITE, "unclipped");
}

#[test]
fn animated_artists_change_without_making_the_figure_stale() {
    let (fig, _, line) = figure_with_line(0.5);
    fig.canvas().draw().unwrap();
    line.set_data([0.0, 1.0], [0.2, 0.2]).set_animated(true);
    assert!(!fig.is_stale());
    line.set_animated(false);
    assert!(fig.is_stale(), "an ordinary artist again");
}

#[test]
fn regions_go_stale_with_a_draw_a_resize_or_another_figure() {
    let (fig, _, _) = figure_with_line(0.5);
    let canvas = fig.canvas();
    canvas.draw().unwrap();
    let region = canvas.copy_from_bbox(&fig.bbox()).unwrap();
    canvas.draw().unwrap();
    assert!(matches!(
        canvas.restore_region(&region),
        Err(CanvasError::StaleRegion)
    ));
    let region = canvas.copy_from_bbox(&fig.bbox()).unwrap();
    fig.set_dpi(100.0);
    assert!(matches!(
        canvas.restore_region(&region),
        Err(CanvasError::StaleRegion)
    ));
    let (other, _, _) = figure_with_line(0.5);
    other.canvas().draw().unwrap();
    let foreign = other.canvas().copy_from_bbox(&other.bbox()).unwrap();
    canvas.draw().unwrap();
    assert!(matches!(
        canvas.restore_region(&foreign),
        Err(CanvasError::StaleRegion)
    ));
}

#[test]
fn boxes_outside_the_canvas_save_what_lies_inside() {
    let (fig, _, _) = figure_with_line(0.5);
    let canvas = fig.canvas();
    canvas.draw().unwrap();
    let clean = pixels(&fig);
    let partly = canvas
        .copy_from_bbox(&Bbox::from_extents(-20.0, -20.0, 30.5, 30.5))
        .unwrap();
    // x truncated from 30.5; y flipped from the top: 50 - 30 and 50 + 20.
    assert_eq!(partly.extents(), PixelBox::new(-20, 20, 30, 70));
    assert_eq!((partly.width(), partly.height()), (50, 50));
    // Its image is the whole box, transparent beyond the canvas.
    let image = partly.to_image().unwrap();
    assert_eq!((image.width(), image.height()), (50, 50));
    assert_eq!(image.pixel(0, 0), Some([0, 0, 0, 0]));
    assert_eq!(image.pixel(20, 49), Some([0, 0, 0, 0]), "below the canvas");
    let inside = |x: usize, y: usize| clean[(y * 50 + x) * 4..][..4].to_vec();
    assert_eq!(image.pixel(20, 0).map(Vec::from), Some(inside(0, 20)));
    assert_eq!(image.pixel(49, 29).map(Vec::from), Some(inside(29, 49)));
    let outside = canvas
        .copy_from_bbox(&Bbox::from_extents(1e6, 1e6, 2e6, 2e6))
        .unwrap();
    // Found by fuzzing: a box right of the canvas, level with it, sliced
    // its rows past their end.
    let beside = canvas
        .copy_from_bbox(&Bbox::from_extents(60.0, 10.0, 70.0, 20.0))
        .unwrap();
    canvas.restore_region(&beside).unwrap();
    let (min, max) = (f64::from(i32::MIN), f64::from(i32::MAX));
    let everywhere = canvas
        .copy_from_bbox(&Bbox::from_extents(min, min, max, max))
        .unwrap();
    canvas.restore_region(&partly).unwrap();
    canvas.restore_region(&outside).unwrap();
    canvas.restore_region(&everywhere).unwrap();
    canvas
        .restore_region_with(&everywhere, None, Some(Point::new(max, min)))
        .unwrap();
    canvas
        .restore_region_with(
            &everywhere,
            Some(Bbox::from_extents(min, min, max, max)),
            None,
        )
        .unwrap();
    assert_eq!(pixels(&fig), clean);
    for bad in [
        Bbox::from_extents(f64::NAN, 0.0, 1.0, 1.0),
        Bbox::from_extents(0.0, 0.0, f64::INFINITY, 1.0),
        Bbox::from_extents(0.0, 0.0, 1e12, 1.0),
        Bbox::from_extents(10.0, 0.0, 5.0, 1.0),
    ] {
        assert!(matches!(
            canvas.copy_from_bbox(&bad),
            Err(CanvasError::InvalidBox(_))
        ));
    }
    assert!(matches!(
        canvas.blit(Some(&Bbox::from_extents(0.0, f64::NAN, 1.0, 1.0))),
        Err(CanvasError::InvalidBox(_))
    ));
}

#[test]
fn parts_of_regions_move_where_asked() {
    let (fig, ax, line) = figure_with_line(0.5);
    let canvas = fig.canvas();
    canvas.draw().unwrap();
    let region = canvas.copy_from_bbox(&ax.bbox()).unwrap();
    assert_eq!(region.extents(), PixelBox::new(10, 10, 40, 40));
    // The line covers rows 22 to 27.
    ax.draw_artist(&line).unwrap();
    let drawn = pixels(&fig);
    // Rows 15 to 19 of the background, put back over rows 25 to 29: as
    // in matplotlib, `bbox` is in canvas pixels from the top with its
    // edges included, and `xy` places the region's corner.
    let part = Bbox::from_extents(10.0, 15.0, 39.0, 19.0);
    canvas
        .restore_region_with(&region, Some(part), Some(Point::new(10.0, 20.0)))
        .unwrap();
    assert_eq!(pixel(&fig, 25, 25), WHITE, "covered");
    assert_eq!(pixel(&fig, 25, 29), WHITE);
    assert_ne!(pixel(&fig, 25, 23), WHITE, "above the part");
    let row = |data: &[u8], y: usize| data[y * 200..(y + 1) * 200].to_vec();
    assert_eq!(row(&pixels(&fig), 30), row(&drawn, 30), "below the part");
    // Without `xy`, the corner of `bbox` places the region's corner: here
    // where it came from.
    let whole = Bbox::from_extents(10.0, 10.0, 39.0, 39.0);
    canvas
        .restore_region_with(&region, Some(whole), None)
        .unwrap();
    assert_eq!(pixel(&fig, 25, 23), WHITE, "all of the background");
    assert!(matches!(
        canvas.restore_region_with(&region, None, Some(Point::new(f64::NAN, 0.0))),
        Err(CanvasError::InvalidBox(_))
    ));
}

#[test]
fn redrawing_in_frame_leaves_the_decorations_alone() {
    let (fig, ax, _) = figure_with_line(0.5);
    ax.set_title("title");
    let canvas = fig.canvas();
    canvas.draw().unwrap();
    let before = pixels(&fig);
    ax.set_facecolor("k");
    ax.redraw_in_frame().unwrap();
    assert_eq!(pixel(&fig, 25, 25), [0, 0, 0, 255], "the new background");
    let after = pixels(&fig);
    let row = |data: &[u8], y: usize| data[y * 200..(y + 1) * 200].to_vec();
    assert_eq!(row(&after, 2), row(&before, 2), "the title is not redrawn");
    assert!(ax.xaxis().visible(), "the axis is shown again");
}

#[test]
fn draw_artist_draws_artists_of_other_figures_where_they_are() {
    // Both figures draw an empty axes: their lines are animated.
    let (fig, ax, _) = figure_with_line(0.5);
    let (other, _, foreign) = figure_with_line(0.2);
    fig.canvas().draw().unwrap();
    other.canvas().draw().unwrap();
    let before = pixels(&fig);
    ax.draw_artist(&foreign).unwrap();
    assert_ne!(pixels(&fig), before, "the other figure's line is drawn");
    other.draw_artist(&foreign).unwrap();
    assert_eq!(pixels(&fig), pixels(&other), "where its figure places it");
    fig.canvas().draw().unwrap();
    fig.draw_artist(&foreign).unwrap();
    assert_eq!(pixels(&fig), pixels(&other));
}

#[test]
fn blits_are_recorded_for_presenters_only() {
    let (fig, ax, _) = figure_with_line(0.5);
    let canvas = fig.canvas();
    canvas.draw().unwrap();
    canvas.blit(Some(&ax.bbox())).unwrap();
    assert_eq!(canvas.take_presentation(), Presentation::Nothing);
    let presenting = Presenting::new(&fig);
    canvas
        .blit(Some(&Bbox::from_extents(10.5, 10.5, 20.5, 20.5)))
        .unwrap();
    canvas
        .blit(Some(&Bbox::from_extents(-5.0, 45.0, 60.0, 60.0)))
        .unwrap();
    assert_eq!(
        canvas.take_presentation(),
        Presentation::Regions(vec![
            PixelBox::new(10, 29, 21, 40),
            PixelBox::new(0, 0, 50, 5)
        ])
    );
    canvas.draw().unwrap();
    canvas.blit(None).unwrap();
    assert_eq!(canvas.take_presentation(), Presentation::Whole);
    drop(presenting);
    canvas.draw().unwrap();
    assert_eq!(canvas.take_presentation(), Presentation::Nothing);
}

#[test]
fn lines_of_text_are_drawn_onto_the_canvas() {
    use crate::fonts::FontProperties;
    use crate::text::LineKind;

    let fig = Figure::with_size(1.0, 1.0, 50.0);
    let canvas = fig.canvas();
    canvas.draw().unwrap();
    let blank = pixels(&fig);
    let font = FontProperties::from_rc(&crate::rc::params());
    let gc = pltrs_render::GraphicsContext::default();
    let at = Point::new(5.0, 20.0);
    canvas
        .draw_text(&gc, "Wg", &font, LineKind::Plain, at, 0.0)
        .unwrap();
    let plain = pixels(&fig);
    assert_ne!(plain, blank);
    canvas.draw().unwrap();
    canvas
        .draw_text(&gc, "$W^2$", &font, LineKind::Math, at, 30.0)
        .unwrap();
    assert_ne!(pixels(&fig), plain);
    assert!(matches!(
        canvas.draw_text(&gc, r"$\frac$", &font, LineKind::Math, at, 0.0),
        Err(CanvasError::Text(_))
    ));
}

#[test]
fn idle_draws_wait_for_presenters() {
    let (fig, _, _) = figure_with_line(0.5);
    let draws = Rc::new(Cell::new(0));
    let counted = Rc::clone(&draws);
    fig.connect(EventKind::Draw, move |_| counted.set(counted.get() + 1));
    fig.canvas().draw_idle().unwrap();
    assert_eq!(draws.get(), 1, "drawn at once");
    let presenting = Presenting::new(&fig);
    fig.canvas().draw_idle().unwrap();
    assert_eq!(draws.get(), 1);
    assert!(fig.is_stale(), "left to the presenter");
    drop(presenting);
}

#[test]
fn raster_prints_show_what_their_draw_callbacks_draw() {
    let (fig, ax, line) = figure_with_line(0.5);
    // The row of the line once the callback has moved it to y = 0.2.
    let moved_pixel = |fig: &Figure| {
        let print = fig.render_raster().unwrap().to_image();
        print.pixel(25, 34).unwrap()
    };
    assert_eq!(moved_pixel(&fig), WHITE, "the print shows the line at 0.5");
    let drawn = ax.clone();
    fig.connect(EventKind::Draw, move |_| {
        line.set_ydata(vec![0.2, 0.2]);
        drawn.draw_artist(&line).unwrap();
    });
    assert_ne!(
        moved_pixel(&fig),
        WHITE,
        "the callback drew the moved line onto the print"
    );
    assert_ne!(pixel(&fig, 25, 34), WHITE, "the canvas keeps the print");
}
