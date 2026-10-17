//! The UI engine: owns the widget tree and the framebuffer, lays out,
//! paints and routes input (see [`routing`]).

mod routing;
#[cfg(test)]
mod tests;

use plt_core::Point;
use plt_platform::{ButtonState, Frame, PhysicalSize, WindowEvent};
use plt_raster::{Painter, Pixmap, Rgba8};

use crate::canvas::Canvas;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::theme::Theme;
use crate::widget::{LayoutCx, PaintCx, Widget, WidgetId, WidgetPod};
use routing::Routing;

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
        self.theme = theme;
        self.request_layout();
        self
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
            WindowEvent::Key(key) => {
                self.deliver(self.routing.key_target(), &UiEvent::Key(key.clone()));
            }
            _ => {}
        }
        Ok(self.take_messages())
    }

    /// Lays out if needed, paints every widget and returns the frame to
    /// present.
    pub fn paint(&mut self) -> Result<Frame<'_>, UiError> {
        self.layout()?;
        self.framebuffer.fill(Rgba8::from(self.theme.background));
        let full = Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY);
        let mut painting = Painting {
            target: &mut self.framebuffer,
            painter: &mut self.painter,
            theme: &self.theme,
            routing: &self.routing,
            scale_factor: self.scale_factor,
        };
        painting.paint(&self.root, full)?;
        self.needs_paint = false;
        let size = self.size();
        Ok(Frame::new(
            size.width,
            size.height,
            self.framebuffer.data(),
        )?)
    }

    /// Measures and arranges the tree to fill the framebuffer, if needed.
    fn layout(&mut self) -> Result<(), UiError> {
        if !self.needs_layout {
            return Ok(());
        }
        let size = self.size();
        let size = Size::new(f64::from(size.width), f64::from(size.height));
        let mut cx = LayoutCx {
            scale_factor: self.scale_factor,
            theme: &self.theme,
            messages: &mut self.messages,
        };
        self.root.measure(&mut cx, Constraints::tight(size))?;
        self.root
            .arrange(&mut cx, Rect::from_origin_size(Point::ORIGIN, size))?;
        self.needs_layout = false;
        self.needs_paint = true;
        Ok(())
    }
}

/// The state of one paint pass.
struct Painting<'a> {
    target: &'a mut Pixmap,
    painter: &'a mut Painter,
    theme: &'a Theme,
    routing: &'a Routing,
    scale_factor: f64,
}

impl Painting<'_> {
    /// Paints `pod` and then its children, each clipped to its rectangle
    /// and its ancestors' `clip`.
    fn paint<M: 'static>(&mut self, pod: &WidgetPod<M>, clip: Rect) -> Result<(), UiError> {
        let clip = clip.intersect(pod.rect());
        if clip.is_empty() {
            return Ok(());
        }
        let mut cx = PaintCx {
            canvas: Canvas::new(self.target, self.painter, clip, self.scale_factor),
            theme: self.theme,
            status: self.routing.status(pod.id()),
        };
        pod.widget().paint(&mut cx, pod.rect())?;
        for child in pod.widget().children() {
            self.paint(child, clip)?;
        }
        Ok(())
    }
}
