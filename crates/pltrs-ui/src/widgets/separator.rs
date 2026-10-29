//! Thin lines between groups of widgets.

use crate::error::UiError;
use crate::geometry::{Constraints, Rect, Size};
use crate::theme::Spacing;
use crate::widget::{LayoutCx, PaintCx, Widget};

/// The space on both sides of a vertical separator, and above and below
/// its line.
const MARGIN: Spacing = Spacing::Sm;

/// A one-pixel line in the theme's separator colour: vertical between
/// groups of buttons in a row (with space on both sides and a shorter
/// line), or horizontal across a column (edge to edge, e.g. above a
/// toolbar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Separator {
    horizontal: bool,
}

impl Separator {
    /// A vertical line, for rows.
    pub const fn vertical() -> Self {
        Self { horizontal: false }
    }

    /// A horizontal line, for columns.
    pub const fn horizontal() -> Self {
        Self { horizontal: true }
    }
}

impl<M> Widget<M> for Separator {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let scale = cx.scale_factor();
        let size = if self.horizontal {
            Size::new(0.0, scale)
        } else {
            Size::new((1.0 + 2.0 * cx.theme().space(MARGIN)) * scale, 0.0)
        };
        Ok(constraints.constrain(size))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let line = if self.horizontal {
            Rect::new(rect.x, rect.y, rect.width, scale)
        } else {
            let inset = theme.space(MARGIN) * scale;
            Rect::new(
                (rect.center().x - scale / 2.0).round(),
                rect.y + inset,
                scale,
                (rect.height - 2.0 * inset).max(0.0),
            )
        };
        cx.canvas().fill_rect(line, theme.separator)
    }
}
