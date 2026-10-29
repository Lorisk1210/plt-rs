//! Patches drawn from boxes known only at draw time (matplotlib's
//! `BboxPatch`, `BboxConnector` and `BboxConnectorPatch` of
//! `axes_grid1.inset_locator`): the outline of a box, a line between
//! corners of two boxes, and the quadrilateral two such lines span.
//!
//! The boxes are [`BboxSource`]s, resolved in display pixels at every
//! draw, so the patches follow axes that move or change their limits.
//! The patches are drawn in display pixels and never count towards data
//! limits.

use pltrs_core::{Bbox, Path, Point};

use crate::artist::{ArtistBase, ArtistHandle, ArtistId, LayoutContext};
use crate::axes::{Axes, AxesState};
use crate::offsetbox::transformed_bbox;
use crate::patches::{DisplayPath, PatchError, PatchShape, PatchStyle, ShapeFrame};
use crate::transforms::Transform;

/// A corner of a box (matplotlib's codes 1 to 4 of `loc1` and `loc2`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Corner {
    /// `1`.
    UpperRight,
    /// `2`.
    UpperLeft,
    /// `3`.
    LowerLeft,
    /// `4`.
    LowerRight,
}

impl Corner {
    /// The corner with matplotlib's code `1` to `4`.
    pub fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            1 => Self::UpperRight,
            2 => Self::UpperLeft,
            3 => Self::LowerLeft,
            4 => Self::LowerRight,
            _ => return None,
        })
    }

    /// The corner of `bbox`, read from its stored points like
    /// matplotlib's `get_bbox_edge_pos` (a flipped box swaps corners).
    pub fn of(self, bbox: &Bbox) -> Point {
        let (p0, p1) = (bbox.p0, bbox.p1);
        match self {
            Self::UpperRight => p1,
            Self::UpperLeft => Point::new(p0.x, p1.y),
            Self::LowerLeft => p0,
            Self::LowerRight => Point::new(p1.x, p0.y),
        }
    }
}

/// A box in display pixels, worked out at draw time (the `Bbox` and
/// `TransformedBbox` arguments of matplotlib's bbox patches).
#[derive(Clone, Debug, PartialEq)]
pub enum BboxSource {
    /// The box of an axes (`ax.bbox`).
    Axes(ArtistId),
    /// A fixed box in the coordinates of a transform
    /// (`TransformedBbox(bbox, transform)`), keeping its orientation.
    Transformed(Bbox, Transform),
    /// The view limits of an axes in the coordinates of a transform, such
    /// as another axes' data (`TransformedBbox(ax.viewLim, transform)`).
    ViewLimits(ArtistId, Transform),
}

impl BboxSource {
    /// The box of `axes` (`ax.bbox`).
    pub fn axes(axes: &Axes) -> Self {
        Self::Axes(axes.id())
    }

    /// `bbox` in the coordinates of `transform`.
    pub fn transformed(bbox: Bbox, transform: Transform) -> Self {
        Self::Transformed(bbox, transform)
    }

    /// The view limits of `axes` in the coordinates of `transform`.
    pub fn view_limits(axes: &Axes, transform: Transform) -> Self {
        Self::ViewLimits(axes.id(), transform)
    }

    /// The box in display pixels, `None` when an axes it refers to is
    /// gone.
    pub fn resolve(&self, layout: LayoutContext<'_>) -> Option<Bbox> {
        match self {
            Self::Axes(axes) => layout.axes_extent(*axes),
            Self::Transformed(bbox, transform) => {
                Some(transformed_bbox(bbox, &layout.resolve(transform)?))
            }
            Self::ViewLimits(axes, transform) => {
                let view = layout.store().get_as::<AxesState>(*axes)?.view_lim;
                Some(transformed_bbox(&view, &layout.resolve(transform)?))
            }
        }
    }
}

/// The line from corner `loc1` of `bbox1` to corner `loc2` of `bbox2`
/// (matplotlib's `BboxConnector.connect_bbox`).
///
/// ```
/// use pltrs::axes_grid1::{Corner, connect_bbox};
/// use pltrs::transforms::{Bbox, Point};
/// let path = connect_bbox(
///     &Bbox::from_extents(0.0, 0.0, 1.0, 1.0),
///     &Bbox::from_extents(2.0, 2.0, 4.0, 3.0),
///     Corner::UpperRight,
///     Corner::LowerLeft,
/// );
/// assert_eq!(path.vertices(), [Point::new(1.0, 1.0), Point::new(2.0, 2.0)]);
/// ```
pub fn connect_bbox(bbox1: &Bbox, bbox2: &Bbox, loc1: Corner, loc2: Corner) -> Path {
    Path::polyline(vec![loc1.of(bbox1), loc2.of(bbox2)])
}

/// Patch defaults shared by the bbox patches: display coordinates (which
/// matplotlib forces with `IdentityTransform`).
fn display_coordinates(base: &mut ArtistBase) {
    base.transform = Some(Transform::Display);
}

/// The outline of a box (matplotlib's `BboxPatch`), filled by default
/// like any patch.
///
/// ```
/// use pltrs::axes_grid1::{BboxPatch, BboxSource};
/// use pltrs::prelude::*;
/// use pltrs::transforms::Bbox;
/// let ax = Figure::new().subplot();
/// let span = Bbox::from_extents(0.2, 0.0, 0.4, 1.0);
/// let patch = ax.add_patch(BboxPatch::new(BboxSource::transformed(span, ax.trans_xaxis())));
/// patch.update_props(PatchProps::default().facecolor("tab:blue").alpha(0.2));
/// assert!(ax.data_lim().bbox.is_null(), "no data limits");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct BboxPatch {
    /// The box outlined.
    pub bbox: BboxSource,
}

impl BboxPatch {
    /// The outline of `bbox`.
    pub fn new(bbox: BboxSource) -> Self {
        Self { bbox }
    }
}

impl PatchShape for BboxPatch {
    fn path(&self) -> Path {
        Path::polyline(Vec::new())
    }

    fn skips_autoscale(&self) -> bool {
        true
    }

    fn init_style(&self, _style: &mut PatchStyle, base: &mut ArtistBase) {
        display_coordinates(base);
    }

    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let Some(bbox) = self.bbox.resolve(frame.layout) else {
            return Ok(Vec::new());
        };
        let corners = [
            Corner::LowerLeft,
            Corner::LowerRight,
            Corner::UpperRight,
            Corner::UpperLeft,
        ]
        .map(|corner| corner.of(&bbox));
        Ok(vec![DisplayPath::display(Path::polygon(corners), true)])
    }
}

/// A straight line from a corner of one box to a corner of another
/// (matplotlib's `BboxConnector`). Unlike other patches it is not filled
/// by default: matplotlib fills it only when given a face colour, so set
/// `fill` together with the face colour.
#[derive(Clone, Debug, PartialEq)]
pub struct BboxConnector {
    /// The box the line starts from.
    pub bbox1: BboxSource,
    /// The box the line ends at.
    pub bbox2: BboxSource,
    /// The corner of `bbox1`.
    pub loc1: Corner,
    /// The corner of `bbox2`.
    pub loc2: Corner,
}

impl BboxConnector {
    /// The line from corner `loc1` of `bbox1` to corner `loc2` of `bbox2`
    /// (`None` = the same corner as `loc1`).
    pub fn new(bbox1: BboxSource, bbox2: BboxSource, loc1: Corner, loc2: Option<Corner>) -> Self {
        Self {
            bbox1,
            bbox2,
            loc1,
            loc2: loc2.unwrap_or(loc1),
        }
    }
}

impl PatchShape for BboxConnector {
    fn path(&self) -> Path {
        Path::polyline(Vec::new())
    }

    fn skips_autoscale(&self) -> bool {
        true
    }

    fn init_style(&self, style: &mut PatchStyle, base: &mut ArtistBase) {
        style.fill = false;
        display_coordinates(base);
    }

    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let line = self
            .bbox1
            .resolve(frame.layout)
            .zip(self.bbox2.resolve(frame.layout))
            .map(|(bbox1, bbox2)| connect_bbox(&bbox1, &bbox2, self.loc1, self.loc2));
        Ok(line
            .map(|line| DisplayPath::display(line, true))
            .into_iter()
            .collect())
    }
}

/// The quadrilateral between two boxes spanned by two connecting lines
/// and the box edges between their ends (matplotlib's
/// `BboxConnectorPatch`). Like [`BboxConnector`] it is not filled by
/// default.
///
/// ```
/// use pltrs::axes_grid1::{BboxConnectorPatch, BboxSource, Corner};
/// use pltrs::prelude::*;
/// let fig = Figure::new();
/// let (top, bottom) = (fig.add_subplot(2, 1, 1)?, fig.add_subplot(2, 1, 2)?);
/// let zoom = BboxConnectorPatch {
///     bbox1: BboxSource::axes(&top),
///     bbox2: BboxSource::axes(&bottom),
///     loc1a: Corner::LowerLeft,
///     loc2a: Corner::UpperLeft,
///     loc1b: Corner::LowerRight,
///     loc2b: Corner::UpperRight,
/// };
/// let patch = bottom.add_patch(zoom);
/// patch.update_props(PatchProps::default().facecolor("0.9").fill(true));
/// patch.set_clip_on(false);
/// # Ok::<(), pltrs::gridspec::GridSpecError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct BboxConnectorPatch {
    /// The first box.
    pub bbox1: BboxSource,
    /// The second box.
    pub bbox2: BboxSource,
    /// The corner of `bbox1` the first line starts from.
    pub loc1a: Corner,
    /// The corner of `bbox2` the first line ends at.
    pub loc2a: Corner,
    /// The corner of `bbox1` the second line ends at.
    pub loc1b: Corner,
    /// The corner of `bbox2` the second line starts from.
    pub loc2b: Corner,
}

impl PatchShape for BboxConnectorPatch {
    fn path(&self) -> Path {
        Path::polyline(Vec::new())
    }

    fn skips_autoscale(&self) -> bool {
        true
    }

    fn init_style(&self, style: &mut PatchStyle, base: &mut ArtistBase) {
        style.fill = false;
        display_coordinates(base);
    }

    /// matplotlib's `get_path`: the first line, the second line run
    /// backwards, and the first point again, as an open polyline.
    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let (Some(bbox1), Some(bbox2)) = (
            self.bbox1.resolve(frame.layout),
            self.bbox2.resolve(frame.layout),
        ) else {
            return Ok(Vec::new());
        };
        let start = self.loc1a.of(&bbox1);
        let vertices = vec![
            start,
            self.loc2a.of(&bbox2),
            self.loc2b.of(&bbox2),
            self.loc1b.of(&bbox1),
            start,
        ];
        Ok(vec![DisplayPath::display(Path::polyline(vertices), true)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::patches::PatchProps;
    use crate::testing::{Call, RecordingRenderer};

    #[test]
    fn corners_follow_matplotlib_codes() {
        let bbox = Bbox::from_bounds(1.0, 2.0, 3.0, 4.0);
        assert_eq!(
            Corner::from_code(1).map(|c| c.of(&bbox)),
            Some(Point::new(4.0, 6.0))
        );
        assert_eq!(
            Corner::from_code(3).map(|c| c.of(&bbox)),
            Some(Point::new(1.0, 2.0))
        );
        assert_eq!(Corner::from_code(5), None);
        let flipped = Bbox::from_extents(4.0, 6.0, 1.0, 2.0);
        assert_eq!(Corner::UpperRight.of(&flipped), Point::new(1.0, 2.0));
    }

    #[test]
    fn sources_resolve_in_display_pixels() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig
            .add_axes([0.0, 0.0, 0.5, 0.5])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let other = fig
            .add_axes([0.5, 0.5, 0.5, 0.5])
            .expect("a valid rectangle");
        other
            .set_xlim(2.0, 4.0)
            .and_then(|ax| ax.set_ylim(0.0, 5.0))
            .expect("finite limits");
        fig.layout();
        let state = fig.state();
        let layout = LayoutContext::new(&state);
        let resolve = |source: BboxSource| source.resolve(layout).expect("resolvable");
        assert_eq!(
            resolve(BboxSource::axes(&ax)),
            Bbox::from_extents(0.0, 0.0, 200.0, 200.0)
        );
        let span = Bbox::from_extents(5.0, 0.0, 10.0, 1.0);
        assert_eq!(
            resolve(BboxSource::transformed(span, ax.trans_xaxis())),
            Bbox::from_extents(100.0, 0.0, 200.0, 200.0)
        );
        assert_eq!(
            resolve(BboxSource::view_limits(&other, ax.trans_data())),
            Bbox::from_extents(40.0, 0.0, 80.0, 100.0)
        );
    }

    #[test]
    fn connector_patch_runs_back_along_the_second_line() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let top = fig
            .add_axes([0.25, 0.5, 0.5, 0.25])
            .expect("a valid rectangle");
        let bottom = fig
            .add_axes([0.0, 0.0, 1.0, 0.25])
            .expect("a valid rectangle");
        let patch = bottom.add_patch(BboxConnectorPatch {
            bbox1: BboxSource::axes(&top),
            bbox2: BboxSource::axes(&bottom),
            loc1a: Corner::LowerLeft,
            loc2a: Corner::UpperLeft,
            loc1b: Corner::LowerRight,
            loc2b: Corner::UpperRight,
        });
        patch.update_props(PatchProps::default().facecolor("k").fill(true));
        patch.set_clip_on(false);
        let line = bottom.add_patch(BboxConnector::new(
            BboxSource::axes(&top),
            BboxSource::axes(&bottom),
            Corner::UpperRight,
            None,
        ));
        line.set_clip_on(false);
        let mut renderer = RecordingRenderer::new(400.0, 400.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        let paths: Vec<(Vec<Point>, bool)> = renderer
            .calls
            .into_iter()
            .filter_map(|call| match call {
                Call::Path { points, fill, .. } => Some((points, fill.is_some())),
                _ => None,
            })
            .collect();
        let quad = [
            Point::new(100.0, 200.0),
            Point::new(0.0, 100.0),
            Point::new(400.0, 100.0),
            Point::new(300.0, 200.0),
            Point::new(100.0, 200.0),
        ];
        assert!(paths.iter().any(|(p, filled)| p == &quad && *filled));
        let connector = [Point::new(300.0, 300.0), Point::new(400.0, 100.0)];
        assert!(paths.iter().any(|(p, filled)| p == &connector && !*filled));
        assert!(bottom.data_lim().bbox.is_null());
    }
}
