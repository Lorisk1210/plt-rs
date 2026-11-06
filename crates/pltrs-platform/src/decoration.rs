//! Client-side window decorations: the title bar and resize borders a
//! window draws itself where the desktop draws none, as Wayland
//! compositors without server-side decorations (GNOME's) expect.
//!
//! The platform owns their geometry and behaviour: the title bar's
//! height, which buttons it has and on which side ([`ButtonLayout`]),
//! and what the pointer does there (drag to move, the buttons, the
//! actions of double, middle and right clicks, resizing at the borders);
//! on Wayland, `wayland::decoration` does. Buttons and actions follow
//! the desktop's settings where it has them (GNOME's, read through the
//! XDG desktop portal), as GTK's windows do. Drawing the title bar is
//! left to a [`DecorationPainter`], so that a UI toolkit can match its
//! own theme;
//! [`EventLoop::set_decoration_painter`](crate::EventLoop::set_decoration_painter)
//! installs one. Without one, a plain built-in painter draws the bar and
//! its buttons but no title text.

use std::fmt;

use crate::geometry::{PhysicalRect, PhysicalSize};

/// A button of the title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TitleBarButton {
    /// Minimizes the window.
    Minimize,
    /// Maximizes the window, or restores it.
    Maximize,
    /// Asks to close the window
    /// ([`WindowEvent::CloseRequested`](crate::WindowEvent::CloseRequested)).
    Close,
}

/// Which buttons a title bar has, on its left and on its right, in
/// GNOME's and GTK's notation (`org.gnome.desktop.wm.preferences
/// button-layout`, `gtk-decoration-layout`): the left side's buttons, a
/// colon, the right side's, each side's separated by commas.
///
/// ```
/// use pltrs_platform::{ButtonLayout, TitleBarButton};
///
/// let gnome = ButtonLayout::parse("appmenu:close");
/// assert_eq!(gnome.left(), []);
/// assert_eq!(gnome.right(), [TitleBarButton::Close]);
/// let mac_like = ButtonLayout::parse("close,minimize,maximize:");
/// assert_eq!(mac_like.left()[0], TitleBarButton::Close);
/// assert_eq!(ButtonLayout::default(), ButtonLayout::parse(":minimize,maximize,close"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ButtonLayout {
    left: Vec<TitleBarButton>,
    right: Vec<TitleBarButton>,
}

impl ButtonLayout {
    /// The layout `layout` describes. Items other than `minimize`,
    /// `maximize` and `close` (`appmenu`, `icon`, `spacer`, ...) are
    /// skipped, as are buttons named twice; without a colon, every
    /// button is on the left, as in GTK.
    pub fn parse(layout: &str) -> Self {
        let (left, right) = layout.split_once(':').unwrap_or((layout, ""));
        let mut seen = Vec::new();
        let mut side = |names: &str| {
            names
                .split(',')
                .filter_map(|name| match name.trim() {
                    "minimize" => Some(TitleBarButton::Minimize),
                    "maximize" => Some(TitleBarButton::Maximize),
                    "close" => Some(TitleBarButton::Close),
                    _ => None,
                })
                .filter(|button| {
                    let new = !seen.contains(button);
                    seen.push(*button);
                    new
                })
                .collect()
        };
        let left = side(left);
        let right = side(right);
        Self { left, right }
    }

    /// The buttons on the left, from the left edge inwards.
    pub fn left(&self) -> &[TitleBarButton] {
        &self.left
    }

    /// The buttons on the right, from the inside outwards (the last is
    /// at the right edge).
    pub fn right(&self) -> &[TitleBarButton] {
        &self.right
    }

    /// The layout without the buttons `keep` refuses (those the
    /// compositor or the window cannot honour).
    #[cfg(target_os = "linux")]
    #[must_use]
    pub(crate) fn only(&self, keep: impl Fn(TitleBarButton) -> bool) -> Self {
        Self {
            left: self.left.iter().copied().filter(|&b| keep(b)).collect(),
            right: self.right.iter().copied().filter(|&b| keep(b)).collect(),
        }
    }
}

impl Default for ButtonLayout {
    /// Minimize, maximize and close on the right: GTK's layout without
    /// its menu, for desktops that have no settings.
    fn default() -> Self {
        Self {
            left: Vec::new(),
            right: vec![
                TitleBarButton::Minimize,
                TitleBarButton::Maximize,
                TitleBarButton::Close,
            ],
        }
    }
}

/// What a click on the title bar (outside its buttons) does
/// (`org.gnome.desktop.wm.preferences action-double-click-titlebar`, and
/// its middle and right clicks).
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TitleBarAction {
    /// Nothing.
    Nothing,
    /// Maximizes the window, or restores it (also GNOME's
    /// `toggle-maximize-horizontally` and `-vertically`, which Wayland
    /// windows cannot ask for).
    ToggleMaximize,
    /// Minimizes the window.
    Minimize,
    /// Opens the compositor's window menu.
    Menu,
}

#[cfg(target_os = "linux")]
impl TitleBarAction {
    /// The action GNOME names `name`; `lower`, `toggle-shade` and
    /// others Wayland windows cannot ask for do nothing, as in GTK.
    pub(crate) fn parse(name: &str) -> Self {
        match name {
            "minimize" => Self::Minimize,
            "menu" => Self::Menu,
            name if name.starts_with("toggle-maximize") => Self::ToggleMaximize,
            _ => Self::Nothing,
        }
    }
}

/// How the title bar behaves: its buttons and the actions of its clicks.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TitleBarSettings {
    /// The buttons.
    pub(crate) layout: ButtonLayout,
    /// What a double click does.
    pub(crate) double_click: TitleBarAction,
    /// What a middle click does.
    pub(crate) middle_click: TitleBarAction,
    /// What a right click does.
    pub(crate) right_click: TitleBarAction,
}

#[cfg(target_os = "linux")]
impl Default for TitleBarSettings {
    /// The default layout; GNOME's and GTK's default actions.
    fn default() -> Self {
        Self {
            layout: ButtonLayout::default(),
            double_click: TitleBarAction::ToggleMaximize,
            middle_click: TitleBarAction::Nothing,
            right_click: TitleBarAction::Menu,
        }
    }
}

#[cfg(target_os = "linux")]
impl TitleBarSettings {
    /// The settings GNOME's keys give (`button-layout` and
    /// `action-*-click-titlebar` of `org.gnome.desktop.wm.preferences`),
    /// the defaults for those missing.
    pub(crate) fn from_gnome(setting: impl Fn(&str) -> Option<String>) -> Self {
        let defaults = Self::default();
        let action =
            |key: &str, default| setting(key).map_or(default, |name| TitleBarAction::parse(&name));
        Self {
            layout: setting("button-layout")
                .map_or(defaults.layout, |layout| ButtonLayout::parse(&layout)),
            double_click: action("action-double-click-titlebar", defaults.double_click),
            middle_click: action("action-middle-click-titlebar", defaults.middle_click),
            right_click: action("action-right-click-titlebar", defaults.right_click),
        }
    }
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
    /// The buttons, and on which side.
    pub layout: &'a ButtonLayout,
}

impl TitleBar<'_> {
    /// The buttons and where they are, in physical pixels of the title
    /// bar: squares in a row from each end, as [`TitleBar::layout`] puts
    /// them.
    pub fn buttons(&self) -> Vec<(TitleBarButton, PhysicalRect)> {
        buttons(self.layout, self.size.width, self.size.height)
    }

    /// The part of the bar left for the title, between the buttons on
    /// the left and those on the right.
    pub fn title_area(&self) -> PhysicalRect {
        let (width, height) = (self.size.width, self.size.height);
        let left = side_width(self.layout.left.len(), height).min(width);
        let right = width
            .saturating_sub(side_width(self.layout.right.len(), height))
            .max(left);
        PhysicalRect::new(left, 0, right - left, height)
    }
}

/// The width of `count` square buttons `height` high.
fn side_width(count: usize, height: u32) -> u32 {
    u32::try_from(count).map_or(u32::MAX, |count| count.saturating_mul(height))
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

/// The buttons of a `width` × `height` title bar (in any unit) laid out
/// as `layout` says: squares from the left edge, and from the right
/// edge, cut where the bar is too narrow.
pub(crate) fn buttons(
    layout: &ButtonLayout,
    width: u32,
    height: u32,
) -> Vec<(TitleBarButton, PhysicalRect)> {
    let square = |x0: u32, x1: u32| PhysicalRect::new(x0, 0, x1 - x0, height);
    let mut left = 0;
    let mut placed: Vec<(TitleBarButton, PhysicalRect)> = layout
        .left
        .iter()
        .map(|&button| {
            let x0 = left.min(width);
            left = left.saturating_add(height);
            (button, square(x0, left.min(width)))
        })
        .collect();
    let mut right = width;
    let mut right_side: Vec<_> = layout
        .right
        .iter()
        .rev()
        .map(|&button| {
            let x1 = right;
            right = right.saturating_sub(height);
            (button, square(right, x1))
        })
        .collect();
    right_side.reverse();
    placed.extend(right_side);
    placed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(layout: &ButtonLayout, width: u32) -> TitleBar<'_> {
        TitleBar {
            title: "t",
            size: PhysicalSize::new(width, 30),
            scale_factor: 1.0,
            focused: true,
            maximized: false,
            hovered: None,
            pressed: None,
            layout,
        }
    }

    #[test]
    fn buttons_sit_at_the_ends_the_layout_names() {
        use TitleBarButton::{Close, Maximize, Minimize};
        let right = ButtonLayout::parse(":maximize,close");
        let bar = bar(&right, 200);
        assert_eq!(
            bar.buttons(),
            [
                (Maximize, PhysicalRect::new(140, 0, 30, 30)),
                (Close, PhysicalRect::new(170, 0, 30, 30)),
            ]
        );
        assert_eq!(bar.title_area(), PhysicalRect::new(0, 0, 140, 30));
        let narrow = TitleBar {
            size: PhysicalSize::new(40, 30),
            ..bar
        };
        assert_eq!(narrow.buttons()[0].1, PhysicalRect::new(0, 0, 10, 30));
        assert_eq!(narrow.title_area().width, 0);
        let both = ButtonLayout::parse("close,minimize:maximize");
        let bar = TitleBar {
            layout: &both,
            ..bar
        };
        assert_eq!(
            bar.buttons(),
            [
                (Close, PhysicalRect::new(0, 0, 30, 30)),
                (Minimize, PhysicalRect::new(30, 0, 30, 30)),
                (Maximize, PhysicalRect::new(170, 0, 30, 30)),
            ]
        );
        assert_eq!(bar.title_area(), PhysicalRect::new(60, 0, 110, 30));
        let none = ButtonLayout::parse(":");
        assert_eq!(
            TitleBar {
                layout: &none,
                ..bar
            }
            .buttons(),
            []
        );
        let tiny = TitleBar {
            size: PhysicalSize::new(20, 30),
            ..bar
        };
        assert_eq!(tiny.title_area(), PhysicalRect::new(20, 0, 0, 30));
        assert!(
            tiny.buttons()
                .iter()
                .all(|(_, rect)| rect.x + rect.width <= 20)
        );
    }

    #[test]
    fn layouts_read_gnomes_notation() {
        use TitleBarButton::{Close, Maximize, Minimize};
        let gnome = ButtonLayout::parse("appmenu:close");
        assert_eq!((gnome.left(), gnome.right()), (&[][..], &[Close][..]));
        let full = ButtonLayout::parse("icon:minimize,spacer,maximize,close");
        assert_eq!(full.right(), [Minimize, Maximize, Close]);
        let left = ButtonLayout::parse(" close , maximize ");
        assert_eq!(
            (left.left(), left.right()),
            (&[Close, Maximize][..], &[][..])
        );
        let twice = ButtonLayout::parse("close:close,minimize");
        assert_eq!(
            (twice.left(), twice.right()),
            (&[Close][..], &[Minimize][..])
        );
        assert_eq!(ButtonLayout::parse("").right(), []);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn buttons_can_be_left_out() {
        use TitleBarButton::{Close, Maximize, Minimize};
        let without = ButtonLayout::default().only(|button| button != Maximize);
        assert_eq!(without.right(), [Minimize, Close]);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn title_bar_actions_follow_gnomes_keys() {
        assert_eq!(
            TitleBarAction::parse("toggle-maximize"),
            TitleBarAction::ToggleMaximize
        );
        assert_eq!(
            TitleBarAction::parse("toggle-maximize-vertically"),
            TitleBarAction::ToggleMaximize
        );
        assert_eq!(TitleBarAction::parse("minimize"), TitleBarAction::Minimize);
        assert_eq!(TitleBarAction::parse("menu"), TitleBarAction::Menu);
        assert_eq!(TitleBarAction::parse("lower"), TitleBarAction::Nothing);
        assert_eq!(TitleBarAction::parse("none"), TitleBarAction::Nothing);
        let keys = [
            ("button-layout", "close:"),
            ("action-double-click-titlebar", "minimize"),
            ("action-right-click-titlebar", "none"),
        ];
        let settings = TitleBarSettings::from_gnome(|key| {
            keys.iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).to_owned())
        });
        assert_eq!(settings.layout.left(), [TitleBarButton::Close]);
        assert_eq!(settings.double_click, TitleBarAction::Minimize);
        assert_eq!(
            settings.middle_click,
            TitleBarAction::Nothing,
            "the default"
        );
        assert_eq!(settings.right_click, TitleBarAction::Nothing);
        assert_eq!(
            TitleBarSettings::from_gnome(|_| None),
            TitleBarSettings::default()
        );
    }
}
