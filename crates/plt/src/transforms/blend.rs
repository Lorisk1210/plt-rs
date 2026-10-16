//! Blending two affine transforms (matplotlib's `BlendedAffine2D`).

use plt_core::Affine2D;

use super::TransformError;

/// x from `x_transform` and y from `y_transform`, for two affine
/// transforms (matplotlib's `blended_transform_factory` on `Affine2D`s).
///
/// The blend is only meaningful when each child maps its axis
/// independently, so both must be separable (no rotation or shear);
/// otherwise this fails with [`TransformError::NotSeparable`].
pub fn blended_affine(
    x_transform: &Affine2D,
    y_transform: &Affine2D,
) -> Result<Affine2D, TransformError> {
    if x_transform.is_separable() && y_transform.is_separable() {
        Ok(blend_rows(x_transform, y_transform))
    } else {
        Err(TransformError::NotSeparable)
    }
}

/// The matrix with the first row of `x` and the second row of `y`.
pub(super) fn blend_rows(x: &Affine2D, y: &Affine2D) -> Affine2D {
    let [[xa, xc, xe], _, _] = x.matrix();
    let [_, [yb, yd, yf], _] = y.matrix();
    Affine2D::from_rows([[xa, xc, xe], [yb, yd, yf]])
}

#[cfg(test)]
mod tests {
    use plt_core::Point;

    use super::*;

    #[test]
    fn takes_x_row_and_y_row() {
        let x = Affine2D::scaling(2.0, 3.0).translate(1.0, 4.0);
        let y = Affine2D::scaling(5.0, 7.0).translate(-1.0, -2.0);
        let blended = blended_affine(&x, &y).expect("separable");
        assert_eq!(
            blended,
            Affine2D::from_values(2.0, 0.0, 0.0, 7.0, 1.0, -2.0)
        );
        assert_eq!(
            blended.transform_point(Point::new(1.0, 2.0)),
            Point::new(3.0, 12.0)
        );
    }

    #[test]
    fn identical_children_blend_to_themselves() {
        let t = Affine2D::scaling(2.0, 3.0);
        assert_eq!(blended_affine(&t, &t), Ok(t));
    }

    #[test]
    fn rotation_or_shear_is_rejected() {
        let rotated = Affine2D::identity().rotate_deg(90.0);
        let sheared = Affine2D::identity().skew(0.1, 0.0);
        let plain = Affine2D::IDENTITY;
        assert_eq!(
            blended_affine(&rotated, &plain),
            Err(TransformError::NotSeparable)
        );
        assert_eq!(
            blended_affine(&plain, &sheared),
            Err(TransformError::NotSeparable)
        );
        // rotate(0) leaves exact zeros off the diagonal.
        let unrotated = Affine2D::identity().rotate(0.0);
        assert!(blended_affine(&unrotated, &plain).is_ok());
    }
}
