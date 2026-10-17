//! The [`Widget`] trait, the [`WidgetPod`]s that hold widgets in the tree,
//! and the contexts widgets receive.

use std::any::Any;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::canvas::Canvas;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::theme::Theme;

/// Identifies a widget, unique within the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(u64);

impl WidgetId {
    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// A node of the UI: something that takes space, paints itself and
/// reacts to input. `M` is the type of the messages widgets send to the
/// application (e.g. "the Home button was clicked").
///
/// Layout has two passes: [`measure`](Self::measure) picks a size within
/// the parent's constraints (containers measure their children), then
/// [`arrange`](Self::arrange) receives the final rectangle (containers
/// arrange their children inside it). The engine paints a widget before
/// its children and routes each event to a single widget; see
/// [`UiEvent`] for the routing rules.
pub trait Widget<M>: Any {
    /// Chooses a size within `constraints`.
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError>;

    /// Takes the final rectangle; containers arrange their children here.
    fn arrange(&mut self, _cx: &mut LayoutCx<'_, M>, _rect: Rect) -> Result<(), UiError> {
        Ok(())
    }

    /// Paints the widget (not its children) into `rect`.
    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError>;

    /// Reacts to an input event; `rect` is the widget's rectangle.
    fn event(&mut self, _cx: &mut EventCx<'_, M>, _event: &UiEvent, _rect: Rect) {}

    /// Whether clicking the widget gives it keyboard focus.
    fn accepts_focus(&self) -> bool {
        false
    }

    /// The child widgets, in painting order (later ones are on top).
    fn children(&self) -> &[WidgetPod<M>] {
        &[]
    }

    /// The child widgets, mutably.
    fn children_mut(&mut self) -> &mut [WidgetPod<M>] {
        &mut []
    }

    /// Every child widget, including those not shown (the hidden pages
    /// of a [`Stack`](crate::Stack)), for finding widgets by id.
    fn all_children(&self) -> &[WidgetPod<M>] {
        self.children()
    }

    /// Every child widget, mutably.
    fn all_children_mut(&mut self) -> &mut [WidgetPod<M>] {
        self.children_mut()
    }
}

/// A widget in the tree, with its identity and laid-out rectangle.
pub struct WidgetPod<M> {
    id: WidgetId,
    rect: Rect,
    widget: Box<dyn Widget<M>>,
}

impl<M: 'static> WidgetPod<M> {
    /// Wraps a widget, giving it a fresh id.
    pub fn new(widget: impl Widget<M>) -> Self {
        Self {
            id: WidgetId::next(),
            rect: Rect::default(),
            widget: Box::new(widget),
        }
    }

    /// The widget's id.
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// The rectangle from the last layout, in window pixels.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Measures the widget (see [`Widget::measure`]).
    pub fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        self.widget.measure(cx, constraints)
    }

    /// Places the widget at `rect` (see [`Widget::arrange`]).
    pub fn arrange(&mut self, cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        self.rect = rect;
        self.widget.arrange(cx, rect)
    }

    /// The widget.
    pub fn widget(&self) -> &dyn Widget<M> {
        self.widget.as_ref()
    }

    /// The widget, mutably.
    pub fn widget_mut(&mut self) -> &mut dyn Widget<M> {
        self.widget.as_mut()
    }

    /// The widget as its concrete type, if it is a `W`.
    pub fn downcast_mut<W: Widget<M>>(&mut self) -> Option<&mut W> {
        let widget: &mut dyn Any = self.widget.as_mut();
        widget.downcast_mut()
    }

    /// The pod with id `id` in this subtree.
    pub(crate) fn find(&self, id: WidgetId) -> Option<&Self> {
        if self.id == id {
            return Some(self);
        }
        self.widget
            .all_children()
            .iter()
            .find_map(|child| child.find(id))
    }

    /// The pod with id `id` in this subtree, mutably.
    pub(crate) fn find_mut(&mut self, id: WidgetId) -> Option<&mut Self> {
        if self.id == id {
            return Some(self);
        }
        self.widget
            .all_children_mut()
            .iter_mut()
            .find_map(|child| child.find_mut(id))
    }
}

impl<M> fmt::Debug for WidgetPod<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WidgetPod")
            .field("id", &self.id)
            .field("rect", &self.rect)
            .finish_non_exhaustive()
    }
}

/// What widgets may use during layout.
pub struct LayoutCx<'a, M> {
    pub(crate) scale_factor: f64,
    pub(crate) theme: &'a Theme,
    pub(crate) messages: &'a mut Vec<M>,
}

impl<'a, M> LayoutCx<'a, M> {
    /// The window's scale factor.
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// The theme.
    pub fn theme(&self) -> &'a Theme {
        self.theme
    }

    /// Sends a message to the application (e.g. "this surface now has
    /// that size").
    pub fn emit(&mut self, message: M) {
        self.messages.push(message);
    }
}

impl<M> fmt::Debug for LayoutCx<'_, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayoutCx")
            .field("scale_factor", &self.scale_factor)
            .finish_non_exhaustive()
    }
}

/// How a widget is involved in the current interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Status {
    /// The pointer is over the widget (and no other widget captured it).
    pub hot: bool,
    /// The widget captured the pointer: a button was pressed on it and
    /// is still held.
    pub active: bool,
    /// The widget has keyboard focus.
    pub focused: bool,
}

/// What widgets may use while painting.
#[derive(Debug)]
pub struct PaintCx<'a> {
    pub(crate) canvas: Canvas<'a>,
    pub(crate) theme: &'a Theme,
    pub(crate) status: Status,
}

impl<'a> PaintCx<'a> {
    /// The canvas, clipped to the widget.
    pub fn canvas(&mut self) -> &mut Canvas<'a> {
        &mut self.canvas
    }

    /// The theme.
    pub fn theme(&self) -> &'a Theme {
        self.theme
    }

    /// The window's scale factor.
    pub fn scale_factor(&self) -> f64 {
        self.canvas.scale_factor()
    }

    /// The widget's interaction status.
    pub fn status(&self) -> Status {
        self.status
    }
}

/// What widgets may do while handling an event.
pub struct EventCx<'a, M> {
    pub(crate) messages: &'a mut Vec<M>,
    pub(crate) paint_requested: bool,
    pub(crate) layout_requested: bool,
}

impl<'a, M> EventCx<'a, M> {
    pub(crate) fn new(messages: &'a mut Vec<M>) -> Self {
        Self {
            messages,
            paint_requested: false,
            layout_requested: false,
        }
    }

    /// Sends a message to the application.
    pub fn emit(&mut self, message: M) {
        self.messages.push(message);
    }

    /// Asks for the UI to be repainted.
    pub fn request_paint(&mut self) {
        self.paint_requested = true;
    }

    /// Asks for the UI to be laid out (and repainted) again, e.g. after
    /// the widget's preferred size changed.
    pub fn request_layout(&mut self) {
        self.layout_requested = true;
    }
}

impl<M> fmt::Debug for EventCx<'_, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventCx")
            .field("paint_requested", &self.paint_requested)
            .field("layout_requested", &self.layout_requested)
            .finish_non_exhaustive()
    }
}
