//! The UI engine: owns the widget tree and the framebuffer, lays out,
//! paints (see [`painting`]) and routes input (see [`routing`] and
//! [`focus`]), and shows tooltips (see [`tooltip`]).

mod focus;
#[cfg(test)]
mod layer_tests;
mod painting;
mod routing;
#[cfg(test)]
mod tests;
mod tooltip;

use plt_core::Point;
use plt_platform::{ButtonState, Frame, PhysicalSize, WindowEvent};
use plt_raster::{Painter, Pixmap};

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::theme::Theme;
use crate::widget::{LayoutCx, Widget, WidgetId, WidgetPod};
use routing::Routing;
use tooltip::Tooltip;

/// A widget tree shown in a window-sized RGBA framebuffer.
///
/// Feed it the window's events with [`Ui::handle`], which returns the
/// messages widgets emitted, and call [`Ui::paint`] when it
/// [needs painting](Ui::needs_paint) to get the frame to present.
/// [`UiWindow`](crate::UiWindow) does both for a `plt-platform` window.
#[derive(Debug)]
pub struct Ui<M> {
    root: WidgetPod<M>,
    theme: Theme,
    scale_factor: f64,
    framebuffer: Pixmap,
    painter: Painter,
    routing: Routing,
    tooltip: Tooltip,
    /// Messages emitted outside of [`Ui::handle`] (during layout).
    messages: Vec<M>,
    needs_layout: bool,
    needs_paint: bool,
}

impl<M: 'static> Ui<M> {
    /// A UI of `size` physical pixels showing `root`, laid out at once:
    /// messages widgets emit meanwhile (such as the size of a
    /// [`Surface`](crate::Surface)) wait in [`Ui::take_messages`].
    pub fn new(
        root: impl Widget<M>,
        size: PhysicalSize,
        scale_factor: f64,
    ) -> Result<Self, UiError> {
        let mut ui = Self {
            root: WidgetPod::new(root),
            theme: Theme::default(),
            scale_factor,
            framebuffer: Pixmap::new(size.width, size.height)?,
            painter: Painter::new(),
            routing: Routing::default(),
            tooltip: Tooltip::default(),
            messages: Vec::new(),
            needs_layout: true,
            needs_paint: true,
        };
        ui.layout()?;
        Ok(ui)
    }

    /// Replaces the theme.
    #[must_use]
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.set_theme(theme);
        self
    }

    /// Replaces the theme of a UI already shown.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.request_layout();
    }

    /// The theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// The framebuffer size.
    pub fn size(&self) -> PhysicalSize {
        PhysicalSize::new(self.framebuffer.width(), self.framebuffer.height())
    }

    /// The scale factor widgets lay out for.
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// The id of the root widget.
    pub fn root_id(&self) -> WidgetId {
        self.root.id()
    }

    /// The widget that receives key events, if any.
    pub fn focus(&self) -> Option<WidgetId> {
        self.routing.focus
    }

    /// Whether a widget's popup is open (see
    /// [`EventCx::open_popup`](crate::EventCx::open_popup)), so that, for
    /// instance, Escape should close it rather than the window.
    pub fn has_popup(&self) -> bool {
        self.routing.popup.is_some()
    }

    /// Gives keyboard focus to widget `id`, if it accepts focus, or takes
    /// it from every widget when `None`.
    pub fn set_focus(&mut self, id: Option<WidgetId>) {
        let focus = match id {
            Some(id)
                if self
                    .root
                    .find(id)
                    .is_some_and(|pod| pod.widget().accepts_focus()) =>
            {
                Some(id)
            }
            Some(_) => return,
            None => None,
        };
        if focus != self.routing.focus {
            self.routing.focus = focus;
            self.needs_paint = true;
        }
    }

    /// The rectangle of a widget from the last layout.
    pub fn widget_rect(&self, id: WidgetId) -> Option<Rect> {
        self.root.find(id).map(WidgetPod::rect)
    }

    /// The widget `id` as its concrete type, for the application to
    /// update it. Assumes the change affects layout and appearance.
    pub fn widget_mut<W: Widget<M>>(&mut self, id: WidgetId) -> Option<&mut W> {
        self.request_layout();
        self.root.find_mut(id)?.downcast_mut()
    }

    /// Schedules a new layout (and paint) before the next frame.
    pub fn request_layout(&mut self) {
        self.needs_layout = true;
        self.needs_paint = true;
    }

    /// Whether the UI changed since the last [`Ui::paint`].
    pub fn needs_paint(&self) -> bool {
        self.needs_paint
    }

    /// Removes the messages emitted during layout since the last call.
    pub fn take_messages(&mut self) -> Vec<M> {
        std::mem::take(&mut self.messages)
    }

    /// Changes the framebuffer size and scale factor, and lays out again.
    pub fn resize(&mut self, size: PhysicalSize, scale_factor: f64) -> Result<(), UiError> {
        if size != self.size() {
            self.framebuffer = Pixmap::new(size.width, size.height)?;
        }
        self.scale_factor = scale_factor;
        self.request_layout();
        self.layout()
    }

    /// Updates the UI for a window event and returns the messages widgets
    /// emitted (plus any emitted during layout since the last call).
    pub fn handle(&mut self, event: &WindowEvent) -> Result<Vec<M>, UiError> {
        let point = |p: &plt_platform::PhysicalPosition| Point::new(p.x, p.y);
        match event {
            WindowEvent::Resized(size) => self.resize(*size, self.scale_factor)?,
            WindowEvent::ScaleFactorChanged(scale) => self.resize(self.size(), *scale)?,
            WindowEvent::CursorEntered { position } => self.update_hot(Some(point(position))),
            WindowEvent::CursorLeft => self.update_hot(None),
            WindowEvent::CursorMoved {
                position,
                modifiers,
            } => self.pointer_moved(point(position), *modifiers),
            WindowEvent::MouseInput {
                button,
                state,
                position,
                modifiers,
                click_count,
            } => {
                let position = point(position);
                match state {
                    ButtonState::Pressed => {
                        self.pointer_down(*button, position, *modifiers, *click_count);
                    }
                    ButtonState::Released => self.pointer_up(*button, position, *modifiers),
                }
            }
            WindowEvent::MouseWheel {
                delta,
                position,
                modifiers,
            } => {
                let position = point(position);
                self.update_hot(Some(position));
                let wheel = UiEvent::Wheel {
                    delta: *delta,
                    position,
                    modifiers: *modifiers,
                };
                self.deliver(self.routing.pointer_target(), &wheel);
            }
            WindowEvent::Key(key) => self.key(key),
            _ => {}
        }
        Ok(self.take_messages())
    }

    /// The size the widget tree asks for when nothing constrains it, in
    /// physical pixels: what a window fitted to its content needs (see
    /// [`UiWindow::open_fitted`](crate::UiWindow::open_fitted)).
    pub fn natural_size(&mut self) -> Result<Size, UiError> {
        let mut cx = LayoutCx {
            scale_factor: self.scale_factor,
            theme: &self.theme,
            messages: &mut self.messages,
            window: Size::INFINITE,
        };
        let size = self
            .root
            .measure(&mut cx, Constraints::loose(Size::INFINITE))?;
        // The next layout measures again within the framebuffer.
        self.request_layout();
        Ok(size)
    }

    /// Lays out if needed, paints every widget (and the open popup and
    /// the tooltip shown, on top) and returns the frame to present.
    pub fn paint(&mut self) -> Result<Frame<'_>, UiError> {
        self.layout()?;
        self.paint_layers()?;
        self.needs_paint = false;
        let size = self.size();
        Ok(Frame::new(
            size.width,
            size.height,
            self.framebuffer.data(),
        )?)
    }

    /// The framebuffer's size.
    fn window_size(&self) -> Size {
        let size = self.size();
        Size::new(f64::from(size.width), f64::from(size.height))
    }

    /// Measures and arranges the tree to fill the framebuffer, if needed.
    fn layout(&mut self) -> Result<(), UiError> {
        if !self.needs_layout {
            return Ok(());
        }
        let size = self.window_size();
        let mut cx = LayoutCx {
            scale_factor: self.scale_factor,
            theme: &self.theme,
            messages: &mut self.messages,
            window: size,
        };
        self.root.measure(&mut cx, Constraints::tight(size))?;
        self.root
            .arrange(&mut cx, Rect::from_origin_size(Point::ORIGIN, size))?;
        self.needs_layout = false;
        self.needs_paint = true;
        Ok(())
    }
}
