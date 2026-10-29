//! How a surface's buffer pixels relate to its size in surface
//! coordinates, and where the scale comes from.
//!
//! With `wp_fractional_scale_v1` and `wp_viewporter`, buffers are
//! `round(logical × scale)` pixels, shown at the logical size through a
//! viewport. Otherwise the surface's buffer scale is an integer: the
//! compositor's preference (`wl_surface.preferred_buffer_scale`), else
//! the largest scale of the outputs the surface is on.

use crate::geometry::{PhysicalPosition, PhysicalSize};

/// The denominator of fractional scales.
const FRACTION: u32 = 120;

/// A surface scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scale {
    /// Buffers scaled by `n / 120` through a viewport.
    Fractional(u32),
    /// Buffers `n` times the logical size (`wl_surface.set_buffer_scale`).
    Integer(u32),
}

impl Scale {
    /// The device pixel ratio.
    pub(crate) fn factor(self) -> f64 {
        match self {
            Self::Fractional(n) => f64::from(n) / f64::from(FRACTION),
            Self::Integer(n) => f64::from(n),
        }
    }

    /// The buffer length of `logical` surface units (fractional scales
    /// round half away from zero, as `fractional-scale-v1` advises).
    pub(crate) fn to_physical_len(self, logical: u32) -> u32 {
        let physical = match self {
            Self::Fractional(n) => {
                (u64::from(logical) * u64::from(n) + u64::from(FRACTION / 2)) / u64::from(FRACTION)
            }
            Self::Integer(n) => u64::from(logical) * u64::from(n),
        };
        u32::try_from(physical).unwrap_or(u32::MAX)
    }

    /// The buffer size of a `logical` surface size.
    pub(crate) fn to_physical(self, (width, height): (u32, u32)) -> PhysicalSize {
        PhysicalSize::new(self.to_physical_len(width), self.to_physical_len(height))
    }

    /// The surface length closest to `physical` pixels (at least 1).
    pub(crate) fn to_logical_len(self, physical: u32) -> u32 {
        ((f64::from(physical) / self.factor()).round() as u32).max(1)
    }

    /// A position in surface coordinates, in buffer pixels.
    pub(crate) fn position(self, x: f64, y: f64) -> PhysicalPosition {
        let factor = self.factor();
        PhysicalPosition::new(x * factor, y * factor)
    }
}

/// What decides a surface's scale.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ScaleSources {
    /// `wp_fractional_scale_v1.preferred_scale`, in 120ths.
    pub(crate) fractional: Option<u32>,
    /// `wl_surface.preferred_buffer_scale`.
    pub(crate) preferred: Option<u32>,
    /// The outputs the surface is on.
    pub(crate) outputs: Vec<u32>,
}

impl ScaleSources {
    /// The scale to draw at: fractional when the compositor supports it
    /// (`fractional`), else an integer. `output_scale` gives the scale of
    /// an output; `fallback`, the largest output scale, serves surfaces
    /// not yet shown on any output.
    pub(crate) fn scale(
        &self,
        fractional: bool,
        output_scale: impl Fn(u32) -> Option<u32>,
        fallback: u32,
    ) -> Scale {
        let integer = self
            .preferred
            .or_else(|| self.outputs.iter().filter_map(|&id| output_scale(id)).max())
            .unwrap_or(fallback)
            .clamp(1, 16);
        match self.fractional {
            Some(n) if fractional && n > 0 => Scale::Fractional(n.min(16 * FRACTION)),
            _ if fractional => Scale::Fractional(integer * FRACTION),
            _ => Scale::Integer(integer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_scales_round_half_away_from_zero() {
        let scale = Scale::Fractional(180);
        assert!((scale.factor() - 1.5).abs() < f64::EPSILON);
        assert_eq!(scale.to_physical((101, 2)), PhysicalSize::new(152, 3));
        assert_eq!(scale.to_logical_len(152), 101);
        assert_eq!(scale.to_logical_len(0), 1);
        assert_eq!(scale.position(2.0, 4.0), PhysicalPosition::new(3.0, 6.0));
        assert_eq!(Scale::Fractional(150).to_physical_len(3), 4, "3.75");
    }

    #[test]
    fn integer_scales_multiply() {
        let scale = Scale::Integer(2);
        assert_eq!(scale.to_physical((320, 200)), PhysicalSize::new(640, 400));
        assert_eq!(scale.to_logical_len(641), 321);
        assert_eq!(Scale::Integer(16).to_physical_len(u32::MAX), u32::MAX);
    }

    #[test]
    fn sources_choose_the_scale() {
        let outputs = |id: u32| (id < 10).then_some(id);
        let mut sources = ScaleSources::default();
        assert_eq!(sources.scale(false, outputs, 2), Scale::Integer(2));
        assert_eq!(sources.scale(true, outputs, 1), Scale::Fractional(120));
        sources.outputs = vec![1, 3, 77];
        assert_eq!(sources.scale(false, outputs, 1), Scale::Integer(3));
        sources.preferred = Some(2);
        assert_eq!(sources.scale(false, outputs, 1), Scale::Integer(2));
        sources.fractional = Some(150);
        assert_eq!(sources.scale(true, outputs, 1), Scale::Fractional(150));
        assert_eq!(sources.scale(false, outputs, 1), Scale::Integer(2));
        sources.preferred = Some(1000);
        sources.fractional = Some(0);
        assert_eq!(
            sources.scale(false, outputs, 1),
            Scale::Integer(16),
            "absurd scales are capped"
        );
    }
}
