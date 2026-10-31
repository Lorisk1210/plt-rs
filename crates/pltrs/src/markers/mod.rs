//! Marker shapes for lines and scatter plots (matplotlib.markers).
//!
//! A [`Marker`] is what users ask for (`"o"`, a polygon tuple, a custom
//! path, mathtext such as `"$\alpha$"`); [`MarkerStyle`] resolves it,
//! together with a [`FillStyle`], into the exact paths and transforms
//! matplotlib draws.

mod mathtext;
mod shapes;
mod spec;
mod style;

pub use spec::{FillStyle, Marker, MarkerError, MarkerKey, MarkerSymbol, NoMarker, PolygonStyle};
pub use style::MarkerStyle;

use pltrs_render::Sketch;

/// The sketch markers are drawn with: half the wobble and length and twice
/// the randomness of their line's (matplotlib's `Line2D.draw`).
pub(crate) fn marker_sketch(sketch: Option<Sketch>) -> Option<Sketch> {
    sketch.map(|line| Sketch {
        scale: line.scale / 2.0,
        length: line.length / 2.0,
        randomness: 2.0 * line.randomness,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_wobble_half_as_much_twice_as_irregularly() {
        let line = Sketch {
            scale: 1.0,
            length: 100.0,
            randomness: 2.0,
        };
        let marker = marker_sketch(Some(line)).expect("sketched");
        assert_eq!(
            (marker.scale, marker.length, marker.randomness),
            (0.5, 50.0, 4.0)
        );
        assert_eq!(marker_sketch(None), None);
    }
}
