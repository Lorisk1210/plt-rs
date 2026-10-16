//! Straight-alpha RGBA colours.

/// A colour with straight (non-premultiplied) alpha; components in `[0, 1]`.
///
/// This is the resolved form of every matplotlib colour specification;
/// parsing names, hex strings and cycle references lives in `plt::colors`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rgba {
    /// Red.
    pub r: f64,
    /// Green.
    pub g: f64,
    /// Blue.
    pub b: f64,
    /// Opacity: 0 is fully transparent, 1 fully opaque.
    pub a: f64,
}

impl Rgba {
    /// Opaque black.
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    /// Opaque white.
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    /// Fully transparent black (matplotlib's `"none"`).
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    /// Creates a colour from its components.
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    /// Creates an opaque colour.
    pub const fn opaque(r: f64, g: f64, b: f64) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Creates a colour from 8-bit channels.
    pub fn from_rgba8([r, g, b, a]: [u8; 4]) -> Self {
        let unit = |v: u8| f64::from(v) / 255.0;
        Self::new(unit(r), unit(g), unit(b), unit(a))
    }

    /// The same colour with a different alpha.
    #[must_use]
    pub const fn with_alpha(self, a: f64) -> Self {
        Self { a, ..self }
    }

    /// The colour with its alpha multiplied by `factor`.
    #[must_use]
    pub fn fade(self, factor: f64) -> Self {
        self.with_alpha(self.a * factor)
    }

    /// Returns `true` when the colour is fully transparent.
    pub fn is_transparent(self) -> bool {
        self.a <= 0.0
    }

    /// The `[r, g, b, a]` components.
    pub const fn to_array(self) -> [f64; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Quantises to 8-bit channels, rounding half up like Anti-Grain
    /// Geometry's `rgba8` conversion. Out-of-range input is clamped.
    pub fn to_rgba8(self) -> [u8; 4] {
        let q = |v: f64| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        [q(self.r), q(self.g), q(self.b), q(self.a)]
    }

    /// Relative luminance (ITU-R BT.709 on the encoded components), handy
    /// for choosing contrasting text colours.
    pub fn luminance(self) -> f64 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }
}

impl From<[f64; 4]> for Rgba {
    fn from([r, g, b, a]: [f64; 4]) -> Self {
        Self::new(r, g, b, a)
    }
}

impl From<[f64; 3]> for Rgba {
    fn from([r, g, b]: [f64; 3]) -> Self {
        Self::opaque(r, g, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantisation_rounds_half_up_and_clamps() {
        assert_eq!(Rgba::new(0.5, 1.0, 0.0, 1.0).to_rgba8(), [128, 255, 0, 255]);
        assert_eq!(Rgba::new(-1.0, 2.0, 0.2, 0.0).to_rgba8(), [0, 255, 51, 0]);
    }

    #[test]
    fn round_trip_8bit() {
        let c = [12, 34, 56, 78];
        assert_eq!(Rgba::from_rgba8(c).to_rgba8(), c);
    }

    #[test]
    fn alpha_helpers() {
        let c = Rgba::opaque(0.1, 0.2, 0.3);
        assert_eq!(c.with_alpha(0.5).a, 0.5);
        assert_eq!(c.fade(0.5).fade(0.5).a, 0.25);
        assert!(Rgba::TRANSPARENT.is_transparent());
    }
}
