//! Colour swatches.

use pltrs_core::{Path, Point, Rgba};

use crate::error::UiError;
use crate::geometry::{Constraints, Rect, Size};
use crate::icon::Icon;
use crate::widget::{LayoutCx, PaintCx, Widget};

/// The slash crossing out a swatch without a colour.
fn slash() -> Icon {
    Icon::new(24.0).stroke(Path::polyline(vec![
        Point::new(5.0, 19.0),
        Point::new(19.0, 5.0),
    ]))
}

/// A square showing a colour, e.g. beside the field where it is typed:
/// translucent colours show over the surface colour, and no colour (text
/// that is not a colour) shows as a crossed-out square.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Swatch {
    color: Option<Rgba>,
}

impl Swatch {
    /// A swatch showing `color`, or crossed out when `None`.
    pub fn new(color: Option<Rgba>) -> Self {
        Self { color }
    }

    /// The colour shown.
    pub fn color(&self) -> Option<Rgba> {
        self.color
    }

    /// Shows `color`, or a crossed-out square when `None`.
    pub fn set_color(&mut self, color: Option<Rgba>) {
        self.color = color;
    }
}

impl<M> Widget<M> for Swatch {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let side = cx.theme().control_height * cx.scale_factor();
        Ok(constraints.constrain(Size::new(side, side)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let side = rect.width.min(rect.height);
        let square = rect.centered(Size::new(side, side)).snapped();
        let radius = theme.corner_radius * scale;
        cx.canvas()
            .fill_rounded_rect(square, radius, theme.surface)?;
        if let Some(color) = self.color {
            cx.canvas().fill_rounded_rect(square, radius, color)?;
        } else {
            slash().paint(cx.canvas(), square, theme.danger)?;
        }
        cx.canvas()
            .stroke_rounded_rect(square, radius, scale, theme.border)
    }
}
