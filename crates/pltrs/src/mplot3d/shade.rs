//! Shading: faces lit by a light source and scatter markers faded with
//! depth (matplotlib's `LightSource`, `_generate_normals`,
//! `_shade_colors` and `_zalpha`).

use pltrs_core::Rgba;

use super::Axes3DError;
use super::math::{Vec3, cross, dot, norm, sub};
pub use crate::colors::LightSource;

/// A normal of a planar polygon from three of its vertices spread around
/// it; it points towards the viewer when the vertices run
/// counter-clockwise (matplotlib's `_generate_normals`).
pub(crate) fn polygon_normal(polygon: &[Vec3]) -> Vec3 {
    let n = polygon.len();
    if n == 0 {
        return [f64::NAN; 3];
    }
    let (p1, p2, p3) = (polygon[0], polygon[n / 3], polygon[2 * n / 3]);
    cross(sub(p1, p2), sub(p2, p3))
}

/// `colors` (one, or one per face) darkened where their face, a planar
/// polygon, turns away from `light`, keeping their alpha (matplotlib's
/// `_shade_colors(colors, _generate_normals(faces), lightsource)`, which
/// a shaded `Poly3DCollection` applies to the colours it is given).
///
/// ```
/// use pltrs::colors::Rgba;
/// use pltrs::mplot3d::{LightSource, shade_faces};
/// let up = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]];
/// let down: Vec<_> = up.iter().rev().copied().collect();
/// let light = LightSource::new(0.0, 90.0);
/// let shaded = shade_faces(&[up, down], &[Rgba::opaque(1.0, 1.0, 1.0)], &light)?;
/// assert!((shaded[0].r - 1.0).abs() < 1e-12 && (shaded[1].r - 0.3).abs() < 1e-12);
/// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
/// ```
///
/// # Errors
///
/// [`Axes3DError::ShadeColors`] for neither one colour nor one per face.
pub fn shade_faces(
    faces: &[Vec<Vec3>],
    colors: &[Rgba],
    light: &LightSource,
) -> Result<Vec<Rgba>, Axes3DError> {
    let normals: Vec<Vec3> = faces.iter().map(|face| polygon_normal(face)).collect();
    shade_colors(colors, &normals, light)
}

/// `colors` (one, or one per normal) darkened where their face turns away
/// from the light, keeping their alpha (matplotlib's `_shade_colors`).
///
/// # Errors
///
/// [`Axes3DError::ShadeColors`] for neither one colour nor one per
/// normal (when some normal is defined: otherwise the colours stay as
/// they are, as in matplotlib).
pub(crate) fn shade_colors(
    colors: &[Rgba],
    normals: &[Vec3],
    light: &LightSource,
) -> Result<Vec<Rgba>, Axes3DError> {
    let direction = light.direction();
    let shades: Vec<f64> = normals
        .iter()
        .map(|&n| {
            let length = norm(n);
            dot([n[0] / length, n[1] / length, n[2] / length], direction)
        })
        .collect();
    if shades.iter().all(|s| s.is_nan()) || colors.is_empty() {
        return Ok(colors.to_vec());
    }
    if colors.len() != 1 && colors.len() != normals.len() {
        return Err(Axes3DError::ShadeColors {
            faces: normals.len(),
            colors: colors.len(),
        });
    }
    // Normalize(-1, 1) followed by Normalize(0.3, 1).inverse.
    let (vmin, vmax) = (0.3, 1.0);
    Ok(shades
        .iter()
        .enumerate()
        .map(|(i, &shade)| {
            let shade = if shade.is_nan() { 0.0 } else { shade };
            let fraction = (shade - -1.0) / 2.0;
            let factor = vmin + fraction * (vmax - vmin);
            let color = colors[if colors.len() == 1 { 0 } else { i }];
            Rgba::new(
                factor * color.r,
                factor * color.g,
                factor * color.b,
                color.a,
            )
        })
        .collect())
}

/// The alpha factor of each point at projected depth `zs`: 1 for the
/// nearest, fading to `min_alpha` over `data_scale` (matplotlib's
/// `_zalpha`).
pub(crate) fn depth_alphas(zs: &[f64], min_alpha: f64, data_scale: f64) -> Vec<f64> {
    let min_alpha = min_alpha.clamp(0.0, 1.0);
    if data_scale == 0.0 {
        return vec![1.0; zs.len()];
    }
    let nearest = zs
        .iter()
        .copied()
        .filter(|z| !z.is_nan())
        .fold(f64::INFINITY, f64::min);
    zs.iter()
        .map(|z| (1.0 - (z - nearest) / data_scale).clamp(min_alpha, 1.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_direction_matches_matplotlib() {
        let d = LightSource::default().direction();
        let expected = [-0.5, 0.5, std::f64::consts::FRAC_1_SQRT_2];
        for (got, want) in d.iter().zip(expected) {
            assert!((got - want).abs() < 1e-12);
        }
    }

    #[test]
    fn normals_follow_the_right_hand_rule() {
        let square = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let n = polygon_normal(&square);
        assert!(n[2] > 0.0 && n[0] == 0.0 && n[1] == 0.0);
        assert!(polygon_normal(&[]).iter().all(|c| c.is_nan()));
    }

    #[test]
    fn faces_towards_the_light_stay_bright() {
        let light = LightSource::mplot3d();
        let toward = light.direction();
        let away = [-toward[0], -toward[1], -toward[2]];
        let red = Rgba::new(1.0, 0.0, 0.0, 0.5);
        let shaded = shade_colors(&[red], &[toward, away, [0.0; 3]], &light).expect("one colour");
        assert!((shaded[0].r - 1.0).abs() < 1e-12);
        assert!((shaded[1].r - 0.3).abs() < 1e-12);
        assert!(
            (shaded[2].r - 0.65).abs() < 1e-12,
            "degenerate faces are half lit"
        );
        assert!(shaded.iter().all(|c| (c.a - 0.5).abs() < 1e-12));
    }

    #[test]
    fn colours_neither_one_nor_one_per_face_fail_as_numpy_broadcasting() {
        // Found by fuzzing: six colours for the eighteen faces of three
        // bars indexed past them (matplotlib's `ValueError`).
        let light = LightSource::mplot3d();
        let up = light.direction();
        let red = Rgba::new(1.0, 0.0, 0.0, 1.0);
        let err = shade_colors(&[red; 6], &[up; 18], &light).expect_err("6 for 18");
        assert_eq!(
            err.to_string(),
            "operands could not be broadcast together with shapes (18,1) (6,4) "
        );
        assert!(shade_colors(&[red; 18], &[up; 18], &light).is_ok());
    }

    #[test]
    fn depth_fades_alpha_down_to_the_minimum() {
        assert_eq!(depth_alphas(&[0.0, 1.0, 10.0], 0.3, 2.0), [1.0, 0.5, 0.3]);
        assert_eq!(depth_alphas(&[0.0, 1.0], 0.3, 0.0), [1.0, 1.0]);
    }
}
