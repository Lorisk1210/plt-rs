//! Collections built before they are added to an axes: matplotlib's
//! collection classes (`PatchCollection`, `LineCollection`,
//! `PolyCollection`, `EllipseCollection`, `CircleCollection`,
//! `RegularPolyCollection`, `EventCollection`).

use plt_core::path::DataLimits;
use plt_core::{Path, Point};

use super::{CollectionArtist, CollectionKind, CollectionProps, ColorMapping, EdgeColors, Mapped};
use crate::artist::{Artist, ArtistId};
use crate::colors::{Color, ColorError};
use crate::orientation::Orientation;
use crate::patches::{PatchProps, PatchShape, PatchStyle};
use crate::transforms::Transform;

/// The unit of the widths and heights of an ellipse collection
/// (matplotlib's `units`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EllipseUnits {
    /// Data units, x and y each in their own scale.
    Xy,
    /// Data units of the x axis.
    X,
    /// Data units of the y axis.
    Y,
    /// Inches.
    Inches,
    /// Points.
    #[default]
    Points,
    /// Fractions of the axes width.
    Width,
    /// Fractions of the axes height.
    Height,
    /// Display pixels.
    Dots,
}

/// The ellipses of an ellipse collection: half axes and angles, scaled at
/// draw time according to their units.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Ellipses {
    pub(crate) half_widths: Vec<f64>,
    pub(crate) half_heights: Vec<f64>,
    /// Counter-clockwise rotations in radians.
    pub(crate) angles: Vec<f64>,
    pub(crate) units: EllipseUnits,
}

/// One member of a patch collection: an outline in data coordinates and
/// the properties of the patch it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct CollectionPatch {
    /// The outline, placed in data coordinates.
    pub path: Path,
    /// The patch's own properties (used with `match_original`).
    pub style: PatchStyle,
    /// The patch's alpha.
    pub alpha: Option<f64>,
    /// Whether the patch draws an edge by default.
    pub edge_default: bool,
}

impl CollectionPatch {
    /// The outline of `shape` with the properties `props` give a patch of
    /// that shape.
    pub fn new<S: PatchShape>(shape: &S, props: PatchProps) -> Self {
        let mut style = PatchStyle::from_rc(&crate::rc::params());
        let base = props.apply_style(&mut style);
        Self {
            path: shape.placed_path(),
            style,
            alpha: base.alpha,
            edge_default: S::EDGE_DEFAULT,
        }
    }
}

/// A collection ready for [`Axes::add_collection`](crate::axes::Axes::add_collection).
///
/// ```
/// use plt::prelude::*;
/// use plt::collections::{CollectionProps, CollectionSpec};
/// use plt::transforms::Point;
/// let ax = Figure::new().subplot();
/// let segments = vec![vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]];
/// let lines = CollectionSpec::lines(segments)
///     .with_props(CollectionProps::default().linewidth(3.0));
/// ax.add_collection(lines);
/// ```
#[derive(Debug)]
pub struct CollectionSpec {
    pub(crate) artist: CollectionArtist,
}

impl CollectionSpec {
    fn new(kind: CollectionKind, paths: Vec<Path>) -> Self {
        Self {
            artist: CollectionArtist::new(kind, paths, crate::rc::params()),
        }
    }

    /// Shapes drawn in display pixels at data offsets.
    fn displayed(mut self) -> Self {
        self.artist.base_mut().transform = Some(Transform::Display);
        self
    }

    /// Patches drawn together (matplotlib's `PatchCollection`); with
    /// `match_original` each keeps its face (when filled), edge, width,
    /// style and anti-aliasing, but not its hatch.
    pub fn patches(patches: &[CollectionPatch], match_original: bool) -> Result<Self, ColorError> {
        let mut spec = Self::new(
            CollectionKind::Generic,
            patches.iter().map(|p| p.path.clone()).collect(),
        );
        if match_original {
            let rc = crate::rc::params();
            let resolved = patches
                .iter()
                .map(|p| p.style.resolve(&rc, p.alpha, p.edge_default))
                .collect::<Result<Vec<_>, _>>()?;
            let artist = &mut spec.artist;
            artist.facecolors = Some(
                patches
                    .iter()
                    .zip(&resolved)
                    .map(|(p, r)| {
                        Color::from(if p.style.fill {
                            r.face
                        } else {
                            plt_core::Rgba::TRANSPARENT
                        })
                    })
                    .collect(),
            );
            artist.edgecolors =
                EdgeColors::Colors(resolved.iter().map(|r| Color::from(r.edge)).collect());
            artist.linewidths = Some(resolved.iter().map(|r| r.linewidth).collect());
            artist.linestyles = patches.iter().map(|p| p.style.linestyle.clone()).collect();
            artist.antialiased = Some(resolved.iter().map(|r| r.antialiased).collect());
        }
        Ok(spec)
    }

    /// Arbitrary paths in data coordinates (matplotlib's `PathCollection`
    /// without sizes).
    pub fn paths(paths: Vec<Path>) -> Self {
        Self::new(CollectionKind::Path, paths)
    }

    /// Polylines in data coordinates (matplotlib's `LineCollection`).
    pub fn lines(segments: Vec<Vec<Point>>) -> Self {
        Self::new(
            CollectionKind::Line,
            segments.into_iter().map(Path::polyline).collect(),
        )
    }

    /// Closed polygons in data coordinates (matplotlib's `PolyCollection`).
    pub fn polygons(verts: Vec<Vec<Point>>) -> Self {
        Self::new(
            CollectionKind::Poly,
            verts.into_iter().map(Path::polygon).collect(),
        )
    }

    /// Ellipses with full axes `widths × heights` in `units`, rotated by
    /// `angles` degrees, one at each offset (matplotlib's
    /// `EllipseCollection`); set the offsets with
    /// [`with_offsets`](Self::with_offsets).
    pub fn ellipses(widths: &[f64], heights: &[f64], angles: &[f64], units: EllipseUnits) -> Self {
        let mut spec = Self::new(CollectionKind::Generic, vec![Path::unit_circle()]).displayed();
        spec.artist.ellipses = Some(Ellipses {
            half_widths: widths.iter().map(|w| 0.5 * w).collect(),
            half_heights: heights.iter().map(|h| 0.5 * h).collect(),
            angles: angles.iter().map(|a| a.to_radians()).collect(),
            units,
        });
        spec
    }

    /// Circles of `sizes` points² area, one at each offset (matplotlib's
    /// `CircleCollection`).
    pub fn circles(sizes: Vec<f64>) -> Self {
        let mut spec = Self::new(CollectionKind::Generic, vec![Path::unit_circle()]).displayed();
        spec.artist.sizes = Some(sizes);
        spec.artist.size_factor = std::f64::consts::PI.sqrt().recip();
        spec
    }

    /// Regular polygons of `numsides` sides, rotated by `rotation` radians,
    /// whose circumscribing circles have `sizes` points² area, one at each
    /// offset (matplotlib's `RegularPolyCollection`).
    pub fn regular_polygons(numsides: usize, rotation: f64, sizes: Vec<f64>) -> Self {
        let mut spec = Self::new(
            CollectionKind::Generic,
            vec![Path::unit_regular_polygon(numsides)],
        )
        .displayed();
        spec.artist.sizes = Some(sizes);
        spec.artist.size_factor = std::f64::consts::PI.sqrt().recip();
        spec.artist.size_rotation = rotation;
        spec
    }

    /// One line across `lineoffset` of `linelength` at each position, along
    /// x for [`Orientation::Horizontal`] (matplotlib's `EventCollection`;
    /// positions are sorted first).
    pub fn events(
        positions: &[f64],
        orientation: Orientation,
        lineoffset: f64,
        linelength: f64,
    ) -> Self {
        let mut sorted = positions.to_vec();
        sorted.sort_by(f64::total_cmp);
        let (high, low) = (lineoffset + linelength / 2.0, lineoffset - linelength / 2.0);
        let segments = sorted
            .into_iter()
            .map(|p| match orientation {
                Orientation::Horizontal => vec![Point::new(p, high), Point::new(p, low)],
                Orientation::Vertical => vec![Point::new(high, p), Point::new(low, p)],
            })
            .collect();
        Self::lines(segments)
    }

    /// The same collection with each item translated to an offset in
    /// `transform` coordinates.
    #[must_use]
    pub fn with_offsets(mut self, offsets: Vec<Point>, transform: Transform) -> Self {
        self.artist.offsets = offsets;
        self.artist.offset_transform = Some(transform);
        self
    }

    /// The same collection drawn through `transform` instead of the
    /// axes' data coordinates (matplotlib's `set_transform` before
    /// `add_collection`, which the data limits then follow).
    #[must_use]
    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.artist.base_mut().transform = Some(transform);
        self
    }

    /// The same collection taking no part in the data and view limits
    /// (matplotlib's `add_collection(..., autolim=False)`): adding it
    /// neither extends nor autoscales them, and [`relim`] leaves it out.
    ///
    /// [`relim`]: crate::axes::Axes::relim
    #[must_use]
    pub fn without_autolim(mut self) -> Self {
        self.artist.in_autoscale = false;
        self
    }

    /// The same collection with these properties.
    #[must_use]
    pub fn with_props(mut self, props: CollectionProps) -> Self {
        props.apply(&mut self.artist);
        self
    }

    /// The same collection coloured by mapping values through a norm and
    /// colormap (matplotlib's `cmap=` with `set_array`).
    #[must_use]
    pub fn with_mapping(mut self, mapping: ColorMapping) -> Self {
        self.artist.mapping = Some(Mapped::new(mapping));
        self
    }
}

impl CollectionArtist {
    /// The data limits the collection adds to `axes`, and in which
    /// directions they are data values (matplotlib's `get_datalim` and
    /// `_update_collection_limits`).
    pub(crate) fn data_limits(&self, axes: ArtistId) -> (DataLimits, (bool, bool)) {
        let transform = self.transform();
        let (x_data, y_data) = transform.data_directions(axes);
        let (ox_data, oy_data) = self
            .offset_transform
            .as_ref()
            .map_or((false, false), |t| t.data_directions(axes));
        let directions = (x_data || ox_data, y_data || oy_data);
        let offsets_usable = self
            .offset_transform
            .as_ref()
            .is_none_or(|t| ox_data || oy_data || *t == Transform::Display);
        if !offsets_usable || self.paths.is_empty() {
            return (DataLimits::NULL, directions);
        }
        if x_data || y_data {
            return (self.shape_extents(), directions);
        }
        if self.offset_transform.is_some() {
            let mut limits = DataLimits::NULL;
            limits.update_from_points(&self.offsets, true, true);
            return (limits, directions);
        }
        (DataLimits::NULL, directions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patches::{Circle, Rectangle};

    #[test]
    fn events_are_sorted_and_centred() {
        let spec = CollectionSpec::events(&[3.0, 1.0], Orientation::Horizontal, 2.0, 1.0);
        let first = spec.artist.paths[0].vertices();
        assert_eq!(first, [Point::new(1.0, 2.5), Point::new(1.0, 1.5)]);
        let vertical = CollectionSpec::events(&[1.0], Orientation::Vertical, 8.0, 1.0);
        assert_eq!(vertical.artist.paths[0].vertices()[0], Point::new(8.5, 1.0));
    }

    #[test]
    fn a_colour_only_colours_the_lines_of_line_collections() {
        let events = CollectionSpec::events(&[1.0], Orientation::Horizontal, 0.0, 1.0)
            .with_props(CollectionProps::default().color("r"));
        assert_eq!(events.artist.facecolors, None);
        assert_eq!(
            events.artist.edgecolors,
            EdgeColors::Colors(vec![Color::from("r")])
        );
        let polygons = CollectionSpec::polygons(vec![vec![Point::new(0.0, 0.0); 3]])
            .with_props(CollectionProps::default().color("r"));
        assert_eq!(polygons.artist.facecolors, Some(vec![Color::from("r")]));
    }

    #[test]
    fn matched_patches_keep_their_colours() {
        let patches = [
            CollectionPatch::new(
                &Circle::new((0.0, 0.0), 1.0),
                PatchProps::default().facecolor("red"),
            ),
            CollectionPatch::new(
                &Rectangle::new((0.0, 0.0), 1.0, 1.0),
                PatchProps::default().fill(false).linewidth(3.0),
            ),
        ];
        let spec = CollectionSpec::patches(&patches, true).unwrap();
        let faces = spec.artist.facecolors.clone().unwrap();
        assert_eq!(
            faces[0].resolve().unwrap(),
            plt_core::Rgba::new(1.0, 0.0, 0.0, 1.0)
        );
        assert_eq!(faces[1].resolve().unwrap().a, 0.0);
        assert_eq!(spec.artist.linewidths, Some(vec![1.0, 3.0]));
        let EdgeColors::Colors(edges) = &spec.artist.edgecolors else {
            panic!("explicit edges");
        };
        assert_eq!(
            edges[0].resolve().unwrap().a,
            0.0,
            "filled patches have no edge"
        );
        assert_eq!(edges[1].resolve().unwrap().a, 1.0, "unfilled ones do");
    }

    #[test]
    fn collections_extend_the_data_limits() {
        let fig = crate::figure::Figure::new();
        let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
        let segments = vec![vec![Point::new(1.0, 2.0), Point::new(3.0, -4.0)]];
        ax.add_collection(CollectionSpec::lines(segments));
        let limits = ax.data_lim().bbox;
        assert_eq!(
            (limits.x0(), limits.y0(), limits.x1(), limits.y1()),
            (1.0, -4.0, 3.0, 2.0)
        );
        let circles = CollectionSpec::circles(vec![100.0])
            .with_offsets(vec![Point::new(10.0, 10.0)], ax.trans_data());
        ax.add_collection(circles);
        assert_eq!(
            ax.data_lim().bbox.x1(),
            10.0,
            "offsets count, not the shapes"
        );
    }

    #[test]
    fn collections_without_autolim_leave_the_limits_alone() {
        let fig = crate::figure::Figure::new();
        let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
        ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        let before = (ax.data_lim(), ax.xlim(), ax.ylim());
        let far = vec![vec![Point::new(-50.0, 20.0), Point::new(80.0, 90.0)]];
        ax.add_collection(CollectionSpec::lines(far).without_autolim());
        assert_eq!((ax.data_lim(), ax.xlim(), ax.ylim()), before);
        ax.relim(false);
        assert_eq!(ax.data_lim(), before.0, "relim leaves it out too");
    }

    #[test]
    fn a_transform_set_before_adding_decides_the_data_limits() {
        let fig = crate::figure::Figure::new();
        let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
        let spiral = vec![vec![Point::new(0.0, 0.0), Point::new(40.0, 30.0)]];
        let offsets = vec![Point::new(1.0, 2.0), Point::new(3.0, 4.0)];
        // Shapes in display units at data offsets: only the offsets count,
        // as for matplotlib's `col.set_transform(...)` before
        // `add_collection`.
        let lines = CollectionSpec::lines(spiral)
            .with_offsets(offsets, ax.trans_data())
            .with_transform(Transform::Display);
        ax.add_collection(lines);
        let limits = ax.data_lim().bbox;
        assert_eq!(
            (limits.x0(), limits.y0(), limits.x1(), limits.y1()),
            (1.0, 2.0, 3.0, 4.0)
        );
    }

    #[test]
    fn sized_collections_draw_in_display_space() {
        let circles = CollectionSpec::circles(vec![10.0]);
        assert_eq!(circles.artist.base().transform, Some(Transform::Display));
        assert_eq!(
            circles.artist.size_factor,
            std::f64::consts::PI.sqrt().recip()
        );
        let ellipses = CollectionSpec::ellipses(&[2.0], &[1.0], &[90.0], EllipseUnits::Xy);
        let geometry = ellipses.artist.ellipses.unwrap();
        assert_eq!(geometry.half_widths, [1.0]);
        assert!((geometry.angles[0] - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
    }
}
