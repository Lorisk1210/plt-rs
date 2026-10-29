//! Title bars of client-side window decorations, drawn in a [`Theme`]: the
//! platform decides where the bar and its buttons are and what they do,
//! this draws them like the rest of the window's chrome.

use plt_core::{Path, Point, Rgba};
use plt_platform::{DecorationPainter, PhysicalRect, TitleBar, TitleBarButton};
use plt_raster::{Painter, Pixmap};

use crate::canvas::Canvas;
use crate::error::UiError;
use crate::geometry::{Rect, Size};
use crate::icon::Icon;
use crate::text::{Cut, TextLayout};
use crate::theme::Theme;

/// The side of a button's symbol, in logical pixels.
const SYMBOL_SIZE: f64 = 16.0;
/// How far a button's backdrop stays inside its square, in logical
/// pixels.
const BACKDROP_INSET: f64 = 5.0;
/// The space left and right of the title, in logical pixels.
const TITLE_PADDING: f64 = 12.0;

/// Draws title bars in a theme's colours and type: the chrome's
/// background, the title centred in bold, and the buttons as flat icons
/// that show a face when hovered or pressed.
///
/// ```
/// use plt_platform::{DecorationPainter, PhysicalSize, TitleBar};
/// use plt_ui::{Theme, TitleBarPainter};
///
/// let bar = TitleBar {
///     title: "Figure 1",
///     size: PhysicalSize::new(300, 34),
///     scale_factor: 1.0,
///     focused: true,
///     maximized: false,
///     hovered: None,
///     pressed: None,
/// };
/// let mut pixels = vec![0; 300 * 34 * 4];
/// TitleBarPainter::new(Theme::modern()).paint(&bar, &mut pixels);
/// assert!(pixels.chunks(4).all(|px| px[3] == 255), "an opaque bar");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TitleBarPainter {
    theme: Theme,
}

impl TitleBarPainter {
    /// A painter drawing in `theme`.
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    /// Draws `bar` into a new pixmap.
    fn draw(&self, bar: &TitleBar<'_>) -> Result<Pixmap, UiError> {
        let mut pixmap = Pixmap::new(bar.size.width, bar.size.height)?;
        let mut painter = Painter::new();
        let (width, height) = (f64::from(bar.size.width), f64::from(bar.size.height));
        let whole = Rect::new(0.0, 0.0, width, height);
        let scale = bar.scale_factor;
        let mut canvas = Canvas::new(&mut pixmap, &mut painter, whole, scale);
        let theme = &self.theme;
        canvas.fill_rect(whole, theme.background)?;
        let line = Rect::new(0.0, height - scale.max(1.0), width, scale.max(1.0));
        canvas.fill_rect(line, theme.separator)?;
        let ink = if bar.focused {
            theme.text
        } else {
            theme.text_secondary
        };
        self.draw_title(&mut canvas, bar, ink)?;
        for (button, area) in bar.buttons() {
            let area = rect(area);
            let face = if bar.pressed == Some(button) {
                Some(theme.pressed)
            } else if bar.hovered == Some(button) {
                Some(theme.hover)
            } else {
                None
            };
            if let Some(face) = face {
                let backdrop = area.inset(BACKDROP_INSET * scale);
                canvas.fill_rounded_rect(backdrop, theme.corner_radius * scale, face)?;
            }
            let symbol = area.centered(Size::new(SYMBOL_SIZE * scale, SYMBOL_SIZE * scale));
            symbol_icon(button, bar.maximized).paint(&mut canvas, symbol, ink)?;
        }
        Ok(pixmap)
    }

    /// The title, centred on the bar where it fits beside the buttons,
    /// shortened with an ellipsis where it does not.
    fn draw_title(
        &self,
        canvas: &mut Canvas<'_>,
        bar: &TitleBar<'_>,
        ink: Rgba,
    ) -> Result<(), UiError> {
        let scale = bar.scale_factor;
        let padding = TITLE_PADDING * scale;
        let area = rect(bar.title_area());
        let room = area.width - 2.0 * padding;
        if room <= 0.0 {
            return Ok(());
        }
        let style = self.theme.text_style().bold();
        let text = TextLayout::fitted(bar.title, &style, scale, room, Cut::End)?;
        let size = text.size();
        let centred = (f64::from(bar.size.width) - size.width) / 2.0;
        let x = centred
            .min(area.max_x() - padding - size.width)
            .max(padding);
        let y = (f64::from(bar.size.height) - size.height) / 2.0;
        canvas.draw_text(&text, Point::new(x, y), ink)
    }
}

impl DecorationPainter for TitleBarPainter {
    fn paint(&self, bar: &TitleBar<'_>, pixels: &mut [u8]) {
        // A bar that cannot be drawn (fonts failing) stays transparent:
        // its buttons still work.
        if let Ok(pixmap) = self.draw(bar) {
            let premultiplied = pixmap.to_premultiplied();
            if premultiplied.len() == pixels.len() {
                pixels.copy_from_slice(&premultiplied);
            }
        }
    }
}

/// A platform rectangle as a UI one.
fn rect(area: PhysicalRect) -> Rect {
    Rect::new(
        f64::from(area.x),
        f64::from(area.y),
        f64::from(area.width),
        f64::from(area.height),
    )
}

/// The symbol of a title bar button, on a 24-unit grid.
fn symbol_icon(button: TitleBarButton, maximized: bool) -> Icon {
    let p = Point::new;
    let square = |x0: f64, y0: f64, x1: f64, y1: f64| {
        Path::polyline(vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1), p(x0, y0)])
    };
    match button {
        TitleBarButton::Close => Icon::new(24.0)
            .stroke(Path::polyline(vec![p(6.0, 6.0), p(18.0, 18.0)]))
            .stroke(Path::polyline(vec![p(18.0, 6.0), p(6.0, 18.0)])),
        TitleBarButton::Maximize if maximized => Icon::new(24.0)
            .stroke(square(5.0, 8.0, 16.0, 19.0))
            .stroke(Path::polyline(vec![
                p(8.0, 8.0),
                p(8.0, 5.0),
                p(19.0, 5.0),
                p(19.0, 16.0),
                p(16.0, 16.0),
            ])),
        // `TitleBarButton` may gain buttons; draw them as a frame.
        _ => Icon::new(24.0).stroke(square(5.0, 5.0, 19.0, 19.0)),
    }
}

#[cfg(test)]
mod tests {
    use plt_platform::PhysicalSize;

    use super::*;

    fn bar(title: &str, hovered: Option<TitleBarButton>, focused: bool) -> TitleBar<'_> {
        TitleBar {
            title,
            size: PhysicalSize::new(240, 34),
            scale_factor: 1.0,
            focused,
            maximized: false,
            hovered,
            pressed: None,
        }
    }

    fn paint(bar: &TitleBar<'_>) -> Vec<u8> {
        let mut pixels = vec![0; bar.size.area() as usize * 4];
        TitleBarPainter::new(Theme::modern()).paint(bar, &mut pixels);
        pixels
    }

    fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
        &pixels[(y * 240 + x) * 4..(y * 240 + x) * 4 + 4]
    }

    #[test]
    fn the_title_is_drawn_between_the_edge_and_the_buttons() {
        let theme = Theme::modern();
        let background = theme.background.to_rgba8();
        let blank = paint(&bar("", None, true));
        let titled = paint(&bar("Figure 1", None, true));
        assert_eq!(pixel(&blank, 2, 2), background);
        let changed: Vec<usize> = (0..240)
            .filter(|&x| (0..34).any(|y| pixel(&blank, x, y) != pixel(&titled, x, y)))
            .collect();
        let (first, last) = (changed[0], changed[changed.len() - 1]);
        assert!(first >= 12 && last < 240 - 2 * 34, "{first}..{last}");
        let long = paint(&bar(&"long title ".repeat(20), None, false));
        assert_eq!(
            pixel(&long, 240 - 2 * 34 - 1, 17),
            background,
            "elided before the buttons"
        );
    }

    #[test]
    fn hovered_buttons_show_a_face() {
        let theme = Theme::modern();
        let background = theme.background.to_rgba8();
        let hovered = paint(&bar("t", Some(TitleBarButton::Close), true));
        assert_ne!(pixel(&hovered, 240 - 34 + 7, 8), background);
        let idle = paint(&bar("t", None, true));
        assert_eq!(pixel(&idle, 240 - 34 + 7, 8), background);
        let mut maximized = bar("t", None, true);
        maximized.maximized = true;
        assert_ne!(paint(&maximized), idle, "the restore symbol");
    }

    #[test]
    fn wrong_buffer_sizes_are_left_alone() {
        let mut pixels = vec![7; 12];
        TitleBarPainter::new(Theme::dark()).paint(&bar("t", None, true), &mut pixels);
        assert_eq!(pixels, [7; 12]);
    }
}
