//! The [`Widget`] trait, the [`WidgetPod`]s that hold widgets in the tree,
//! and the contexts widgets receive.

use std::any::Any;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use plt_platform::KeyEvent;

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
/// arrange their children inside it). Measured with unbounded
/// constraints, a widget reports its natural size, which
/// [`Ui::natural_size`](crate::Ui::natural_size) uses to fit windows to
/// their content.
///
/// The engine paints a widget before its children, clipped to its
/// rectangle grown by the theme's
/// [`focus_ring_width`](crate::Theme::focus_ring_width) (room for a focus
/// ring), and routes each event to a single widget, but for wheel events
/// left unused; see [`UiEvent`] for the routing rules.
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

    /// Paints the widget's popup over every other widget, while it is
    /// open (see [`EventCx::open_popup`]); `rect` is the widget's
    /// rectangle and the canvas covers the window.
    fn paint_popup(&self, _cx: &mut PaintCx<'_>, _rect: Rect) -> Result<(), UiError> {
        Ok(())
    }

    /// Reacts to an input event; `rect` is the widget's rectangle.
    fn event(&mut self, _cx: &mut EventCx<'_, M>, _event: &UiEvent, _rect: Rect) {}

    /// Whether the widget takes keyboard focus: Tab and Shift+Tab move
    /// the focus between such widgets, in tree order.
    fn accepts_focus(&self) -> bool {
        false
    }

    /// Whether clicking the widget gives it keyboard focus; by default,
    /// when it accepts focus. Tool buttons and tab bars take it only from
    /// the keyboard, as in Qt, so that clicking them leaves the focus
    /// where the user types.
    fn focus_on_click(&self) -> bool {
        self.accepts_focus()
    }

    /// Whether the widget acts on the key press `key` although it does
    /// not have the focus, when the focused widget leaves the key unused
    /// (see [`EventCx::set_handled`]): the default button takes Enter, a
    /// tab bar Ctrl+Tab. The engine then delivers the key to the first
    /// such widget shown, in tree order.
    fn handles_shortcut(&self, _key: &KeyEvent) -> bool {
        false
    }

    /// Whether the widget, when focused, receives Tab presses instead of
    /// the focus moving on (e.g. a canvas forwarding every key).
    fn wants_tab(&self) -> bool {
        false
    }

    /// Tells the widget that Tab or Shift+Tab gave it the focus (a text
    /// field selects its text, ready to be typed over).
    fn focused_by_keyboard(&mut self) {}

    /// The text of the tooltip shown when the pointer rests on the
    /// widget, or when the keyboard moves the focus to it, if it has one.
    fn tooltip(&self) -> Option<&str> {
        None
    }

    /// Whether the widget's children are clipped to its rectangle, as in
    /// a scrolled view whose child reaches past it, rather than to the
    /// rectangle grown by the room for focus rings.
    fn clips_children(&self) -> bool {
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
    pub(crate) window: Size,
}

#[cfg(test)]
impl<'a, M> LayoutCx<'a, M> {
    /// A context for tests, in an unbounded window.
    pub(crate) fn for_tests(theme: &'a Theme, messages: &'a mut Vec<M>, scale_factor: f64) -> Self {
        Self {
            scale_factor,
            theme,
            messages,
            window: Size::INFINITE,
        }
    }
}

impl<'a, M> LayoutCx<'a, M> {
    /// The window's scale factor.
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// The window's size, e.g. to keep a popup inside it.
    pub fn window_size(&self) -> Size {
        self.window
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
    /// Whether the widget has keyboard focus.
    pub focus: Focus,
}

impl Status {
    /// Whether the widget has keyboard focus.
    pub fn focused(self) -> bool {
        self.focus != Focus::None
    }

    /// Whether the widget has keyboard focus and should show its focus
    /// ring (see [`Focus::Visible`]).
    pub fn focus_visible(self) -> bool {
        self.focus == Focus::Visible
    }
}

/// Whether a widget has keyboard focus, and how it shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    /// The widget does not have the focus.
    #[default]
    None,
    /// The widget has the focus, but a click came after the keyboard last
    /// moved it: controls show no focus ring (text fields still do, as
    /// they wait for typing).
    Focused,
    /// The widget has the focus and the user moves the focus with the
    /// keyboard: controls show their focus ring.
    Visible,
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

/// What a widget asked of the clipboard (see
/// [`Ui::take_clipboard_requests`](crate::Ui::take_clipboard_requests)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardRequest {
    /// Put this text on the clipboard.
    Copy(String),
    /// Read the clipboard's text, for the widget to paste (it receives
    /// [`UiEvent::Paste`]).
    Paste,
}

/// What widgets may do while handling an event.
pub struct EventCx<'a, M> {
    pub(crate) messages: &'a mut Vec<M>,
    pub(crate) paint_requested: bool,
    pub(crate) layout_requested: bool,
    /// Whether the widget opened (`Some(true)`) or closed its popup.
    pub(crate) popup: Option<bool>,
    /// Whether the widget acted on the key it received.
    pub(crate) handled: bool,
    /// What the widget asked of the clipboard.
    pub(crate) clipboard: Option<ClipboardRequest>,
}

impl<'a, M> EventCx<'a, M> {
    pub(crate) fn new(messages: &'a mut Vec<M>) -> Self {
        Self {
            messages,
            paint_requested: false,
            layout_requested: false,
            popup: None,
            handled: false,
            clipboard: None,
        }
    }

    /// Tells the engine that the widget acted on the key or the wheel
    /// event it received, so that it goes no further: keys left unused
    /// may be shortcuts of other widgets (see
    /// [`Widget::handles_shortcut`]), such as Enter pressing the default
    /// button, and wheel events left unused go on to the widget's
    /// ancestors, such as a view scrolling it.
    pub fn set_handled(&mut self) {
        self.handled = true;
    }

    /// Puts `text` on the clipboard.
    pub fn copy_text(&mut self, text: String) {
        self.clipboard = Some(ClipboardRequest::Copy(text));
    }

    /// Asks for the clipboard's text, which the widget receives later as
    /// [`UiEvent::Paste`].
    pub fn request_paste(&mut self) {
        self.clipboard = Some(ClipboardRequest::Paste);
    }

    /// Opens the widget's popup: until [closed](Self::close_popup), the
    /// widget receives every pointer and key event wherever the pointer
    /// is, and paints its popup over the window (see
    /// [`Widget::paint_popup`]).
    pub fn open_popup(&mut self) {
        self.popup = Some(true);
        self.paint_requested = true;
    }

    /// Closes the widget's popup.
    pub fn close_popup(&mut self) {
        self.popup = Some(false);
        self.paint_requested = true;
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
            .field("popup", &self.popup)
            .field("handled", &self.handled)
            .field("clipboard", &self.clipboard)
            .finish_non_exhaustive()
    }
}
