//! Static text.

use plt_core::{Point, Rgba};

use crate::error::UiError;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::{Cut, TextLayout, TextStyle};
use crate::theme::Theme;
use crate::widget::{LayoutCx, PaintCx, Widget};

/// How prominent a label's text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    /// The theme's text colour.
    #[default]
    Normal,
    /// The quieter secondary text colour: hints, status lines.
    Secondary,
    /// The danger colour: problems the user should fix.
    Danger,
}

impl Tone {
    fn color(self, theme: &Theme) -> Rgba {
        match self {
            Self::Normal => theme.text,
            Self::Secondary => theme.text_secondary,
            Self::Danger => theme.danger,
        }
    }
}

/// Which side of its rectangle a label's text keeps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// The left edge.
    #[default]
    Start,
    /// The right edge; text too wide for the label starts at the left
    /// edge instead, so that its beginning stays visible.
    End,
}

/// What a label does with text wider than its rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    /// The text is cut at the rectangle's edge; the label asks for the
    /// text's width.
    #[default]
    Clip,
    /// The start of the text gives way to an ellipsis, keeping its end
    /// (e.g. the last directories of a path); the label asks for no
    /// width and takes what its container gives.
    ElideStart,
    /// The end of the text gives way to an ellipsis; the label asks for
    /// no width and takes what its container gives.
    ElideEnd,
}

impl Overflow {
    /// The end of the text an ellipsis replaces, if any.
    fn cut(self) -> Option<Cut> {
        match self {
            Self::Clip => None,
            Self::ElideStart => Some(Cut::Start),
            Self::ElideEnd => Some(Cut::End),
        }
    }
}

/// A line of text, vertically centred in its rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    text: String,
    /// The narrowest the label gets, in logical pixels.
    min_width: f64,
    bold: bool,
    tone: Tone,
    align: Align,
    overflow: Overflow,
    /// The text, or what is left of it, laid out by the last layout.
    layout: Option<TextLayout>,
}

impl Label {
    /// A label showing `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            min_width: 0.0,
            bold: false,
            tone: Tone::Normal,
            align: Align::Start,
            overflow: Overflow::Clip,
            layout: None,
        }
    }

    /// The label handling text wider than itself with `overflow`.
    #[must_use]
    pub fn with_overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// The label in bold, e.g. as the heading of a group of fields.
    #[must_use]
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// The label in `tone`.
    #[must_use]
    pub fn with_tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// The label's text kept to the `align` side.
    #[must_use]
    pub fn with_align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// The label at least `width` logical pixels wide.
    #[must_use]
    pub fn with_min_width(mut self, width: f64) -> Self {
        self.min_width = width;
        self
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

    fn style(&self, theme: &Theme) -> TextStyle {
        let style = theme.text_style();
        if self.bold { style.bold() } else { style }
    }
}

impl<M> Widget<M> for Label {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let style = self.style(cx.theme());
        let layout = TextLayout::new(&self.text, &style, cx.scale_factor())?;
        let size = layout.size();
        self.layout = Some(layout);
        let text_width = match self.overflow {
            Overflow::Clip => size.width,
            Overflow::ElideStart | Overflow::ElideEnd => 0.0,
        };
        let width = text_width.max(self.min_width * cx.scale_factor());
        Ok(constraints.constrain(Size::new(width, size.height)))
    }

    fn arrange(&mut self, cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        let too_wide = self
            .layout
            .as_ref()
            .is_some_and(|layout| layout.size().width > rect.width);
        if let (true, Some(cut)) = (too_wide, self.overflow.cut()) {
            let style = self.style(cx.theme());
            let fitted = TextLayout::fitted(&self.text, &style, cx.scale_factor(), rect.width, cut);
            self.layout = Some(fitted?);
        }
        Ok(())
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let Some(layout) = &self.layout else {
            return Ok(());
        };
        let size = layout.size();
        let x = match self.align {
            Align::End if size.width <= rect.width => rect.max_x() - size.width,
            Align::Start | Align::End => rect.x,
        };
        let origin = Point::new(x, rect.center().y - size.height / 2.0);
        let color = self.tone.color(cx.theme());
        cx.canvas().draw_text(layout, origin, color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lay_out(label: &mut Label, width: f64) -> Size {
        let theme = Theme::default();
        let mut messages: Vec<()> = Vec::new();
        let mut cx = LayoutCx::for_tests(&theme, &mut messages, 1.0);
        let natural = label
            .measure(&mut cx, Constraints::loose(Size::INFINITE))
            .unwrap();
        label
            .measure(
                &mut cx,
                Constraints::tight(Size::new(width, natural.height)),
            )
            .unwrap();
        label
            .arrange(&mut cx, Rect::new(0.0, 0.0, width, natural.height))
            .unwrap();
        natural
    }

    #[test]
    fn long_text_gives_way_at_its_start() {
        let path = "/home/user/projects/plots/results";
        let mut label = Label::new(path).with_overflow(Overflow::ElideStart);
        assert_eq!(lay_out(&mut label, 400.0).width, 0.0, "asks for no width");
        assert_eq!(
            label.layout.as_ref().unwrap().run().glyphs.len(),
            path.len()
        );
        lay_out(&mut label, 80.0);
        let shown = label.layout.as_ref().unwrap();
        assert!(shown.size().width <= 80.0, "{}", shown.size().width);
        assert!(shown.run().glyphs.len() < path.len(), "the start is gone");
        lay_out(&mut label, 1.0);
        assert_eq!(
            label.layout.as_ref().unwrap().run().glyphs.len(),
            1,
            "an ellipsis"
        );
        let mut plain = Label::new(path);
        assert!(
            lay_out(&mut plain, 80.0).width > 80.0,
            "clipping labels ask for their text"
        );
    }

    #[test]
    fn long_text_gives_way_at_its_end() {
        let hint = "Edit axis, curve and image parameters";
        let mut label = Label::new(hint).with_overflow(Overflow::ElideEnd);
        lay_out(&mut label, 120.0);
        let shown = label.layout.as_ref().unwrap();
        assert!(shown.size().width <= 120.0);
        let glyphs = shown.run().glyphs.len();
        assert!(glyphs > 5 && glyphs < hint.len(), "{glyphs} glyphs");
        let mut wider = Label::new(hint).with_overflow(Overflow::ElideEnd);
        lay_out(&mut wider, 160.0);
        assert!(wider.layout.as_ref().unwrap().run().glyphs.len() > glyphs);
    }
}
