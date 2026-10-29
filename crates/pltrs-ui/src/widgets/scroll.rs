//! A view scrolling a taller widget up and down.

use pltrs_platform::{ButtonState, Key, KeyEvent, MouseButton};

use super::scroll_bar::{ScrollBar, wheel_pixels};
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget, WidgetPod};

/// A viewport onto a child taller than the room it gets. The child is
/// laid out at its natural height across the view, its parts outside the
/// view clipped away; while it overflows, a scroll bar like the lists'
/// takes a strip on the right, as wide as a pointer target. The wheel
/// scrolls the view, and so do the arrow keys (a line), Page Up and Page
/// Down (the view's height less a line) and Home and End (to the ends)
/// when the focused widget leaves them unused.
///
/// Measured without constraints, the view asks for its child's natural
/// size; it scrolls once something gives it less height, such as a window
/// fitted to its content within a limit
/// ([`UiWindow::open_fitted_within`](crate::UiWindow::open_fitted_within)).
/// The view itself takes no focus and does not follow a focused child: it
/// suits content to read, such as a list of shortcuts.
///
/// ```
/// use pltrs_ui::{Label, Scroll};
/// let help: Scroll<()> = Scroll::new(Label::new("Press h to go home"));
/// assert_eq!(help.offset(), 0.0);
/// ```
pub struct Scroll<M> {
    child: [WidgetPod<M>; 1],
    bar: ScrollBar,
    /// The height of a line, from the last measure.
    line: f64,
    /// The width of the scroll bar's strip, from the last measure.
    gutter: f64,
}

impl<M> std::fmt::Debug for Scroll<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scroll")
            .field("bar", &self.bar)
            .finish_non_exhaustive()
    }
}

impl<M: 'static> Scroll<M> {
    /// A view of `child`, scrolled to the top.
    pub fn new(child: impl Widget<M>) -> Self {
        Self {
            child: [WidgetPod::new(child)],
            bar: ScrollBar::default(),
            line: 0.0,
            gutter: 0.0,
        }
    }

    /// How far the child is scrolled up, in pixels.
    pub fn offset(&self) -> f64 {
        self.bar.offset()
    }

    /// The scroll bar's strip in a view at `rect`.
    fn track(&self, rect: Rect) -> Rect {
        Rect::new(rect.max_x() - self.gutter, rect.y, self.gutter, rect.height)
    }

    /// The offset a navigation key leads to, if `key` is one.
    fn navigated(&self, key: &KeyEvent) -> Option<f64> {
        if key.state != ButtonState::Pressed || key.modifiers.control() || key.modifiers.alt() {
            return None;
        }
        let offset = self.bar.offset();
        let page = (self.bar.viewport() - self.line).max(self.line);
        match key.key {
            Key::Up => Some(offset - self.line),
            Key::Down => Some(offset + self.line),
            Key::PageUp => Some(offset - page),
            Key::PageDown => Some(offset + page),
            Key::Home => Some(0.0),
            Key::End => Some(self.bar.max_offset()),
            _ => None,
        }
    }
}

impl<M: 'static> Widget<M> for Scroll<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        self.line = theme.control_height * scale;
        self.gutter = theme.target_size * scale;
        self.bar.set_metrics(theme, scale);
        let [child] = &mut self.child;
        let width = constraints.max.width;
        let natural = child.measure(cx, Constraints::loose(Size::new(width, f64::INFINITY)))?;
        let size = if natural.height > constraints.max.height {
            let room = Size::new((width - self.gutter).max(0.0), f64::INFINITY);
            let narrower = child.measure(cx, Constraints::loose(room))?;
            Size::new(narrower.width + self.gutter, narrower.height)
        } else {
            natural
        };
        Ok(constraints.constrain(size))
    }

    fn arrange(&mut self, cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        let across = |width: f64| Constraints {
            min: Size::new(width, 0.0),
            max: Size::new(width, f64::INFINITY),
        };
        let [child] = &mut self.child;
        let mut width = rect.width;
        let mut content = child.measure(cx, across(width))?.height;
        if content > rect.height {
            width = (rect.width - self.gutter).max(0.0);
            content = child.measure(cx, across(width))?.height;
        }
        self.bar.set_extent(content, rect.height);
        // Whole pixels keep the child's text crisp.
        let top = rect.y - self.bar.offset().round();
        child.arrange(cx, Rect::new(rect.x, top, width, content))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let track = self.track(rect);
        let hot = cx.status().hot;
        self.bar.paint(cx, track, track, hot)
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        let track = self.track(rect);
        let moved = match event {
            UiEvent::PointerDown {
                button: MouseButton::Left,
                position,
                ..
            } => self.bar.press(track, *position),
            UiEvent::PointerMove { position, .. } => self.bar.drag(track, position.y),
            UiEvent::PointerUp {
                button: MouseButton::Left,
                ..
            } => {
                let dragged = self.bar.dragging();
                self.bar.release();
                dragged
            }
            UiEvent::Wheel { delta, .. } if self.bar.overflows() => {
                cx.set_handled();
                let pixels = wheel_pixels(*delta, self.line);
                self.bar.scroll_to(self.bar.offset() - pixels)
            }
            UiEvent::Key(key) => self.navigated(key).is_some_and(|offset| {
                cx.set_handled();
                self.bar.scroll_to(offset)
            }),
            _ => false,
        };
        if moved {
            cx.request_layout();
        }
    }

    fn handles_shortcut(&self, key: &KeyEvent) -> bool {
        self.bar.overflows() && self.navigated(key).is_some()
    }

    fn clips_children(&self) -> bool {
        true
    }

    fn children(&self) -> &[WidgetPod<M>] {
        &self.child
    }

    fn children_mut(&mut self) -> &mut [WidgetPod<M>] {
        &mut self.child
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Point;
    use pltrs_platform::{Modifiers, PhysicalPosition, PhysicalSize, ScrollDelta, WindowEvent};

    use super::*;
    use crate::Ui;
    use crate::widgets::testing::Fixed;
    use crate::widgets::{Button, Flex};

    /// A view of a 100 × 400 child in a column above a button, in a
    /// 200 × 150 window; returns the UI and the view's id.
    fn scrolled() -> (Ui<&'static str>, crate::WidgetId) {
        let mut column = Flex::column();
        let view = column.push(Scroll::new(Fixed(Size::new(100.0, 400.0))), 1.0);
        column.push(Button::new("Close", "close"), 0.0);
        let ui = Ui::new(column, PhysicalSize::new(200, 150), 1.0).unwrap();
        (ui, view)
    }

    fn offset(ui: &mut Ui<&'static str>, view: crate::WidgetId) -> f64 {
        ui.widget_mut_without_layout::<Scroll<&'static str>>(view)
            .map_or(f64::NAN, |view| view.offset())
    }

    fn key(key: Key) -> WindowEvent {
        WindowEvent::Key(KeyEvent {
            key,
            state: ButtonState::Pressed,
            repeat: false,
            text: None,
            modifiers: Modifiers::NONE,
            scancode: 0,
        })
    }

    #[test]
    fn views_ask_for_their_childs_natural_size_and_scroll_when_given_less() {
        let mut view: Scroll<()> = Scroll::new(Fixed(Size::new(100.0, 400.0)));
        let theme = crate::Theme::default();
        let mut messages = Vec::new();
        let mut cx = LayoutCx::for_tests(&theme, &mut messages, 1.0);
        let natural = view
            .measure(&mut cx, Constraints::loose(Size::INFINITE))
            .unwrap();
        assert_eq!(natural, Size::new(100.0, 400.0));
        let limited = Constraints::loose(Size::new(f64::INFINITY, 120.0));
        let size = view.measure(&mut cx, limited).unwrap();
        assert_eq!(size, Size::new(124.0, 120.0), "room for the scroll bar");
        view.arrange(&mut cx, Rect::from_origin_size(Point::ORIGIN, size))
            .unwrap();
        assert_eq!(view.child[0].rect(), Rect::new(0.0, 0.0, 100.0, 400.0));
    }

    #[test]
    fn keys_the_focus_leaves_unused_scroll_the_view() {
        let (mut ui, view) = scrolled();
        ui.focus_first();
        // The view is 122 pixels high: the window less the button.
        ui.handle(&key(Key::Down)).unwrap();
        assert_eq!(offset(&mut ui, view), 28.0, "a line");
        ui.handle(&key(Key::PageDown)).unwrap();
        assert_eq!(offset(&mut ui, view), 28.0 + 94.0, "a page less a line");
        ui.handle(&key(Key::End)).unwrap();
        assert_eq!(offset(&mut ui, view), 278.0);
        ui.handle(&key(Key::Up)).unwrap();
        ui.handle(&key(Key::Home)).unwrap();
        assert_eq!(offset(&mut ui, view), 0.0);
        ui.paint().unwrap();
    }

    #[test]
    fn the_wheel_scrolls_the_view_under_the_pointer() {
        let (mut ui, view) = scrolled();
        let wheel = WindowEvent::MouseWheel {
            delta: ScrollDelta::Lines { x: 0.0, y: -1.0 },
            position: PhysicalPosition::new(50.0, 50.0),
            modifiers: Modifiers::NONE,
        };
        ui.handle(&wheel).unwrap();
        assert_eq!(offset(&mut ui, view), 84.0, "three lines");
        ui.paint().unwrap();
    }

    #[test]
    fn the_scroll_bar_drags_the_view() {
        let (mut ui, view) = scrolled();
        let at = |y| PhysicalPosition::new(190.0, y);
        let click = |state, y| WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            position: at(y),
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        ui.handle(&click(ButtonState::Pressed, 10.0)).unwrap();
        let moved = WindowEvent::CursorMoved {
            position: at(200.0),
            modifiers: Modifiers::NONE,
        };
        ui.handle(&moved).unwrap();
        ui.handle(&click(ButtonState::Released, 200.0)).unwrap();
        assert_eq!(offset(&mut ui, view), 278.0, "dragged to the bottom");
    }
}
