//! Anchor positions for placing a box inside a container.

/// Where to place a shrunken box inside its container (see
/// [`Bbox::anchored`](super::Bbox::anchored)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    /// Centre.
    C,
    /// Lower left.
    SW,
    /// Bottom centre.
    S,
    /// Lower right.
    SE,
    /// Right centre.
    E,
    /// Upper right.
    NE,
    /// Top centre.
    N,
    /// Upper left.
    NW,
    /// Left centre.
    W,
    /// Fractional position `(cx, cy)` with `(0, 0)` = lower left.
    Fraction(f64, f64),
}

impl Anchor {
    /// The fractional `(cx, cy)` position this anchor stands for
    /// (matplotlib's `Bbox.coefs`).
    pub const fn coefficients(self) -> (f64, f64) {
        match self {
            Self::C => (0.5, 0.5),
            Self::SW => (0.0, 0.0),
            Self::S => (0.5, 0.0),
            Self::SE => (1.0, 0.0),
            Self::E => (1.0, 0.5),
            Self::NE => (1.0, 1.0),
            Self::N => (0.5, 1.0),
            Self::NW => (0.0, 1.0),
            Self::W => (0.0, 0.5),
            Self::Fraction(cx, cy) => (cx, cy),
        }
    }

    /// Parses matplotlib's anchor codes (`"C"`, `"SW"`, ..., `"W"`); the
    /// codes are case-sensitive.
    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "C" => Self::C,
            "SW" => Self::SW,
            "S" => Self::S,
            "SE" => Self::SE,
            "E" => Self::E,
            "NE" => Self::NE,
            "N" => Self::N,
            "NW" => Self::NW,
            "W" => Self::W,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip_to_coefficients() {
        assert_eq!(
            Anchor::from_code("NE").map(Anchor::coefficients),
            Some((1.0, 1.0))
        );
        assert_eq!(
            Anchor::from_code("W").map(Anchor::coefficients),
            Some((0.0, 0.5))
        );
        assert_eq!(Anchor::Fraction(0.25, 2.0).coefficients(), (0.25, 2.0));
    }

    #[test]
    fn unknown_and_lowercase_codes_are_rejected() {
        for code in ["X", "ne", "c", ""] {
            assert_eq!(Anchor::from_code(code), None, "{code:?}");
        }
    }
}
