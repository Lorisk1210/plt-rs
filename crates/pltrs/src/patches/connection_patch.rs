//! `ConnectionPatch`: an arrow between two points given in different
//! coordinate systems, possibly in different axes.

use pltrs_core::{Path, Point};

use super::fancy_arrow::{ArrowPatches, ArrowSpec, round_ends};
use super::{ArrowStyle, CurveEnd, DisplayPath, PatchError, PatchShape, PatchStyle, ShapeFrame};
use crate::artist::{ArtistBase, ArtistId, LayoutContext};
use crate::coords::{CoordFrame, CoordOrigin, CoordUnit, Coords, axes_contains_point};

/// One end of a [`ConnectionPatch`].
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionEnd {
    /// The point.
    pub xy: Point,
    /// What its coordinates mean.
    pub coords: Coords,
    /// The axes of data and axes coordinates; `None` uses the axes the
    /// patch is added to.
    pub axes: Option<ArtistId>,
}

pltrs_core::archive_struct!(ConnectionEnd { xy, coords, axes });

impl ConnectionEnd {
    /// `xy` in `coords`, relative to `axes`.
    pub fn new(xy: impl Into<Point>, coords: Coords, axes: Option<ArtistId>) -> Self {
        Self {
            xy: xy.into(),
            coords,
            axes,
        }
    }
}

/// A line or arrow from one point to another, each in its own coordinate
/// system (matplotlib's `ConnectionPatch`). Unlike other patches it is not
/// clipped by default, so it can join two axes; add it to the figure to
/// draw it above both.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionPatch {
    /// The start.
    pub a: ConnectionEnd,
    /// The end.
    pub b: ConnectionEnd,
    /// How the arrow is built (matplotlib's defaults here: a plain `-`
    /// line, no shrinking, mutation scale 10).
    pub spec: ArrowSpec,
    /// Whether to skip drawing when an end lies outside its axes: always
    /// (`Some(true)`), never (`Some(false)`), or for ends in data
    /// coordinates (`None`).
    pub annotation_clip: Option<bool>,
    /// The patches whose outlines, as drawn, clip the start and the end
    /// (matplotlib's `patchA` and `patchB`).
    pub patches: ArrowPatches,
}

pltrs_core::archive_struct!(ConnectionPatch {
    a,
    b,
    spec,
    annotation_clip,
    patches
});

impl ConnectionPatch {
    /// A plain line from `a` to `b`.
    pub fn new(a: ConnectionEnd, b: ConnectionEnd) -> Self {
        Self {
            a,
            b,
            spec: ArrowSpec {
                arrowstyle: ArrowStyle::curve(CurveEnd::Plain, CurveEnd::Plain),
                shrink_a: 0.0,
                shrink_b: 0.0,
                mutation_scale: 10.0,
                ..ArrowSpec::default()
            },
            annotation_clip: None,
            patches: (None, None),
        }
    }

    /// The same patch clipped to the outlines of the patches (or the
    /// boxes of the texts) `a` at its start and `b` at its end
    /// (matplotlib's `patchA` and `patchB`).
    #[must_use]
    pub fn with_patches(mut self, a: Option<ArtistId>, b: Option<ArtistId>) -> Self {
        self.patches = (a, b);
        self
    }

    /// The display position of `end` (`ConnectionPatch._get_xy`: unlike
    /// annotations, negative pixel and point values count from the right
    /// or top edge).
    fn position(
        end: &ConnectionEnd,
        layout: LayoutContext<'_>,
        axes: Option<ArtistId>,
        owner: Option<ArtistId>,
    ) -> Result<Point, PatchError> {
        let frame = CoordFrame {
            layout,
            axes,
            owner,
            offset_origin: None,
            fontsize: 0.0,
        };
        let from_edges = |unit: CoordUnit, origin: CoordOrigin| -> Result<Point, PatchError> {
            let scale = match unit {
                CoordUnit::Points => layout.points_to_pixels(1.0),
                _ => 1.0,
            };
            let unit_box = Coords::Relative {
                origin,
                unit: CoordUnit::Fraction,
            }
            .resolve(&frame)
            .map_err(PatchError::Coords)?;
            let p0 = unit_box.transform_point(Point::ORIGIN);
            let p1 = unit_box.transform_point(Point::new(1.0, 1.0));
            let wrap = |v: f64, low: f64, high: f64| if v >= 0.0 { low + v } else { high + v };
            Ok(Point::new(
                wrap(end.xy.x * scale, p0.x, p1.x),
                wrap(end.xy.y * scale, p0.y, p1.y),
            ))
        };
        match &end.coords {
            Coords::Relative {
                origin: origin @ (CoordOrigin::Figure | CoordOrigin::Axes),
                unit: unit @ (CoordUnit::Pixels | CoordUnit::Points),
            }
            | Coords::Relative {
                origin: origin @ CoordOrigin::Subfigure,
                unit: unit @ CoordUnit::Pixels,
            } => from_edges(*unit, *origin),
            Coords::Data
            | Coords::Polar
            | Coords::Transform(_)
            | Coords::Relative {
                unit: CoordUnit::Fraction,
                ..
            } => Ok(end
                .coords
                .resolve(&frame)
                .map_err(PatchError::Coords)?
                .transform_point(end.xy)),
            _ => Err(PatchError::UnsupportedCoords),
        }
    }

    /// The axes the patch was added to, which places ends given no axes
    /// of their own; `None` for a patch of a figure.
    fn parent_axes(layout: LayoutContext<'_>, parent: Option<ArtistId>) -> Option<ArtistId> {
        parent.filter(|&parent| layout.axes_extent(parent).is_some())
    }

    /// Whether `point` (an end in `coords`) passes the annotation clip
    /// test (`ConnectionPatch._check_xy`).
    fn visible(
        &self,
        end: &ConnectionEnd,
        point: Point,
        layout: LayoutContext<'_>,
        axes: Option<ArtistId>,
    ) -> bool {
        let clip = self.annotation_clip.unwrap_or(end.coords == Coords::Data);
        !clip || axes.is_some_and(|axes| axes_contains_point(layout, axes, point))
    }
}

impl PatchShape for ConnectionPatch {
    const REBUILDS_PATH: bool = true;

    const MAPS_OUTLINE_ALONE: bool = false;

    const EDGE_DEFAULT: bool = true;

    fn path(&self) -> Path {
        Path::polyline(Vec::new())
    }

    fn skips_autoscale(&self) -> bool {
        true
    }

    fn init_style(&self, style: &mut PatchStyle, base: &mut ArtistBase) {
        round_ends(style);
        base.clip_on = false;
    }

    /// Skipped while an end fails the annotation clip test (matplotlib's
    /// `ConnectionPatch.draw` checks `_check_xy` first); its outline and
    /// extent are measured all the same.
    fn skips_draw(&self, layout: LayoutContext<'_>, parent: Option<ArtistId>) -> bool {
        [&self.a, &self.b].into_iter().any(|end| {
            let axes = end.axes.or_else(|| Self::parent_axes(layout, parent));
            Self::position(end, layout, axes, parent)
                .is_ok_and(|point| !self.visible(end, point, layout, axes))
        })
    }

    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let layout = frame.layout;
        let parent_axes = Self::parent_axes(layout, frame.parent);
        let locate = |end: &ConnectionEnd| {
            Self::position(end, layout, end.axes.or(parent_axes), frame.parent)
        };
        let (pos_a, pos_b) = (locate(&self.a)?, locate(&self.b)?);
        let dpi_cor = frame.dpi_cor;
        let path = self
            .spec
            .clipped_to(&layout, self.patches)
            .route(pos_a, pos_b, dpi_cor)?;
        self.spec.display_paths(&path, dpi_cor, frame.linewidth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    #[test]
    fn ends_in_two_axes_are_joined() {
        let fig = Figure::new();
        let left = fig
            .add_axes([0.1, 0.1, 0.3, 0.8])
            .expect("a valid rectangle");
        let right = fig
            .add_axes([0.6, 0.1, 0.3, 0.8])
            .expect("a valid rectangle");
        let link = ConnectionPatch::new(
            ConnectionEnd::new((1.0, 1.0), Coords::Data, Some(left.id())),
            ConnectionEnd::new((0.0, 0.0), Coords::Data, Some(right.id())),
        );
        let bounds = fig.add_artist(link).window_extent().expect("drawn");
        assert!((bounds.x0() - 256.0).abs() < 1e-9 && (bounds.y1() - 432.0).abs() < 1e-9);
        assert!((bounds.x1() - 384.0).abs() < 1e-9 && (bounds.y0() - 48.0).abs() < 1e-9);
    }

    #[test]
    fn data_ends_outside_their_axes_hide_the_patch() {
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.1, 0.1, 0.8, 0.8])
            .expect("a valid rectangle");
        let link = |clip: Option<bool>| ConnectionPatch {
            annotation_clip: clip,
            ..ConnectionPatch::new(
                ConnectionEnd::new((0.5, 0.5), Coords::Data, None),
                ConnectionEnd::new((5.0, 5.0), Coords::Data, None),
            )
        };
        let hidden = ax.add_artist(link(None));
        let shown = ax.add_artist(link(Some(false)));
        // Measured all the same, as matplotlib measures its path.
        assert_eq!(hidden.window_extent(), shown.window_extent());
        shown.set_visible(false);
        let blank = fig.to_png().expect("drawn");
        shown.set_visible(true);
        hidden.set_visible(false);
        assert_ne!(
            fig.to_png().expect("drawn"),
            blank,
            "the unclipped patch draws"
        );
        hidden.set_visible(true);
        shown.set_visible(false);
        assert_eq!(
            fig.to_png().expect("drawn"),
            blank,
            "the clipped one does not"
        );
    }

    #[test]
    fn negative_pixels_count_from_the_far_edge() {
        let fig = Figure::new();
        let link = ConnectionPatch::new(
            ConnectionEnd::new((10.0, 10.0), "figure pixels".parse().expect("valid"), None),
            ConnectionEnd::new(
                (-10.0, -10.0),
                "figure pixels".parse().expect("valid"),
                None,
            ),
        );
        let bounds = fig.add_artist(link).window_extent().expect("drawn");
        assert!((bounds.x1() - 630.0).abs() < 1e-9 && (bounds.y1() - 470.0).abs() < 1e-9);
    }

    #[test]
    fn patches_clip_the_ends_where_they_are_drawn() {
        // A 1000 px square axes over (0, 10)²: circles of radius 1 are
        // 100 px wide.
        let fig = Figure::with_size(1000.0 / 72.0, 1000.0 / 72.0, 72.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let start = ax.add_patch(crate::patches::Circle::new((1.0, 5.0), 1.0));
        let end = ax.add_patch(crate::patches::Circle::new((9.0, 5.0), 1.0));
        let link = ConnectionPatch::new(
            ConnectionEnd::new((1.0, 5.0), Coords::Data, None),
            ConnectionEnd::new((9.0, 5.0), Coords::Data, None),
        )
        .with_patches(Some(start.id()), Some(end.id()));
        let bounds = ax.add_artist(link).window_extent().expect("drawn");
        assert!((bounds.x0() - 200.0).abs() < 1.0, "{bounds:?}");
        assert!((bounds.x1() - 800.0).abs() < 1.0, "{bounds:?}");
    }
}
