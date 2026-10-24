//! Inset indicators (matplotlib's `inset` module): a rectangle marking an
//! area of an axes, optionally joined by connector lines to the inset axes
//! that shows the area enlarged ([`Axes::indicate_inset`] and
//! [`Axes::indicate_inset_zoom`]).

use plt_core::{Bbox, Path, Point};

use crate::artist::{ArtistBase, ArtistHandle, ArtistId, LayoutContext};
use crate::axes::{Axes, AxesState};
use crate::colors::Color;
use crate::coords::{CoordOrigin, CoordUnit, Coords};
use crate::patches::{
    ConnectionEnd, ConnectionPatch, DisplayPath, Patch, PatchError, PatchProps, PatchShape,
    PatchStyle, ShapeFrame,
};

/// The z-order of indicators, just below the default of inset axes.
const INDICATOR_ZORDER: f64 = 4.99;

/// The corners of the inset axes the connectors start from, in its axes
/// fractions: lower left, upper left, lower right, upper right.
const INSET_CORNERS: [(f64, f64); 4] = [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)];

/// The geometry of an inset indicator (matplotlib's `InsetIndicator`): a
/// rectangle in the data coordinates of its axes and, when it indicates an
/// inset axes, the two connectors from the rectangle to the corners of the
/// inset that do not cross it.
///
/// The rectangle and the visible connectors are drawn as one outline in
/// the patch's style, as matplotlib does while the connectors share the
/// rectangle's style.
#[derive(Clone, Debug, PartialEq)]
pub struct InsetIndicator {
    /// `[x0, y0, width, height]` in data coordinates; `None` follows the
    /// view limits of the inset axes.
    bounds: Option<[f64; 4]>,
    /// The inset axes the connectors lead to.
    inset: Option<ArtistId>,
}

impl InsetIndicator {
    /// The rectangle `[x0, y0, width, height]` in data coordinates
    /// (matplotlib's `_bounds_from_inset_ax` when following the inset).
    fn bounds(&self, layout: LayoutContext<'_>) -> Option<[f64; 4]> {
        self.bounds.or_else(|| {
            let view = layout.store().get_as::<AxesState>(self.inset?)?.view_lim;
            Some([view.x0(), view.y0(), view.width(), view.height()])
        })
    }

    /// The visible connector lines in display pixels (matplotlib's
    /// `_update_connectors`); `rect` is the rectangle as matplotlib
    /// measures it to choose them, in display pixels (see
    /// [`measured_rectangle`]).
    fn connectors(
        &self,
        frame: &ShapeFrame<'_>,
        [x, y, width, height]: [f64; 4],
        rect: &Bbox,
    ) -> Result<Vec<Path>, PatchError> {
        let layout = frame.layout;
        let Some((inset, inset_box)) = self
            .inset
            .and_then(|id| Some((id, layout.axes_extent(id)?)))
        else {
            return Ok(Vec::new());
        };
        let (x_inverted, y_inverted) = frame
            .parent
            .and_then(|axes| layout.store().get_as::<AxesState>(axes))
            .map_or((false, false), |axes| {
                let view = axes.view_lim;
                (view.x1() < view.x0(), view.y1() < view.y0())
            });
        let mut lines = Vec::new();
        let corners = INSET_CORNERS
            .into_iter()
            .zip(visible_connectors(rect, &inset_box));
        for (corner, _) in corners.filter(|&(_, visible)| visible) {
            let ex = if x_inverted { 1.0 - corner.0 } else { corner.0 };
            let ey = if y_inverted { 1.0 - corner.1 } else { corner.1 };
            let inset_fraction = Coords::Relative {
                origin: CoordOrigin::Axes,
                unit: CoordUnit::Fraction,
            };
            let connector = ConnectionPatch {
                annotation_clip: Some(false),
                ..ConnectionPatch::new(
                    ConnectionEnd::new(corner, inset_fraction, Some(inset)),
                    ConnectionEnd::new(
                        (x + ex * width, y + ey * height),
                        Coords::Data,
                        frame.parent,
                    ),
                )
            };
            let pieces = connector.display_paths(frame)?;
            lines.extend(pieces.iter().map(DisplayPath::to_display));
        }
        Ok(lines)
    }
}

/// The rectangle `[x, y, width, height]` as matplotlib measures it to
/// choose the connectors: `Bbox.from_bounds(x, y, width, height)` mapped by
/// the rectangle's `get_transform()`, whose patch transform maps the unit
/// square onto the bounds once more, so the corners compared are
/// `(x + x·width, y + y·height)` and `(x + (x + width)·width, y + (y +
/// height)·height)` in data coordinates. Not normalised, like matplotlib's
/// `Bbox.transformed`: on inverted axes the first corner may lie right of
/// or above the second.
fn measured_rectangle(frame: &ShapeFrame<'_>, [x, y, width, height]: [f64; 4]) -> Bbox {
    let corner = |(cx, cy): (f64, f64)| {
        frame
            .transform
            .transform_point(Point::new(x + cx * width, y + cy * height))
    };
    Bbox::new(corner((x, y)), corner((x + width, y + height)))
}

/// Which of the connectors to the inset's lower-left, upper-left,
/// lower-right and upper-right corners are drawn, so that they do not
/// cross the indicated rectangle (both boxes in display pixels).
fn visible_connectors(rect: &Bbox, inset: &Bbox) -> [bool; 4] {
    let x0 = rect.x0() < inset.x0();
    let x1 = rect.x1() < inset.x1();
    let y0 = rect.y0() < inset.y0();
    let y1 = rect.y1() < inset.y1();
    [x0 ^ y0, x0 == y1, x1 == y0, x1 ^ y1]
}

impl PatchShape for InsetIndicator {
    fn path(&self) -> Path {
        Path::polyline(Vec::new())
    }

    fn skips_autoscale(&self) -> bool {
        true
    }

    fn init_style(&self, style: &mut PatchStyle, base: &mut ArtistBase) {
        style.facecolor = Some(Color::none());
        style.edgecolor = Some(Color::from("0.5"));
        base.alpha = Some(0.5);
        base.zorder = INDICATOR_ZORDER;
        base.clip_on = false;
        base.label = "_indicate_inset".into();
    }

    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let Some(bounds) = self.bounds(frame.layout) else {
            return Ok(Vec::new());
        };
        let [x, y, width, height] = bounds;
        let corners = [
            (x, y),
            (x + width, y),
            (x + width, y + height),
            (x, y + height),
            (x, y),
        ];
        let vertices: Vec<Point> = corners
            .into_iter()
            .map(|(cx, cy)| frame.transform.transform_point(Point::new(cx, cy)))
            .collect();
        let mut outlines = vec![Path::closed(vertices)];
        outlines.extend(self.connectors(frame, bounds, &measured_rectangle(frame, bounds))?);
        Ok(vec![DisplayPath::display(Path::compound(&outlines), true)])
    }
}

impl Axes {
    /// Marks the area `[x0, y0, width, height]` (data coordinates) with a
    /// rectangle, joined to the corners of `inset` if given (matplotlib's
    /// `indicate_inset`). The rectangle is unfilled, grey and half
    /// transparent, drawn just below inset axes and not clipped, unless
    /// `props` say otherwise.
    ///
    /// ```
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let marker = ax.indicate_inset([1.0, 1.0, 2.0, 2.0], None, PatchProps::default());
    /// assert_eq!((marker.zorder(), marker.alpha()), (4.99, Some(0.5)));
    /// ```
    pub fn indicate_inset(
        &self,
        bounds: [f64; 4],
        inset: Option<&Axes>,
        props: PatchProps,
    ) -> Patch<InsetIndicator> {
        let indicator = InsetIndicator {
            bounds: Some(bounds),
            inset: inset.map(ArtistHandle::id),
        };
        self.add_indicator(indicator, props)
    }

    /// Marks the area shown by `inset` (its view limits, followed when
    /// they change) and joins it to the inset's corners (matplotlib's
    /// `indicate_inset_zoom`); styled as [`Axes::indicate_inset`].
    pub fn indicate_inset_zoom(&self, inset: &Axes, props: PatchProps) -> Patch<InsetIndicator> {
        let indicator = InsetIndicator {
            bounds: None,
            inset: Some(inset.id()),
        };
        self.add_indicator(indicator, props)
    }

    /// Adds `indicator` styled by `props` over its defaults.
    fn add_indicator(&self, indicator: InsetIndicator, props: PatchProps) -> Patch<InsetIndicator> {
        let patch = self.add_artist(indicator);
        patch.update_props(props);
        patch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn zoom_indicators_follow_the_inset_limits() {
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.1, 0.1, 0.8, 0.8])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let inset = ax.inset_axes([0.6, 0.6, 0.3, 0.3]);
        inset
            .set_xlim(1.0, 2.0)
            .and_then(|ax| ax.set_ylim(1.0, 2.0))
            .expect("finite limits");
        let marker = ax.indicate_inset_zoom(&inset, PatchProps::default());
        let extent = marker.window_extent().expect("drawn");
        // The rectangle starts at data (1, 1) = pixels (115.2, 86.4); the
        // connectors reach the inset's right edge at x = 0.82 * 640.
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(
            close(extent.x0(), 115.2) && close(extent.y0(), 86.4),
            "{extent:?}"
        );
        assert!(close(extent.x1(), 524.8), "{extent:?}");
        inset.set_xlim(0.0, 2.0).expect("finite limits");
        let moved = marker.window_extent().expect("drawn");
        assert!(close(moved.x0(), 64.0), "{moved:?}");
    }

    #[test]
    fn connectors_avoid_the_rectangle() {
        let inset = Bbox::from_extents(60.0, 60.0, 90.0, 90.0);
        // Rectangle below left of the inset: upper-left and lower-right.
        let below_left = Bbox::from_extents(10.0, 10.0, 30.0, 30.0);
        assert_eq!(
            visible_connectors(&below_left, &inset),
            [false, true, true, false]
        );
        // Rectangle above left: lower-left and upper-right.
        let above_left = Bbox::from_extents(10.0, 95.0, 30.0, 99.0);
        assert_eq!(
            visible_connectors(&above_left, &inset),
            [true, false, false, true]
        );
    }

    #[test]
    fn connectors_are_chosen_from_the_rectangle_mapped_twice() {
        // matplotlib 3.11 (test_streamplot.py's test_integration_options):
        // the rectangle (-1.2, -0.8, 0.5, 0.4) reaches right of the inset's
        // right edge, but measured as (-1.8, -1.12)..(-1.55, -0.96) it does
        // not, so the lower-right connector is drawn, not the upper-right.
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(-2.0, 2.0)
            .and_then(|ax| ax.set_ylim(-1.5, 1.5))
            .expect("finite limits");
        let inset = ax.inset_axes([0.0, 0.7, 0.18, 0.3]);
        inset
            .set_xlim(-1.2, -0.7)
            .and_then(|ax| ax.set_ylim(-0.8, -0.4))
            .expect("finite limits");
        let marker = ax.indicate_inset_zoom(&inset, PatchProps::default());
        let extent = marker.window_extent().expect("drawn");
        // The lower-right connector ends at the rectangle's bottom; the
        // upper-right one would reach the inset's top at y = 480.
        assert!(extent.y1() < 480.0 - 1.0, "{extent:?}");
    }

    #[test]
    fn indicators_without_an_inset_are_a_plain_rectangle() {
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 1.0)
            .and_then(|ax| ax.set_ylim(1.0, 0.0))
            .expect("finite limits");
        let marker = ax.indicate_inset(
            [0.25, 0.25, 0.5, 0.5],
            None,
            PatchProps::default().alpha(1.0),
        );
        let extent = marker.window_extent().expect("drawn");
        assert_eq!(
            (extent.x0(), extent.x1(), extent.y0(), extent.y1()),
            (160.0, 480.0, 120.0, 360.0)
        );
        assert_eq!(marker.alpha(), Some(1.0));
    }
}
