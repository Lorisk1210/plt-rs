//! Tooltips: the text a widget shows when the pointer rests on it.

use pltrs_core::Point;

use super::Ui;
use crate::canvas::Canvas;
use crate::error::UiError;
use crate::geometry::{Rect, Size};
use crate::text::TextLayout;
use crate::theme::{Spacing, Theme, TooltipStyle};
use crate::widget::WidgetId;

/// Space between a tooltip and the window's edges, and between a bubble
/// and its widget.
const MARGIN: Spacing = Spacing::Xs;

/// Where the tooltip of the widget under the pointer (or given the focus
/// with the keyboard) is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Tooltip {
    /// No widget with a tooltip is under the pointer or was focused.
    #[default]
    Hidden,
    /// The pointer rests on the widget, or the keyboard focused it; its
    /// tooltip shows once the theme's delay has passed.
    Pending(WidgetId),
    /// The widget's tooltip is shown.
    Shown(WidgetId),
    /// A click on the widget (or a key it used) hid its tooltip until the
    /// pointer leaves.
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

    /// Hides the tooltip after a click or a key until the pointer leaves
    /// its widget.
    pub(super) fn dismiss_tooltip(&mut self) {
        if let Some(id) = self.tooltip.widget() {
            self.set_tooltip(Tooltip::Dismissed(id));
        }
    }

    pub(super) fn set_tooltip(&mut self, tooltip: Tooltip) {
        let shown = |t: Tooltip| matches!(t, Tooltip::Shown(_));
        if shown(self.tooltip) || shown(tooltip) {
            self.needs_paint = true;
        }
        self.tooltip = tooltip;
    }
}

/// Paints `text` in a tooltip next to `anchor`, the rectangle of its
/// widget, in a window of `window` pixels, as the theme's
/// [`TooltipStyle`] shows tooltips, never past the window's edges.
pub(super) fn paint(
    canvas: &mut Canvas<'_>,
    theme: &Theme,
    text: &str,
    anchor: Rect,
    window: Size,
) -> Result<(), UiError> {
    let scale = canvas.scale_factor();
    let style = theme.tooltip_style;
    let (lines, text_size) = lay_out(text, theme, scale)?;
    let (across, down) = padding(style);
    let padding = Size::new(theme.space(across) * scale, theme.space(down) * scale);
    let size = Size::new(
        text_size.width + 2.0 * padding.width,
        text_size.height + 2.0 * padding.height,
    );
    let margin = theme.space(MARGIN) * scale;
    let corner = place(style, anchor, size, window, margin);
    let tip = Rect::from_origin_size(corner, size).snapped();
    match style {
        TooltipStyle::Bubble => {
            let radius = theme.corner_radius * scale;
            let shadow = Rect::new(tip.x, tip.y + scale, tip.width, tip.height);
            canvas.fill_rounded_rect(shadow.inset(-scale), radius + scale, theme.shadow)?;
            canvas.fill_rounded_rect(tip, radius, theme.tooltip)?;
            canvas.stroke_rounded_rect(tip, radius, scale, theme.tooltip_border)?;
        }
        TooltipStyle::Tk => {
            canvas.fill_rect(tip, theme.tooltip)?;
            canvas.stroke_rect(tip, scale, theme.tooltip_border)?;
        }
    }
    let mut origin = Point::new(tip.x + padding.width, tip.y + padding.height);
    for line in &lines {
        canvas.draw_text(line, origin, theme.tooltip_text)?;
        origin.y += line.size().height;
    }
    Ok(())
}

/// The lines of `text` laid out in the theme's text style, and the box
/// they fill together: tooltips may have several lines, as matplotlib's
/// toolbar gives them (``"Zoom to rectangle\nx/y fixes axis"``), which
/// Qt shows one under the other.
fn lay_out(text: &str, theme: &Theme, scale: f64) -> Result<(Vec<TextLayout>, Size), UiError> {
    let style = theme.text_style();
    let mut lines = text
        .lines()
        .map(|line| TextLayout::new(line, &style, scale))
        .collect::<Result<Vec<_>, _>>()?;
    if lines.is_empty() {
        lines.push(TextLayout::new("", &style, scale)?);
    }
    let width = lines
        .iter()
        .map(|line| line.size().width)
        .fold(0.0, f64::max);
    let height = lines.iter().map(|line| line.size().height).sum();
    Ok((lines, Size::new(width, height)))
}

/// The space between a tooltip's edge and its text, across and down: Tk
/// sets its tooltips tightly.
fn padding(style: TooltipStyle) -> (Spacing, Spacing) {
    match style {
        TooltipStyle::Bubble => (Spacing::Sm, Spacing::Xs),
        TooltipStyle::Tk => (Spacing::Xs, Spacing::Xxs),
    }
}

/// The top-left corner of a tooltip of `size` for the widget at `anchor`
/// in a window of `window` pixels, at least `margin` from the window's
/// edges: a bubble centred below the widget (above it when there is no
/// room below), a Tk tooltip right of the widget level with its top (left
/// of it when there is no room on the right).
fn place(style: TooltipStyle, anchor: Rect, size: Size, window: Size, margin: f64) -> Point {
    let (x, y) = match style {
        TooltipStyle::Bubble => {
            let below = anchor.max_y() + margin;
            let y = if below + size.height <= window.height - margin {
                below
            } else {
                anchor.y - margin - size.height
            };
            (anchor.center().x - size.width / 2.0, y)
        }
        TooltipStyle::Tk => {
            let x = if anchor.max_x() + size.width <= window.width - margin {
                anchor.max_x()
            } else {
                anchor.x - size.width
            };
            (x, anchor.y.min(window.height - margin - size.height))
        }
    };
    let x = x.min(window.width - margin - size.width).max(margin);
    Point::new(x, y.max(margin))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Size = Size::new(400.0, 300.0);
    const TIP: Size = Size::new(60.0, 20.0);

    #[test]
    fn bubbles_centre_below_their_widget_or_above_it() {
        let button = Rect::new(100.0, 100.0, 32.0, 32.0);
        let corner = place(TooltipStyle::Bubble, button, TIP, WINDOW, 4.0);
        assert_eq!(corner, Point::new(86.0, 136.0));
        let low = Rect::new(100.0, 270.0, 32.0, 28.0);
        let corner = place(TooltipStyle::Bubble, low, TIP, WINDOW, 4.0);
        assert_eq!(corner, Point::new(86.0, 246.0), "above, without room below");
        let left = Rect::new(0.0, 100.0, 32.0, 32.0);
        assert_eq!(place(TooltipStyle::Bubble, left, TIP, WINDOW, 4.0).x, 4.0);
    }

    #[test]
    fn tooltips_set_each_line_of_their_text() {
        // matplotlib's toolbar tooltips have two lines; the line break
        // showed as a missing glyph and hid the second line (GNOME, 200 %).
        let theme = Theme::modern();
        let (lines, size) = lay_out("Zoom to rectangle\nx/y fixes axis", &theme, 2.0).unwrap();
        let (first, one) = lay_out("Zoom to rectangle", &theme, 2.0).unwrap();
        let (second, _) = lay_out("x/y fixes axis", &theme, 2.0).unwrap();
        assert_eq!(lines, [first[0].clone(), second[0].clone()]);
        assert_eq!(size, Size::new(one.width, 2.0 * one.height));
        let (_, empty) = lay_out("", &theme, 2.0).unwrap();
        assert_eq!(empty, Size::new(0.0, one.height));
    }

    #[test]
    fn tk_tooltips_sit_right_of_their_widget_level_with_its_top() {
        let button = Rect::new(100.0, 100.0, 32.0, 32.0);
        let corner = place(TooltipStyle::Tk, button, TIP, WINDOW, 4.0);
        assert_eq!(corner, Point::new(132.0, 100.0));
        let right = Rect::new(360.0, 100.0, 32.0, 32.0);
        let corner = place(TooltipStyle::Tk, right, TIP, WINDOW, 4.0);
        assert_eq!(corner, Point::new(300.0, 100.0), "left, without room right");
        let low = Rect::new(100.0, 280.0, 32.0, 20.0);
        let corner = place(TooltipStyle::Tk, low, TIP, WINDOW, 4.0);
        assert_eq!(corner, Point::new(132.0, 276.0), "inside the window");
    }
}
