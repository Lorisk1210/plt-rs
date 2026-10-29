//! User code that pltrs runs may use the handles of the figure it runs
//! for: reading and changing other artists from a closure or a trait
//! method never panics with a figure already borrowed.
//!
//! Each test re-enters the figure from one public API that runs user
//! code. The APIs that run user code while the figure is laid out or
//! drawn (formatters, scale and norm functions, axes locators, image
//! filters, colorbar label functions, legend handlers and projections)
//! take `Send` closures and types instead, which cannot hold a handle:
//! that is checked at compile time, not here.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use pltrs::animation::{AnimationOptions, Frames, FuncAnimation};
use pltrs::artist::{Artist, ArtistBase, DrawContext, Picker};
use pltrs::axes::{AxisName, ProjectionKind};
use pltrs::events::{Event, EventKind, EventSource};
use pltrs::gui::{Icon, ToolKind, ToolSpec};
use pltrs::navigation::PointerButton;
use pltrs::patches::Rectangle;
use pltrs::prelude::*;
use pltrs::projections::{self, Projection, ProjectionView};
use pltrs::transforms::ResolvedTransform;
use pltrs::units::{ConversionError, ConversionInterface, Convertible, UnitAxis};
use pltrs::widgets::{Orientation, Slider, SliderOptions, SpanSelector, SpanSelectorOptions};

/// A small figure with one line.
fn figure_with_line() -> (Figure, Axes, Line2D) {
    let fig = Figure::with_size(2.0, 2.0, 50.0);
    let ax = fig.subplot();
    let line = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("valid data");
    (fig, ax, line)
}

/// Reads and changes the figure through other handles, as user code may.
fn touch(ax: &Axes, line: &Line2D) {
    let width = line.linewidth();
    line.set_linewidth(width + 1.0);
    let _ = ax.xlim();
    ax.set_title("touched");
}

/// An artist of the tests' own, which reads another artist when drawn.
#[derive(Clone, Debug)]
struct Watcher {
    base: ArtistBase,
    watched: Line2D,
    seen: Rc<Cell<f64>>,
}

impl Artist for Watcher {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, _ctx: &mut DrawContext<'_>) {
        self.seen.set(self.watched.linewidth());
    }
}

#[test]
fn custom_artists_are_read_and_changed_through_copies() {
    let (fig, ax, line) = figure_with_line();
    let initial = line.linewidth();
    let seen = Rc::new(Cell::new(0.0));
    let watcher = ax.add_custom_artist(Watcher {
        base: ArtistBase::default(),
        watched: line.clone(),
        seen: Rc::clone(&seen),
    });
    watcher.update(|watcher| {
        touch(&ax, &line);
        watcher.base.zorder = 5.0;
    });
    let copy = watcher.get();
    touch(&ax, &copy.watched);
    assert_eq!(watcher.zorder(), 5.0);
    fig.to_png().expect("drawable");
    assert_eq!(
        seen.get(),
        initial + 2.0,
        "draw read the line, widened twice"
    );
}

#[test]
fn patch_shapes_change_through_copies() {
    let (_, ax, line) = figure_with_line();
    let rect = ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
    rect.update_shape(|shape| {
        touch(&ax, &line);
        shape.width = rect.shape().width * 2.0;
    });
    assert_eq!(rect.shape().width, 2.0);
}

/// A rectilinear projection with a setting of its own.
#[derive(Clone, Debug)]
struct Tilted {
    degrees: f64,
}

impl Projection for Tilted {
    fn name(&self) -> &'static str {
        "reentrancy_tilted"
    }

    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        view.rectilinear()
    }
}

#[test]
fn projections_are_read_and_changed_through_copies() {
    let kind = projections::register(|| Tilted { degrees: 10.0 }).expect("registered");
    assert_eq!(kind, ProjectionKind::Registered("reentrancy_tilted"));
    let fig = Figure::with_size(2.0, 2.0, 50.0);
    let ax = fig.subplot_with_projection(kind).expect("made");
    let line = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("valid data");
    let degrees = ax.with_projection(|tilted: &Tilted| {
        touch(&ax, &line);
        tilted.degrees
    });
    assert_eq!(degrees, Some(10.0));
    ax.with_projection_mut(|tilted: &mut Tilted| {
        touch(&ax, &line);
        tilted.degrees = 20.0;
    });
    assert_eq!(
        ax.with_projection(|tilted: &Tilted| tilted.degrees),
        Some(20.0)
    );
}

#[test]
fn rc_updates_may_read_the_parameters_and_make_figures() {
    pltrs::rc::context(|| {
        pltrs::rc::update(|params| {
            params.lines_linewidth = pltrs::rc::params().lines_linewidth * 2.0;
            let (_, _, line) = figure_with_line();
            assert_ne!(
                line.linewidth(),
                params.lines_linewidth,
                "made before the change"
            );
        });
        let (_, _, line) = figure_with_line();
        assert_eq!(line.linewidth(), pltrs::rc::params().lines_linewidth);
    });
}

#[test]
fn event_callbacks_may_use_the_figure() {
    let (fig, ax, line) = figure_with_line();
    let initial = line.linewidth();
    let presses = Rc::new(Cell::new(0));
    let (seen, inner_ax, inner_line) = (Rc::clone(&presses), ax.clone(), line.clone());
    fig.connect(EventKind::ButtonPress, move |_| {
        touch(&inner_ax, &inner_line);
        seen.set(seen.get() + 1);
    });
    let (drawn_ax, drawn_line) = (ax.clone(), line.clone());
    fig.connect(EventKind::Draw, move |_| touch(&drawn_ax, &drawn_line));
    let at = ax.data_to_display((0.5, 0.5)).expect("laid out");
    EventSource::new().button_press(&fig, at.x, at.y, PointerButton::Left, false);
    fig.to_png().expect("drawable");
    assert_eq!(presses.get(), 1);
    assert!(
        line.linewidth() >= initial + 2.0,
        "touched on the press and the draws"
    );
}

#[test]
fn timers_tools_and_widgets_may_use_the_figure() {
    let (fig, ax, line) = figure_with_line();
    let initial = line.linewidth();
    let (timer_ax, timer_line) = (ax.clone(), line.clone());
    let timer = fig.add_timer(Duration::from_millis(10), move || {
        touch(&timer_ax, &timer_line);
        true
    });
    assert!(fig.fire_timer(timer));

    let spec = ToolSpec {
        name: "touch".into(),
        description: "Touches the line".into(),
        icon: Icon::new(24.0),
        keys: Vec::new(),
        kind: ToolKind::Trigger,
        ..ToolSpec::default()
    };
    let tool_line = line.clone();
    fig.add_tool(spec, move |event| {
        touch(&event.figure.axes()[0], &tool_line);
    })
    .expect("a new tool");
    assert!(fig.trigger_tool("touch"));

    let slider_axes = fig.add_axes([0.1, 0.0, 0.8, 0.1]).expect("a rectangle");
    let slider =
        Slider::new(&slider_axes, "s", 0.0, 1.0, SliderOptions::default()).expect("valid options");
    let (slider_ax, slider_line) = (ax.clone(), line.clone());
    slider.on_changed(move |_| touch(&slider_ax, &slider_line));
    slider.set_val(0.5);
    assert_eq!(line.linewidth(), initial + 3.0, "touched three times");
}

#[test]
fn animation_frames_may_use_the_figure() {
    let (fig, ax, line) = figure_with_line();
    let frames = Rc::new(Cell::new(0));
    let seen = Rc::clone(&frames);
    let animated = line.clone();
    let animation = FuncAnimation::new(&fig, Frames::Count(2), AnimationOptions::default(), {
        move |_| {
            touch(&ax, &animated);
            seen.set(seen.get() + 1);
            animated.clone()
        }
    });
    animation.to_gif().expect("encoded");
    assert!(frames.get() >= 2);
}

#[test]
fn blocked_callbacks_may_use_the_figure_and_block_from_callbacks() {
    let (fig, ax, line) = figure_with_line();
    let initial = line.linewidth();
    let closes = Rc::new(Cell::new(0));
    let seen = Rc::clone(&closes);
    fig.connect(EventKind::Close, move |_| seen.set(seen.get() + 1));
    let (inner, inner_ax, inner_line) = (fig.clone(), ax.clone(), line.clone());
    // A press blocks the close callbacks while it sends a close event.
    fig.connect(EventKind::ButtonPress, move |_| {
        inner.with_callbacks_blocked(Some(EventKind::Close), || {
            touch(&inner_ax, &inner_line);
            inner.process_event(&Event::Close);
        });
    });
    let at = ax.data_to_display((0.5, 0.5)).expect("laid out");
    EventSource::new().button_press(&fig, at.x, at.y, PointerButton::Left, false);
    let drawn = fig.with_callbacks_blocked(None, || {
        touch(&ax, &line);
        fig.to_png().is_ok()
    });
    assert!(drawn);
    fig.process_event(&Event::Close);
    assert_eq!(closes.get(), 1, "only the unblocked close");
    assert_eq!(line.linewidth(), initial + 2.0);
}

#[test]
fn animations_are_paused_from_their_own_frames() {
    let (fig, ax, line) = figure_with_line();
    let slot: Rc<RefCell<Option<FuncAnimation>>> = Rc::default();
    let animation: Weak<RefCell<Option<FuncAnimation>>> = Rc::downgrade(&slot);
    let frames = Rc::new(Cell::new(0));
    let seen = Rc::clone(&frames);
    let playing = FuncAnimation::new(
        &fig,
        Frames::Endless,
        AnimationOptions::default(),
        move |i| {
            touch(&ax, &line);
            seen.set(seen.get() + 1);
            if i == 2
                && let Some(slot) = animation.upgrade()
                && let Some(animation) = slot.borrow().as_ref()
            {
                animation.pause();
            }
        },
    );
    *slot.borrow_mut() = Some(playing);
    fig.canvas().draw().expect("drawable");
    let (timer, _) = fig.timers()[0];
    assert!(fig.fire_timer(timer) && fig.fire_timer(timer));
    assert!(!fig.fire_timer(timer), "frame 2 paused the animation");
    assert_eq!(fig.timers(), []);
    assert_eq!(frames.get(), 4, "the initial frame, then frames 0 to 2");
    if let Some(animation) = slot.borrow().as_ref() {
        animation.resume();
    }
    assert_eq!(fig.timers().len(), 1);
}

#[test]
fn selectors_move_to_other_axes_from_their_callbacks() {
    let (fig, ax, line) = figure_with_line();
    ax.set_xlim(0.0, 1.0).expect("valid limits");
    let other = fig.add_subplot(1, 2, 2).expect("a valid cell");
    let span = Rc::new(SpanSelector::new(
        &ax,
        Orientation::Horizontal,
        SpanSelectorOptions::default(),
    ));
    let (mover, target, touched) = (Rc::downgrade(&span), other.clone(), line.clone());
    let selector_ax = ax.clone();
    span.on_select(move |_, _| {
        touch(&selector_ax, &touched);
        if let Some(span) = mover.upgrade() {
            span.new_axes(&target);
        }
    });
    let from = ax.data_to_display((0.2, 0.5)).expect("laid out");
    let to = ax.data_to_display((0.6, 0.5)).expect("laid out");
    let mut source = EventSource::new();
    source.button_press(&fig, from.x, from.y, PointerButton::Left, false);
    source.motion(&fig, to.x, to.y);
    source.button_release(&fig, to.x, to.y, PointerButton::Left);
    assert_eq!(span.ax(), other);
}

#[test]
fn pickers_and_findobj_may_use_the_figure() {
    let (fig, ax, line) = figure_with_line();
    let initial = line.linewidth();
    let (picker_ax, picker_line) = (ax.clone(), line.clone());
    line.set_picker(Some(Picker::custom(move |_, _| {
        touch(&picker_ax, &picker_line);
        None
    })));
    let at = ax.data_to_display((0.5, 0.5)).expect("laid out");
    EventSource::new().button_press(&fig, at.x, at.y, PointerButton::Left, false);
    let found = fig.findobj(|_| {
        touch(&ax, &line);
        false
    });
    assert_eq!(found, [] as [pltrs::artist::RawHandle; 0]);
    assert!(
        line.linewidth() >= initial + 2.0,
        "touched by the picker and the search"
    );
}

/// Prices, converted by a converter that reads and changes its axis and
/// another artist.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Price(f64);

thread_local! {
    /// The artist the converter touches.
    static TOUCHED: RefCell<Option<(Axes, Line2D)>> = const { RefCell::new(None) };
}

struct PriceConverter;

impl ConversionInterface for PriceConverter {
    type Value = Price;
    type Unit = ();

    fn convert(
        &self,
        values: &[Price],
        _: Option<&()>,
        axis: &UnitAxis,
    ) -> Result<Vec<f64>, ConversionError> {
        if let UnitAxis::Axes(axis) = axis {
            axis.set_inverted(axis.inverted());
        }
        TOUCHED.with_borrow(|touched| {
            if let Some((ax, line)) = touched {
                touch(ax, line);
            }
        });
        Ok(values.iter().map(|price| price.0).collect())
    }
}

impl Convertible for Price {
    type Converter = PriceConverter;

    fn converter() -> PriceConverter {
        PriceConverter
    }
}

#[test]
fn unit_converters_may_use_the_figure() {
    let (fig, ax, line) = figure_with_line();
    let initial = line.linewidth();
    TOUCHED.with_borrow_mut(|touched| *touched = Some((ax.clone(), line.clone())));
    ax.plot([1.0, 2.0], vec![Price(1.0), Price(2.0)])
        .expect("converted");
    ax.text(1.5, Price(1.5), "converted again when drawn");
    fig.to_png().expect("drawable");
    TOUCHED.with_borrow_mut(|touched| *touched = None);
    assert!(line.linewidth() > initial, "touched by the conversions");
}

#[test]
fn limit_callbacks_may_use_the_figure() {
    let (fig, ax, line) = figure_with_line();
    let other = fig.add_subplot(1, 2, 2).expect("a valid cell");
    let initial = line.linewidth();
    let (touched_ax, touched_line, follower) = (ax.clone(), line.clone(), other.clone());
    ax.on_limits_changed(AxisName::X, move |changed| {
        touch(&touched_ax, &touched_line);
        // Keeping two axes in step changes limits from inside a callback.
        let (low, high) = changed.xlim();
        let _ = follower.set_xlim(low, high);
    });
    ax.set_xlim(2.0, 5.0).expect("valid limits");
    fig.to_png().expect("drawable");
    assert!(line.linewidth() > initial, "touched by the callback");
    assert_eq!(other.xlim(), (2.0, 5.0));
}
