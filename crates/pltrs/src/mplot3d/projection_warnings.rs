//! The runtime warnings numpy issues where matplotlib projects the data of
//! a 3-D artist as it draws it: `np.dot(M, vec)` and the division by the
//! homogeneous coordinate in `proj3d`'s transforms, each reporting the
//! floating-point exceptions it raised once (`invalid value encountered
//! in dot` for an infinite coordinate multiplied by a zero of `M`, `invalid
//! value encountered in divide` for an infinite one over another).
//!
//! matplotlib's line collections, path and patch collections and
//! converted collections (`Line3DCollection`, `Path3DCollection`,
//! `Patch3DCollection`, `Collection3D`) project their data as given, the
//! points their scales cannot show included (a mask does not reach
//! `np.dot`); its lines, texts and patches project those points as NaN,
//! which raises nothing, and its polygon collections project theirs with
//! the exceptions silenced, but not the sort position they are given
//! (`set_sort_zpos`). pltrs (Rust) projects with its own arithmetic, many
//! times per draw; the warnings are issued once per projection matplotlib
//! computes, where the axes are drawn and where a program projects an
//! artist (`do_3d_projection`).

use super::art3d::Art3D;
use super::camera::Projection;
use super::math::{Matrix4, Vec3};
use crate::numeric::numpy_arith::Exceptions;

/// Issues numpy's warnings for projecting `points` (in transformed
/// coordinates) through `matrix` all at once: `np.dot(M, vec)` of their
/// homogeneous coordinates (where numpy's products warn at all:
/// [`products_warn`](crate::warnings::products_warn)), then the division
/// of each projected point by its last coordinate
/// (`proj3d._proj_transform_vec` and its kin).
pub(crate) fn warn_projecting(matrix: &Matrix4, points: &[Vec3]) {
    if points.is_empty() {
        return;
    }
    let (mut dot, mut division) = (Exceptions::default(), Exceptions::default());
    for &[x, y, z] in points {
        let v = [x, y, z, 1.0];
        let projected = matrix.rows().map(|row| {
            (1..4).fold(dot.multiply_add(row[0], v[0], None), |sum, k| {
                dot.multiply_add(row[k], v[k], Some(sum))
            })
        });
        for &coordinate in &projected[..3] {
            division.divide(coordinate, projected[3]);
        }
    }
    if crate::warnings::products_warn(crate::warnings::Product::Dot) {
        dot.warn("dot");
    }
    division.warn("divide");
}

/// Issues numpy's warnings for matplotlib's move of a point of an axis
/// away from the box's centre (`_move_from_center` with the axis masked:
/// its own coordinate moves by `0 × delta`, NaN for an infinite `delta`,
/// a label pad of inf) and for projecting the point it gives
/// (`proj3d.proj_transform`), as an axis places its offset text and its
/// label. `moved` is the point moved along the other axes, with its own
/// coordinate `index` as it was.
pub(crate) fn warn_moving(matrix: &Matrix4, moved: Vec3, index: usize, delta: f64) {
    let mut product = Exceptions::default();
    let own = product.multiply_add(0.0, delta, None);
    product.warn("multiply");
    let mut moved = moved;
    moved[index] += own;
    warn_projecting(matrix, &[moved]);
}

impl Art3D {
    /// Issues numpy's warnings for the projections matplotlib computes of
    /// the geometry as it projects it through `projection` (see the module
    /// documentation).
    pub(crate) fn warn_projecting(&self, projection: &Projection) {
        let given = |points: &[Vec3]| -> Vec<Vec3> {
            points
                .iter()
                .map(|&p| projection.scaled_as_given(p))
                .collect()
        };
        match self {
            Self::Paths(paths) => {
                for path in paths {
                    warn_projecting(&projection.matrix, &given(&path.vertices));
                }
            }
            Self::Segments(segments) => {
                warn_projecting(&projection.matrix, &given(&segments.lines.concat()));
            }
            Self::Points(points) => {
                warn_projecting(&projection.matrix, &given(&points.positions));
            }
            Self::Polys(polys) => {
                if let Some(z) = polys.sort_zpos {
                    warn_projecting(&projection.matrix, &[[0.0, 0.0, z]]);
                }
            }
            Self::Line(_)
            | Self::Text { .. }
            | Self::Patch(_)
            | Self::PathPatch(_)
            | Self::Program { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::warnings::capturing_runtime;

    /// A projection with zeros where matplotlib's level views have them
    /// (the screen's x takes no z), and a last row making `w = 1 - z`.
    fn level() -> Matrix4 {
        Matrix4::from_rows([
            [1.0, 0.5, 0.0, 0.1],
            [0.2, 1.0, 1.0, 0.0],
            [0.3, 0.4, 1.0, 0.0],
            [0.0, 0.0, -1.0, 1.0],
        ])
    }

    fn warned(points: &[Vec3]) -> Vec<String> {
        capturing_runtime(|| warn_projecting(&level(), points)).1
    }

    #[test]
    fn finite_points_and_none_warn_nothing() {
        assert_eq!(
            warned(&[[1.0, 2.0, 3.0], [0.0, 0.0, 0.0]]),
            Vec::<String>::new()
        );
        assert_eq!(warned(&[]), Vec::<String>::new());
    }

    /// `warnings`, without those of products where numpy's products warn
    /// of nothing.
    fn as_numpy_warns<'a>(warnings: &[&'a str]) -> Vec<&'a str> {
        warnings
            .iter()
            .copied()
            .filter(|warning| {
                crate::warnings::products_warn(crate::warnings::Product::Dot)
                    || !warning.ends_with(" in dot")
            })
            .collect()
    }

    #[test]
    fn an_infinite_coordinate_times_a_zero_is_invalid_once_per_call() {
        assert_eq!(
            warned(&[[0.0, 0.0, f64::INFINITY], [1.0, 1.0, f64::INFINITY]]),
            as_numpy_warns(&[
                "invalid value encountered in dot",
                "invalid value encountered in divide"
            ])
        );
    }

    #[test]
    fn an_infinite_coordinate_meeting_no_zero_is_invalid_in_divide_only() {
        let dense = Matrix4::from_rows([[1.0; 4], [2.0; 4], [3.0; 4], [1.0, 1.0, 1.0, 2.0]]);
        let ((), warned) =
            capturing_runtime(|| warn_projecting(&dense, &[[f64::INFINITY, 0.0, 0.0]]));
        assert_eq!(warned, ["invalid value encountered in divide"]);
    }

    #[test]
    fn nan_raises_nothing() {
        assert_eq!(warned(&[[f64::NAN, 1.0, f64::NAN]]), Vec::<String>::new());
    }

    #[test]
    fn a_zero_last_coordinate_divides_by_zero() {
        assert_eq!(
            warned(&[[1.0, 1.0, 1.0]]),
            ["divide by zero encountered in divide"]
        );
    }

    #[test]
    fn a_point_moved_by_an_infinite_pad_is_invalid_in_multiply_and_dot() {
        let ((), warned) = capturing_runtime(|| {
            warn_moving(
                &level(),
                [f64::INFINITY, -f64::INFINITY, 0.5],
                2,
                f64::INFINITY,
            );
        });
        assert_eq!(
            warned,
            as_numpy_warns(&[
                "invalid value encountered in multiply",
                "invalid value encountered in dot"
            ])
        );
        let ((), warned) = capturing_runtime(|| warn_moving(&level(), [0.1, 0.2, 0.5], 2, 0.3));
        assert_eq!(warned, Vec::<String>::new());
    }
}
