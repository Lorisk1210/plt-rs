//! The UI engine: owns the widget tree and the framebuffer, lays out,
//! paints (see [`painting`]) and routes input (see [`routing`], [`keys`]
//! and [`focus`]), and shows tooltips (see [`tooltip`]).

mod focus;
#[cfg(test)]
mod fuzz_tests;
#[cfg(test)]
mod keyboard_tests;
mod keys;
#[cfg(test)]
mod layer_tests;
mod painting;
mod routing;
#[cfg(test)]
mod tests;
mod tooltip;

use pltrs_core::Point;
use pltrs_platform::{ButtonState, Frame, PhysicalRect, PhysicalSize, WindowEvent};
use pltrs_raster::{Painter, Pixmap};

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::theme::Theme;
use crate::widget::{ClipboardRequest, LayoutCx, Widget, WidgetId, WidgetPod};
use routing::Routing;
use tooltip::Tooltip;

/// A widget tree shown in a window-sized RGBA framebuffer.
///
/// Feed it the window's events with [`Ui::handle`], which returns the
/// messages widgets emitted, and call [`Ui::paint`] when it
/// [needs painting](Ui::needs_paint) to get the frame to present.
/// [`UiWindow`](crate::UiWindow) does both for a `pltrs-platform` window.
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
    /// What widgets asked of the clipboard, for the application.
    clipboard_requests: Vec<ClipboardRequest>,
    /// The widget that asked to paste the clipboard's text.
    paste_target: Option<WidgetId>,
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
            clipboard_requests: Vec::new(),
            paste_target: None,
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

    /// The widget `id` as its concrete type, for a change that alters only
    /// how it looks, not its size or what it holds: nothing is laid out
    /// again, and only what the caller repaints with [`Ui::paint_region`]
    /// changes on screen until the next full paint.
    pub fn widget_mut_without_layout<W: Widget<M>>(&mut self, id: WidgetId) -> Option<&mut W> {
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

    /// Removes what widgets asked of the clipboard since the last call,
    /// for the application to do: put text on the clipboard, or ask for
    /// its text and hand it back as [`WindowEvent::ClipboardText`].
    /// [`UiWindow`](crate::UiWindow) does this with the platform's
    /// clipboard.
    pub fn take_clipboard_requests(&mut self) -> Vec<ClipboardRequest> {
        std::mem::take(&mut self.clipboard_requests)
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
        let point = |p: &pltrs_platform::PhysicalPosition| Point::new(p.x, p.y);
        match event {
            WindowEvent::Resized(size) => self.resize(*size, self.scale_factor)?,
            WindowEvent::ScaleFactorChanged(scale) => self.resize(self.size(), *scale)?,
            WindowEvent::CursorEntered { position } => self.update_hot(Some(point(position))),
            WindowEvent::CursorLeft { position } => self.pointer_left(position.as_ref().map(point)),
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
                self.wheel(&wheel);
            }
            WindowEvent::Key(key) => self.key(key),
            WindowEvent::ClipboardText(text) => {
                let target = self.paste_target.take();
                if let Some(text) = text {
                    self.deliver(target, &UiEvent::Paste(text.clone()));
                }
            }
            _ => {}
        }
        Ok(self.take_messages())
    }

    /// The size the widget tree asks for when nothing constrains it, in
    /// physical pixels: what a window fitted to its content needs (see
    /// [`UiWindow::open_fitted`](crate::UiWindow::open_fitted)).
    pub fn natural_size(&mut self) -> Result<Size, UiError> {
        self.measure_within(Size::INFINITE)
    }

    /// The size the widget tree asks for within `max` physical pixels:
    /// its [natural size](Ui::natural_size) when that fits, else the size
    /// it asks for when measured within the limit, so that a
    /// [`Scroll`](crate::Scroll) in it takes the room left and a strip for
    /// its scroll bar.
    pub fn natural_size_within(&mut self, max: Size) -> Result<Size, UiError> {
        let natural = self.natural_size()?;
        if natural.width <= max.width && natural.height <= max.height {
            return Ok(natural);
        }
        let limit = |natural: f64, max: f64| if natural > max { max } else { f64::INFINITY };
        self.measure_within(Size::new(
            limit(natural.width, max.width),
            limit(natural.height, max.height),
        ))
    }

    /// The size the widget tree asks for within `max` physical pixels.
    fn measure_within(&mut self, max: Size) -> Result<Size, UiError> {
        let mut cx = LayoutCx {
            scale_factor: self.scale_factor,
            theme: &self.theme,
            messages: &mut self.messages,
            window: Size::INFINITE,
        };
        let size = self.root.measure(&mut cx, Constraints::loose(max))?;
        // The next layout measures again within the framebuffer.
        self.request_layout();
        Ok(size)
    }

    /// Lays out if needed, paints every widget (and the open popup and
    /// the tooltip shown, on top) and returns the frame to present.
    pub fn paint(&mut self) -> Result<Frame<'_>, UiError> {
        self.layout()?;
        self.paint_layers(PhysicalRect::from_size(self.size()))?;
        self.needs_paint = false;
        self.frame()
    }

    /// Repaints only `region` of the framebuffer (in window pixels,
    /// clipped to the window), for a change that alters only how widgets
    /// look there, and returns the frame with the region to present. When
    /// the UI has to be laid out first, everything is laid out and
    /// repainted, and the region is the whole window.
    pub fn paint_region(
        &mut self,
        region: PhysicalRect,
    ) -> Result<(Frame<'_>, PhysicalRect), UiError> {
        let painted = if self.needs_layout {
            self.layout()?;
            self.needs_paint = false;
            PhysicalRect::from_size(self.size())
        } else {
            region.clip_to(self.size())
        };
        self.paint_layers(painted)?;
        Ok((self.frame()?, painted))
    }

    /// The framebuffer as a frame to present.
    fn frame(&self) -> Result<Frame<'_>, UiError> {
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
        // Pages may have been switched or widgets disabled meanwhile.
        self.keep_focus_shown();
        Ok(())
    }
}
