//! Keyboard shortcuts drawn as keycaps.

use plt_core::Point;

use crate::error::UiError;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::TextLayout;
use crate::theme::Spacing;
use crate::widget::{LayoutCx, PaintCx, Widget};

/// Space between neighbouring keycaps.
const GAP: Spacing = Spacing::Xs;
/// Space left and right of a key's name.
const PADDING: Spacing = Spacing::Sm;
/// How much shorter than controls keycaps are, above and below.
const SHRINK: Spacing = Spacing::Xs;

/// A row of key names, each on a small raised key, e.g. the shortcuts
/// of a command in a help window.
#[derive(Debug, Clone, PartialEq)]
pub struct Keycaps {
    keys: Vec<String>,
    /// The names laid out by the last measure.
    layouts: Vec<TextLayout>,
}

impl Keycaps {
    /// Keycaps for `keys`, in order.
    pub fn new(keys: Vec<String>) -> Self {
        Self {
            keys,
            layouts: Vec::new(),
        }
    }

    /// The key names.
    pub fn keys(&self) -> &[String] {
        &self.keys
    }
}

impl<M> Widget<M> for Keycaps {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let style = theme.text_style().monospace();
        self.layouts = self
            .keys
            .iter()
            .map(|key| TextLayout::new(key, &style, scale))
            .collect::<Result<_, _>>()?;
        let padding = theme.space(PADDING) * scale;
        let gaps = self.layouts.len().saturating_sub(1) as f64 * theme.space(GAP) * scale;
        let width = self
            .layouts
            .iter()
            .map(|layout| layout.size().width + 2.0 * padding)
            .sum::<f64>()
            + gaps;
        let height = (theme.control_height - 2.0 * theme.space(SHRINK)) * scale;
        Ok(constraints.constrain(Size::new(width, height)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let padding = theme.space(PADDING) * scale;
        let gap = theme.space(GAP) * scale;
        let height = (theme.control_height - 2.0 * theme.space(SHRINK)) * scale;
        // Slightly tighter corners than full-size controls.
        let radius = (theme.corner_radius - theme.space(Spacing::Xxs)) * scale;
        let top = rect.center().y - height / 2.0;
        let mut x = rect.x;
        for layout in &self.layouts {
            let size = layout.size();
            let key = Rect::new(x, top, size.width + 2.0 * padding, height).snapped();
            // The key's lower edge is a darker outline, so it looks raised.
            cx.canvas().fill_rounded_rect(key, radius, theme.border)?;
            let face = Rect::new(
                key.x + scale,
                key.y + scale,
                key.width - 2.0 * scale,
                key.height - 3.0 * scale,
            );
            cx.canvas()
                .fill_rounded_rect(face, radius - scale, theme.raised)?;
            let origin = Point::new(
                (key.x + padding).round(),
                face.center().y - size.height / 2.0,
            );
            cx.canvas().draw_text(layout, origin, theme.text)?;
            x = key.max_x() + gap;
        }
        Ok(())
    }
}
