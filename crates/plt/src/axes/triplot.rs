//! Triangulation edges and points (matplotlib's `triplot`).

use super::{Axes, PlotError};
use crate::lines::{Line2D, LineProps, LineStyle};
use crate::markers::Marker;
use crate::tri::Triangulation;

/// The two lines of a [`Axes::triplot`]: the edges, then the points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriplotLines {
    /// Every edge of an unmasked triangle, drawn without markers.
    pub edges: Line2D,
    /// The points, drawn as markers without a line.
    pub markers: Line2D,
}

impl Axes {
    /// Draws the edges of the unmasked triangles of `tri` and markers at
    /// its points (matplotlib's `triplot`), styled by the format string
    /// `format` (e.g. `"go-"`) overridden by `props`. Edges sit at
    /// z-order 1 unless `props` says otherwise; each line draws its colour
    /// from the property cycle when none is given.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::lines::LineProps;
    /// use plt::tri::Triangulation;
    /// let ax = Figure::new().subplot();
    /// let tri = Triangulation::delaunay(&[0.0, 1.0, 0.0], &[0.0, 0.0, 1.0])?;
    /// let lines = ax.triplot(&tri, "o-", LineProps::default().color("k"))?;
    /// assert_eq!(lines.markers.xdata(), [0.0, 1.0, 0.0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `format` is not a valid format string.
    pub fn triplot(
        &self,
        tri: &Triangulation,
        format: &str,
        props: LineProps,
    ) -> Result<TriplotLines, PlotError> {
        let style = LineProps::fmt(format)?.overlay(props);
        let draws = |part: &Option<LineStyle>| part.as_ref().is_some_and(|s| *s != LineStyle::None);
        let mut edge_style = style.clone().marker(Marker::default());
        edge_style.base.zorder.get_or_insert(1.0);
        let (x, y) = if draws(&style.linestyle) {
            edge_polylines(tri)
        } else {
            (Vec::new(), Vec::new())
        };
        let edges = self.plot_with(x, y, edge_style)?;
        let has_marker = style.marker.as_ref().is_some_and(|m| !m.is_nothing());
        let mut marker_style = style.linestyle(LineStyle::None);
        marker_style.base.label = None;
        let (x, y) = if has_marker {
            (tri.x().to_vec(), tri.y().to_vec())
        } else {
            (Vec::new(), Vec::new())
        };
        let markers = self.plot_with(x, y, marker_style)?;
        Ok(TriplotLines { edges, markers })
    }
}

/// The edges as one polyline per edge, separated by NaN (matplotlib's
/// `np.insert(x[edges], 2, np.nan, axis=1).ravel()`).
fn edge_polylines(tri: &Triangulation) -> (Vec<f64>, Vec<f64>) {
    let (x, y) = (tri.x(), tri.y());
    tri.edges()
        .iter()
        .flat_map(|&[a, b]| [(x[a], y[a]), (x[b], y[b]), (f64::NAN, f64::NAN)])
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    fn triangle() -> Triangulation {
        Triangulation::delaunay(&[0.0, 1.0, 0.0], &[0.0, 0.0, 1.0]).unwrap()
    }

    #[test]
    fn edges_are_separated_by_nan() {
        let (x, y) = edge_polylines(&triangle());
        assert_eq!(x.len(), 9);
        assert!(x[2].is_nan() && y[5].is_nan());
    }

    #[test]
    fn lines_and_markers_are_drawn_apart() {
        let ax = Figure::new().subplot();
        let lines = ax
            .triplot(&triangle(), "o-", LineProps::default().label("mesh"))
            .unwrap();
        assert!(lines.edges.marker().is_nothing());
        assert_eq!(lines.edges.zorder(), 1.0);
        assert_eq!(lines.edges.xdata().len(), 9);
        assert_eq!(lines.markers.linestyle(), LineStyle::None);
        assert_eq!(lines.edges.label(), "mesh");
        assert_ne!(lines.markers.label(), "mesh");
        let bare = ax.triplot(&triangle(), "", LineProps::default()).unwrap();
        assert_eq!(bare.markers.xdata(), Vec::<f64>::new());
        assert!(ax.triplot(&triangle(), "q", LineProps::default()).is_err());
    }
}
