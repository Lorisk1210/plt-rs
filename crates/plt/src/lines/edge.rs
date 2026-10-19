//! The colour of a line's marker edges (matplotlib's
//! `Line2D.get_markeredgecolor`).

use super::line::LineArtist;
use crate::colors::Color;
use crate::markers::{FillStyle, Marker, MarkerStyle, MarkerSymbol};
use crate::rc::ColorOrAuto;

impl LineArtist {
    /// The marker edge colour before alpha. Like matplotlib, an automatic
    /// edge reads `_internal.classic_mode` when asked: see
    /// [`marker_edge_color`].
    pub(crate) fn marker_edge(&self) -> Color {
        marker_edge_color(
            &self.markeredgecolor,
            &self.color,
            &self.marker,
            self.fillstyle,
            crate::rc::params().internal_classic_mode,
        )
    }
}

/// The marker edge colour of a line drawn in `line_color`: `edge` itself,
/// or for `auto` the line colour — except in matplotlib 1.x's classic mode,
/// where filled markers other than points and pixels get black edges.
fn marker_edge_color(
    edge: &ColorOrAuto,
    line_color: &Color,
    marker: &Marker,
    fillstyle: FillStyle,
    classic: bool,
) -> Color {
    match edge {
        ColorOrAuto::Color(color) => color.clone(),
        ColorOrAuto::Auto if classic && has_classic_black_edge(marker, fillstyle) => {
            Color::from("k")
        }
        ColorOrAuto::Auto => line_color.clone(),
    }
}

/// Whether classic mode draws the automatic edge of `marker` in black: it
/// has a face that is filled, and it is neither a point nor a pixel.
fn has_classic_black_edge(marker: &Marker, fillstyle: FillStyle) -> bool {
    !matches!(
        marker,
        Marker::Symbol(MarkerSymbol::Point | MarkerSymbol::Pixel)
    ) && fillstyle != FillStyle::None
        && MarkerStyle::new(marker.clone(), fillstyle).is_ok_and(|style| style.is_filled())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auto_edge(symbol: MarkerSymbol, fillstyle: FillStyle, classic: bool) -> Color {
        marker_edge_color(
            &ColorOrAuto::Auto,
            &Color::from("b"),
            &Marker::Symbol(symbol),
            fillstyle,
            classic,
        )
    }

    #[test]
    fn automatic_edges_follow_the_line_outside_classic_mode() {
        for symbol in [
            MarkerSymbol::Circle,
            MarkerSymbol::Point,
            MarkerSymbol::Plus,
        ] {
            assert_eq!(auto_edge(symbol, FillStyle::Full, false), Color::from("b"));
        }
    }

    #[test]
    fn classic_mode_draws_filled_marker_edges_black() {
        for symbol in [
            MarkerSymbol::Circle,
            MarkerSymbol::Square,
            MarkerSymbol::Diamond,
        ] {
            assert_eq!(auto_edge(symbol, FillStyle::Full, true), Color::from("k"));
        }
        assert_eq!(
            auto_edge(MarkerSymbol::Circle, FillStyle::Left, true),
            Color::from("k")
        );
    }

    #[test]
    fn classic_mode_keeps_the_line_colour_for_points_and_unfilled_markers() {
        for symbol in [
            MarkerSymbol::Point,
            MarkerSymbol::Pixel,
            MarkerSymbol::Plus,
            MarkerSymbol::X,
        ] {
            assert_eq!(auto_edge(symbol, FillStyle::Full, true), Color::from("b"));
        }
        assert_eq!(
            auto_edge(MarkerSymbol::Circle, FillStyle::None, true),
            Color::from("b")
        );
    }

    #[test]
    fn explicit_edges_win_in_every_mode() {
        let explicit = ColorOrAuto::Color(Color::from("r"));
        for classic in [false, true] {
            let color = marker_edge_color(
                &explicit,
                &Color::from("b"),
                &Marker::Symbol(MarkerSymbol::Circle),
                FillStyle::Full,
                classic,
            );
            assert_eq!(color, Color::from("r"));
        }
    }
}
