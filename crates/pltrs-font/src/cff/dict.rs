//! CFF and CFF2 DICT data as FreeType's parser reads it (`cffparse.c`):
//! operands are kept as they are encoded and converted by the operator
//! that takes them (to an integer, to 16.16, or to 16.16 scaled by a power
//! of ten), so that each value comes out as FreeType stores it.

use crate::error::{FontError, Result};
use crate::fixed::div_fix;
use crate::types::Tag;

/// A DICT operator: one byte, or `12 x` encoded as `0x100 | x`.
pub(crate) type Operator = u16;

/// `power_tens`.
const POWER_TENS: [i64; 10] = [
    1,
    10,
    100,
    1_000,
    10_000,
    100_000,
    1_000_000,
    10_000_000,
    100_000_000,
    1_000_000_000,
];

/// The 16.16 value FreeType saturates large numbers to.
const SATURATED: i64 = 0x7FFF_FFFF;

/// One operand, as encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operand<'d> {
    /// An integer.
    Int(i64),
    /// A real number: its nibbles, from the byte after the `30` to the end
    /// of the DICT.
    Real(&'d [u8]),
    /// A CFF2 blend result in 16.16 (FreeType's internal operator 255).
    Blended(i32),
}

/// What a DICT is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Token<'d> {
    Operand(Operand<'d>),
    Operator(Operator),
}

/// The token at `at` of `data` and its length in bytes.
fn token(data: &[u8], at: usize, tag: Tag) -> Result<(Token<'_>, usize)> {
    let byte = |offset: usize| {
        data.get(at + offset)
            .copied()
            .ok_or(FontError::Truncated(tag))
    };
    let b0 = byte(0)?;
    Ok(match b0 {
        12 => (Token::Operator(0x100 | Operator::from(byte(1)?)), 2),
        0..=26 | 31 | 255 => (Token::Operator(Operator::from(b0)), 1),
        28 => {
            let value = i16::from_be_bytes([byte(1)?, byte(2)?]);
            (Token::Operand(Operand::Int(i64::from(value))), 3)
        }
        29 => {
            // FreeType reads the four bytes as an unsigned long.
            let value = u32::from_be_bytes([byte(1)?, byte(2)?, byte(3)?, byte(4)?]);
            (Token::Operand(Operand::Int(i64::from(value))), 5)
        }
        30 => {
            let nibbles = &data[at + 1..];
            let length = nibbles
                .iter()
                .position(|&b| b >> 4 == 0xF || b & 0xF == 0xF)
                .map_or(nibbles.len(), |end| end + 1);
            (Token::Operand(Operand::Real(nibbles)), 1 + length)
        }
        247..=250 => {
            let value = (i64::from(b0) - 247) * 256 + i64::from(byte(1)?) + 108;
            (Token::Operand(Operand::Int(value)), 2)
        }
        251..=254 => {
            let value = -(i64::from(b0) - 251) * 256 - i64::from(byte(1)?) - 108;
            (Token::Operand(Operand::Int(value)), 2)
        }
        // 32 to 246, and 27, which FreeType reads as a one-byte number.
        _ => (Token::Operand(Operand::Int(i64::from(b0) - 139)), 1),
    })
}

/// The tokens of DICT `data` (`cff_parser_run`), failing on data cut
/// short inside a number or an operator.
pub(crate) fn tokens(data: &[u8], tag: Tag) -> impl Iterator<Item = Result<Token<'_>>> {
    let mut at = 0;
    std::iter::from_fn(move || {
        if at >= data.len() {
            return None;
        }
        Some(token(data, at, tag).map(|(token, length)| {
            at += length;
            token
        }))
    })
}

/// The entries of DICT `data` as written: each operator with the bytes of
/// its entry, its operands and the operator itself (operands after the
/// last operator belong to none).
pub(crate) fn raw_entries(data: &[u8], tag: Tag) -> Result<Vec<(Operator, &[u8])>> {
    let mut entries = Vec::new();
    let (mut start, mut at) = (0, 0);
    while at < data.len() {
        let (token, length) = token(data, at, tag)?;
        at = (at + length).min(data.len());
        if let Token::Operator(operator) = token {
            entries.push((operator, &data[start..at]));
            start = at;
        }
    }
    Ok(entries)
}

/// The digits of a real number, one nibble at a time.
struct Nibbles<'d> {
    bytes: &'d [u8],
    at: usize,
}

impl Nibbles<'_> {
    /// The next nibble; `None` past the end of the DICT.
    fn next(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.at / 2)?;
        let nibble = if self.at.is_multiple_of(2) {
            byte >> 4
        } else {
            byte & 0xF
        };
        self.at += 1;
        Some(nibble)
    }
}

/// The digits and exponent of a real number.
#[derive(Default)]
struct Decimal {
    number: i64,
    negative: bool,
    exponent: i64,
    integer_length: i64,
    fraction_length: i64,
    /// An exponent too large to keep, and whether it was negative.
    overflow: Option<bool>,
}

impl Decimal {
    /// Reads the integer part, the fraction and the exponent of a real
    /// (FreeType keeps nine significant digits); `None` when the DICT ends
    /// inside it.
    fn read(nibbles: &[u8]) -> Option<Self> {
        let mut digits = Nibbles {
            bytes: nibbles,
            at: 0,
        };
        let mut decimal = Self::default();
        let mut exponent_add = 0;
        let mut nibble;
        loop {
            nibble = digits.next()?;
            if nibble == 0xE {
                decimal.negative = true;
            } else if nibble > 9 {
                break;
            } else if decimal.number >= 0xCCC_CCCC {
                exponent_add += 1;
            } else if nibble != 0 || decimal.number != 0 {
                decimal.integer_length += 1;
                decimal.number = decimal.number * 10 + i64::from(nibble);
            }
        }
        if nibble == 0xA {
            loop {
                nibble = digits.next()?;
                if nibble >= 10 {
                    break;
                }
                if nibble == 0 && decimal.number == 0 {
                    exponent_add -= 1;
                } else if decimal.number < 0xCCC_CCCC && decimal.fraction_length < 9 {
                    decimal.fraction_length += 1;
                    decimal.number = decimal.number * 10 + i64::from(nibble);
                }
            }
        }
        if nibble == 11 || nibble == 12 {
            let negative = nibble == 12;
            let mut exponent = 0;
            loop {
                nibble = digits.next()?;
                if nibble >= 10 {
                    break;
                }
                if exponent > 1000 {
                    decimal.overflow = Some(negative);
                } else {
                    exponent = exponent * 10 + i64::from(nibble);
                }
            }
            decimal.exponent = if negative { -exponent } else { exponent };
        }
        decimal.exponent += exponent_add;
        Some(decimal)
    }

    /// The value in 16.16 with the exponent scaled to as few digits as
    /// keep it in range, and that power of ten.
    fn scaled(mut self) -> (i64, i64) {
        self.fraction_length += self.integer_length;
        let mut exponent = self.exponent + self.integer_length;
        let length = self.fraction_length;
        if length <= 5 {
            if self.number > 0x7FFF {
                return (div_fix(self.number, 10), exponent - length + 1);
            }
            if exponent > 0 {
                let new_length = exponent.min(5);
                let shift = new_length - length;
                if shift > 0 {
                    exponent -= new_length;
                    self.number *= POWER_TENS[shift as usize];
                    if self.number > 0x7FFF {
                        self.number /= 10;
                        exponent += 1;
                    }
                } else {
                    exponent -= length;
                }
            } else {
                exponent -= length;
            }
            (self.number << 16, exponent)
        } else if self.number / POWER_TENS[(length - 5) as usize] > 0x7FFF {
            let divisor = POWER_TENS[(length - 4) as usize];
            (div_fix(self.number, divisor), exponent - 4)
        } else {
            let divisor = POWER_TENS[(length - 5) as usize];
            (div_fix(self.number, divisor), exponent - 5)
        }
    }

    /// The value in 16.16, saturating.
    fn fixed(mut self) -> i64 {
        let integer_length = self.integer_length + self.exponent;
        let mut fraction_length = self.fraction_length - self.exponent;
        if integer_length > 5 {
            return SATURATED;
        }
        if integer_length < -5 {
            return 0;
        }
        if integer_length < 0 {
            self.number /= POWER_TENS[(-integer_length) as usize];
            fraction_length += integer_length;
        }
        if fraction_length == 10 {
            self.number /= 10;
            fraction_length -= 1;
        }
        if fraction_length > 0 {
            let divisor = POWER_TENS[fraction_length as usize];
            if self.number / divisor > 0x7FFF {
                0
            } else {
                div_fix(self.number, divisor)
            }
        } else {
            let number = self.number * POWER_TENS[(-fraction_length) as usize];
            if number > 0x7FFF {
                SATURATED
            } else {
                number << 16
            }
        }
    }
}

/// `cff_parse_real` without scaling: a real number times `10^power_ten`
/// in 16.16.
fn parse_real(nibbles: &[u8], power_ten: i64) -> i64 {
    let Some(mut decimal) = Decimal::read(nibbles) else {
        return 0;
    };
    if decimal.number == 0 {
        return 0;
    }
    let negative = decimal.negative;
    let value = if let Some(underflow) = decimal.overflow {
        if underflow { 0 } else { SATURATED }
    } else {
        decimal.exponent += power_ten;
        decimal.fixed()
    };
    if negative { -value } else { value }
}

/// `cff_parse_real` with scaling: a real number in 16.16, as precise as
/// possible, and the power of ten it is to be scaled by.
fn parse_real_scaled(nibbles: &[u8]) -> (i64, i64) {
    let Some(decimal) = Decimal::read(nibbles) else {
        return (0, 0);
    };
    if decimal.number == 0 {
        return (0, 0);
    }
    let negative = decimal.negative;
    let (value, scaling) = if let Some(underflow) = decimal.overflow {
        (if underflow { 0 } else { SATURATED }, 0)
    } else {
        decimal.scaled()
    };
    (if negative { -value } else { value }, scaling)
}

impl Operand<'_> {
    /// `cff_parse_num`: an integer (reals truncated, blends rounded to 16
    /// bits).
    pub(crate) fn num(self) -> i64 {
        match self {
            Self::Int(value) => value,
            Self::Real(nibbles) => parse_real(nibbles, 0) >> 16,
            Self::Blended(value) => {
                i64::from(((value as u32 >> 8).wrapping_add(0x80) >> 8) as u16 as i16)
            }
        }
    }

    /// `cff_parse_fixed`: the number in 16.16.
    pub(crate) fn fixed(self) -> i64 {
        self.fixed_scaled(0)
    }

    /// `cff_parse_fixed_scaled` (`do_fixed`): the number times
    /// `10^power_ten` in 16.16, saturating.
    pub(crate) fn fixed_scaled(self, power_ten: usize) -> i64 {
        let factor = POWER_TENS[power_ten];
        let limit = i64::MAX / factor;
        let saturated = |value: i64| if value > 0 { SATURATED } else { -SATURATED };
        match self {
            Self::Real(nibbles) => parse_real(nibbles, power_ten as i64),
            Self::Blended(value) => {
                let value = i64::from(value);
                if value.abs() > limit {
                    saturated(value)
                } else {
                    value * factor
                }
            }
            Self::Int(value) => {
                if (value.abs() << 16) > limit {
                    return saturated(value);
                }
                let value = value * factor;
                if value.abs() > 0x7FFF {
                    saturated(value)
                } else {
                    value << 16
                }
            }
        }
    }

    /// `cff_parse_fixed_dynamic`: the number in 16.16 as precise as
    /// possible, and the power of ten it is to be scaled by.
    pub(crate) fn fixed_dynamic(self) -> (i64, i64) {
        let number = match self {
            Self::Real(nibbles) => return parse_real_scaled(nibbles),
            Self::Int(_) | Self::Blended(_) => self.num(),
        };
        if number <= 0x7FFF {
            return (number << 16, 0);
        }
        let integer_length = (5..10)
            .find(|&length| number < POWER_TENS[length])
            .unwrap_or(10);
        let scaling = if number / POWER_TENS[integer_length - 5] > 0x7FFF {
            integer_length - 4
        } else {
            integer_length - 5
        };
        (div_fix(number, POWER_TENS[scaling]), scaling as i64)
    }
}

/// The DICTs FreeType parses, each with its own operators
/// (`cfftoken.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DictKind {
    /// A CFF Top DICT, or a Font DICT of a CID-keyed CFF font.
    CffTop,
    CffPrivate,
    Cff2Top,
    Cff2Font,
    Cff2Private,
}

impl DictKind {
    /// The fewest operands FreeType reads `operator` with in this DICT
    /// (`None` for operators it ignores; delta arrays may be empty).
    pub(crate) fn arity(self, operator: Operator) -> Option<usize> {
        let fields: &[(Operator, usize)] = match self {
            Self::CffTop => &[
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
                (4, 1),
                (5, 4),
                (13, 1),
                (15, 1),
                (16, 1),
                (17, 1),
                (18, 2),
                (0x100, 1),
                (0x101, 1),
                (0x102, 1),
                (0x103, 1),
                (0x104, 1),
                (0x105, 1),
                (0x106, 1),
                (0x107, 6),
                (0x108, 1),
                (0x114, 1),
                (0x115, 1),
                (0x118, 5),
                (0x11E, 3),
                (0x11F, 1),
                (0x120, 1),
                (0x121, 1),
                (0x122, 1),
                (0x123, 1),
                (0x124, 1),
                (0x125, 1),
                (0x126, 1),
            ],
            Self::CffPrivate => &[
                (6, 0),
                (7, 0),
                (8, 0),
                (9, 0),
                (10, 1),
                (11, 1),
                (19, 1),
                (20, 1),
                (21, 1),
                (0x109, 1),
                (0x10A, 1),
                (0x10B, 1),
                (0x10C, 0),
                (0x10D, 0),
                (0x10E, 1),
                (0x10F, 1),
                (0x110, 1),
                (0x111, 1),
                (0x112, 1),
                (0x113, 1),
            ],
            Self::Cff2Top => &[
                (17, 1),
                (24, 1),
                (25, 1),
                (0x107, 6),
                (0x124, 1),
                (0x125, 1),
            ],
            Self::Cff2Font => &[(18, 2), (0x107, 6)],
            Self::Cff2Private => &[
                (6, 0),
                (7, 0),
                (8, 0),
                (9, 0),
                (10, 1),
                (11, 1),
                (19, 1),
                (22, 1),
                (23, 1),
                (0x109, 1),
                (0x10A, 1),
                (0x10B, 1),
                (0x10C, 0),
                (0x10D, 0),
                (0x111, 1),
                (0x112, 1),
            ],
        };
        fields
            .iter()
            .find(|&&(op, _)| op == operator)
            .map(|&(_, arity)| arity)
    }

    /// Fails when `operator` has fewer operands than FreeType reads it
    /// with (`Stack_Underflow`).
    pub(crate) fn check(self, operator: Operator, operands: usize, tag: Tag) -> Result<()> {
        match self.arity(operator) {
            Some(arity) if operands < arity => Err(FontError::malformed(
                tag,
                "DICT operator without its operands",
            )),
            _ => Ok(()),
        }
    }
}

/// `power_tens[exponent]`, for the font matrix's scalings (one outside
/// the table).
pub(crate) fn power_ten(exponent: i64) -> i64 {
    usize::try_from(exponent)
        .ok()
        .and_then(|exponent| POWER_TENS.get(exponent))
        .copied()
        .unwrap_or(1)
}

/// The `(operator, operands)` entries of a DICT without blends.
pub(crate) type Entries<'d> = Vec<(Operator, Vec<Operand<'d>>)>;

/// The entries of the `kind` DICT `data`, whose parser holds `stack`
/// entries: an operator after as many operands, or one FreeType reads
/// with more operands than it has, is an error.
pub(crate) fn entries(data: &[u8], kind: DictKind, stack: usize, tag: Tag) -> Result<Entries<'_>> {
    let mut entries = Vec::new();
    let mut operands = Vec::new();
    for token in tokens(data, tag) {
        match token? {
            Token::Operand(operand) => {
                if operands.len() >= stack {
                    return Err(FontError::malformed(tag, "too many DICT operands"));
                }
                operands.push(operand);
            }
            Token::Operator(operator) => {
                if operands.len() >= stack {
                    return Err(FontError::malformed(tag, "too many DICT operands"));
                }
                kind.check(operator, operands.len(), tag)?;
                entries.push((operator, std::mem::take(&mut operands)));
            }
        }
    }
    Ok(entries)
}

/// The operands of the last `operator` entry of `entries`: FreeType keeps
/// the value written last.
pub(crate) fn last<'e, 'd>(
    entries: &'e Entries<'d>,
    operator: Operator,
) -> Option<&'e [Operand<'d>]> {
    entries
        .iter()
        .rev()
        .find(|(op, _)| *op == operator)
        .map(|(_, operands)| operands.as_slice())
}

/// The single number `operator` gives (`cff_kind_num`), when the entry is
/// there with an operand.
pub(crate) fn number(entries: &Entries<'_>, operator: Operator) -> Option<i64> {
    last(entries, operator)?
        .first()
        .map(|operand| operand.num())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG: Tag = Tag::new(b"CFF ");

    fn read(data: &[u8], stack: usize) -> Result<Entries<'_>> {
        entries(data, DictKind::CffTop, stack, TAG)
    }

    #[test]
    fn tokens_and_numbers() {
        // 100, 1000, -1000, 12345 (short), 65536 (long), real -2.25, op 17,
        // then op 12 7.
        let data = [
            239, 250, 124, 254, 124, 28, 0x30, 0x39, 29, 0, 1, 0, 0, 30, 0xE2, 0xA2, 0x5F, 17, 139,
            12, 7,
        ];
        let entries = entries(&data, DictKind::CffPrivate, 48, TAG).unwrap();
        assert_eq!(entries[0].0, 17);
        let values: Vec<i64> = entries[0].1.iter().map(|op| op.num()).collect();
        assert_eq!(values, [100, 1000, -1000, 12345, 65536, -3]);
        assert_eq!(entries[0].1[5].fixed(), -0x24000);
        assert_eq!(number(&entries, 0x107), Some(0));
        assert_eq!(number(&entries, 18), None);
        // A long number is unsigned, as FreeType reads it on 64-bit hosts.
        let long = read(&[29, 0xFF, 0xFF, 0xFF, 0xFF, 17], 48).unwrap();
        assert_eq!(number(&long, 17), Some(0xFFFF_FFFF));
        assert!(read(&[28, 1], 48).is_err());
        assert!(read(&[139, 139, 17], 2).is_err());
        assert!(read(&[139, 17], 2).is_ok());
        assert!(read(&[12], 48).is_err());
        // `CharStrings` without its offset, `Private` with one value.
        assert!(read(&[17], 48).is_err());
        assert!(read(&[139, 18], 48).is_err());
        assert_eq!(DictKind::Cff2Private.arity(6), Some(0));
        assert_eq!(DictKind::Cff2Font.arity(17), None);
    }

    #[test]
    fn raw_entries_keep_their_bytes() {
        // 100 17, then the real -2.25 and 0 as op 12 7, then a lone 5.
        let data = [239, 17, 30, 0xE2, 0xA2, 0x5F, 139, 12, 7, 144];
        let entries = raw_entries(&data, TAG).unwrap();
        assert_eq!(entries, [(17, &data[..2]), (0x107, &data[2..9])]);
        assert!(raw_entries(&[28, 1], TAG).is_err());
        assert_eq!(raw_entries(&[30, 0x12], TAG).unwrap(), []);
    }

    #[test]
    fn reals_as_freetype_reads_them() {
        // 0.039625 times 1000 (`BlueScale`).
        assert_eq!(parse_real(&[0x0A, 0x03, 0x96, 0x25, 0xFF], 3), 2_596_864);
        // 0.001 with dynamic scaling: 1 scaled by 10^-3.
        assert_eq!(parse_real_scaled(&[0x0A, 0x00, 0x1F]), (1 << 16, -3));
        assert_eq!(parse_real_scaled(&[0x1F]), (1 << 16, 0));
        // 1.5, -0.5, 2.5E2, 1E-2, a huge value and cut short.
        assert_eq!(parse_real(&[0x1A, 0x5F], 0), 0x18000);
        assert_eq!(parse_real(&[0xE0, 0xA5, 0xFF], 0), -0x8000);
        assert_eq!(parse_real(&[0x2A, 0x5B, 0x2F], 0), 250 << 16);
        assert_eq!(parse_real(&[0x0A, 0x25, 0xFF], 0), 0x4000);
        // FreeType keeps no digit below a tenth outside a scaled read.
        assert_eq!(parse_real(&[0x1C, 0x2F], 0), 0);
        assert_eq!(parse_real(&[0x1C, 0x2F], 3), 10 << 16);
        assert_eq!(parse_real(&[0x99, 0x99, 0x99, 0xFF], 0), SATURATED);
        assert_eq!(parse_real(&[0xFF], 0), 0);
        assert_eq!(parse_real(&[0x12], 0), 0);
    }

    #[test]
    fn conversions_saturate() {
        assert_eq!(Operand::Int(40000).fixed(), SATURATED);
        assert_eq!(Operand::Int(-3).fixed(), -3 << 16);
        assert_eq!(Operand::Int(40).fixed_scaled(3), SATURATED);
        assert_eq!(Operand::Blended(0x18000).num(), 2);
        assert_eq!(Operand::Blended(-0x18000).num(), -1);
        assert_eq!(Operand::Blended(0x18000).fixed_scaled(1), 0xF0000);
        assert_eq!(Operand::Int(1000).fixed_dynamic(), (1000 << 16, 0));
        assert_eq!(Operand::Int(100_000).fixed_dynamic(), (10_000 << 16, 1));
        assert_eq!(power_ten(3), 1000);
        assert_eq!(power_ten(-1), 1);
    }
}
