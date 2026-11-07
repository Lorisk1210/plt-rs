//! The `FontMatrix` of CFF Top and Font DICTs (`cff_parse_font_matrix`)
//! and FreeType's normalization of it (`cff_face_init`): the vertical
//! scale becomes one unit per em, its size moves into the units per em,
//! and a Font DICT's matrix is concatenated with the Top DICT's.

use super::dict::{Operand, power_ten};
use super::index::CFF;
use crate::error::{FontError, Result};
use crate::fixed::div_fix;
use crate::matrix::{FontTransform, matrix_is_usable};

/// A DICT's font matrix: as parsed (offset in 16.16), then normalized
/// (offset in font units).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DictMatrix {
    pub(super) transform: FontTransform,
    pub(super) units_per_em: i64,
    /// Whether the DICT has a `FontMatrix`.
    pub(super) given: bool,
}

impl DictMatrix {
    /// A DICT without `FontMatrix` in a font of `units_per_em`.
    pub(super) const fn absent(units_per_em: i64) -> Self {
        Self {
            transform: FontTransform::IDENTITY,
            units_per_em,
            given: false,
        }
    }

    /// The matrix of a `FontMatrix` entry: the six values scaled by the
    /// largest power of ten among them, which gives the units per em; a
    /// matrix FreeType finds unlikely is the identity of one unit per em.
    pub(super) fn parse(operands: &[Operand<'_>]) -> Result<Self> {
        let operands = operands
            .get(..6)
            .ok_or(FontError::malformed(CFF, "FontMatrix needs six values"))?;
        let unlikely = Self {
            transform: FontTransform::IDENTITY,
            units_per_em: 1,
            given: true,
        };
        let parsed: Vec<(i64, i64)> = operands.iter().map(|op| op.fixed_dynamic()).collect();
        let scalings = parsed
            .iter()
            .filter(|(value, _)| *value != 0)
            .map(|&(_, scaling)| scaling);
        let (Some(max), Some(min)) = (scalings.clone().max(), scalings.min()) else {
            return Ok(unlikely);
        };
        if !(-9..=0).contains(&max) || !(0..=9).contains(&(max - min)) {
            return Ok(unlikely);
        }
        let values: Vec<i64> = parsed
            .iter()
            .map(|&(value, scaling)| {
                if value == 0 {
                    return 0;
                }
                let divisor = power_ten(max - scaling);
                let half = divisor >> 1;
                if value < 0 {
                    (value - half) / divisor
                } else {
                    (value + half) / divisor
                }
            })
            .collect();
        let transform = FontTransform {
            xx: values[0],
            yx: values[1],
            xy: values[2],
            yy: values[3],
            offset: (values[4], values[5]),
        };
        if !matrix_is_usable(transform.xx, transform.xy, transform.yx, transform.yy) {
            return Ok(unlikely);
        }
        Ok(Self {
            transform,
            units_per_em: power_ten(-max),
            given: true,
        })
    }

    /// FreeType's normalization: the matrix divided by its vertical scale
    /// (`yy`, or `yx` when that is zero), the units per em with it, and the
    /// offset to whole font units.
    #[must_use]
    pub(super) fn normalized(self) -> Self {
        let mut normal = self;
        let t = &mut normal.transform;
        let scale = if t.yy == 0 { t.yx.abs() } else { t.yy.abs() };
        if scale != 0x10000 {
            normal.units_per_em = div_fix(normal.units_per_em, scale);
            for value in [&mut t.xx, &mut t.yx, &mut t.xy, &mut t.yy] {
                *value = div_fix(*value, scale);
            }
            t.offset = (div_fix(t.offset.0, scale), div_fix(t.offset.1, scale));
        }
        t.offset = (t.offset.0 >> 16, t.offset.1 >> 16);
        normal
    }

    /// A Font DICT's matrix, from its own (`self`, as parsed) and the Top
    /// DICT's (`top`, normalized), normalized: its own is concatenated
    /// with the Top DICT's, and a Font DICT without one takes the Top
    /// DICT's (whose offset FreeType shifts once more).
    #[must_use]
    pub(super) fn under(self, top: &Self) -> Self {
        let combined = match (self.given, top.given) {
            (true, true) => {
                let scaling = if top.units_per_em > 1 && self.units_per_em > 1 {
                    top.units_per_em.min(self.units_per_em)
                } else {
                    1
                };
                let mut transform = top.transform.multiply_scaled(&self.transform, scaling);
                transform.offset = top
                    .transform
                    .transform_scaled(self.transform.offset, scaling);
                Self {
                    transform,
                    units_per_em: crate::fixed::mul_div(
                        self.units_per_em,
                        top.units_per_em,
                        scaling,
                    ),
                    given: true,
                }
            }
            (true, false) => self,
            (false, _) => Self {
                given: false,
                ..*top
            },
        };
        combined.normalized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The operands of `[0.001 0 0 0.001 0 0]`.
    fn thousandth() -> Vec<Operand<'static>> {
        const MILLI: &[u8] = &[0x0A, 0x00, 0x1F];
        vec![
            Operand::Real(MILLI),
            Operand::Int(0),
            Operand::Int(0),
            Operand::Real(MILLI),
            Operand::Int(0),
            Operand::Int(0),
        ]
    }

    #[test]
    fn usual_matrix_is_the_identity_of_a_thousand_units() {
        let matrix = DictMatrix::parse(&thousandth()).unwrap().normalized();
        assert_eq!(matrix.transform, FontTransform::IDENTITY);
        assert_eq!(matrix.units_per_em, 1000);
        assert!(DictMatrix::parse(&thousandth()[..5]).is_err());
        let zeros = [Operand::Int(0); 6];
        let unlikely = DictMatrix::parse(&zeros).unwrap();
        assert_eq!((unlikely.units_per_em, unlikely.given), (1, true));
    }

    #[test]
    fn font_dicts_inherit_or_combine() {
        let top = DictMatrix::parse(&thousandth()).unwrap().normalized();
        let inherited = DictMatrix::absent(2048).under(&top);
        assert_eq!((inherited.units_per_em, inherited.given), (1000, false));
        // The usual CID-keyed font: an identity Top DICT matrix, Font
        // DICTs of a thousandth.
        let mut identity = thousandth();
        identity[0] = Operand::Int(1);
        identity[3] = Operand::Int(1);
        let unit = DictMatrix::parse(&identity).unwrap().normalized();
        assert_eq!(
            (unit.transform, unit.units_per_em),
            (FontTransform::IDENTITY, 1)
        );
        let own = DictMatrix::parse(&thousandth()).unwrap();
        let combined = own.under(&unit);
        assert_eq!(combined.transform, FontTransform::IDENTITY);
        assert_eq!(combined.units_per_em, 1000);
        // Two thousandths make a millionth, within FreeType's rounding.
        let twice = own.under(&top);
        assert_eq!(twice.transform.xx, 0x10000);
        assert_eq!(twice.units_per_em, 992_970);
        let half = DictMatrix {
            transform: FontTransform {
                xx: 0x8000,
                yy: 0x8000,
                ..FontTransform::IDENTITY
            },
            units_per_em: 1000,
            given: true,
        };
        assert_eq!(half.normalized().units_per_em, 2000);
        assert_eq!(DictMatrix::absent(2048).normalized().units_per_em, 2048);
    }
}
