//! Characters sized to fit: auto-sized delimiters and radicals
//! (`AutoHeightChar`) and wide accents (`AutoWidthChar`).

use super::{Char, List, Node, State};
use crate::mathtext::font_name::FontName;
use crate::mathtext::fonts::{Fonts, GlyphError};

/// The error for a symbol without any size variant.
fn no_alternative(symbol: &str) -> GlyphError {
    GlyphError::UnknownSymbol(symbol.to_owned())
}

/// A character as close as possible to `height + depth` tall: the first
/// size variant that is tall enough, scaled to fit unless it is the
/// regular-size STIX glyph. `factor` forces the scale.
pub(crate) fn auto_height_char(
    symbol: &str,
    height: f64,
    depth: f64,
    state: &State,
    factor: Option<f64>,
    fonts: &mut Fonts<'_>,
) -> Result<List, GlyphError> {
    let alternatives = fonts.sized_alternatives(state.font, symbol)?;
    let x_height = fonts.x_height(state.font, state.fontsize)?;
    let mut state = *state;
    let target_total = height + depth;
    let mut chosen = None;
    for (font, sym) in &alternatives {
        state.set_font(*font);
        let c = Char::new(sym, &state, fonts)?;
        // Ensure that size 0 is chosen when the text is regular sized but
        // with descender glyphs.
        let tall_enough = c.height + c.depth >= target_total - 0.2 * x_height;
        chosen = Some((c, sym));
        if tall_enough {
            break;
        }
    }
    let (mut c, sym) = chosen.ok_or_else(|| no_alternative(symbol))?;
    let mut shift = 0.0;
    if state.font != FontName::Size(0) || alternatives.len() == 1 {
        let factor = factor.unwrap_or(target_total / (c.height + c.depth));
        state.fontsize *= factor;
        c = Char::new(sym, &state, fonts)?;
        shift = depth - c.depth;
    }
    let mut list = List::hlist(vec![Node::Char(c)], fonts)?;
    list.shift_amount = shift;
    Ok(list)
}

/// A character as close as possible to `width` wide: the first variant
/// that is wide enough, scaled to exactly `width`. Accents are measured by
/// their ink box.
pub(crate) fn auto_width_char(
    symbol: &str,
    width: f64,
    state: &State,
    accent: bool,
    fonts: &mut Fonts<'_>,
) -> Result<List, GlyphError> {
    let make = |sym: &str, state: &State, fonts: &mut Fonts<'_>| {
        if accent {
            Char::accent(sym, state, fonts)
        } else {
            Char::new(sym, state, fonts)
        }
    };
    let alternatives = fonts.sized_alternatives(state.font, symbol)?;
    let mut state = *state;
    let mut chosen = None;
    for (font, sym) in &alternatives {
        state.set_font(*font);
        let c = make(sym, &state, fonts)?;
        let wide_enough = c.width >= width;
        chosen = Some((c, sym));
        if wide_enough {
            break;
        }
    }
    let (c, sym) = chosen.ok_or_else(|| no_alternative(symbol))?;
    let factor = width / c.width;
    state.fontsize *= factor;
    let c = make(sym, &state, fonts)?;
    let char_width = c.width;
    let mut list = List::hlist(vec![Node::Char(c)], fonts)?;
    list.width = char_width;
    Ok(list)
}
