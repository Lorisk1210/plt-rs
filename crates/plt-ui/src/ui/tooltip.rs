//! Tooltips: the text a widget shows when the pointer rests on it.

use plt_core::Point;

use super::Ui;
use crate::canvas::Canvas;
use crate::error::UiError;
use crate::geometry::{Rect, Size};
use crate::text::{TextLayout, TextStyle};
use crate::theme::{Spacing, Theme};
use crate::widget::WidgetId;

/// Space between a tooltip's edge and its text, left and right.
const PADDING_X: Spacing = Spacing::Sm;
/// Space between a tooltip's edge and its text, above and below.
const PADDING_Y: Spacing = Spacing::Xs;
/// Space between a tooltip and its widget, and the window's edges.
const MARGIN: Spacing = Spacing::Xs;

/// Where the tooltip of the widget under the pointer is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Tooltip {
    /// No widget with a tooltip is under the pointer.
    #[default]
    Hidden,
    /// The pointer rests on the widget; its tooltip shows once the
    /// theme's delay has passed.
    Pending(WidgetId),
    /// The widget's tooltip is shown.
    Shown(WidgetId),
    /// A click on the widget hid its tooltip until the pointer leaves.
    Dismissed(WidgetId),
}

impl Tooltip {
    /// The widget the tooltip belongs to.
    fn widget(self) -> Option<WidgetId> {
        match self {
            Self::Hidden => None,
            Self::Pending(id) | Self::Shown(id) | Self::Dismissed(id) => Some(id),
        }
    }
}

impl<M: 'static> Ui<M> {
    /// The widget whose tooltip waits for the pointer to rest on it for
    /// [`Theme::tooltip_delay`]; show it then with [`Ui::show_tooltip`].
    /// [`UiWindow`](crate::UiWindow) does this with a platform timer.
    pub fn pending_tooltip(&self) -> Option<WidgetId> {
        match self.tooltip {
            Tooltip::Pending(id) => Some(id),
            _ => None,
        }
    }

    /// The widget whose tooltip is shown.
    pub fn shown_tooltip(&self) -> Option<WidgetId> {
        match self.tooltip {
            Tooltip::Shown(id) => Some(id),
            _ => None,
        }
    }

    /// Shows the pending tooltip, if any.
    pub fn show_tooltip(&mut self) {
        if let Tooltip::Pending(id) = self.tooltip {
            self.tooltip = Tooltip::Shown(id);
            self.needs_paint = true;
        }
    }

    /// Follows the pointer to the innermost widget under it that has a
    /// tooltip. Once a tooltip shows, those of the widgets the pointer
    /// moves on to show at once, as in desktop toolbars.
    pub(super) fn update_tooltip(&mut self) {
        let shown = |id| {
            self.root
                .find(id)
                .is_some_and(|pod| pod.widget().tooltip().is_some())
        };
        // Popups cover the widgets: no tooltip while one is open.
        let target = match self.routing.popup {
            Some(_) => None,
            None => self
                .routing
                .hot()
                .iter()
                .rev()
                .copied()
                .find(|&id| shown(id)),
        };
        if target == self.tooltip.widget() {
            return;
        }
        let next = match (target, self.tooltip) {
            (None, _) => Tooltip::Hidden,
            (Some(id), Tooltip::Shown(_)) => Tooltip::Shown(id),
            (Some(id), _) => Tooltip::Pending(id),
        };
        self.set_tooltip(next);
    }

    /// Hides the tooltip after a click until the pointer leaves its
    /// widget.
    pub(super) fn dismiss_tooltip(&mut self) {
        if let Some(id) = self.tooltip.widget() {
            self.set_tooltip(Tooltip::Dismissed(id));
        }
    }

    fn set_tooltip(&mut self, tooltip: Tooltip) {
        let shown = |t: Tooltip| matches!(t, Tooltip::Shown(_));
        if shown(self.tooltip) || shown(tooltip) {
            self.needs_paint = true;
        }
        self.tooltip = tooltip;
    }
}

/// Paints `text` in a tooltip next to `anchor`, the rectangle of its
/// widget: below it when there is room in a window of `window` pixels,
/// else above, and never past the window's sides.
pub(super) fn paint(
    canvas: &mut Canvas<'_>,
    theme: &Theme,
    text: &str,
    anchor: Rect,
    window: Size,
) -> Result<(), UiError> {
    let scale = canvas.scale_factor();
    let layout = TextLayout::new(text, &TextStyle::new(theme.font_size), scale)?;
    let text_size = layout.size();
    let padding = Size::new(
        theme.space(PADDING_X) * scale,
        theme.space(PADDING_Y) * scale,
    );
    let margin = theme.space(MARGIN) * scale;
    let size = Size::new(
        text_size.width + 2.0 * padding.width,
        text_size.height + 2.0 * padding.height,
    );
    let below = anchor.max_y() + margin;
    let y = if below + size.height <= window.height - margin {
        below
    } else {
        anchor.y - margin - size.height
    };
    let centered = anchor.center().x - size.width / 2.0;
    let x = centered.min(window.width - margin - size.width).max(margin);
    let bubble = Rect::new(x, y, size.width, size.height).snapped();
    let radius = theme.corner_radius * scale;
    let shadow = Rect::new(bubble.x, bubble.y + scale, bubble.width, bubble.height);
    canvas.fill_rounded_rect(shadow.inset(-scale), radius + scale, theme.shadow)?;
    canvas.fill_rounded_rect(bubble, radius, theme.tooltip)?;
    let origin = Point::new(bubble.x + padding.width, bubble.y + padding.height);
    canvas.draw_text(&layout, origin, theme.tooltip_text)
}
