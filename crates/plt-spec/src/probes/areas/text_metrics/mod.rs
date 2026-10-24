//! Probe operations for the `text_metrics` area: the font manager
//! (`findfont`, font lists, `FontProperties`, fontconfig patterns), text
//! extents and layout, wrapping, mathtext metrics and hinted glyphs.
//!
//! Every op runs under the reference configuration documented in
//! `spec/reference/probes/test_text_metrics.py`. Font lookups use only
//! matplotlib's bundled fonts, as the reference does; text measurement uses
//! the fonts text is drawn with, which agree whenever the host's own fonts
//! do not shadow the families asked for.

mod args;
mod fonts;
mod glyphs;
mod text;

pub(crate) use args::{
    apply_text_kw, font_dict, font_properties, rc_text, rotation, weight, with_rc,
};
pub(crate) use fonts::{findfont, font_properties_op, parse_fontconfig_pattern, properties_json};
pub(crate) use glyphs::{bundled_font, database, load_glyph};

use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("font_manager.font_list", fonts::font_list),
    Op::new("font_manager.findfont", fonts::findfont),
    Op::new(
        "font_manager.FontManager._find_fonts_by_props",
        fonts::find_fonts_by_props,
    ),
    Op::new("font_manager.FontProperties", fonts::font_properties_op),
    Op::new(
        "_fontconfig_pattern.parse_fontconfig_pattern",
        fonts::parse_fontconfig_pattern,
    ),
    Op::new("text.Text.get_window_extent", text::window_extent),
    Op::new("text.Text._get_layout", text::layout),
    Op::new("text.Text._get_wrapped_text", text::wrapped_text),
    Op::new(
        "backend_agg.RendererAgg.get_text_width_height_descent",
        text::width_height_descent,
    ),
    Op::new("ft2font.FT2Font.load_glyph", glyphs::load_glyph),
];
