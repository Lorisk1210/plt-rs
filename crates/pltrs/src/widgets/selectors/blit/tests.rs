//! How selectors show their changes, against what matplotlib 3.11's Agg
//! canvas does for the same events (draw events counted after each, and
//! pixels of the canvas).

use std::cell::Cell;
use std::rc::Rc;

use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::{EventKind, EventSource};
use crate::figure::Figure;
use crate::navigation::PointerButton;
use crate::widgets::{
    Lasso, LassoOptions, LassoSelector, LassoSelectorOptions, Orientation, PolygonSelector,
    PolygonSelectorOptions, RectangleSelector, RectangleSelectorOptions, SpanSelector,
    SpanSelectorOptions,
};

/// A 400 × 400 figure at 100 dpi with one axes over the data square
/// [0, 10]², drawn once, and the count of its draw events from then on.
fn drawn_axes() -> (Figure, Axes, Rc<Cell<usize>>) {
    let fig = Figure::with_size(4.0, 4.0, 100.0);
    let ax = fig
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 10.0))
        .expect("finite limits");
    (fig, ax, Rc::new(Cell::new(0)))
}

/// Starts counting the draw events of `fig` into `draws`, then draws it.
fn count_draws(fig: &Figure, draws: &Rc<Cell<usize>>) {
    let counter = Rc::clone(draws);
    fig.connect(EventKind::Draw, move |_| counter.set(counter.get() + 1));
    fig.canvas().draw().expect("drawn");
    draws.set(0);
}

/// A mouse step: press, motion or release at display `(x, y)`.
#[derive(Clone, Copy)]
enum Step {
    Press(f64, f64),
    Move(f64, f64),
    Release(f64, f64),
}

/// The draw events counted after each of `steps`, sent with the left
/// button.
fn draws_after(fig: &Figure, draws: &Cell<usize>, steps: &[Step]) -> Vec<usize> {
    let mut source = EventSource::new();
    steps
        .iter()
        .map(|&step| {
            match step {
                Step::Press(x, y) => source.button_press(fig, x, y, PointerButton::Left, false),
                Step::Move(x, y) => source.motion(fig, x, y),
                Step::Release(x, y) => source.button_release(fig, x, y, PointerButton::Left),
            }
            draws.get()
        })
        .collect()
}

/// A diagonal drag from (50, 50) to (200, 200).
const DRAG: [Step; 5] = [
    Step::Press(50.0, 50.0),
    Step::Move(100.0, 100.0),
    Step::Move(150.0, 150.0),
    Step::Move(200.0, 200.0),
    Step::Release(200.0, 200.0),
];

/// A lasso's strokes: the button goes down (a lasso takes no presses),
/// two motions and the release.
const STROKES: [Step; 4] = [
    Step::Press(100.0, 100.0),
    Step::Move(100.0, 100.0),
    Step::Move(150.0, 150.0),
    Step::Release(150.0, 150.0),
];

#[test]
fn selectors_that_do_not_blit_draw_the_figure_at_each_update() {
    // matplotlib: the draw events of an Agg canvas after each step.
    let (fig, ax, draws) = drawn_axes();
    let _span = SpanSelector::new(&ax, Orientation::Horizontal, SpanSelectorOptions::default());
    count_draws(&fig, &draws);
    assert_eq!(draws_after(&fig, &draws, &DRAG), [2, 3, 4, 5, 6]);

    let (fig, ax, draws) = drawn_axes();
    let options = RectangleSelectorOptions {
        interactive: true,
        ..RectangleSelectorOptions::default()
    };
    let _rectangle = RectangleSelector::new(&ax, options);
    count_draws(&fig, &draws);
    let twice = [DRAG, DRAG].concat();
    assert_eq!(
        draws_after(&fig, &draws, &twice),
        [2, 3, 4, 5, 6, 6, 7, 8, 9, 10]
    );

    let (fig, ax, draws) = drawn_axes();
    let options = LassoSelectorOptions {
        useblit: false,
        ..LassoSelectorOptions::default()
    };
    let _lasso = LassoSelector::new(&ax, options);
    count_draws(&fig, &draws);
    assert_eq!(draws_after(&fig, &draws, &DRAG), [0, 1, 2, 3, 3]);

    let (fig, ax, draws) = drawn_axes();
    let _polygon = PolygonSelector::new(&ax, PolygonSelectorOptions::default());
    count_draws(&fig, &draws);
    let clicks = [
        Step::Press(50.0, 50.0),
        Step::Release(50.0, 50.0),
        Step::Move(100.0, 100.0),
        Step::Press(150.0, 50.0),
        Step::Release(150.0, 50.0),
        Step::Move(200.0, 200.0),
    ];
    assert_eq!(draws_after(&fig, &draws, &clicks), [0, 0, 1, 1, 1, 2]);

    let (fig, ax, draws) = drawn_axes();
    let options = LassoOptions {
        useblit: false,
        ..LassoOptions::default()
    };
    let lasso = Lasso::new(&ax, (5.0, 5.0), options);
    assert!(!lasso.useblit());
    count_draws(&fig, &draws);
    assert_eq!(draws_after(&fig, &draws, &STROKES), [0, 1, 2, 2]);
}

/// A white pixel.
const WHITE: [u8; 4] = [255, 255, 255, 255];
/// A pixel of the half-transparent red span over white.
const SPAN: [u8; 4] = [255, 127, 127, 255];

#[test]
fn blitting_selectors_draw_only_their_animated_artists() {
    let (fig, ax, draws) = drawn_axes();
    let options = SpanSelectorOptions {
        interactive: true,
        useblit: true,
        ..SpanSelectorOptions::default()
    };
    let span = SpanSelector::new(&ax, Orientation::Horizontal, options);
    assert!(span.useblit());
    assert!(span.artist().animated() && span.handles().iter().all(ArtistHandle::animated));
    count_draws(&fig, &draws);
    // matplotlib: (255, 127, 127) inside the half-transparent red span,
    // white before it reaches the pixel and once it is cleared.
    let pixel = || {
        let image = fig.canvas().buffer_rgba().expect("drawn");
        image.pixel(125, 200).expect("on the canvas")
    };
    let steps = [
        Step::Press(50.0, 200.0),
        Step::Move(100.0, 200.0),
        Step::Move(200.0, 200.0),
        Step::Release(200.0, 200.0),
    ];
    let mut source = EventSource::new();
    let mut seen = Vec::new();
    for step in steps {
        match step {
            Step::Press(x, y) => source.button_press(&fig, x, y, PointerButton::Left, false),
            Step::Move(x, y) => source.motion(&fig, x, y),
            Step::Release(x, y) => source.button_release(&fig, x, y, PointerButton::Left),
        }
        seen.push(pixel());
    }
    assert_eq!(seen, [WHITE, WHITE, SPAN, SPAN]);
    assert_eq!(draws.get(), 0, "blitting draws no figure");
    assert!(!fig.is_stale(), "the animated span leaves the figure drawn");
    // A draw of the figure leaves the span out, saves the background and
    // draws the span back over it: two draws, the second without it.
    fig.canvas().draw().expect("drawn");
    assert_eq!((draws.get(), pixel()), (2, SPAN));
    // Turned on again, the selector saves its background anew.
    span.set_active(false).set_active(true);
    assert_eq!((draws.get(), pixel()), (3, SPAN));
    span.set_props(crate::patches::PatchProps::default().facecolor("blue"));
    assert_eq!((draws.get(), pixel()), (3, [127, 127, 255, 255]));
    span.clear();
    assert_eq!((draws.get(), pixel()), (3, WHITE));
}

#[test]
fn a_blitting_lasso_draws_over_the_background_it_saved() {
    let (fig, ax, draws) = drawn_axes();
    count_draws(&fig, &draws);
    let lasso = Lasso::new(&ax, (5.0, 5.0), LassoOptions::default());
    assert!(lasso.useblit() && !lasso.line().animated());
    assert_eq!(draws_after(&fig, &draws, &STROKES), [0, 0, 0, 0]);
    let image = fig.canvas().buffer_rgba().expect("drawn");
    assert_ne!(
        image.pixel(125, 275),
        Some([255; 4]),
        "the line from (100, 100) to (150, 150), blitted"
    );
}

#[test]
fn selectors_of_hidden_axes_show_no_changes() {
    let (fig, ax, draws) = drawn_axes();
    let span = SpanSelector::new(&ax, Orientation::Horizontal, SpanSelectorOptions::default());
    count_draws(&fig, &draws);
    span.set_extents((2.0, 4.0));
    assert_eq!(draws.get(), 1, "matplotlib's extents setter updates");
    ax.set_visible(false);
    span.set_extents((3.0, 5.0));
    assert_eq!(draws.get(), 1);
}
