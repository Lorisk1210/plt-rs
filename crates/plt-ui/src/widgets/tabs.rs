//! Tab bars: the names of the pages of a [`Stack`](crate::Stack), one of
//! them selected.

use std::fmt;
use std::rc::Rc;

use plt_core::Point;
use plt_platform::MouseButton;

use super::segments::Segments;
use crate::error::UiError;
use crate::event::UiEvent;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::TextStyle;
use crate::theme::Spacing;
use crate::widget::{EventCx, LayoutCx, PaintCx, Widget};

/// Space left and right of a tab's name.
const PADDING: Spacing = Spacing::Md;
/// How much taller than controls the bar is.
const EXTRA_HEIGHT: Spacing = Spacing::Sm;
/// Space between the top and bottom of a tab and its hover wash.
const WASH_INSET: Spacing = Spacing::Xs;

/// Makes the message sent for a tab.
type TabMessage<M> = Rc<dyn Fn(usize) -> M>;

/// A row of tabs over a hairline spanning the bar; the selected tab's
/// name is in the accent colour and underlined. Pressing a tab selects it
/// and sends a message with its index.
pub struct Tabs<M> {
    labels: Vec<String>,
    selected: usize,
    on_select: TabMessage<M>,
    segments: Segments,
}

impl<M> fmt::Debug for Tabs<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tabs")
            .field("labels", &self.labels)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl<M> Tabs<M> {
    /// Tabs named `labels` with the first selected, sending
    /// `on_select(index)` when the user selects one.
    pub fn new(labels: Vec<String>, on_select: impl Fn(usize) -> M + 'static) -> Self {
        Self {
            labels,
            selected: 0,
            on_select: Rc::new(on_select),
            segments: Segments::default(),
        }
    }

    /// The tab names.
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// The selected tab.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Selects tab `index` (ignored when out of range); no message is
    /// sent.
    pub fn set_selected(&mut self, index: usize) {
        if index < self.labels.len() {
            self.selected = index;
        }
    }
}

impl<M: 'static> Widget<M> for Tabs<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        self.segments.measure(
            &self.labels,
            &TextStyle::new(theme.font_size),
            scale,
            theme.space(PADDING) * scale,
            0.0,
        )?;
        let height = (theme.control_height + theme.space(EXTRA_HEIGHT)) * scale;
        Ok(constraints.constrain(Size::new(self.segments.total_width(), height)))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let hairline = Rect::new(rect.x, rect.max_y() - scale, rect.width, scale);
        cx.canvas().fill_rect(hairline, theme.separator)?;
        let indicator = theme.indicator_width * scale;
        let inset = theme.space(WASH_INSET) * scale;
        let radius = theme.corner_radius * scale;
        let hot = self.segments.hot;
        for (index, x, width) in self.segments.placed(rect.x) {
            let tab = Rect::new(x, rect.y, width, rect.height - indicator).snapped();
            let selected = index == self.selected;
            if selected {
                let line = Rect::new(tab.x, rect.max_y() - indicator, tab.width, indicator);
                cx.canvas()
                    .fill_rounded_rect(line.snapped(), indicator / 2.0, theme.accent)?;
            } else if hot == Some(index) {
                let wash = Rect::new(tab.x, tab.y + inset, tab.width, tab.height - 2.0 * inset);
                cx.canvas().fill_rounded_rect(wash, radius, theme.hover)?;
            }
            let color = if selected {
                theme.accent
            } else if hot == Some(index) {
                theme.text
            } else {
                theme.text_secondary
            };
            let layout = &self.segments.layouts[index];
            let size = layout.size();
            let origin = Point::new(
                (tab.x + (tab.width - size.width) / 2.0).round(),
                tab.center().y - size.height / 2.0,
            );
            cx.canvas().draw_text(layout, origin, color)?;
        }
        Ok(())
    }

    fn event(&mut self, cx: &mut EventCx<'_, M>, event: &UiEvent, rect: Rect) {
        if let UiEvent::PointerDown {
            button: MouseButton::Left,
            position,
            ..
        } = event
            && let Some(index) = self.segments.at(position.x - rect.x)
            && index != self.selected
        {
            self.selected = index;
            cx.emit((self.on_select)(index));
            cx.request_paint();
        }
        if self.segments.track_hover(event, rect) {
            cx.request_paint();
        }
    }
}

#[cfg(test)]
mod tests {
    use plt_platform::Modifiers;

    use super::*;

    #[test]
    fn pressing_a_tab_selects_it() {
        let mut tabs = Tabs::new(vec!["Axes".into(), "Curves".into()], |i| i);
        tabs.segments.widths = vec![50.0, 70.0];
        let rect = Rect::new(10.0, 0.0, 300.0, 36.0);
        let press = |x| UiEvent::PointerDown {
            button: MouseButton::Left,
            position: Point::new(x, 10.0),
            modifiers: Modifiers::NONE,
            click_count: 1,
        };
        let mut messages = Vec::new();
        let mut cx = EventCx::new(&mut messages);
        tabs.event(&mut cx, &press(100.0), rect);
        tabs.event(&mut cx, &press(100.0), rect);
        tabs.event(&mut cx, &press(200.0), rect);
        assert_eq!(messages, [1], "only a change of tab is reported");
        assert_eq!(tabs.selected(), 1);
        tabs.set_selected(9);
        assert_eq!(tabs.selected(), 1);
    }
}
