//! Segmented controls: a row of options, one of them chosen.

use std::fmt;
use std::rc::Rc;

use plt_core::Point;
use plt_platform::MouseButton;

use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::{TextLayout, TextStyle};
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// Makes the message sent for an option.
type OptionMessage<M> = Rc<dyn Fn(usize) -> M>;

/// A row of labelled segments of which exactly one is chosen; clicking a
/// segment chooses it and sends a message with its index.
pub struct Choice<M> {
    options: Vec<String>,
    chosen: usize,
    on_choose: OptionMessage<M>,
    /// The labels laid out by the last measure, and the segment widths.
    layouts: Vec<TextLayout>,
    widths: Vec<f64>,
}

impl<M> fmt::Debug for Choice<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Choice")
            .field("options", &self.options)
            .field("chosen", &self.chosen)
            .finish_non_exhaustive()
    }
}

impl<M> Choice<M> {
    /// Segments for `options` with the first chosen, sending
    /// `on_choose(index)` when the user picks one.
    pub fn new(options: Vec<String>, on_choose: impl Fn(usize) -> M + 'static) -> Self {
        Self {
            options,
            chosen: 0,
            on_choose: Rc::new(on_choose),
            layouts: Vec::new(),
            widths: Vec::new(),
        }
    }

    /// The options.
    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// The chosen option.
    pub fn chosen(&self) -> usize {
        self.chosen
    }

    /// Chooses option `index` (ignored when out of range); no message is
    /// sent.
    pub fn set_chosen(&mut self, index: usize) {
        if index < self.options.len() {
            self.chosen = index;
        }
    }

    /// The segment containing `x` pixels from the left edge.
    fn segment_at(&self, x: f64) -> Option<usize> {
        let mut right = 0.0;
        self.widths.iter().position(|width| {
            right += width;
            x < right
        })
    }
}

impl<M: 'static> Widget<M> for Choice<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let style = TextStyle::new(cx.theme().font_size);
        let scale = cx.scale_factor();
        let padding = cx.theme().padding * scale;
        self.layouts = self
            .options
            .iter()
            .map(|option| TextLayout::new(option, &style, scale))
            .collect::<Result<_, _>>()?;
        self.widths = self
            .layouts
            .iter()
            .map(|layout| layout.size().width + 2.0 * padding)
            .collect();
        let height = TextLayout::new("", &style, scale)?.size().height + padding;
        Ok(constraints.constrain(Size::new(self.widths.iter().sum(), height)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let radius = theme.corner_radius * scale;
        // The segments keep their natural widths, from the left.
        let used: f64 = self.widths.iter().sum();
        let outline = Rect::new(rect.x, rect.y, used.min(rect.width), rect.height);
        cx.canvas()
            .fill_rounded_rect(outline, radius, theme.separator)?;
        cx.canvas()
            .fill_rounded_rect(outline.inset(scale), radius - scale, theme.button)?;
        let mut x = rect.x;
        for (index, (layout, width)) in self.layouts.iter().zip(&self.widths).enumerate() {
            let segment = Rect::new(x, rect.y, *width, rect.height);
            if index == self.chosen {
                cx.canvas().fill_rounded_rect(
                    segment.inset(2.0 * scale),
                    radius - 2.0 * scale,
                    theme.button_checked,
                )?;
            } else if index > 0 && index != self.chosen + 1 {
                let divider = Rect::new(x, rect.y + rect.height * 0.25, scale, rect.height * 0.5);
                cx.canvas().fill_rect(divider, theme.separator)?;
            }
            let size = layout.size();
            let origin = Point::new(
                x + (width - size.width) / 2.0,
                rect.y + (rect.height - size.height) / 2.0,
            );
            let color = if index == self.chosen {
                theme.accent
            } else {
                theme.text
            };
            cx.canvas().draw_text(layout, origin, color)?;
            x += width;
        }
        Ok(())
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        if let UiEvent::PointerDown {
            button: MouseButton::Left,
            position,
            ..
        } = event
            && let Some(index) = self.segment_at(position.x - rect.x)
            && index != self.chosen
        {
            self.chosen = index;
            cx.emit((self.on_choose)(index));
            cx.request_paint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_under_the_pointer() {
        let mut choice = Choice::new(vec!["PNG".into(), "SVG".into(), "PDF".into()], |i| i);
        choice.widths = vec![30.0, 30.0, 40.0];
        assert_eq!(choice.segment_at(10.0), Some(0));
        assert_eq!(choice.segment_at(65.0), Some(2));
        assert_eq!(choice.segment_at(101.0), None);
        choice.set_chosen(2);
        assert_eq!(choice.chosen(), 2);
        choice.set_chosen(7);
        assert_eq!(choice.chosen(), 2);
    }
}
