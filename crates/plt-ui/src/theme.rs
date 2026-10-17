//! Colours and metrics shared by the widgets.

use plt_core::Rgba;

/// The look of a UI. Lengths are in logical pixels; widgets multiply them
/// by the window's scale factor.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Window background, behind every widget.
    pub background: Rgba,
    /// Text and icon colour.
    pub text: Rgba,
    /// Text and icons of disabled widgets.
    pub text_disabled: Rgba,
    /// Highlight colour: checked tools, selections.
    pub accent: Rgba,
    /// Button face.
    pub button: Rgba,
    /// Button face under the pointer.
    pub button_hot: Rgba,
    /// Button face while pressed.
    pub button_active: Rgba,
    /// Face of a checked (toggled on) button.
    pub button_checked: Rgba,
    /// Thin lines separating groups of widgets.
    pub separator: Rgba,
    /// Background of an image surface before it has an image.
    pub surface: Rgba,
    /// Font size of labels and buttons, in points.
    pub font_size: f64,
    /// Space between a widget's border and its content.
    pub padding: f64,
    /// Space between neighbouring widgets.
    pub spacing: f64,
    /// Corner radius of buttons.
    pub corner_radius: f64,
}

/// An opaque colour from 8-bit channels.
fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    Rgba::new(
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
        1.0,
    )
}

impl Theme {
    /// plt-rs' own light look: a quiet off-white chrome, flat buttons that
    /// only show a face when hovered or pressed, rounded corners and a
    /// blue accent for active tools and selections.
    pub fn modern() -> Self {
        Self {
            background: rgb(246, 247, 249),
            text: rgb(31, 35, 40),
            text_disabled: rgb(170, 175, 182),
            accent: rgb(47, 111, 228),
            button: rgb(246, 247, 249),
            button_hot: rgb(232, 234, 238),
            button_active: rgb(218, 222, 228),
            button_checked: rgb(219, 231, 253),
            separator: rgb(222, 225, 230),
            surface: rgb(255, 255, 255),
            font_size: 9.5,
            padding: 6.0,
            spacing: 2.0,
            corner_radius: 6.0,
        }
    }

    /// A light theme close to matplotlib's Tk figure windows.
    pub fn classic() -> Self {
        let gray = |level: f64| Rgba::new(level, level, level, 1.0);
        Self {
            background: gray(0.94),
            text: gray(0.1),
            text_disabled: gray(0.6),
            accent: gray(0.1),
            button: gray(0.94),
            button_hot: gray(0.86),
            button_active: gray(0.75),
            button_checked: gray(0.8),
            separator: gray(0.8),
            surface: gray(1.0),
            font_size: 10.0,
            padding: 6.0,
            spacing: 2.0,
            corner_radius: 3.0,
        }
    }
}

impl Default for Theme {
    /// The [modern](Theme::modern) theme.
    fn default() -> Self {
        Self::modern()
    }
}
