use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::events::EventSource;
use crate::figure::Figure;

/// A 400 × 300 figure with one axes over the data square [0, 10]².
fn axes() -> (Figure, Axes) {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let ax = fig
        .add_axes([0.0, 0.0, 1.0, 1.0])
        .expect("a valid rectangle");
    ax.set_xlim(0.0, 10.0)
        .and_then(|ax| ax.set_ylim(0.0, 10.0))
        .expect("finite limits");
    (fig, ax)
}

fn drag(fig: &Figure, from: f64, to: f64) {
    let mut source = EventSource::new();
    source.button_press(fig, from * 40.0, 150.0, PointerButton::Left, false);
    source.motion(fig, to * 40.0, 150.0);
    source.button_release(fig, to * 40.0, 150.0, PointerButton::Left);
}

#[test]
fn dragging_selects_a_range() {
    let (fig, ax) = axes();
    let options = SpanSelectorOptions {
        interactive: true,
        ..SpanSelectorOptions::default()
    };
    let span = SpanSelector::new(&ax, Orientation::Horizontal, options);
    let log = Rc::new(RefCell::new(Vec::new()));
    let seen = Rc::clone(&log);
    span.on_select(move |a, b| seen.borrow_mut().push((a, b)));
    drag(&fig, 6.0, 2.0);
    assert_eq!(span.extents(), (2.0, 6.0));
    assert_eq!(*log.borrow(), [(2.0, 6.0)]);
    assert!(span.artist().visible() && span.handles()[1].visible());
    // Dragging the upper handle resizes the span.
    drag(&fig, 6.0, 8.0);
    assert_eq!(span.extents(), (2.0, 8.0));
    // A click outside removes it.
    drag(&fig, 9.5, 9.5);
    assert!(!span.artist().visible());
    assert_eq!(log.borrow().last(), Some(&(9.5, 9.5)));
}

#[test]
fn snapping_and_locks() {
    let (fig, ax) = axes();
    let options = SpanSelectorOptions {
        snap_values: Some(vec![0.0, 2.5, 5.0, 7.5, 10.0]),
        ..SpanSelectorOptions::default()
    };
    let span = SpanSelector::new(&ax, Orientation::Horizontal, options);
    drag(&fig, 1.0, 6.0);
    assert_eq!(span.extents(), (0.0, 5.0));
    // Spans are not interactive by default: the rectangle hides.
    assert!(!span.artist().visible());
    let tool = crate::events::LockOwner::new();
    fig.lock_widgets(tool).unwrap();
    drag(&fig, 2.0, 9.0);
    assert_eq!(span.extents(), (0.0, 5.0), "a locked figure is ignored");
    assert_eq!(
        span.add_state(SelectorState::Square),
        Err(SelectorError::UnsupportedState(SelectorState::Square))
    );
}

#[test]
fn snap_values_and_states_can_change() {
    let (_fig, ax) = axes();
    let options = SpanSelectorOptions {
        snap_values: Some(vec![0.0, 4.0, 8.0]),
        ..SpanSelectorOptions::default()
    };
    let span = SpanSelector::new(&ax, Orientation::Horizontal, options);
    assert!(!span.selection_completed());
    span.set_extents((1.0, 7.0));
    assert_eq!(span.extents(), (0.0, 8.0));
    assert!(span.selection_completed());
    span.set_snap_values(None);
    assert_eq!(span.snap_values(), None);
    span.set_extents((1.0, 7.0));
    assert_eq!(span.extents(), (1.0, 7.0));
    span.add_state(SelectorState::Move).unwrap();
    assert_eq!(span.states(), [SelectorState::Move]);
    span.clear();
    assert!(!span.selection_completed());
}

/// The selections reported by `span`, as they come.
fn selections(span: &SpanSelector) -> Rc<RefCell<Vec<(f64, f64)>>> {
    let log = Rc::new(RefCell::new(Vec::new()));
    let seen = Rc::clone(&log);
    span.on_select(move |a, b| seen.borrow_mut().push((a, b)));
    log
}

#[test]
fn new_axes_moves_the_span_and_its_events() {
    // Two axes side by side: [0, 10] over pixels 0 to 200, [100, 200] over
    // pixels 200 to 400.
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let left = fig
        .add_axes([0.0, 0.0, 0.5, 1.0])
        .expect("a valid rectangle");
    let right = fig
        .add_axes([0.5, 0.0, 0.5, 1.0])
        .expect("a valid rectangle");
    left.set_xlim(0.0, 10.0).expect("finite limits");
    right.set_xlim(100.0, 200.0).expect("finite limits");
    let options = SpanSelectorOptions {
        interactive: true,
        ..SpanSelectorOptions::default()
    };
    let span = SpanSelector::new(&left, Orientation::Horizontal, options);
    let log = selections(&span);
    span.set_extents((2.0, 4.0));
    span.set_props(PatchProps::default().facecolor("tab:green"));
    span.new_axes(&right);
    assert_eq!(span.ax(), right);
    assert!(!span.selection_completed(), "the span starts over");
    assert!(left.patches().is_empty() && left.lines().is_empty());
    assert_eq!(right.patches(), [span.artist().raw().clone()]);
    assert_eq!(right.lines().len(), 2, "the handles");
    assert!(!span.artist().visible());
    assert_eq!(
        span.artist().style().facecolor,
        Some(crate::colors::Color::from("tab:green")),
        "the look stays"
    );

    let mut source = EventSource::new();
    source.button_press(&fig, 50.0, 150.0, PointerButton::Left, false);
    source.motion(&fig, 150.0, 150.0);
    source.button_release(&fig, 150.0, 150.0, PointerButton::Left);
    assert!(log.borrow().is_empty(), "the left axes no longer selects");
    source.button_press(&fig, 240.0, 150.0, PointerButton::Left, false);
    source.motion(&fig, 300.0, 150.0);
    source.button_release(&fig, 300.0, 150.0, PointerButton::Left);
    assert_eq!(*log.borrow(), [(120.0, 150.0)]);
    assert_eq!(span.extents(), (120.0, 150.0));
    assert!(span.artist().visible() && span.handles()[1].visible());
}

#[test]
fn new_axes_follows_the_events_of_another_figure() {
    let (first, ax) = axes();
    let (second, other) = axes();
    other.set_xlim(0.0, 100.0).expect("finite limits");
    let span = SpanSelector::new(&ax, Orientation::Horizontal, SpanSelectorOptions::default());
    let log = selections(&span);
    span.new_axes(&other);
    assert_eq!(ax.patches(), [] as [crate::artist::RawHandle; 0]);
    drag(&first, 2.0, 4.0);
    assert!(log.borrow().is_empty(), "the first figure is left alone");
    drag(&second, 2.0, 4.0);
    assert_eq!(*log.borrow(), [(20.0, 40.0)]);
    // Back in the first figure, within the same one again.
    span.new_axes(&ax);
    span.new_axes(&ax);
    drag(&second, 2.0, 4.0);
    drag(&first, 1.0, 3.0);
    assert_eq!(*log.borrow(), [(20.0, 40.0), (1.0, 3.0)]);
}

#[test]
fn a_selection_callback_may_move_the_selector() {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let left = fig
        .add_axes([0.0, 0.0, 0.5, 1.0])
        .expect("a valid rectangle");
    let right = fig
        .add_axes([0.5, 0.0, 0.5, 1.0])
        .expect("a valid rectangle");
    let span = Rc::new(SpanSelector::new(
        &left,
        Orientation::Horizontal,
        SpanSelectorOptions::default(),
    ));
    let mover = Rc::downgrade(&span);
    let target = right.clone();
    span.on_select(move |_, _| {
        if let Some(span) = mover.upgrade() {
            span.new_axes(&target);
        }
    });
    let mut source = EventSource::new();
    source.button_press(&fig, 50.0, 150.0, PointerButton::Left, false);
    source.motion(&fig, 150.0, 150.0);
    source.button_release(&fig, 150.0, 150.0, PointerButton::Left);
    assert_eq!(span.ax(), right);
}
