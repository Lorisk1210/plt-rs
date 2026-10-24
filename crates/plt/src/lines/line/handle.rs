//! The `Line2D` handle and its properties (`LineProps`).

use plt_core::{Affine2D, Path, Point, Rgba};
use plt_render::{CapStyle, JoinStyle};

use super::LineArtist;
use crate::artist::{ArtistHandle, LayoutContext, RawHandle, TypedHandle};
use crate::colors::Color;
use crate::lines::{DrawStyle, LineStyle, MarkEvery, MarkerIndicesError, StepError};
use crate::markers::{FillStyle, Marker};
use crate::rc::ColorOrAuto;
use crate::units::LineSources;

/// A handle to a line in a figure (matplotlib's `Line2D`).
///
/// ```
/// use plt::prelude::*;
/// let fig = Figure::new();
/// let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8])?;
/// let line = ax.plot([1.0, 2.0, 3.0], [4.0, 1.0, 2.0])?;
/// line.set_color("tab:orange").set_linestyle(LineStyle::Dashed).set_linewidth(2.5);
/// assert_eq!(line.linewidth(), 2.5);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line2D {
    raw: RawHandle,
}

impl ArtistHandle for Line2D {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for Line2D {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<LineArtist>().then(|| Self { raw: raw.clone() })
    }
}

impl Line2D {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// The line's data (matplotlib's `get_data`).
    pub fn data(&self) -> (Vec<f64>, Vec<f64>) {
        self.raw
            .with::<LineArtist, _>(|line| (line.xdata.clone(), line.ydata.clone()))
    }

    /// The line's data as points, as many as the shorter of x and y
    /// (matplotlib's `get_xydata`).
    pub fn xydata(&self) -> Vec<Point> {
        self.raw.with::<LineArtist, _>(|line| {
            (line.xdata.iter().zip(&line.ydata))
                .map(|(&x, &y)| Point::new(x, y))
                .collect()
        })
    }

    /// Replaces the line's data.
    pub fn set_data(&self, x: &[f64], y: &[f64]) -> &Self {
        self.raw.with_mut::<LineArtist, _>(|line| {
            line.xdata = x.to_vec();
            line.ydata = y.to_vec();
            line.unit_sources = LineSources::default();
            line.recache();
        });
        self
    }

    /// Keeps the user values the line's data came from, which it converts
    /// again when an axis changes unit.
    pub(crate) fn set_unit_sources(&self, sources: LineSources) -> &Self {
        self.raw
            .with_mut::<LineArtist, _>(|line| line.unit_sources = sources);
        self
    }

    /// The vertices actually drawn (with the draw style applied), in data
    /// coordinates (matplotlib's `get_path()`).
    pub fn path(&self) -> Result<Path, StepError> {
        self.raw.with::<LineArtist, _>(|line| line.path.clone())
    }

    /// The indices of the data points that get markers when the line is
    /// drawn with the figure's current layout, in drawing order
    /// (`markevery` applied; every point without it). Lay the figure out
    /// first ([`crate::figure::Figure::layout`]) so the view is final.
    pub fn marker_indices(&self) -> Result<Vec<usize>, MarkerIndicesError> {
        let state = self.raw.figure_state().borrow();
        self.raw
            .with::<LineArtist, _>(|line| line.marker_indices(LayoutContext::new(&state)))
    }

    /// The colour of the marker edges (matplotlib's `get_markeredgecolor`):
    /// when automatic, the line colour, or black for filled markers in
    /// classic mode.
    pub fn effective_markeredgecolor(&self) -> Color {
        self.raw.with::<LineArtist, _>(LineArtist::marker_edge)
    }

    /// The colour of the marker faces (matplotlib's `get_markerfacecolor`):
    /// none when the fill style is [`FillStyle::None`](crate::markers::FillStyle::None),
    /// else the face colour, the line colour when automatic.
    pub fn effective_markerfacecolor(&self) -> Color {
        self.raw
            .with::<LineArtist, _>(LineArtist::marker_face_color)
    }

    /// The resolved line colour.
    pub fn rgba(&self) -> Result<Rgba, crate::colors::ColorError> {
        self.raw
            .with::<LineArtist, _>(|line| line.color.resolve_alpha(line.base.alpha))
    }
}

artist_properties! {
    Line2D => LineArtist {
        /// The x data.
        xdata, set_xdata: Vec<f64> => xdata_replaced;
        /// The y data.
        ydata, set_ydata: Vec<f64> => ydata_replaced;
    }
}

artist_properties! {
    Line2D => LineArtist, LineProps {
        /// Line colour.
        color, set_color: Color;
        /// Line width in points.
        linewidth, set_linewidth: f64;
        /// Line style (solid, dashed, custom dashes, none).
        linestyle, set_linestyle: LineStyle;
        /// How points are connected (straight or steps).
        drawstyle, set_drawstyle: DrawStyle => recache;
        /// Colour drawn in the gaps of a dashed line (`"none"` leaves them empty).
        gapcolor, set_gapcolor: Color;
        /// Marker drawn at each data point.
        marker, set_marker: Marker;
        /// Which part of the marker is filled with the face colour.
        fillstyle, set_fillstyle: FillStyle;
        /// A transform applied to the marker before it is scaled to
        /// `markersize` (matplotlib's `MarkerStyle(transform=...)`).
        marker_transform, set_marker_transform: Option<Affine2D>;
        /// Cap style of the marker edges, replacing the marker's own
        /// (matplotlib's `MarkerStyle(capstyle=...)`).
        marker_capstyle, set_marker_capstyle: Option<CapStyle>;
        /// Join style of the marker edges, replacing the marker's own where
        /// the marker chooses one (matplotlib's `MarkerStyle(joinstyle=...)`).
        marker_joinstyle, set_marker_joinstyle: Option<JoinStyle>;
        /// Marker size in points.
        markersize, set_markersize: f64;
        /// Marker edge width in points.
        markeredgewidth, set_markeredgewidth: f64;
        /// Marker edge colour (`"auto"` follows the line colour).
        markeredgecolor, set_markeredgecolor: ColorOrAuto;
        /// Marker face colour (`"auto"` follows the line colour).
        markerfacecolor, set_markerfacecolor: ColorOrAuto;
        /// Face colour of the alternate half of half-filled markers.
        markerfacecoloralt, set_markerfacecoloralt: Color;
        /// Which points get markers (`None` = all).
        markevery, set_markevery: Option<MarkEvery>;
        /// How many straight pieces each segment is split into before a
        /// non-affine transform such as the polar one bends it
        /// (matplotlib's `get_path()._interpolation_steps`; 1 draws the
        /// segments straight).
        interpolation_steps, set_interpolation_steps: usize => recache;
        /// Anti-aliasing.
        antialiased, set_antialiased: bool;
        /// How far from the line, in points, a pick still hits it (see
        /// [`crate::artist::Picker`]).
        pickradius, set_pickradius: f64;
        /// Cap style of solid lines.
        solid_capstyle, set_solid_capstyle: CapStyle;
        /// Join style of solid lines.
        solid_joinstyle, set_solid_joinstyle: JoinStyle;
        /// Cap style of dashed lines.
        dash_capstyle, set_dash_capstyle: CapStyle;
        /// Join style of dashed lines.
        dash_joinstyle, set_dash_joinstyle: JoinStyle;
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::lines::{AxLineThrough, LineProps};
    use plt_core::{Bbox, Point};

    #[test]
    fn window_extents_cover_the_finite_points_and_markers() {
        let fig = Figure::with_size(2.0, 2.0, 72.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 1.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .expect("finite limits");
        let props = LineProps::fmt("o-").unwrap().markersize(10.0);
        let line = ax.plot_with([0.25, f64::NAN, 0.75], [0.5, 0.2, 1.0], props);
        let extent = line.unwrap().window_extent().unwrap();
        assert_eq!(extent, Bbox::from_extents(31.0, 67.0, 113.0, 149.0));
        let empty = ax.plot([f64::NAN], [1.0]).unwrap();
        assert_eq!(empty.window_extent(), None);
        let flat = ax
            .axline(
                Point::new(0.0, 0.5),
                AxLineThrough::Slope(0.0),
                LineProps::default(),
            )
            .unwrap();
        assert_eq!(
            flat.window_extent(),
            Some(Bbox::from_extents(0.0, 72.0, 144.0, 72.0))
        );
    }

    #[test]
    fn interpolation_steps_reach_the_drawn_path() {
        let line = Figure::new()
            .subplot()
            .plot([0.0, 1.0], [1.0, 2.0])
            .unwrap();
        assert_eq!(line.path().unwrap().interpolation_steps(), 1);
        line.set_interpolation_steps(100_usize);
        assert_eq!(line.interpolation_steps(), 100);
        assert_eq!(line.path().unwrap().interpolation_steps(), 100);
    }

    #[test]
    fn slopes_are_in_data_units_whatever_the_point_coordinates() {
        // matplotlib's test_axline_transaxes_panzoom.
        let fig = Figure::with_size(2.0, 2.0, 72.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        let line = ax
            .axline(
                Point::new(0.5, 0.5),
                AxLineThrough::Slope(2.0),
                LineProps::default(),
            )
            .unwrap();
        line.set_transform(ax.trans_axes());
        ax.set_xlim(0.0, 5.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        // Through the data point (2.5, 5) with slope 2: y = 2x, from the
        // bottom-left corner to the top-right one.
        assert_eq!(
            line.window_extent(),
            Some(Bbox::from_extents(0.0, 0.0, 144.0, 144.0))
        );
    }
}
