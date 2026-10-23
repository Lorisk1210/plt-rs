//! Painting the widget tree, then the open popup and the tooltip over it.

use plt_platform::{PhysicalRect, PhysicalSize};
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
    /// tooltip into `region` of the framebuffer, leaving the rest as it
    /// is.
    pub(super) fn paint_layers(&mut self, region: PhysicalRect) -> Result<(), UiError> {
        fill(
            &mut self.framebuffer,
            region,
            Rgba8::from(self.theme.background),
        );
        let clip = Rect::new(
            f64::from(region.x),
            f64::from(region.y),
            f64::from(region.width),
            f64::from(region.height),
        );
        let mut painting = Painting {
            target: &mut self.framebuffer,
            painter: &mut self.painter,
            theme: &self.theme,
            routing: &self.routing,
            scale_factor: self.scale_factor,
        };
        painting.paint(&self.root, clip)?;
        let window = self.window_size();
        if let Some(id) = self.routing.popup
            && let Some(pod) = self.root.find(id)
        {
            let mut cx = PaintCx {
                canvas: Canvas::new(
                    &mut self.framebuffer,
                    &mut self.painter,
                    clip,
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
                clip,
                self.scale_factor,
            );
            tooltip::paint(&mut canvas, &self.theme, text, pod.rect(), window)?;
        }
        Ok(())
    }
}

/// Fills `region` of `target` (clipped to it) with `color`.
fn fill(target: &mut Pixmap, region: PhysicalRect, color: Rgba8) {
    let whole = PhysicalSize::new(target.width(), target.height());
    let region = region.clip_to(whole);
    if region == PhysicalRect::from_size(whole) {
        target.fill(color);
        return;
    }
    let (start, end) = (
        region.x as usize * 4,
        (region.x + region.width) as usize * 4,
    );
    for y in region.y..region.y + region.height {
        if let Some(row) = target.row_mut(y) {
            row[start..end]
                .as_chunks_mut::<4>()
                .0
                .fill(color.to_array());
        }
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
