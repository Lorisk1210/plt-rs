//! Static text.

use plt_core::Point;

use crate::error::UiError;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::{TextLayout, TextStyle};
use crate::widget::{LayoutCx, PaintCx, Widget};

/// A line of text, left-aligned and vertically centred in its rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    text: String,
    /// The text laid out by the last measure.
    layout: Option<TextLayout>,
}

impl Label {
    /// A label showing `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            layout: None,
        }
    }

    /// The text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text (the UI must be laid out again, which
    /// [`Ui::widget_mut`](crate::Ui::widget_mut) arranges).
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

impl<M> Widget<M> for Label {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let style = TextStyle::new(cx.theme().font_size);
        let layout = TextLayout::new(&self.text, &style, cx.scale_factor())?;
        let padding = cx.theme().padding * cx.scale_factor();
        let size = layout.size();
        self.layout = Some(layout);
        Ok(constraints.constrain(Size::new(size.width + 2.0 * padding, size.height + padding)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let Some(layout) = &self.layout else {
            return Ok(());
        };
        let theme = cx.theme();
        let padding = theme.padding * cx.scale_factor();
        let origin = Point::new(
            rect.x + padding,
            rect.center().y - layout.size().height / 2.0,
        );
        cx.canvas().draw_text(layout, origin, theme.text)
    }
}
