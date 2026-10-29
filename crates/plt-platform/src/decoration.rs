//! Client-side window decorations: the title bar and resize borders a
//! window draws itself where the desktop draws none, as Wayland
//! compositors without server-side decorations (GNOME's) expect.
//!
//! The platform owns their geometry and behaviour: the title bar's
//! height, where its buttons are, and what the pointer does there (drag
//! to move, double-click to maximize, buttons, the window menu, resizing
//! at the borders); on Wayland, `wayland::decoration` does. Drawing the title bar is left to a
//! [`DecorationPainter`], so that a UI toolkit can match its own theme;
//! [`EventLoop::set_decoration_painter`](crate::EventLoop::set_decoration_painter)
//! installs one. Without one, a plain built-in painter draws the bar and
//! its buttons but no title text.

use std::fmt;

use crate::geometry::{PhysicalRect, PhysicalSize};

/// A button of the title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TitleBarButton {
    /// Maximizes the window, or restores it.
    Maximize,
    /// Asks to close the window
    /// ([`WindowEvent::CloseRequested`](crate::WindowEvent::CloseRequested)).
    Close,
}

/// What a title bar shows: everything a [`DecorationPainter`] needs.
#[derive(Debug, Clone, PartialEq)]
pub struct TitleBar<'a> {
    /// The window title.
    pub title: &'a str,
    /// The title bar's size in physical pixels.
    pub size: PhysicalSize,
    /// The window's scale factor (physical / logical pixels).
    pub scale_factor: f64,
    /// Whether the window has the keyboard focus.
    pub focused: bool,
    /// Whether the window is maximized (the maximize button restores).
    pub maximized: bool,
    /// The button under the pointer.
    pub hovered: Option<TitleBarButton>,
    /// The button held down.
    pub pressed: Option<TitleBarButton>,
}

impl TitleBar<'_> {
    /// The buttons and where they are, in physical pixels of the title
    /// bar: squares at its right end, the close button outermost.
    pub fn buttons(&self) -> [(TitleBarButton, PhysicalRect); 2] {
        buttons(self.size.width, self.size.height)
    }

    /// The part of the bar left for the title, between the left edge and
    /// the buttons.
    pub fn title_area(&self) -> PhysicalRect {
        let buttons_width = self.size.height.saturating_mul(2);
        PhysicalRect::new(
            0,
            0,
            self.size.width.saturating_sub(buttons_width),
            self.size.height,
        )
    }
}

/// Draws the title bars of client-side decorations.
pub trait DecorationPainter {
    /// Draws `title_bar` into `pixels`: premultiplied RGBA8, row-major,
    /// top row first, `title_bar.size` pixels, initially transparent.
    fn paint(&self, title_bar: &TitleBar<'_>, pixels: &mut [u8]);
}

impl fmt::Debug for dyn DecorationPainter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DecorationPainter")
    }
}

/// The buttons of a `width` × `height` title bar (in any unit).
pub(crate) fn buttons(width: u32, height: u32) -> [(TitleBarButton, PhysicalRect); 2] {
    let close_x = width.saturating_sub(height);
    let maximize_x = close_x.saturating_sub(height);
    [
        (
            TitleBarButton::Maximize,
            PhysicalRect::new(maximize_x, 0, close_x - maximize_x, height),
        ),
        (
            TitleBarButton::Close,
            PhysicalRect::new(close_x, 0, width - close_x, height),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_sit_at_the_right_end() {
        let bar = TitleBar {
            title: "t",
            size: PhysicalSize::new(200, 30),
            scale_factor: 1.0,
            focused: true,
            maximized: false,
            hovered: None,
            pressed: None,
        };
        assert_eq!(
            bar.buttons(),
            [
                (TitleBarButton::Maximize, PhysicalRect::new(140, 0, 30, 30)),
                (TitleBarButton::Close, PhysicalRect::new(170, 0, 30, 30)),
            ]
        );
        assert_eq!(bar.title_area(), PhysicalRect::new(0, 0, 140, 30));
        let narrow = TitleBar {
            size: PhysicalSize::new(40, 30),
            ..bar
        };
        assert_eq!(narrow.buttons()[0].1, PhysicalRect::new(0, 0, 10, 30));
        assert_eq!(narrow.title_area().width, 0);
    }
}
