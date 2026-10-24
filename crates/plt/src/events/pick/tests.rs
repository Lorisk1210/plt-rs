use std::cell::RefCell;

use super::*;
use crate::artist::Picker;
use crate::axes::Axes;
use crate::events::{EventKind, EventSource, LockOwner};
use crate::lines::{DrawStyle, LineStyle};
use crate::navigation::PointerButton;
use crate::patches::Rectangle;

/// No picks.
const NONE: [(ArtistId, Vec<usize>); 0] = [];

/// A 640 × 480 figure whose single axes maps data to display pixels.
fn pixel_axes() -> (Figure, Axes) {
    let fig = Figure::new();
    let ax = fig
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 640.0)
        .and_then(|ax| ax.set_ylim(0.0, 480.0))
        .expect("finite limits");
    (fig, ax)
}

/// The picks a press at `(x, y)` fires, as (artist id, indices).
fn picks_at(fig: &Figure, x: f64, y: f64) -> Vec<(ArtistId, Vec<usize>)> {
    let picked = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&picked);
    let id = fig.connect(EventKind::Pick, move |event| {
        if let Event::Pick(pick) = event {
            log.borrow_mut().push((pick.artist.id(), pick.ind.clone()));
        }
    });
    EventSource::new().button_press(fig, x, y, PointerButton::Left, false);
    fig.disconnect(id);
    picked.take()
}

#[test]
fn lines_report_segments_points_and_step_indices() {
    let (fig, ax) = pixel_axes();
    let line = ax
        .plot([100.0, 200.0, 300.0], [100.0, 100.0, 200.0])
        .unwrap();
    assert_eq!(picks_at(&fig, 150.0, 103.0), NONE, "not pickable yet");
    line.set_picker(Some(Picker::Default));
    assert!(line.pickable());
    // 5 points = 6.94 pixels at 100 dpi.
    assert_eq!(picks_at(&fig, 150.0, 106.0), [(line.id(), vec![0])]);
    assert_eq!(picks_at(&fig, 150.0, 108.0), NONE);
    assert_eq!(
        picks_at(&fig, 201.0, 101.0),
        [(line.id(), vec![1])],
        "a vertex"
    );
    line.set_picker(Some(Picker::Radius(10.0)));
    assert_eq!(picks_at(&fig, 150.0, 112.0), [(line.id(), vec![0])]);

    line.set_linestyle(LineStyle::None);
    assert_eq!(picks_at(&fig, 150.0, 100.0), NONE, "only the points count");
    assert_eq!(picks_at(&fig, 296.0, 196.0), [(line.id(), vec![2])]);

    line.set_linestyle(LineStyle::Solid)
        .set_drawstyle(DrawStyle::StepsPost);
    // The riser of the second step is drawn segment 3, data index 1.
    assert_eq!(picks_at(&fig, 300.0, 150.0), [(line.id(), vec![1])]);
}

#[test]
fn collections_patches_and_texts_are_picked() {
    let (fig, ax) = pixel_axes();
    let points = ax
        .scatter(vec![100.0, 110.0, 400.0], vec![100.0, 100.0, 300.0])
        .unwrap();
    points.set_picker(Some(Picker::Default));
    assert_eq!(picks_at(&fig, 105.0, 100.0), [(points.id(), vec![0, 1])]);
    assert_eq!(picks_at(&fig, 400.0, 300.0), [(points.id(), vec![2])]);

    let rect = ax.add_patch(Rectangle::new((200.0, 200.0), 50.0, 50.0));
    rect.set_picker(Some(Picker::Default));
    assert_eq!(picks_at(&fig, 225.0, 225.0), [(rect.id(), vec![])]);
    assert_eq!(
        picks_at(&fig, 250.5, 225.0),
        [(rect.id(), vec![])],
        "1 pixel"
    );
    assert_eq!(picks_at(&fig, 253.0, 225.0), NONE);

    let text = ax.text(500.0, 400.0, "label");
    text.set_picker(Some(Picker::Default));
    assert_eq!(picks_at(&fig, 505.0, 403.0), [(text.id(), vec![])]);
    text.set_visible(false);
    assert_eq!(picks_at(&fig, 505.0, 403.0), NONE);
}

#[test]
fn custom_pickers_decide_and_may_change_the_figure() {
    let (fig, ax) = pixel_axes();
    let line = ax.plot([0.0, 640.0], [0.0, 480.0]).unwrap();
    let asked = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&asked);
    line.set_picker(Some(Picker::custom(move |artist, mouse| {
        log.borrow_mut().push(artist.id());
        // The figure is not borrowed while the picker runs.
        artist.set_alpha(Some(0.5));
        (mouse.location.x > 320.0).then(|| vec![7])
    })));
    assert_eq!(picks_at(&fig, 100.0, 400.0), NONE);
    assert_eq!(picks_at(&fig, 500.0, 10.0), [(line.id(), vec![7])]);
    assert_eq!(*asked.borrow(), [line.id(), line.id()]);
    assert_eq!(line.alpha(), Some(0.5));
}

#[test]
fn other_axes_are_skipped_and_picks_come_before_presses() {
    let fig = Figure::new();
    let left = fig
        .add_axes([0.0, 0.0, 0.5, 1.0])
        .expect("a valid rectangle");
    let right = fig
        .add_axes([0.5, 0.0, 0.5, 1.0])
        .expect("a valid rectangle");
    right
        .set_xlim(0.0, 1.0)
        .and_then(|ax| ax.set_ylim(0.0, 1.0))
        .expect("finite limits");
    // Drawn across the left axes, but part of the right one.
    let stray = right.plot([-1.0, 0.0], [0.5, 0.5]).unwrap();
    stray.set_picker(Some(Picker::Radius(5.0)));
    left.set_picker(Some(Picker::Default));
    let order = Rc::new(RefCell::new(Vec::new()));
    let (picks, presses) = (Rc::clone(&order), Rc::clone(&order));
    fig.connect(EventKind::ButtonPress, move |_| {
        presses.borrow_mut().push("press");
    });
    let picked = left.clone();
    fig.connect(EventKind::Pick, move |event| {
        if let Event::Pick(pick) = event {
            assert!(pick.is(&picked), "the line of the right axes is skipped");
            picks.borrow_mut().push("pick");
        }
    });
    EventSource::new().button_press(&fig, 160.0, 240.0, PointerButton::Left, false);
    assert_eq!(*order.borrow(), ["pick", "press"]);

    fig.lock_widgets(LockOwner::new()).unwrap();
    EventSource::new().button_press(&fig, 160.0, 240.0, PointerButton::Left, false);
    assert_eq!(*order.borrow(), ["pick", "press", "press"]);
}
