//! Offsets measured in physical units (matplotlib's `ScaledTranslation`
//! and the units of `offset_copy`).

use pltrs_core::{Affine2D, Point};

use super::ResolvedTransform;

/// The unit of an [`offset_copy`](super::Transform::offset_copy)
/// displacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffsetUnits {
    /// Display pixels; needs no figure.
    Dots,
    /// Typographic points (1/72 inch), scaled by the figure's dpi.
    Points,
    /// Inches, scaled by the figure's dpi.
    Inches,
}

impl OffsetUnits {
    /// Parses matplotlib's unit names `"dots"`, `"points"` and `"inches"`.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "dots" => Some(Self::Dots),
            "points" => Some(Self::Points),
            "inches" => Some(Self::Inches),
            _ => None,
        }
    }

    /// Whether the displacement depends on the figure's resolution
    /// (matplotlib's `offset_copy` requires a figure for these units).
    pub const fn needs_figure(self) -> bool {
        !matches!(self, Self::Dots)
    }
}

/// matplotlib's `ScaledTranslation(xt, yt, scale_trans)`: a translation by
/// `scale(xt, yt)`.
///
/// The offset goes through `scale` as a *point*, not a vector, so any
/// translation in `scale` adds to it (for the usual `dpi_scale_trans`,
/// a pure scaling, the two agree).
pub fn scaled_translation(xt: f64, yt: f64, scale: &ResolvedTransform) -> Affine2D {
    let offset = scale.transform_point(Point::new(xt, yt));
    Affine2D::translation(offset.x, offset.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_names() {
        assert_eq!(OffsetUnits::from_name("points"), Some(OffsetUnits::Points));
        assert_eq!(OffsetUnits::from_name("cm"), None);
        assert!(!OffsetUnits::Dots.needs_figure());
        assert!(OffsetUnits::Inches.needs_figure() && OffsetUnits::Points.needs_figure());
    }

    #[test]
    fn offset_is_scaled_as_a_point() {
        let dpi = ResolvedTransform::affine(Affine2D::scaling(100.0, 100.0));
        assert_eq!(
            scaled_translation(0.1, 0.2, &dpi),
            Affine2D::translation(100.0 * 0.1, 100.0 * 0.2)
        );
        let shifted =
            ResolvedTransform::affine(Affine2D::scaling(100.0, 100.0).translate(5.0, 7.0));
        assert_eq!(
            scaled_translation(1.0, 1.0, &shifted),
            Affine2D::translation(105.0, 107.0)
        );
    }

    #[test]
    fn infinite_offsets_follow_ieee_arithmetic() {
        let dpi = ResolvedTransform::affine(Affine2D::scaling(72.0, 72.0));
        let [.., e, f] = scaled_translation(f64::INFINITY, 0.0, &dpi).values();
        assert_eq!(e, f64::INFINITY);
        assert!(f.is_nan(), "0 * inf in the y row");
    }
}
