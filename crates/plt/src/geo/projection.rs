//! The map projections of geographic axes: longitude and latitude in
//! radians onto the plane (matplotlib's `AitoffTransform`,
//! `HammerTransform`, `LambertTransform`, `MollweideTransform` and their
//! inverses), written with numpy's arithmetic so that results agree to the
//! last bit where the libm functions do.

use std::f64::consts::{FRAC_PI_2, PI, SQRT_2};

use plt_core::{Affine2D, Point};

use crate::axes::ProjectionKind;

/// Latitudes this close to a pole (radians) skip the Newton iteration of
/// the Mollweide projection for a series approximation.
const MOLLWEIDE_POLAR_ZONE: f64 = 0.087;

/// The Newton iteration of the Mollweide projection stops once a step is
/// at most this small.
const MOLLWEIDE_TOLERANCE: f64 = 0.001;

/// An upper bound on Newton steps; the iteration converges in a handful
/// outside the polar zones, the bound only guards against pathological
/// input.
const MOLLWEIDE_MAX_STEPS: usize = 100;

/// A map projection of the sphere (matplotlib's `aitoff`, `hammer`,
/// `lambert` and `mollweide` projections).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeoProjection {
    /// Aitoff's equatorial azimuthal projection, stretched horizontally.
    /// It has no closed-form inverse: matplotlib's maps every point to NaN.
    Aitoff,
    /// The Hammer–Aitoff equal-area projection.
    Hammer,
    /// The Lambert azimuthal equal-area projection around a centre given
    /// in radians; the whole sphere fits in a disc of radius 2.
    Lambert {
        /// Longitude of the centre of the map, in radians.
        center_longitude: f64,
        /// Latitude of the centre of the map, in radians.
        center_latitude: f64,
    },
    /// The Mollweide equal-area pseudocylindrical projection.
    Mollweide,
}

impl GeoProjection {
    /// The Lambert projection centred at longitude and latitude 0.
    pub const LAMBERT: Self = Self::Lambert {
        center_longitude: 0.0,
        center_latitude: 0.0,
    };

    /// The kind of axes this projection draws (its matplotlib name).
    pub const fn kind(self) -> ProjectionKind {
        match self {
            Self::Aitoff => ProjectionKind::Aitoff,
            Self::Hammer => ProjectionKind::Hammer,
            Self::Lambert { .. } => ProjectionKind::Lambert,
            Self::Mollweide => ProjectionKind::Mollweide,
        }
    }

    /// Projects `(longitude, latitude)` in radians onto the plane.
    pub fn forward(self, lonlat: Point) -> Point {
        let (lon, lat) = (lonlat.x, lonlat.y);
        match self {
            Self::Aitoff => aitoff(lon, lat),
            Self::Hammer => hammer(lon, lat),
            Self::Lambert {
                center_longitude,
                center_latitude,
            } => lambert(lon - center_longitude, lat, center_latitude),
            Self::Mollweide => mollweide(lon, lat),
        }
    }

    /// The `(longitude, latitude)` in radians projected onto plane point
    /// `xy` (NaN for every point of the Aitoff projection).
    pub fn inverse(self, xy: Point) -> Point {
        match self {
            Self::Aitoff => Point::new(f64::NAN, f64::NAN),
            Self::Hammer => hammer_inverse(xy),
            Self::Lambert {
                center_longitude,
                center_latitude,
            } => lambert_inverse(xy, center_longitude, center_latitude),
            Self::Mollweide => mollweide_inverse(xy),
        }
    }

    /// The map from projected coordinates onto the unit square of the
    /// axes, putting the map's horizontal and vertical extent on the edges
    /// (matplotlib's `_get_affine_transform`).
    pub fn affine(self) -> Affine2D {
        if let Self::Lambert { .. } = self {
            return Affine2D::scaling(0.25, 0.25).translate(0.5, 0.5);
        }
        let x_scale = self.forward(Point::new(PI, 0.0)).x;
        let y_scale = self.forward(Point::new(0.0, FRAC_PI_2)).y;
        Affine2D::scaling(0.5 / x_scale, 0.5 / y_scale).translate(0.5, 0.5)
    }
}

/// numpy's `sinc`: `sin(πx) / (πx)`, with machine epsilon standing in for
/// a zero argument.
fn sinc(x: f64) -> f64 {
    let y = PI * x;
    let y = if y == 0.0 { f64::EPSILON } else { y };
    y.sin() / y
}

fn aitoff(lon: f64, lat: f64) -> Point {
    let half_long = lon / 2.0;
    let cos_latitude = lat.cos();
    let alpha = (cos_latitude * half_long.cos()).acos();
    let sinc_alpha = sinc(alpha / PI);
    Point::new(
        (cos_latitude * half_long.sin()) / sinc_alpha,
        lat.sin() / sinc_alpha,
    )
}

fn hammer(lon: f64, lat: f64) -> Point {
    let half_long = lon / 2.0;
    let cos_latitude = lat.cos();
    let alpha = (1.0 + cos_latitude * half_long.cos()).sqrt();
    Point::new(
        (2.0 * SQRT_2) * (cos_latitude * half_long.sin()) / alpha,
        (SQRT_2 * lat.sin()) / alpha,
    )
}

fn hammer_inverse(xy: Point) -> Point {
    let (x, y) = (xy.x, xy.y);
    let z = (1.0 - (x / 4.0).powi(2) - (y / 2.0).powi(2)).sqrt();
    let longitude = 2.0 * ((z * x) / (2.0 * (2.0 * z.powi(2) - 1.0))).atan();
    Point::new(longitude, (y * z).asin())
}

/// The Lambert projection of a point `diff_long` east of the centre's
/// meridian, with the centre at latitude `clat`.
fn lambert(diff_long: f64, lat: f64, clat: f64) -> Point {
    let (sin_lat, cos_lat) = (lat.sin(), lat.cos());
    let cos_diff_long = diff_long.cos();
    let inner = 1.0 + clat.sin() * sin_lat + clat.cos() * cos_lat * cos_diff_long;
    // numpy's `maximum(inner, 1e-15)` keeps NaN (`f64::max` would not).
    let inner_k = if inner < 1e-15 { 1e-15 } else { inner };
    let k = (2.0 / inner_k).sqrt();
    Point::new(
        k * cos_lat * diff_long.sin(),
        k * (clat.cos() * sin_lat - clat.sin() * cos_lat * cos_diff_long),
    )
}

fn lambert_inverse(xy: Point, clong: f64, clat: f64) -> Point {
    let (x, y) = (xy.x, xy.y);
    let distance = x.hypot(y);
    let p = if distance < 1e-9 { 1e-9 } else { distance };
    let c = 2.0 * (0.5 * p).asin();
    let (sin_c, cos_c) = (c.sin(), c.cos());
    let latitude = (cos_c * clat.sin() + ((y * sin_c * clat.cos()) / p)).asin();
    let longitude =
        clong + ((x * sin_c) / (p * clat.cos() * cos_c - y * clat.sin() * sin_c)).atan();
    Point::new(longitude, latitude)
}

/// The auxiliary angle θ of the Mollweide projection, solving
/// `2θ + sin 2θ = π sin φ`: Newton's method on `2θ`, or a series near the
/// poles.
fn mollweide_auxiliary(lat: f64) -> f64 {
    let clat = FRAC_PI_2 - lat.abs();
    if clat < MOLLWEIDE_POLAR_ZONE {
        let d = 0.5 * (3.0 * PI * clat.powi(2)).powf(1.0 / 3.0);
        return (FRAC_PI_2 - d) * numpy_sign(lat);
    }
    let pi_sin_l = PI * lat.sin();
    let step = |theta: f64| -(theta + theta.sin() - pi_sin_l) / (1.0 + theta.cos());
    let mut theta = 2.0 * lat;
    let mut delta = step(theta);
    for _ in 0..MOLLWEIDE_MAX_STEPS {
        if delta.abs() <= MOLLWEIDE_TOLERANCE || delta.is_nan() {
            break;
        }
        theta += delta;
        delta = step(theta);
    }
    theta / 2.0
}

/// numpy's `sign`: `-1`, `0` or `1`, NaN for NaN.
fn numpy_sign(value: f64) -> f64 {
    if value == 0.0 || value.is_nan() {
        value
    } else {
        value.signum()
    }
}

fn mollweide(lon: f64, lat: f64) -> Point {
    let aux = mollweide_auxiliary(lat);
    Point::new((2.0 * SQRT_2 / PI) * lon * aux.cos(), SQRT_2 * aux.sin())
}

fn mollweide_inverse(xy: Point) -> Point {
    let theta = (xy.y / SQRT_2).asin();
    let longitude = (PI / (2.0 * SQRT_2)) * xy.x / theta.cos();
    let latitude = ((2.0 * theta + (2.0 * theta).sin()) / PI).asin();
    Point::new(longitude, latitude)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point, tolerance: f64) -> bool {
        (a.x - b.x).abs() <= tolerance && (a.y - b.y).abs() <= tolerance
    }

    #[test]
    fn edges_land_on_the_unit_square() {
        for projection in [
            GeoProjection::Aitoff,
            GeoProjection::Hammer,
            GeoProjection::Mollweide,
        ] {
            let affine = projection.affine();
            let east = affine.transform_point(projection.forward(Point::new(PI, 0.0)));
            let north = affine.transform_point(projection.forward(Point::new(0.0, FRAC_PI_2)));
            assert!(close(east, Point::new(1.0, 0.5), 1e-12), "{projection:?}");
            assert!(close(north, Point::new(0.5, 1.0), 1e-12), "{projection:?}");
        }
        let lambert = GeoProjection::LAMBERT;
        let pole = lambert
            .affine()
            .transform_point(lambert.forward(Point::new(0.0, FRAC_PI_2)));
        assert!(close(pole, Point::new(0.5, 0.5 + 0.25 * SQRT_2), 1e-12));
    }

    #[test]
    fn known_values_match_matplotlib() {
        let p = Point::new(1.0, 0.5);
        // Values printed by matplotlib 3.11's transforms.
        let cases = [
            (
                GeoProjection::Aitoff,
                Point::new(0.456_259_622_749_587, 0.519_905_069_406_544),
            ),
            (
                GeoProjection::Hammer,
                Point::new(0.894_435_374_714_529_9, 0.509_601_838_935_732_2),
            ),
            (
                GeoProjection::Mollweide,
                Point::new(0.830_401_349_309_115_4, 0.546_409_282_153_612_3),
            ),
        ];
        for (projection, expected) in cases {
            assert!(
                close(projection.forward(p), expected, 1e-15),
                "{projection:?}: {:?}",
                projection.forward(p)
            );
        }
    }

    #[test]
    fn inverses_round_trip() {
        let p = Point::new(-2.0, 0.7);
        for projection in [
            GeoProjection::Hammer,
            GeoProjection::Mollweide,
            GeoProjection::Lambert {
                center_longitude: -1.0,
                center_latitude: 0.4,
            },
        ] {
            let back = projection.inverse(projection.forward(p));
            assert!(close(back, p, 1e-3), "{projection:?}: {back:?}");
        }
        assert!(GeoProjection::Aitoff.inverse(Point::ORIGIN).x.is_nan());
    }

    #[test]
    fn degenerate_inputs_stay_finite_or_nan() {
        // The antipode of the Lambert centre is clamped, not infinite.
        let antipode = GeoProjection::LAMBERT.forward(Point::new(PI, 0.0));
        assert!(antipode.is_finite());
        assert!(
            GeoProjection::LAMBERT
                .forward(Point::new(f64::NAN, 0.0))
                .x
                .is_nan()
        );
        let pole = GeoProjection::Mollweide.forward(Point::new(1.0, FRAC_PI_2));
        assert!(close(pole, Point::new(0.0, SQRT_2), 1e-12));
        assert!(
            GeoProjection::Mollweide
                .forward(Point::new(0.0, f64::NAN))
                .y
                .is_nan()
        );
        assert_eq!(GeoProjection::Aitoff.forward(Point::ORIGIN), Point::ORIGIN);
    }
}
