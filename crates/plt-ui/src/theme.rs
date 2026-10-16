//! Colours and metrics shared by the widgets.

use plt_core::Rgba;

/// The look of a UI. Lengths are in logical pixels; widgets multiply them
/// by the window's scale factor.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Window background, behind every widget.
    pub background: Rgba,
    /// Text colour.
    pub text: Rgba,
    /// Button face.
    pub button: Rgba,
    /// Button face under the pointer.
    pub button_hot: Rgba,
    /// Button face while pressed.
    pub button_active: Rgba,
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

impl Default for Theme {
    /// A light theme close to matplotlib's Tk figure windows.
    fn default() -> Self {
        let gray = |level: f64| Rgba::new(level, level, level, 1.0);
        Self {
            background: gray(0.94),
            text: gray(0.1),
            button: gray(0.94),
            button_hot: gray(0.86),
            button_active: gray(0.75),
            surface: gray(1.0),
            font_size: 10.0,
            padding: 6.0,
            spacing: 2.0,
            corner_radius: 3.0,
        }
    }
}
