//! Colours, metrics and the spacing scale shared by the widgets.
//!
//! Widgets take every colour and length from the [`Theme`], so the look
//! of a UI is defined in one place: change the theme and every widget
//! follows.

use std::time::Duration;

use plt_core::Rgba;

use crate::text::{FontFamily, TextStyle};

/// A step of the theme's spacing scale: multiples of
/// [`Theme::space_unit`] (4 logical pixels in the built-in themes), used
/// for padding and the gaps between widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Spacing {
    /// No space.
    #[default]
    None,
    /// Half a unit (2 px): between the buttons of a toolbar.
    Xxs,
    /// One unit (4 px): between a heading and what it heads.
    Xs,
    /// Two units (8 px): between related controls.
    Sm,
    /// Three units (12 px): inside cards and footers.
    Md,
    /// Four units (16 px): dialog margins and between sections.
    Lg,
    /// Six units (24 px): between unrelated groups.
    Xl,
}

impl Spacing {
    /// The step in spacing units.
    fn units(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Xxs => 0.5,
            Self::Xs => 1.0,
            Self::Sm => 2.0,
            Self::Md => 3.0,
            Self::Lg => 4.0,
            Self::Xl => 6.0,
        }
    }
}

/// The look of a UI. Lengths are in logical pixels; widgets multiply them
/// by the window's scale factor.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Window background, behind every widget.
    pub background: Rgba,
    /// Background of content: cards, text fields, lists, image surfaces.
    pub surface: Rgba,
    /// Recessed background: the track of segmented controls.
    pub track: Rgba,
    /// Text and icon colour.
    pub text: Rgba,
    /// Quieter text: hints, section headings, the status line.
    pub text_secondary: Rgba,
    /// Text and icons of disabled widgets.
    pub text_disabled: Rgba,
    /// Text on accent-coloured faces.
    pub text_on_accent: Rgba,
    /// Highlight colour: primary buttons, selected tabs, focus, checked
    /// tools.
    pub accent: Rgba,
    /// The accent under the pointer.
    pub accent_hot: Rgba,
    /// The accent while pressed.
    pub accent_active: Rgba,
    /// Outline of controls: text fields, buttons, lists, check boxes.
    pub border: Rgba,
    /// Outline of a control under the pointer.
    pub border_hot: Rgba,
    /// Hairlines: separators and the outlines of cards.
    pub separator: Rgba,
    /// Face of secondary buttons.
    pub button: Rgba,
    /// Face of a secondary button under the pointer.
    pub button_hot: Rgba,
    /// Face of a secondary button while pressed.
    pub button_active: Rgba,
    /// Translucent wash over flat controls (tool buttons, list rows,
    /// tabs) under the pointer.
    pub hover: Rgba,
    /// Translucent wash over flat controls while pressed.
    pub pressed: Rgba,
    /// Translucent accent behind checked tools, chosen segments and the
    /// selection of focused widgets.
    pub selection: Rgba,
    /// Behind the selection of widgets without focus.
    pub selection_inactive: Rgba,
    /// The translucent halo of the focus ring around the focused control
    /// (the ring's edge is in the accent colour).
    pub focus_ring: Rgba,
    /// Errors.
    pub danger: Rgba,
    /// Soft shadow under raised elements (segment thumbs, tooltips).
    pub shadow: Rgba,
    /// Tooltip background.
    pub tooltip: Rgba,
    /// Tooltip text.
    pub tooltip_text: Rgba,
    /// Typeface of text.
    pub font_family: FontFamily,
    /// Font size of text, in points.
    pub font_size: f64,
    /// The unit of the spacing scale (see [`Spacing`]).
    pub space_unit: f64,
    /// Height of controls: buttons, text fields, segmented controls,
    /// list rows.
    pub control_height: f64,
    /// Side of toolbar buttons.
    pub tool_button_size: f64,
    /// Side of toolbar icons.
    pub icon_size: f64,
    /// Side of the icons in list rows.
    pub small_icon_size: f64,
    /// Side of check boxes.
    pub checkbox_size: f64,
    /// Corner radius of controls.
    pub corner_radius: f64,
    /// Corner radius of cards and tooltips.
    pub card_radius: f64,
    /// Width of the ring around the focused control, drawn outside it.
    pub focus_ring_width: f64,
    /// Thickness of the line under the selected tab.
    pub indicator_width: f64,
    /// Width text fields ask for.
    pub field_width: f64,
    /// Narrowest push button, so that buttons side by side match.
    pub button_min_width: f64,
    /// Narrowest label column of forms, so that forms line up.
    pub label_min_width: f64,
    /// Narrowest list.
    pub list_min_width: f64,
    /// Narrowest segment of segmented controls.
    pub segment_min_width: f64,
    /// Width of scroll bars.
    pub scrollbar_width: f64,
    /// How long the pointer rests on a widget before its tooltip shows.
    pub tooltip_delay: Duration,
}

/// An opaque colour from 8-bit channels.
fn rgb(r: u8, g: u8, b: u8) -> Rgba {
    rgba(r, g, b, 1.0)
}

/// A colour from 8-bit channels and an opacity.
fn rgba(r: u8, g: u8, b: u8, alpha: f64) -> Rgba {
    Rgba::new(
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
        alpha,
    )
}

impl Theme {
    /// plt-rs' own light look: a quiet off-white chrome with white cards
    /// and fields, flat tool buttons that only show a face when hovered
    /// or pressed, rounded corners, a blue accent and Inter type.
    pub fn modern() -> Self {
        Self {
            background: rgb(246, 247, 249),
            surface: rgb(255, 255, 255),
            track: rgb(234, 236, 240),
            text: rgb(31, 35, 40),
            text_secondary: rgb(96, 104, 115),
            text_disabled: rgb(170, 175, 182),
            text_on_accent: rgb(255, 255, 255),
            accent: rgb(47, 111, 228),
            accent_hot: rgb(37, 96, 207),
            accent_active: rgb(29, 80, 178),
            border: rgb(208, 213, 221),
            border_hot: rgb(160, 168, 180),
            separator: rgb(225, 228, 233),
            button: rgb(255, 255, 255),
            button_hot: rgb(243, 244, 246),
            button_active: rgb(230, 232, 236),
            hover: rgba(31, 35, 40, 0.07),
            pressed: rgba(31, 35, 40, 0.13),
            selection: rgba(47, 111, 228, 0.16),
            selection_inactive: rgba(47, 111, 228, 0.09),
            focus_ring: rgba(47, 111, 228, 0.35),
            danger: rgb(207, 34, 46),
            shadow: rgba(16, 24, 40, 0.12),
            tooltip: rgb(36, 41, 47),
            tooltip_text: rgb(255, 255, 255),
            font_family: FontFamily::Interface,
            font_size: 9.5,
            space_unit: 4.0,
            control_height: 28.0,
            tool_button_size: 32.0,
            icon_size: 20.0,
            small_icon_size: 16.0,
            checkbox_size: 16.0,
            corner_radius: 6.0,
            card_radius: 10.0,
            focus_ring_width: 3.0,
            indicator_width: 2.0,
            field_width: 180.0,
            button_min_width: 80.0,
            label_min_width: 64.0,
            list_min_width: 160.0,
            segment_min_width: 52.0,
            scrollbar_width: 6.0,
            tooltip_delay: Duration::from_millis(600),
        }
    }

    /// plt-rs' own dark look, for figures on dark backgrounds: the modern
    /// layout in charcoal, with light text and a brighter accent.
    pub fn dark() -> Self {
        Self {
            background: rgb(30, 32, 36),
            surface: rgb(40, 43, 48),
            track: rgb(23, 25, 28),
            text: rgb(230, 233, 237),
            text_secondary: rgb(158, 166, 176),
            text_disabled: rgb(96, 102, 111),
            text_on_accent: rgb(255, 255, 255),
            accent: rgb(76, 139, 245),
            accent_hot: rgb(98, 154, 247),
            accent_active: rgb(61, 121, 222),
            border: rgb(70, 75, 83),
            border_hot: rgb(104, 111, 122),
            separator: rgb(52, 56, 62),
            button: rgb(48, 52, 58),
            button_hot: rgb(57, 61, 68),
            button_active: rgb(66, 71, 79),
            hover: rgba(255, 255, 255, 0.08),
            pressed: rgba(255, 255, 255, 0.14),
            selection: rgba(76, 139, 245, 0.28),
            selection_inactive: rgba(76, 139, 245, 0.14),
            focus_ring: rgba(76, 139, 245, 0.5),
            danger: rgb(248, 81, 73),
            shadow: rgba(0, 0, 0, 0.4),
            tooltip: rgb(232, 234, 237),
            tooltip_text: rgb(30, 32, 36),
            ..Self::modern()
        }
    }

    /// A light theme close to matplotlib's Tk figure windows: greys,
    /// square-ish corners and matplotlib's `DejaVu Sans`.
    pub fn classic() -> Self {
        let gray = |level: f64| Rgba::new(level, level, level, 1.0);
        let wash = |alpha: f64| Rgba::new(0.0, 0.0, 0.0, alpha);
        let blue = Rgba::new(0.18, 0.36, 0.66, 1.0);
        Self {
            background: gray(0.94),
            surface: gray(1.0),
            track: gray(0.88),
            text: gray(0.1),
            text_secondary: gray(0.35),
            text_disabled: gray(0.6),
            text_on_accent: gray(1.0),
            accent: blue,
            accent_hot: Rgba::new(0.15, 0.31, 0.58, 1.0),
            accent_active: Rgba::new(0.12, 0.26, 0.5, 1.0),
            border: gray(0.7),
            border_hot: gray(0.5),
            separator: gray(0.8),
            button: gray(0.94),
            button_hot: gray(0.86),
            button_active: gray(0.75),
            hover: wash(0.08),
            pressed: wash(0.18),
            selection: Rgba::new(0.18, 0.36, 0.66, 0.2),
            selection_inactive: wash(0.1),
            focus_ring: Rgba::new(0.18, 0.36, 0.66, 0.4),
            danger: Rgba::new(0.75, 0.1, 0.1, 1.0),
            shadow: wash(0.15),
            tooltip: Rgba::new(1.0, 1.0, 0.88, 1.0),
            tooltip_text: gray(0.0),
            font_family: FontFamily::SansSerif,
            font_size: 10.0,
            space_unit: 4.0,
            control_height: 26.0,
            tool_button_size: 32.0,
            icon_size: 20.0,
            small_icon_size: 16.0,
            checkbox_size: 14.0,
            corner_radius: 3.0,
            card_radius: 4.0,
            focus_ring_width: 2.0,
            indicator_width: 2.0,
            field_width: 180.0,
            button_min_width: 80.0,
            label_min_width: 64.0,
            list_min_width: 160.0,
            segment_min_width: 52.0,
            scrollbar_width: 8.0,
            tooltip_delay: Duration::from_millis(600),
        }
    }

    /// The style of body text: the theme's typeface at its font size.
    pub fn text_style(&self) -> TextStyle {
        TextStyle {
            family: self.font_family,
            ..TextStyle::new(self.font_size)
        }
    }

    /// The length of a step of the spacing scale, in logical pixels.
    pub fn space(&self, step: Spacing) -> f64 {
        step.units() * self.space_unit
    }
}

impl Default for Theme {
    /// The [modern](Theme::modern) theme.
    fn default() -> Self {
        Self::modern()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_themes_set_text_in_inter_and_classic_in_dejavu() {
        assert_eq!(Theme::modern().text_style().family, FontFamily::Interface);
        assert_eq!(Theme::dark().text_style().family, FontFamily::Interface);
        let classic = Theme::classic().text_style();
        assert_eq!(classic.family, FontFamily::SansSerif);
        assert_eq!(classic.size, Theme::classic().font_size);
    }

    #[test]
    fn the_spacing_scale_grows_in_units() {
        let theme = Theme::modern();
        let steps = [
            Spacing::None,
            Spacing::Xxs,
            Spacing::Xs,
            Spacing::Sm,
            Spacing::Md,
            Spacing::Lg,
            Spacing::Xl,
        ];
        let lengths: Vec<f64> = steps.iter().map(|&step| theme.space(step)).collect();
        assert_eq!(lengths, [0.0, 2.0, 4.0, 8.0, 12.0, 16.0, 24.0]);
        assert!(lengths.windows(2).all(|pair| pair[0] < pair[1]));
    }

    /// The WCAG contrast ratio of two opaque colours.
    fn contrast(a: Rgba, b: Rgba) -> f64 {
        let linear = |c: f64| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        let luminance =
            |c: Rgba| 0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b);
        let (hi, lo) = (
            luminance(a).max(luminance(b)),
            luminance(a).min(luminance(b)),
        );
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn text_reads_on_every_background() {
        for theme in [Theme::modern(), Theme::dark(), Theme::classic()] {
            for back in [theme.background, theme.surface, theme.button] {
                assert!(contrast(theme.text, back) > 7.0);
                assert!(contrast(theme.text_secondary, back) > 3.0);
            }
            assert!(contrast(theme.text_on_accent, theme.accent) > 3.0);
            assert!(contrast(theme.tooltip_text, theme.tooltip) > 7.0);
        }
    }

    #[test]
    fn focus_rings_stand_out() {
        // WCAG asks focus indicators for a 3:1 contrast with what is
        // around them: the ring's accent edge against the window and the
        // fields.
        for theme in [Theme::modern(), Theme::dark(), Theme::classic()] {
            for back in [theme.background, theme.surface] {
                assert!(contrast(theme.accent, back) >= 3.0);
            }
            assert!(theme.focus_ring.a > 0.0 && theme.focus_ring_width > 0.0);
        }
    }

    #[test]
    fn controls_fit_the_toolbar_and_the_grid() {
        for theme in [Theme::modern(), Theme::dark(), Theme::classic()] {
            assert!(theme.icon_size < theme.tool_button_size);
            assert!(theme.checkbox_size < theme.control_height);
            assert!(theme.corner_radius * 2.0 < theme.control_height);
            assert_eq!(theme.control_height % (theme.space_unit / 2.0), 0.0);
        }
    }
}
