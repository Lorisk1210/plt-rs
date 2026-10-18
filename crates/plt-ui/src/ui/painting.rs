//! Painting the widget tree, then the open popup and the tooltip over it.

use plt_core::Point;
use plt_raster::{Painter, Pixmap, Rgba8};

use super::routing::Routing;
use super::{Ui, tooltip};
use crate::canvas::Canvas;
use crate::error::UiError;
use crate::geometry::Rect;
use crate::theme::Theme;
use crate::widget::{PaintCx, WidgetPod};

impl<M: 'static> Ui<M> {
    /// Paints the background, the widgets, the open popup and the shown
    /// tooltip into the framebuffer.
    pub(super) fn paint_layers(&mut self) -> Result<(), UiError> {
        self.framebuffer.fill(Rgba8::from(self.theme.background));
        let mut painting = Painting {
            target: &mut self.framebuffer,
            painter: &mut self.painter,
            theme: &self.theme,
            routing: &self.routing,
            scale_factor: self.scale_factor,
        };
        let unbounded = Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY);
        painting.paint(&self.root, unbounded)?;
        let window = self.window_size();
        let whole = Rect::from_origin_size(Point::ORIGIN, window);
        if let Some(id) = self.routing.popup
            && let Some(pod) = self.root.find(id)
        {
            let mut cx = PaintCx {
                canvas: Canvas::new(
                    &mut self.framebuffer,
                    &mut self.painter,
                    whole,
                    self.scale_factor,
                ),
                theme: &self.theme,
                status: self.routing.status(id),
            };
            pod.widget().paint_popup(&mut cx, pod.rect())?;
        }
        if let Some(id) = self.shown_tooltip()
            && let Some(pod) = self.root.find(id)
            && let Some(text) = pod.widget().tooltip()
        {
            let mut canvas = Canvas::new(
                &mut self.framebuffer,
                &mut self.painter,
                whole,
                self.scale_factor,
            );
            tooltip::paint(&mut canvas, &self.theme, text, pod.rect(), window)?;
        }
        Ok(())
    }
}

/// The state of one pass over the widget tree.
struct Painting<'a> {
    target: &'a mut Pixmap,
    painter: &'a mut Painter,
    theme: &'a Theme,
    routing: &'a Routing,
    scale_factor: f64,
}

impl Painting<'_> {
    /// Paints `pod` and then its children, each clipped to its rectangle
    /// (grown by the room for focus rings) and its ancestors' `clip`.
    fn paint<M: 'static>(&mut self, pod: &WidgetPod<M>, clip: Rect) -> Result<(), UiError> {
        let halo = self.theme.focus_ring_width * self.scale_factor;
        let clip = clip.intersect(pod.rect().inset(-halo));
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
