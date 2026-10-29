//! The modern chrome's design tokens as CSS custom properties: the colours
//! and metrics of pltrs' windows ([`Theme`]), so that the pages and the
//! windows share every value. The light theme is the page's; the dark one
//! applies under `.pltrs-dark`, to pages and figures drawn on a dark
//! background.

use std::fmt::Write as _;

use pltrs_core::Rgba;
use pltrs_ui::{Spacing, Theme};

/// The class that gives an element (a page or a figure) the dark theme.
pub(crate) const DARK_CLASS: &str = "pltrs-dark";

/// The colours, by custom property name (without the `--plt-` prefix).
fn colors(theme: &Theme) -> [(&'static str, Rgba); 18] {
    [
        ("background", theme.background),
        ("surface", theme.surface),
        ("text", theme.text),
        ("text-secondary", theme.text_secondary),
        ("text-disabled", theme.text_disabled),
        ("accent", theme.accent),
        ("accent-text", theme.accent_text),
        ("border", theme.border),
        ("border-hot", theme.border_hot),
        ("border-strong", theme.border_strong),
        ("separator", theme.separator),
        ("hover", theme.hover),
        ("pressed", theme.pressed),
        ("selection", theme.selection),
        ("focus-ring", theme.focus_ring),
        ("shadow", theme.shadow),
        ("rubber-band", theme.rubber_band),
        ("rubber-band-edge", theme.rubber_band_edge),
    ]
}

/// The lengths in CSS pixels, by custom property name.
fn lengths(theme: &Theme) -> [(&'static str, f64); 11] {
    [
        ("space-xs", theme.space(Spacing::Xs)),
        ("space-sm", theme.space(Spacing::Sm)),
        ("space-md", theme.space(Spacing::Md)),
        ("space-xl", theme.space(Spacing::Xl)),
        ("control-height", theme.control_height),
        ("tool-button-size", theme.tool_button_size),
        ("icon-size", theme.icon_size),
        ("corner-radius", theme.corner_radius),
        ("card-radius", theme.card_radius),
        ("focus-ring-width", theme.focus_ring_width),
        ("target-size", theme.target_size),
    ]
}

/// A colour as CSS writes it: `#rrggbb`, or `#rrggbbaa` when translucent.
fn css_color(color: Rgba) -> String {
    color.to_hex(color.a < 1.0)
}

/// The rule giving the elements `selector` matches `theme`, whose native
/// controls and scroll bars follow `scheme` (`light` or `dark`).
fn rule(selector: &str, scheme: &str, theme: &Theme) -> String {
    let mut rule = format!("{selector} {{\n  color-scheme: {scheme};\n");
    // Writing to a String cannot fail.
    for (name, color) in colors(theme) {
        let _ = writeln!(rule, "  --pltrs-{name}: {};", css_color(color));
    }
    for (name, length) in lengths(theme) {
        let _ = writeln!(rule, "  --pltrs-{name}: {length}px;");
    }
    let _ = writeln!(rule, "  --pltrs-font-size: {}pt;\n}}", theme.font_size);
    rule
}

/// The custom properties of both themes: the light one on the page, the
/// dark one on [`DARK_CLASS`].
pub(super) fn declarations() -> String {
    format!(
        "{}\n{}",
        rule(":root", "light", &Theme::modern()),
        rule(&format!(".{DARK_CLASS}"), "dark", &Theme::dark())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_themes_declare_every_token() {
        let css = declarations();
        let (light, dark) = css.split_once(".pltrs-dark {").expect("a dark rule");
        assert!(light.starts_with(":root {\n  color-scheme: light;\n"));
        assert!(dark.starts_with("\n  color-scheme: dark;\n"));
        for (name, _) in colors(&Theme::modern()) {
            let property = format!("  --pltrs-{name}: #");
            assert_eq!(light.matches(&property).count(), 1, "{name}");
            assert_eq!(dark.matches(&property).count(), 1, "{name}");
        }
        assert!(light.contains("  --pltrs-background: #f6f7f9;\n"));
        assert!(dark.contains("  --pltrs-background: #1e2024;\n"));
        assert!(light.contains("  --pltrs-tool-button-size: 32px;\n"));
        assert!(light.contains("  --pltrs-space-xl: 24px;\n"));
        assert!(light.contains("  --pltrs-font-size: 9.5pt;\n}"));
    }

    #[test]
    fn translucent_colours_keep_their_opacity() {
        assert_eq!(css_color(Rgba::new(1.0, 0.0, 0.0, 1.0)), "#ff0000");
        assert_eq!(css_color(Rgba::new(0.0, 0.0, 0.0, 0.0)), "#00000000");
        assert_eq!(css_color(Theme::modern().hover), "#1f232812");
    }
}
