//! The shape of a spine: a line along its side, or an arc or a circle in
//! the fractions of its axes (matplotlib's `Spine.set_patch_line`,
//! `set_patch_arc` and `set_patch_circle`), and the coordinates of a
//! line spine (`get_spine_transform`).

use pltrs_core::{Affine2D, Path, Point};

use super::{Spine, SpineArtist};
use crate::artist::ArtistHandle;
use crate::transforms::Transform;

/// The shape a spine is drawn with.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum SpinePatch {
    /// A line along its side, spanning its bounds or its axis' view (or,
    /// on polar axes, the arc or radial line of the sector).
    #[default]
    Line,
    /// The arc of the circle of `radius` around `center` (axes fractions)
    /// from `theta1` to `theta2` degrees counter-clockwise.
    Arc {
        /// The centre of the circle.
        center: Point,
        /// The radius.
        radius: f64,
        /// Where the arc starts, in degrees.
        theta1: f64,
        /// Where the arc ends, in degrees.
        theta2: f64,
    },
    /// The circle of `radius` around `center` (axes fractions).
    Circle {
        /// The centre.
        center: Point,
        /// The radius.
        radius: f64,
    },
    /// A path of the program's, drawn through the spine's transform
    /// (matplotlib's `Spine(axes, spine_type, path)` of a type naming no
    /// side, such as the polygon around the radar chart example's axes).
    Path(Path),
}

pltrs_core::archive_enum!(SpinePatch {
    Line,
    Arc { center, radius, theta1, theta2 },
    Circle { center, radius },
    Path(v0),
});

impl SpinePatch {
    /// The outline of an arc or circle spine in the fractions of its
    /// axes; `None` for a line spine and a path of the program's.
    pub(super) fn outline(&self) -> Option<Path> {
        let (path, center, radius) = match *self {
            Self::Line | Self::Path(_) => return None,
            Self::Arc {
                center,
                radius,
                theta1,
                theta2,
            } => (Path::arc(theta1, theta2, None), center, radius),
            Self::Circle { center, radius } => (Path::unit_circle(), center, radius),
        };
        let placed = Affine2D::scaling(radius, radius).translate(center.x, center.y);
        Some(path.transformed(&placed))
    }
}

impl Spine {
    /// Draws the spine as the arc of the circle of `radius` around
    /// `center`, in axes fractions, from `theta1` to `theta2` degrees
    /// (matplotlib's `set_patch_arc`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::with_size(4.0, 4.0, 100.0).subplot();
    /// let top = &ax.spines().top;
    /// top.set_patch_arc(Point::new(0.5, 0.5), 0.5, 0.0, 180.0);
    /// let arc = top.display_path().expect("drawn").extents();
    /// let frame = ax.window_extent().expect("an extent");
    /// assert!((arc.y1() - frame.y1()).abs() < 1e-6 && arc.y0() > frame.y0());
    /// ```
    pub fn set_patch_arc(&self, center: Point, radius: f64, theta1: f64, theta2: f64) -> &Self {
        self.set_patch(SpinePatch::Arc {
            center,
            radius,
            theta1,
            theta2,
        })
    }

    /// Draws the spine as the circle of `radius` around `center`, in axes
    /// fractions (matplotlib's `set_patch_circle`).
    pub fn set_patch_circle(&self, center: Point, radius: f64) -> &Self {
        self.set_patch(SpinePatch::Circle { center, radius })
    }

    /// Draws the spine as a line along its side again (matplotlib's
    /// `set_patch_line`).
    pub fn set_patch_line(&self) -> &Self {
        self.set_patch(SpinePatch::Line)
    }

    /// Draws the spine as `path` through its transform, the axes' data
    /// transform unless it is given another (matplotlib's `Spine(axes,
    /// spine_type, path)` for a type naming no side).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// use pltrs::path::Path;
    /// use pltrs::transforms::Transform;
    /// let ax = Figure::with_size(4.0, 4.0, 100.0).subplot();
    /// let top = &ax.spines().top;
    /// let triangle = Path::unit_regular_polygon(3);
    /// top.set_patch_path(triangle);
    /// top.set_transform(Transform::Axes(ax.id()));
    /// let drawn = top.display_path().expect("drawn").extents();
    /// let frame = ax.window_extent().expect("an extent");
    /// assert!((drawn.y1() - frame.y1()).abs() < 1e-6, "{drawn:?} in {frame:?}");
    /// ```
    pub fn set_patch_path(&self, path: Path) -> &Self {
        self.set_patch(SpinePatch::Path(path))
    }

    fn set_patch(&self, patch: SpinePatch) -> &Self {
        self.raw.with_mut::<SpineArtist, _>(|s| s.patch = patch);
        self
    }

    /// The shape the spine is drawn with.
    pub fn patch(&self) -> SpinePatch {
        self.raw.with::<SpineArtist, _>(|s| s.patch.clone())
    }

    /// The coordinate system of the path of a line spine: axes fractions
    /// across the spine and data along it, moved as its position says
    /// (matplotlib's `get_spine_transform`); `None` once the spine is in
    /// no axes.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpinePosition;
    /// let ax = Figure::new().subplot();
    /// let left = &ax.spines().left;
    /// assert_eq!(
    ///     left.spine_transform(),
    ///     Some(pltrs::transforms::Transform::blended(ax.trans_axes(), ax.trans_data()))
    /// );
    /// left.set_position(SpinePosition::Outward(10.0));
    /// assert_ne!(left.spine_transform(), Some(pltrs::transforms::Transform::blended(ax.trans_axes(), ax.trans_data())));
    /// ```
    pub fn spine_transform(&self) -> Option<Transform> {
        let axes = self.axes()?;
        Some(
            self.raw
                .with::<SpineArtist, _>(|s| s.spine_transform(axes.id())),
        )
    }
}
