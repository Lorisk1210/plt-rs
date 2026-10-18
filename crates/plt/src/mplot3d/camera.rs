//! The camera of a 3-D axes: view angles, projection type and box aspect,
//! and the projection matrix they produce (matplotlib's `view_init`,
//! `set_proj_type`, `set_box_aspect` and `get_proj`).

use super::coords::{Axis3, roll};
use super::error::Axes3DError;
use super::math::{
    Matrix4, Vec3, add, ortho_transformation, persp_transformation, scaled, view_axes,
    view_transformation, world_transformation,
};
use super::scaling::AxisScale;

/// Distance of the camera from the centre of the box (matplotlib's
/// `_dist`).
pub(crate) const DIST: f64 = 10.0;

/// The default box aspect, x:y:z.
const DEFAULT_BOX_ASPECT: Vec3 = [4.0, 4.0, 3.0];

/// Scale of the box aspect tuned to match matplotlib 3.2's appearance,
/// with the 25/24 factor compensating for matplotlib 3.9's automargin.
const BOX_SCALE: f64 = 1.829_464_072_162_043_4 * 25.0 / 24.0;

/// How 3-D points are projected onto the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjType {
    /// A perspective projection with the given focal length (1 by
    /// default; `focal_length = 1 / tan(FOV / 2)`).
    Persp {
        /// The focal length of the virtual camera; must be positive.
        focal_length: f64,
    },
    /// An orthographic projection.
    Ortho,
}

impl Default for ProjType {
    fn default() -> Self {
        Self::Persp { focal_length: 1.0 }
    }
}

impl ProjType {
    /// The focal length, infinite for orthographic projections.
    fn focal_length(self) -> Result<f64, Axes3DError> {
        match self {
            Self::Persp { focal_length } if focal_length > 0.0 => Ok(focal_length),
            Self::Persp { focal_length } => Err(Axes3DError::FocalLength(focal_length)),
            Self::Ortho => Ok(f64::INFINITY),
        }
    }
}

/// View angles in degrees (matplotlib's `elev`, `azim` and `roll`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewAngles {
    /// Elevation of the camera above the plane normal to the vertical
    /// axis.
    pub elev: f64,
    /// Azimuth of the camera about the vertical axis.
    pub azim: f64,
    /// Rotation of the camera about the viewing axis.
    pub roll: f64,
}

impl Default for ViewAngles {
    /// matplotlib's default view: `elev=30, azim=-60, roll=0`.
    fn default() -> Self {
        Self {
            elev: 30.0,
            azim: -60.0,
            roll: 0.0,
        }
    }
}

/// Everything that determines the projection besides the axis limits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Camera {
    pub(crate) angles: ViewAngles,
    /// The axis displayed vertically.
    pub(crate) vertical: Axis3,
    /// Focal length; infinite for an orthographic projection.
    focal_length: f64,
    /// The scaled box aspect in matplotlib's storage order (rolled back
    /// from the vertical axis).
    box_aspect: Vec3,
}

/// The projection of one draw (matplotlib's `M` preceded by the axis
/// scale transforms).
#[derive(Clone, Debug)]
pub(crate) struct Projection {
    /// Transformed data coordinates to projected coordinates.
    pub(crate) matrix: Matrix4,
    /// Whether points outside the view frustum are clipped.
    perspective: bool,
    /// The scales of the x, y and z axes.
    scales: [AxisScale; 3],
}

impl Projection {
    /// The same projection applied after the scale transforms of the
    /// three axes.
    pub(crate) fn with_scales(self, scales: [AxisScale; 3]) -> Self {
        Self { scales, ..self }
    }

    /// A data point in transformed coordinates; coordinates outside their
    /// scale's domain become NaN (matplotlib's `_apply_scale_transforms`
    /// with `_scale_invalid_mask`).
    pub(crate) fn transform(&self, p: Vec3) -> Vec3 {
        [0, 1, 2].map(|i| self.scales[i].masked_forward(p[i]))
    }

    /// Projects a data point (matplotlib's `_scale_proj_transform`).
    pub(crate) fn project(&self, p: Vec3) -> Vec3 {
        self.matrix.project(self.transform(p))
    }

    /// Projects a point already in transformed coordinates (matplotlib's
    /// `proj_transform`).
    pub(crate) fn project_transformed(&self, q: Vec3) -> Vec3 {
        self.matrix.project(q)
    }

    /// Projects a data point, or `None` when a perspective projection puts
    /// it outside the view frustum (matplotlib's
    /// `_scale_proj_transform_clip`).
    pub(crate) fn project_clipped(&self, p: Vec3) -> Option<Vec3> {
        let projected = self.project(p);
        let [x, y, z] = projected;
        let inside = !self.perspective
            || ((-1.0..=1.0).contains(&x) && (-1.0..=1.0).contains(&y) && z <= 0.0);
        let finite = projected.iter().all(|c| c.is_finite());
        (inside && finite).then_some(projected)
    }
}

impl Camera {
    /// The default camera with the given projection type.
    pub(crate) fn new(proj_type: ProjType) -> Result<Self, Axes3DError> {
        let mut camera = Self {
            angles: ViewAngles::default(),
            vertical: Axis3::Z,
            focal_length: 1.0,
            box_aspect: DEFAULT_BOX_ASPECT,
        };
        camera.set_proj_type(proj_type)?;
        camera.set_box_aspect(None, 1.0)?;
        Ok(camera)
    }

    /// Whether the projection is orthographic.
    pub(crate) fn is_ortho(&self) -> bool {
        self.focal_length.is_infinite()
    }

    /// Changes the projection type (matplotlib's `set_proj_type`).
    pub(crate) fn set_proj_type(&mut self, proj_type: ProjType) -> Result<(), Axes3DError> {
        self.focal_length = proj_type.focal_length()?;
        Ok(())
    }

    /// Sets the x:y:z ratio of the box edges and the zoom (matplotlib's
    /// `set_box_aspect`); `None` restores 4:4:3.
    pub(crate) fn set_box_aspect(
        &mut self,
        aspect: Option<Vec3>,
        zoom: f64,
    ) -> Result<(), Axes3DError> {
        if !(zoom > 0.0 && zoom.is_finite()) {
            return Err(Axes3DError::Zoom(zoom));
        }
        let aspect = aspect.unwrap_or(DEFAULT_BOX_ASPECT);
        if aspect.iter().any(|a| !(a.is_finite() && *a > 0.0)) {
            return Err(Axes3DError::BoxAspect(aspect));
        }
        let length = super::math::norm(aspect);
        let factor = BOX_SCALE * zoom / length;
        self.box_aspect = self.storage_order(scaled(aspect, factor));
        Ok(())
    }

    /// The box aspect in data-axis order.
    pub(crate) fn box_aspect(&self) -> Vec3 {
        self.vertical_order(self.box_aspect)
    }

    /// Rolls an `[x, y, z]` array so that the vertical axis takes the
    /// place of z (matplotlib's `_roll_to_vertical`).
    pub(crate) fn vertical_order(&self, a: Vec3) -> Vec3 {
        roll(a, self.vertical.index() as isize - 2)
    }

    /// Undoes [`Camera::vertical_order`].
    fn storage_order(&self, a: Vec3) -> Vec3 {
        roll(a, 2 - self.vertical.index() as isize)
    }

    /// The projection for the axis limits `[xmin, xmax, ymin, ymax, zmin,
    /// zmax]` in transformed coordinates, for linear axes until
    /// [`Projection::with_scales`] (matplotlib's `get_proj`).
    pub(crate) fn projection(&self, limits: [f64; 6]) -> Projection {
        let box_aspect = self.box_aspect();
        let world = world_transformation(limits, box_aspect);
        let center = scaled(box_aspect, 0.5);
        let direction = self.direction();
        let eye = add(center, scaled(direction, DIST));
        let [u, v, w] = self.view_axes(eye);
        let (view, projection) = if self.is_ortho() {
            (
                view_transformation(u, v, w, eye),
                ortho_transformation(-DIST, DIST),
            )
        } else {
            let eye_focal = add(center, scaled(scaled(direction, DIST), self.focal_length));
            (
                view_transformation(u, v, w, eye_focal),
                persp_transformation(-DIST, DIST, self.focal_length),
            )
        };
        Projection {
            matrix: projection.dot(&view.dot(&world)),
            perspective: !self.is_ortho(),
            scales: Default::default(),
        }
    }

    /// The unit vector from the centre of the box towards the camera.
    fn direction(&self) -> Vec3 {
        let (elev, azim) = (self.angles.elev.to_radians(), self.angles.azim.to_radians());
        self.vertical_order([elev.cos() * azim.cos(), elev.cos() * azim.sin(), elev.sin()])
    }

    /// The view axes `u` (screen right), `v` (screen up) and `w` (out of
    /// the screen) in world coordinates (matplotlib's `_view_u`, `_view_v`
    /// and `_view_w`).
    pub(crate) fn view_frame(&self) -> [Vec3; 3] {
        let center = scaled(self.box_aspect(), 0.5);
        self.view_axes(add(center, scaled(self.direction(), DIST)))
    }

    /// The box aspect as stored, before rolling to the vertical axis
    /// (matplotlib's `_box_aspect`).
    pub(crate) fn stored_box_aspect(&self) -> Vec3 {
        self.box_aspect
    }

    /// The focal length; infinite for an orthographic projection.
    pub(crate) fn focal_length(&self) -> f64 {
        self.focal_length
    }

    /// The view axes for an eye at `eye` (matplotlib's `_calc_view_axes`).
    fn view_axes(&self, eye: Vec3) -> [Vec3; 3] {
        let elev = norm_angle(self.angles.elev).to_radians();
        let roll = norm_angle(self.angles.roll).to_radians();
        let center = scaled(self.box_aspect(), 0.5);
        let mut vertical = [0.0; 3];
        vertical[self.vertical.index()] = if elev.abs() > std::f64::consts::FRAC_PI_2 {
            -1.0
        } else {
            1.0
        };
        view_axes(eye, center, vertical, roll)
    }

    /// The window of projected coordinates shown by the axes, the same in
    /// x and y (matplotlib's `set_top_view`).
    pub(crate) fn view_window() -> (f64, f64) {
        (-0.95 / DIST, 0.9 / DIST)
    }
}

/// An angle normalised to `(-180, 180]` degrees (matplotlib's
/// `_norm_angle`).
pub(crate) fn norm_angle(a: f64) -> f64 {
    let a = (a + 360.0).rem_euclid(360.0);
    if a > 180.0 { a - 360.0 } else { a }
}

/// An angle normalised to `(-90, 90]` degrees, for text that must stay
/// readable (matplotlib's `_norm_text_angle`).
pub(crate) fn norm_text_angle(a: f64) -> f64 {
    let a = (a + 180.0).rem_euclid(180.0);
    if a > 90.0 { a - 180.0 } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// matplotlib's `ax.M` for the default view of an empty axes
    /// (limits `(-1/48, 1 + 1/48)` on every axis).
    #[test]
    fn default_projection_matches_matplotlib() {
        let camera = Camera::new(ProjType::default()).expect("valid camera");
        let (lo, hi) = (-1.0 / 48.0, 1.0 + 1.0 / 48.0);
        let m = camera.projection([lo, hi, lo, hi, lo, hi]).matrix;
        let expected = [
            [
                0.989_743_320_950_125_6,
                0.571_428_572_779_189_3,
                0.0,
                -0.780_585_946_864_658_1,
            ],
            [
                -0.285_714_286_389_594_6,
                0.494_871_660_475_062_7,
                0.742_307_490_712_594_5,
                -0.475_732_432_399_032_3,
            ],
            [0.0, 0.0, 0.0, -10.0],
            [
                -0.494_871_660_475_063,
                0.857_142_859_168_783_9,
                -0.428_571_429_584_391_87,
                10.033_150_115_445_334,
            ],
        ];
        for (row, want) in m.rows().iter().zip(expected) {
            for (got, want) in row.iter().zip(want) {
                assert!((got - want).abs() < 1e-12, "{got} != {want}");
            }
        }
    }

    #[test]
    fn invalid_settings_are_rejected() {
        let mut camera = Camera::new(ProjType::Ortho).expect("valid camera");
        assert!(camera.is_ortho());
        assert!(
            camera
                .set_proj_type(ProjType::Persp { focal_length: 0.0 })
                .is_err()
        );
        assert!(camera.set_box_aspect(Some([1.0, -1.0, 1.0]), 1.0).is_err());
        assert!(camera.set_box_aspect(None, 0.0).is_err());
        assert!(camera.set_box_aspect(None, f64::NAN).is_err());
    }

    #[test]
    fn perspective_clips_points_outside_the_frustum() {
        let camera = Camera::new(ProjType::default()).expect("valid camera");
        let projection = camera.projection([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
        assert!(projection.project_clipped([0.5, 0.5, 0.5]).is_some());
        assert!(projection.project_clipped([500.0, 0.5, 0.5]).is_none());
        assert!(projection.project_clipped([f64::NAN, 0.5, 0.5]).is_none());
    }

    #[test]
    fn angles_are_normalised_like_matplotlib() {
        assert_eq!(norm_angle(190.0), -170.0);
        assert_eq!(norm_angle(-190.0), 170.0);
        assert_eq!(norm_angle(180.0), 180.0);
        assert_eq!(norm_text_angle(100.0), -80.0);
        assert_eq!(norm_text_angle(-95.0), 85.0);
        assert_eq!(norm_text_angle(90.0), 90.0);
    }
}
