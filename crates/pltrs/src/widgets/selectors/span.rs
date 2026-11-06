//! Selecting a range along one axis (matplotlib's `SpanSelector`).

use pltrs_core::Point;

use super::driver::{Core, Driver, Pointer, Tool};
use super::handles::LineHandles;
use super::{ModifierKeys, SelectorError, SelectorEvent, SelectorState, selector_methods};
use crate::artist::{ArtistHandle, RawHandle};
use crate::axes::Axes;
use crate::colors::Color;
use crate::events::CursorShape;
use crate::lines::{Line2D, LineProps};
use crate::navigation::PointerButton;
use crate::orientation::Orientation;
use crate::patches::{Patch, PatchProps, Rectangle};
use crate::widgets::{CallbackId, ordered};

/// Options of a [`SpanSelector`] (matplotlib's keyword arguments).
#[derive(Clone, Debug)]
// matplotlib's keyword arguments, each a switch of its own.
#[allow(clippy::struct_excessive_bools)]
pub struct SpanSelectorOptions {
    /// Spans no longer than this are removed (`minspan`).
    pub minspan: f64,
    /// Look of the span (`props`).
    pub props: PatchProps,
    /// Whether the span stays after the release, with handles to resize
    /// it (`interactive`).
    pub interactive: bool,
    /// The buttons that select; all when `None` (`button`).
    pub buttons: Option<Vec<PointerButton>>,
    /// Look of the handles (`handle_props`); unset colours follow the
    /// span's face colour.
    pub handle_props: LineProps,
    /// Distance in pixels within which a handle is grabbed (`grab_range`).
    pub grab_range: f64,
    /// The keys the selector reacts to (`state_modifier_keys`).
    pub modifier_keys: ModifierKeys,
    /// Whether a press anywhere inside the span moves it
    /// (`drag_from_anywhere`).
    pub drag_from_anywhere: bool,
    /// Whether presses outside the span leave it alone instead of starting
    /// a new one (`ignore_event_outside`).
    pub ignore_event_outside: bool,
    /// Values the edges snap to (`snap_values`).
    pub snap_values: Option<Vec<f64>>,
    /// Whether the span shows its changes by blitting its animated artists
    /// rather than by full draws (`useblit`).
    pub useblit: bool,
}

impl Default for SpanSelectorOptions {
    /// matplotlib's defaults: a red span at half opacity, removed when
    /// empty, without handles.
    fn default() -> Self {
        Self {
            minspan: 0.0,
            props: PatchProps::default().facecolor("red").alpha(0.5),
            interactive: false,
            buttons: None,
            handle_props: LineProps::default(),
            grab_range: 10.0,
            modifier_keys: ModifierKeys::span(),
            drag_from_anywhere: false,
            ignore_event_outside: false,
            snap_values: None,
            useblit: false,
        }
    }
}

/// What a press grabbed (matplotlib's `_active_handle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grab {
    /// The lower edge.
    Min,
    /// The upper edge.
    Max,
    /// The whole span.
    Center,
}

/// The span selector's own state.
#[derive(Debug)]
struct Span {
    direction: Orientation,
    rect: Patch<Rectangle>,
    /// The span's look, for the rectangle of a new direction.
    props: PatchProps,
    handles: Option<LineHandles>,
    handle_props: LineProps,
    minspan: f64,
    grab_range: f64,
    interactive: bool,
    drag_from_anywhere: bool,
    ignore_event_outside: bool,
    snap_values: Option<Vec<f64>>,
    grab: Option<Grab>,
    extents_on_press: Option<(f64, f64)>,
}

/// A hidden span rectangle along `direction`, spanning the axes across it.
fn span_rect(ax: &Axes, direction: Orientation, props: &PatchProps) -> Patch<Rectangle> {
    let (shape, transform) = match direction {
        Orientation::Horizontal => (Rectangle::new((0.0, 0.0), 0.0, 1.0), ax.trans_xaxis()),
        Orientation::Vertical => (Rectangle::new((0.0, 0.0), 1.0, 0.0), ax.trans_yaxis()),
    };
    let rect = ax.add_patch_in(shape, transform, (false, false));
    rect.set_visible(false);
    rect.update_props(props.clone());
    rect
}

/// Each of `values` replaced by the closest of `snap_values` (matplotlib's
/// `SpanSelector._snap`).
fn snap((a, b): (f64, f64), snap_values: &[f64]) -> (f64, f64) {
    // Like matplotlib, nudge ties by the machine precision.
    let eps = snap_values
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .reduce(f64::min)
        .unwrap_or(0.0)
        * 1e-12;
    let closest = |v: f64| {
        let sign = if v > 0.0 {
            1.0
        } else if v < 0.0 {
            -1.0
        } else {
            0.0
        };
        snap_values
            .iter()
            .copied()
            .map(|s| (s, (s - v + sign * eps).abs()))
            .reduce(|best, next| if next.1 < best.1 { next } else { best })
            .map_or(v, |(s, _)| s)
    };
    (closest(a), closest(b))
}

impl Span {
    /// The span's ends in data coordinates (matplotlib's `extents`).
    fn extents(&self) -> (f64, f64) {
        let rect = self.rect.shape();
        match self.direction {
            Orientation::Horizontal => (rect.xy.x, rect.xy.x + rect.width),
            Orientation::Vertical => (rect.xy.y, rect.xy.y + rect.height),
        }
    }

    /// Shows the span from `vmin` to `vmax` (matplotlib's `_draw_shape`).
    fn draw_shape(&self, (vmin, vmax): (f64, f64)) {
        let (vmin, vmax) = ordered((vmin, vmax));
        let direction = self.direction;
        self.rect.update_shape(|rect| match direction {
            Orientation::Horizontal => {
                rect.xy.x = vmin;
                rect.width = vmax - vmin;
            }
            Orientation::Vertical => {
                rect.xy.y = vmin;
                rect.height = vmax - vmin;
            }
        });
    }

    /// Moves the span and its handles (matplotlib's `_set_extents`).
    fn set_extents(&self, core: &Core, extents: (f64, f64)) {
        let extents = self
            .snap_values
            .as_deref()
            .map_or(extents, |values| snap(extents, values));
        self.draw_shape(extents);
        if let Some(handles) = &self.handles {
            handles.set_data(self.extents());
        }
        self.show(core.visible);
        core.request_update();
    }

    /// Replaces the span's rectangle and handles by hidden ones of the same
    /// look on `ax`, along the current direction (matplotlib's `new_axes`
    /// and `_setup_edge_handles`).
    fn rebuild(&mut self, ax: &Axes) {
        self.rect.remove();
        if let Some(handles) = &self.handles {
            handles.remove();
        }
        self.rect = span_rect(ax, self.direction, &self.props);
        if self.interactive {
            self.handles = Some(Self::edge_handles(ax, self.direction, &self.handle_props));
        }
    }

    /// Hidden handles at the current view edges (matplotlib's
    /// `_setup_edge_handles`).
    fn edge_handles(ax: &Axes, direction: Orientation, props: &LineProps) -> LineHandles {
        let limits = match direction {
            Orientation::Horizontal => ax.xlim(),
            Orientation::Vertical => ax.ylim(),
        };
        let (lo, hi) = ordered(limits);
        LineHandles::new(ax, [lo, hi], direction, props)
    }

    /// Asks for the resize cursor of the direction, or the pointer.
    fn set_cursor(&self, core: &Core, resizing: bool) {
        let cursor = match (resizing, self.direction) {
            (false, _) => CursorShape::Pointer,
            (true, Orientation::Horizontal) => CursorShape::ResizeHorizontal,
            (true, Orientation::Vertical) => CursorShape::ResizeVertical,
        };
        core.ax.figure().set_cursor(cursor);
    }

    /// The coordinate along the direction.
    fn along(&self, event: &SelectorEvent) -> f64 {
        match self.direction {
            Orientation::Horizontal => event.xdata,
            Orientation::Vertical => event.ydata,
        }
    }

    /// What a press at `event` grabs (matplotlib's `_set_active_handle`).
    fn set_grab(&mut self, core: &Core, event: &SelectorEvent) {
        let closest = self
            .handles
            .as_ref()
            .and_then(|handles| handles.closest(event.x, event.y));
        self.grab = if core.state.contains(&SelectorState::Move) {
            Some(Grab::Center)
        } else {
            match closest {
                Some((index, distance)) if distance <= self.grab_range => {
                    Some(if index == 0 { Grab::Min } else { Grab::Max })
                }
                _ if self.drag_from_anywhere
                    && self
                        .rect
                        .contains_display_point(Point::new(event.x, event.y)) =>
                {
                    Some(Grab::Center)
                }
                _ => None,
            }
        };
        if self.grab.is_some() {
            self.extents_on_press = Some(self.extents());
        }
    }
}

impl Tool for Span {
    type Selection = (f64, f64);

    fn artists(&self) -> Vec<RawHandle> {
        let handles = self.handles.iter().flat_map(LineHandles::lines);
        std::iter::once(self.rect.raw())
            .chain(handles.map(ArtistHandle::raw))
            .cloned()
            .collect()
    }

    fn show(&self, visible: bool) {
        self.rect.set_visible(visible);
        if let Some(handles) = &self.handles {
            handles.set_visible(visible);
        }
    }

    fn press(&mut self, core: &mut Core, event: &SelectorEvent) {
        self.set_cursor(core, true);
        if self.interactive && self.rect.visible() {
            self.set_grab(core, event);
        } else {
            self.grab = None;
        }
        if self.grab.is_none() || !self.interactive {
            // The span pressed outside of goes before a new one starts.
            core.request_update();
        }
        let v = self.along(event);
        if self.grab.is_none() && !self.ignore_event_outside {
            // A press outside the span starts a new, still hidden, one.
            core.visible = false;
            self.set_extents(core, (v, v));
            core.visible = true;
        } else {
            core.visible = true;
            self.show(true);
        }
    }

    fn drag(&mut self, core: &mut Core, event: &SelectorEvent) -> Option<(f64, f64)> {
        let v = self.along(event);
        let vpress = core
            .eventpress
            .as_ref()
            .map_or(v, |press| self.along(press));
        let (vmin, vmax) = match (self.grab, self.extents_on_press) {
            (Some(Grab::Center), Some((vmin, vmax))) => (vmin + v - vpress, vmax + v - vpress),
            (Some(Grab::Min), Some((_, vmax))) => (v, vmax),
            (Some(Grab::Max), Some((vmin, _))) => (vmin, v),
            _ => {
                if self.ignore_event_outside && core.selection_completed {
                    return None;
                }
                ordered((vpress, v))
            }
        };
        self.set_extents(core, (vmin, vmax));
        Some((vmin, vmax))
    }

    fn release(&mut self, core: &mut Core, _event: &SelectorEvent) -> Option<(f64, f64)> {
        self.set_cursor(core, false);
        if !self.interactive {
            self.rect.set_visible(false);
        }
        if self.grab.is_none() && core.selection_completed && self.ignore_event_outside {
            return None;
        }
        let (vmin, vmax) = self.extents();
        let selection = if vmax - vmin <= self.minspan {
            core.visible = false;
            self.show(false);
            let existed = core.selection_completed;
            core.selection_completed = false;
            // Only removing an existing span is reported.
            existed.then_some((vmin, vmax))
        } else {
            core.selection_completed = true;
            Some((vmin, vmax))
        };
        core.request_update();
        self.grab = None;
        selection
    }

    /// Dragging, then (for interactive spans) the resize cursor over a
    /// handle (matplotlib's `_hover`).
    fn motion(&mut self, core: &mut Core, pointer: &Pointer) -> Option<(f64, f64)> {
        let moved = if core.ignore(pointer) || core.eventpress.is_none() {
            None
        } else {
            core.clean(pointer)
                .and_then(|event| self.drag(core, &event))
        };
        if self.interactive
            && !core.ignore(pointer)
            && self.grab.is_none()
            && core.selection_completed
            && let Some((_, distance)) = self
                .handles
                .as_ref()
                .and_then(|handles| handles.closest(pointer.x, pointer.y))
        {
            self.set_cursor(core, distance <= self.grab_range);
        }
        moved
    }
}

/// A span along one axis selected by dragging the mouse (matplotlib's
/// `SpanSelector`). Its callbacks receive the span's ends in data
/// coordinates.
///
/// ```
/// use pltrs::figure::Figure;
/// use pltrs::widgets::{Orientation, SpanSelector, SpanSelectorOptions};
/// let ax = Figure::new().subplot();
/// let span = SpanSelector::new(&ax, Orientation::Horizontal, SpanSelectorOptions::default());
/// span.on_select(|vmin, vmax| println!("selected {vmin}..{vmax}"));
/// span.set_extents((0.2, 0.6));
/// assert_eq!(span.extents(), (0.2, 0.6));
/// ```
#[derive(Debug)]
pub struct SpanSelector {
    driver: Driver<Span>,
}

selector_methods!(SpanSelector);

impl SpanSelector {
    /// A span selector on `ax` along `direction`.
    pub fn new(ax: &Axes, direction: Orientation, options: SpanSelectorOptions) -> Self {
        let face = options
            .props
            .facecolor
            .clone()
            .unwrap_or_else(|| Color::from("r"));
        let handle_props = LineProps::default()
            .color(face)
            .overlay(options.handle_props.clone());
        let rect = span_rect(ax, direction, &options.props);
        let handles = options
            .interactive
            .then(|| Span::edge_handles(ax, direction, &handle_props));
        let span = Span {
            direction,
            rect,
            props: options.props.clone(),
            handles,
            handle_props,
            minspan: options.minspan,
            grab_range: options.grab_range,
            interactive: options.interactive,
            drag_from_anywhere: options.drag_from_anywhere,
            ignore_event_outside: options.ignore_event_outside,
            snap_values: options.snap_values.clone(),
            grab: None,
            extents_on_press: None,
        };
        let core = Core::new(
            ax,
            options.buttons.clone(),
            options.modifier_keys.clone(),
            options.useblit,
        );
        Self {
            driver: Driver::new(core, span),
        }
    }

    /// Calls `callback` with the span's ends after each release that makes,
    /// changes or removes a span (matplotlib's `onselect`); returns its id
    /// for [`SpanSelector::disconnect`].
    pub fn on_select(&self, mut callback: impl FnMut(f64, f64) + 'static) -> CallbackId {
        self.driver
            .selected
            .connect(move |&(vmin, vmax)| callback(vmin, vmax))
    }

    /// Calls `callback` with the span's ends while it is dragged
    /// (matplotlib's `onmove_callback`); returns its id for
    /// [`SpanSelector::disconnect`].
    pub fn on_move(&self, mut callback: impl FnMut(f64, f64) + 'static) -> CallbackId {
        self.driver
            .moved
            .connect(move |&(vmin, vmax)| callback(vmin, vmax))
    }

    /// Disconnects the callback with id `id`, whichever signal it was
    /// connected to; returns whether it was connected.
    pub fn disconnect(&self, id: CallbackId) -> bool {
        self.driver.disconnect(id)
    }

    /// The span's ends in data coordinates; equal without a span
    /// (matplotlib's `extents`).
    pub fn extents(&self) -> (f64, f64) {
        self.driver.shared.borrow().tool.extents()
    }

    /// Shows a completed span between data values `extents`, snapped to the
    /// snap values (matplotlib's `extents` setter).
    pub fn set_extents(&self, extents: (f64, f64)) -> &Self {
        {
            let mut shared = self.driver.shared.borrow_mut();
            let shared = &mut *shared;
            shared.tool.set_extents(&shared.core, extents);
            shared.core.selection_completed = true;
        }
        super::blit::run_requested();
        self
    }

    /// The values the edges snap to, if any (matplotlib's
    /// `snap_values`).
    pub fn snap_values(&self) -> Option<Vec<f64>> {
        self.driver.shared.borrow().tool.snap_values.clone()
    }

    /// Snaps the edges to `values` from now on, or not at all with `None`
    /// (matplotlib's `snap_values` attribute).
    pub fn set_snap_values(&self, values: Option<Vec<f64>>) -> &Self {
        self.driver.shared.borrow_mut().tool.snap_values = values;
        self
    }

    /// The direction spans are selected along.
    pub fn direction(&self) -> Orientation {
        self.driver.shared.borrow().tool.direction
    }

    /// Selects along `direction` from now on; the span starts over
    /// (matplotlib's `direction` setter).
    pub fn set_direction(&self, direction: Orientation) -> &Self {
        {
            let mut shared = self.driver.shared.borrow_mut();
            let shared = &mut *shared;
            let span = &mut shared.tool;
            if span.direction == direction {
                return self;
            }
            let visible = span.rect.visible();
            span.direction = direction;
            shared.core.selection_completed = false;
            span.rebuild(&shared.core.ax);
            span.rect.set_visible(visible);
        }
        self.driver.animate_artists();
        self
    }

    /// Makes the selector work in `ax` from now on, which may belong to
    /// another figure (matplotlib's `new_axes`): the span starts over,
    /// hidden, with its rectangle and handles moved to `ax` in their look,
    /// and presses in `ax` select. matplotlib leaves the old rectangle
    /// and handles on the old axes, and gives the new rectangle the
    /// coordinates of the old axes; pltrs removes them and places the
    /// span in the data of `ax`.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::widgets::{Orientation, SpanSelector, SpanSelectorOptions};
    /// let fig = Figure::new();
    /// let (left, right) = (fig.add_subplot(1, 2, 1)?, fig.add_subplot(1, 2, 2)?);
    /// let span = SpanSelector::new(&left, Orientation::Horizontal, SpanSelectorOptions::default());
    /// span.new_axes(&right);
    /// assert_eq!(span.ax(), right);
    /// assert!(left.patches().is_empty());
    /// assert_eq!(right.patches().len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new_axes(&self, ax: &Axes) {
        {
            let mut shared = self.driver.shared.borrow_mut();
            let shared = &mut *shared;
            shared.core.selection_completed = false;
            shared.tool.rebuild(ax);
        }
        self.driver.move_to(ax);
    }

    /// The rectangle showing the span.
    pub fn artist(&self) -> Patch<Rectangle> {
        self.driver.shared.borrow().tool.rect.clone()
    }

    /// The handle lines of an interactive span.
    pub fn handles(&self) -> Vec<Line2D> {
        self.driver
            .shared
            .borrow()
            .tool
            .handles
            .as_ref()
            .map_or_else(Vec::new, |handles| handles.lines().to_vec())
    }

    /// Changes the span's look (matplotlib's `set_props`).
    pub fn set_props(&self, props: PatchProps) -> &Self {
        {
            let mut shared = self.driver.shared.borrow_mut();
            let shared = &mut *shared;
            let span = &mut shared.tool;
            span.rect.update_props(props.clone());
            span.props = std::mem::take(&mut span.props).overlay(props);
            shared.core.request_blit();
        }
        super::blit::run_requested();
        self
    }

    /// Changes the look of the handles (matplotlib's `set_handle_props`).
    ///
    /// # Errors
    ///
    /// When the span is not interactive and has no handles.
    pub fn set_handle_props(&self, props: LineProps) -> Result<&Self, SelectorError> {
        {
            let mut shared = self.driver.shared.borrow_mut();
            let shared = &mut *shared;
            let span = &mut shared.tool;
            let handles = span.handles.as_ref().ok_or(SelectorError::NoHandles)?;
            handles.update_props(&props);
            span.handle_props = std::mem::take(&mut span.handle_props).overlay(props);
            shared.core.request_blit();
        }
        super::blit::run_requested();
        Ok(self)
    }
}

#[cfg(test)]
mod tests;
