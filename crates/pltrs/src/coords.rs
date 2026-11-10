//! Named coordinate systems for placing annotations and connection patches
//! (matplotlib's `xycoords` and `textcoords`).
//!
//! A [`Coords`] says what the numbers of a point mean: data values of the
//! annotated axes, fractions of the axes or figure, points or pixels from
//! their lower-left corner, an offset from the annotated point, fractions
//! of another artist's extent, or any [`Transform`]. It is resolved at draw
//! time against a [`CoordFrame`].
//!
//! ```
//! use pltrs::coords::{CoordOrigin, CoordUnit, Coords};
//! let coords: Coords = "offset points".parse().unwrap();
//! assert_eq!(
//!     coords,
//!     Coords::Relative { origin: CoordOrigin::Offset, unit: CoordUnit::Points }
//! );
//! assert!("offset fraction".parse::<Coords>().is_err());
//! ```

use std::fmt;
use std::rc::Rc;
use std::str::FromStr;

use pltrs_core::{Affine2D, Bbox, Point};

use crate::artist::{ArtistId, LayoutContext};
use crate::transforms::{PointMap, ResolvedTransform, Step, Transform, unit_to_bbox};

/// Where a [`Coords::Relative`] system starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CoordOrigin {
    /// The lower-left corner of the (root) figure.
    Figure,
    /// The lower-left corner of the subfigure containing the artist.
    Subfigure,
    /// The lower-left corner of the axes.
    Axes,
    /// The annotated point (only for text positions).
    Offset,
}

pltrs_core::archive_enum!(CoordOrigin {
    Figure,
    Subfigure,
    Axes,
    Offset,
});

/// The unit of a [`Coords::Relative`] system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CoordUnit {
    /// Points (1/72 inch).
    Points,
    /// Display pixels.
    Pixels,
    /// Multiples of the text's font size.
    Fontsize,
    /// Fractions of the origin's box (not with [`CoordOrigin::Offset`]).
    Fraction,
}

pltrs_core::archive_enum!(CoordUnit {
    Points,
    Pixels,
    Fontsize,
    Fraction,
});

/// What the coordinates of a point mean.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Coords {
    /// Data coordinates of the axes (`"data"`).
    #[default]
    Data,
    /// `(theta, r)` in radians and data units of the axes (`"polar"`).
    Polar,
    /// Units from a corner or the annotated point (`"axes fraction"`,
    /// `"figure pixels"`, `"offset points"`, ...).
    Relative {
        /// The starting point.
        origin: CoordOrigin,
        /// The unit.
        unit: CoordUnit,
    },
    /// Any coordinate system.
    Transform(Transform),
    /// Fractions of another artist's extent.
    Artist(ArtistId),
    /// Units from a reference point given in fractions of another artist's
    /// extent (matplotlib's `text.OffsetFrom(artist, ref_coord, unit)`,
    /// which takes points or pixels; font sizes and fractions of the
    /// artist's extent work too).
    OffsetFrom {
        /// The artist whose extent holds the reference point.
        artist: ArtistId,
        /// The reference point, in fractions of the artist's extent.
        reference: Point,
        /// The unit of the offset.
        unit: CoordUnit,
    },
    /// Fractions of the part of another artist's extent inside its clip
    /// box (how `bar_label` centres labels on bars).
    ClippedArtist(ArtistId),
    /// x in the first system, y in the second.
    Blended(Box<Coords>, Box<Coords>),
}

pltrs_core::archive_enum!(Coords {
    Data,
    Polar,
    Relative { origin, unit },
    Transform(v0),
    Artist(v0),
    OffsetFrom { artist, reference, unit },
    ClippedArtist(v0),
    Blended(v0, v1),
});

impl Coords {
    /// `x` in `x_coords` and `y` in `y_coords` (matplotlib's tuple form).
    pub fn blended(x_coords: Self, y_coords: Self) -> Self {
        Self::Blended(Box::new(x_coords), Box::new(y_coords))
    }

    /// The artists whose extents these coordinates are relative to.
    pub(crate) fn artists(&self) -> Vec<ArtistId> {
        match self {
            Self::Artist(id) | Self::ClippedArtist(id) | Self::OffsetFrom { artist: id, .. } => {
                vec![*id]
            }
            Self::Blended(x, y) => [x.artists(), y.artists()].concat(),
            Self::Data | Self::Polar | Self::Relative { .. } | Self::Transform(_) => Vec::new(),
        }
    }

    /// Whether this is an offset from the annotated point (in either
    /// direction).
    pub fn is_offset(&self) -> bool {
        match self {
            Self::Relative { origin, .. } => *origin == CoordOrigin::Offset,
            Self::Blended(x, y) => x.is_offset() || y.is_offset(),
            _ => false,
        }
    }

    /// The map from these coordinates to display pixels
    /// (`_AnnotationBase._get_xy_transform`).
    pub fn resolve(&self, frame: &CoordFrame<'_>) -> Result<ResolvedTransform, CoordError> {
        let layout = frame.layout;
        Ok(match self {
            Self::Data => data(frame)?,
            Self::Polar => ResolvedTransform::with_step(
                Step::Plane(Rc::new(PolarToCartesian)),
                Affine2D::IDENTITY,
            )
            .then(data(frame)?),
            Self::Relative { origin, unit } => {
                let (corner, size) = match origin {
                    CoordOrigin::Figure => corner_of(frame.figure()?),
                    CoordOrigin::Subfigure => corner_of(frame.subfigure()?),
                    CoordOrigin::Axes => corner_of(frame.axes_box()?),
                    CoordOrigin::Offset => (frame.offset_origin.ok_or(CoordError::NoOffset)?, None),
                };
                let points = layout.points_to_pixels(1.0);
                let scale = match unit {
                    CoordUnit::Points => Affine2D::scaling(points, points),
                    CoordUnit::Pixels => Affine2D::IDENTITY,
                    CoordUnit::Fontsize => {
                        let s = frame.fontsize * points;
                        Affine2D::scaling(s, s)
                    }
                    CoordUnit::Fraction => {
                        let (w, h) = size.ok_or(CoordError::OffsetFraction)?;
                        Affine2D::scaling(w, h)
                    }
                };
                ResolvedTransform::affine(scale.translate(corner.x, corner.y))
            }
            Self::Transform(transform) => layout.resolve(transform).ok_or(CoordError::Detached)?,
            Self::Artist(id) => ResolvedTransform::affine(unit_to_bbox(&extent_of(frame, *id)?)),
            Self::OffsetFrom {
                artist,
                reference,
                unit,
            } => {
                let extent = extent_of(frame, *artist)?;
                let origin = unit_to_bbox(&extent).transform_point(*reference);
                let points = layout.points_to_pixels(1.0);
                let scale = match unit {
                    CoordUnit::Points => Affine2D::scaling(points, points),
                    CoordUnit::Pixels => Affine2D::IDENTITY,
                    CoordUnit::Fontsize => {
                        let s = frame.fontsize * points;
                        Affine2D::scaling(s, s)
                    }
                    CoordUnit::Fraction => Affine2D::scaling(extent.width(), extent.height()),
                };
                ResolvedTransform::affine(scale.translate(origin.x, origin.y))
            }
            Self::ClippedArtist(id) => {
                let artist = layout.store().get(*id).ok_or(CoordError::Detached)?;
                let extent = artist.window_extent(&layout).ok_or(CoordError::NoExtent)?;
                let clip = artist
                    .base()
                    .clip_box
                    .and_then(|axes| layout.axes_extent(axes))
                    .ok_or(CoordError::NoExtent)?;
                let visible = extent.intersection(&clip).unwrap_or(Bbox::NULL);
                ResolvedTransform::affine(unit_to_bbox(&visible))
            }
            Self::Blended(x, y) => ResolvedTransform::blend(x.resolve(frame)?, y.resolve(frame)?),
        })
    }
}

/// The display extent of the artist `id`.
fn extent_of(frame: &CoordFrame<'_>, id: ArtistId) -> Result<Bbox, CoordError> {
    let layout = frame.layout;
    let artist = layout.store().get(id).ok_or(CoordError::Detached)?;
    artist.window_extent(&layout).ok_or(CoordError::NoExtent)
}

/// Whether the display point lies within the axes box, with matplotlib's
/// one-pixel tolerance (`Axes.contains_point`); `false` for a removed axes.
pub fn axes_contains_point(layout: LayoutContext<'_>, axes: ArtistId, point: Point) -> bool {
    layout.axes_extent(axes).is_some_and(|extent| {
        pltrs_core::Path::unit_rectangle().contains_point(point, Some(&unit_to_bbox(&extent)), 1.0)
    })
}

/// Whether an annotation of the display point `point`, given in
/// `xycoords`, is drawn: always, unless `annotation_clip` asks (or, unset,
/// the point is in data coordinates) to draw it only while the point is
/// inside `axes` (matplotlib's `_AnnotationBase._check_xy`).
pub fn annotation_shown(
    layout: LayoutContext<'_>,
    annotation_clip: Option<bool>,
    xycoords: &Coords,
    axes: Option<ArtistId>,
    point: Point,
) -> bool {
    let clip = annotation_clip.unwrap_or(*xycoords == Coords::Data);
    !clip || axes.is_some_and(|axes| axes_contains_point(layout, axes, point))
}

/// The data transform of the frame's axes.
fn data(frame: &CoordFrame<'_>) -> Result<ResolvedTransform, CoordError> {
    let axes = frame.axes.ok_or(CoordError::NoAxes)?;
    frame
        .layout
        .resolve(&Transform::Data(axes))
        .ok_or(CoordError::Detached)
}

/// The lower-left corner and size of a box.
fn corner_of(bbox: Bbox) -> (Point, Option<(f64, f64)>) {
    (bbox.p0, Some((bbox.width(), bbox.height())))
}

/// matplotlib's `PolarAxes.PolarTransform()` without an axes: `(theta, r)`
/// to `(r cos theta, r sin theta)`.
#[derive(Debug)]
pub(crate) struct PolarToCartesian;

pltrs_core::archive_struct!(PolarToCartesian);

impl PointMap for PolarToCartesian {
    fn forward(&self, point: Point) -> Point {
        let (theta, r) = (point.x, point.y);
        Point::new(r * theta.cos(), r * theta.sin())
    }

    fn inverse(&self, point: Point) -> Point {
        Point::new(point.y.atan2(point.x), point.x.hypot(point.y))
    }
}

/// What coordinates are resolved against.
#[derive(Clone, Copy, Debug)]
pub struct CoordFrame<'a> {
    /// The figure geometry.
    pub layout: LayoutContext<'a>,
    /// The axes of `"data"`, `"polar"` and `"axes ..."` coordinates.
    pub axes: Option<ArtistId>,
    /// The artist's container (an axes or figure), from which the
    /// (sub)figure is found.
    pub owner: Option<ArtistId>,
    /// The annotated point in display pixels, origin of offsets.
    pub offset_origin: Option<Point>,
    /// The text's font size in points (for `"... fontsize"`).
    pub fontsize: f64,
}

impl CoordFrame<'_> {
    /// The `bbox`es of the frames from the owner up to the root figure:
    /// the root figure's is the saved window while part of it is printed
    /// (matplotlib's `adjust_bbox` replaces it, so that `"subfigure ..."`
    /// coordinates of the root figure start at the window's corner).
    fn frames(&self) -> Vec<Bbox> {
        let mut frames = Vec::new();
        let mut next = self.owner;
        while let Some(id) = next {
            if let Some(extent) = self.layout.frame_patch_extent(id) {
                frames.push(extent);
            }
            next = self.layout.store().get(id).and_then(|a| a.base().parent);
        }
        frames
    }

    /// The box of `"figure ..."` coordinates: the root figure's
    /// `figbbox`, which a window printed leaves where it is (see
    /// [`LayoutContext::figure_box`]).
    fn figure(&self) -> Result<Bbox, CoordError> {
        if self.frames().is_empty() {
            return Err(CoordError::NoFigure);
        }
        Ok(self.layout.figure_box())
    }

    fn subfigure(&self) -> Result<Bbox, CoordError> {
        self.frames().first().copied().ok_or(CoordError::NoFigure)
    }

    fn axes_box(&self) -> Result<Bbox, CoordError> {
        let axes = self.axes.ok_or(CoordError::NoAxes)?;
        self.layout.axes_extent(axes).ok_or(CoordError::Detached)
    }
}

/// Why coordinates cannot be resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordError {
    /// Data or axes coordinates outside an axes.
    NoAxes,
    /// Figure coordinates of an artist outside a figure.
    NoFigure,
    /// An offset where there is no annotated point to start from (the
    /// annotated point itself cannot be an offset).
    NoOffset,
    /// `"offset fraction"`: offsets have no box to take fractions of.
    OffsetFraction,
    /// The artist whose extent is used has none.
    NoExtent,
    /// A coordinate system refers to an artist that no longer exists.
    Detached,
}

impl fmt::Display for CoordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoAxes => "data and axes coordinates need an axes",
            Self::NoFigure => "figure coordinates need a figure",
            Self::NoOffset => "offset coordinates need an annotated point",
            Self::OffsetFraction => "'offset fraction' is not a valid coordinate",
            Self::NoExtent => "the reference artist has no extent",
            Self::Detached => "a coordinate system refers to a removed artist",
        })
    }
}

impl std::error::Error for CoordError {}

/// An unknown coordinate system name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoordParseError(pub String);

impl fmt::Display for CoordParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a valid coordinate", self.0)
    }
}

impl std::error::Error for CoordParseError {}

impl FromStr for Coords {
    type Err = CoordParseError;

    /// Parses matplotlib's names: `"data"`, `"polar"` and
    /// `"<figure|subfigure|axes|offset> <points|pixels|fontsize|fraction>"`
    /// (except `"offset fraction"`).
    fn from_str(text: &str) -> Result<Self, CoordParseError> {
        let error = || CoordParseError(text.to_owned());
        match text {
            "data" => return Ok(Self::Data),
            "polar" => return Ok(Self::Polar),
            _ => {}
        }
        let mut words = text.split_whitespace();
        let (Some(origin), Some(unit), None) = (words.next(), words.next(), words.next()) else {
            return Err(error());
        };
        let origin = match origin {
            "figure" => CoordOrigin::Figure,
            "subfigure" => CoordOrigin::Subfigure,
            "axes" => CoordOrigin::Axes,
            "offset" => CoordOrigin::Offset,
            _ => return Err(error()),
        };
        let unit = match unit {
            "points" => CoordUnit::Points,
            "pixels" => CoordUnit::Pixels,
            "fontsize" => CoordUnit::Fontsize,
            "fraction" if origin != CoordOrigin::Offset => CoordUnit::Fraction,
            _ => return Err(error()),
        };
        Ok(Self::Relative { origin, unit })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::close_points;

    #[test]
    fn parses_matplotlib_names() {
        assert_eq!("data".parse(), Ok(Coords::Data));
        assert_eq!("polar".parse(), Ok(Coords::Polar));
        assert_eq!(
            "axes fraction".parse(),
            Ok(Coords::Relative {
                origin: CoordOrigin::Axes,
                unit: CoordUnit::Fraction
            })
        );
        assert_eq!(
            "subfigure fontsize".parse(),
            Ok(Coords::Relative {
                origin: CoordOrigin::Subfigure,
                unit: CoordUnit::Fontsize
            })
        );
        for bad in ["offset fraction", "axes", "axes inches", "data points", ""] {
            assert!(bad.parse::<Coords>().is_err(), "{bad}");
        }
    }

    #[test]
    fn offsets_are_detected_in_either_direction() {
        let offset: Coords = "offset pixels".parse().unwrap();
        assert!(offset.is_offset());
        assert!(Coords::blended(Coords::Data, offset).is_offset());
        assert!(!Coords::Data.is_offset());
    }

    /// Resolves `coords` for a 640 × 480 px figure with one axes over
    /// `(0, 10)²`, offsets starting at `(10, 10)` and 10-point text.
    fn resolve(coords: &str, point: Point) -> Point {
        use crate::artist::ArtistHandle;
        let fig = crate::figure::Figure::new();
        let ax = fig
            .add_axes([0.125, 0.11, 0.775, 0.77])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        ax.unstale_view();
        let state = ax.raw().figure_state().borrow();
        let frame = CoordFrame {
            layout: LayoutContext::new(&state),
            axes: Some(ax.id()),
            owner: Some(ax.id()),
            offset_origin: Some(Point::new(10.0, 10.0)),
            fontsize: 10.0,
        };
        coords
            .parse::<Coords>()
            .expect("valid coords")
            .resolve(&frame)
            .expect("resolvable")
            .transform_point(point)
    }

    #[test]
    fn named_systems_resolve_against_the_figure() {
        let p = Point::new;
        assert!(close_points(resolve("data", p(5.0, 5.0)), p(328.0, 237.6)));
        assert!(close_points(
            resolve("axes fraction", p(0.5, 0.5)),
            p(328.0, 237.6)
        ));
        assert!(close_points(
            resolve("axes pixels", p(1.0, 2.0)),
            p(81.0, 54.8)
        ));
        assert!(close_points(
            resolve("figure points", p(72.0, 36.0)),
            p(100.0, 50.0)
        ));
        assert!(close_points(
            resolve("figure fraction", p(1.0, 1.0)),
            p(640.0, 480.0)
        ));
        assert!(close_points(
            resolve("offset pixels", p(-5.0, 5.0)),
            p(5.0, 15.0)
        ));
        let fontsize = 10.0 * 100.0 / 72.0;
        assert!(close_points(
            resolve("offset fontsize", p(1.0, 0.0)),
            p(10.0 + fontsize, 10.0)
        ));
        let polar = resolve("polar", p(std::f64::consts::FRAC_PI_2, 5.0));
        assert!(close_points(polar, resolve("data", p(0.0, 5.0))));
    }

    #[test]
    fn figure_coordinates_keep_the_whole_figure_while_a_window_is_printed() {
        // matplotlib 3.11: in a tight save an annotation's "figure
        // fraction" is a fraction of the whole figure at the canvas'
        // corner (`figbbox`), its "subfigure fraction" one of the window.
        use crate::artist::ArtistHandle;
        let fig = crate::figure::Figure::new();
        let ax = fig.subplot();
        let cell = ax.raw().figure_state();
        cell.borrow_mut().print_window = Some(Bbox::from_bounds(1.0, 0.5, 4.0, 3.0));
        let state = cell.borrow();
        let frame = CoordFrame {
            layout: LayoutContext::new(&state),
            axes: Some(ax.id()),
            owner: Some(ax.id()),
            offset_origin: None,
            fontsize: 10.0,
        };
        let corner = |coords: &str| {
            let coords: Coords = coords.parse().expect("valid coords");
            let resolved = coords.resolve(&frame).expect("resolvable");
            resolved.transform_point(Point::new(1.0, 1.0))
        };
        assert!(close_points(
            corner("figure fraction"),
            Point::new(640.0, 480.0)
        ));
        assert!(close_points(
            corner("subfigure fraction"),
            Point::new(400.0, 300.0)
        ));
    }

    #[test]
    fn the_axes_box_contains_points_with_a_pixel_of_slack() {
        use crate::artist::ArtistHandle;
        let fig = crate::figure::Figure::new();
        let ax = fig
            .add_axes([0.125, 0.11, 0.775, 0.77])
            .expect("a valid rectangle");
        let state = ax.raw().figure_state().borrow();
        let layout = LayoutContext::new(&state);
        assert!(axes_contains_point(
            layout,
            ax.id(),
            Point::new(300.0, 200.0)
        ));
        assert!(axes_contains_point(
            layout,
            ax.id(),
            Point::new(79.5, 200.0)
        ));
        assert!(!axes_contains_point(
            layout,
            ax.id(),
            Point::new(70.0, 200.0)
        ));
    }

    #[test]
    fn polar_points_map_to_cartesian() {
        let polar = PolarToCartesian;
        let p = polar.forward(Point::new(std::f64::consts::FRAC_PI_2, 2.0));
        assert!((p - Point::new(0.0, 2.0)).length() < 1e-12);
        let back = polar.inverse(p);
        assert!((back - Point::new(std::f64::consts::FRAC_PI_2, 2.0)).length() < 1e-12);
    }
}
