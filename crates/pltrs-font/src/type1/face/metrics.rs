//! FreeType's quick advance reader for Type 1 charstrings
//! (`t1_decoder_parse_metrics`), which computes a face's maximum advance
//! width: it only follows numbers, `div`, `callsubr` and `return` up to
//! the `hsbw` or `sbw` operator.

use crate::fixed::div_fix;

/// `T1_MAX_CHARSTRINGS_OPERANDS`.
const MAX_OPERANDS: usize = 256;
/// `T1_MAX_SUBRS_CALLS`.
const MAX_CALLS: usize = 16;

/// The operators the reader acts on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Value(i64),
    Hsbw,
    Sbw,
    Div,
    CallSubr,
    Return,
}

/// The advance width of a glyph in 16.16 font units, `None` where
/// FreeType's reader fails (a drawing operator before the width, a bad
/// number or subroutine, a stack error).
pub(crate) fn advance_width(code: &[u8], subrs: &[Vec<u8>]) -> Option<i64> {
    let mut zones: Vec<(&[u8], usize)> = Vec::new();
    let (mut zone, mut at) = (code, 0usize);
    let mut stack: Vec<i64> = Vec::new();
    let mut large_int = false;
    while at < zone.len() {
        let byte = zone[at];
        at += 1;
        let op = match byte {
            10 => Op::CallSubr,
            11 => Op::Return,
            13 => Op::Hsbw,
            12 => {
                let escaped = *zone.get(at)?;
                at += 1;
                match escaped {
                    7 => Op::Sbw,
                    12 => Op::Div,
                    _ => return None,
                }
            }
            255 => {
                let bytes = zone.get(at..at + 4)?;
                at += 4;
                let value = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                if !(-32000..=32000).contains(&value) {
                    if large_int {
                        return None;
                    }
                    large_int = true;
                    Op::Value(i64::from(value))
                } else if large_int {
                    Op::Value(i64::from(value))
                } else {
                    Op::Value(i64::from(((value as u32) << 16) as i32))
                }
            }
            32..=254 => {
                let value = match byte {
                    32..=246 => i32::from(byte) - 139,
                    247..=250 => (i32::from(byte) - 247) * 256 + i32::from(*zone.get(at)?) + 108,
                    _ => -((i32::from(byte) - 251) * 256 + i32::from(*zone.get(at)?) + 108),
                };
                if byte >= 247 {
                    at += 1;
                }
                Op::Value(if large_int {
                    i64::from(value)
                } else {
                    i64::from(value << 16)
                })
            }
            _ => return None,
        };
        if large_int && !matches!(op, Op::Value(_) | Op::Div) {
            return None;
        }
        match op {
            Op::Value(value) => {
                if stack.len() >= MAX_OPERANDS {
                    return None;
                }
                stack.push(value);
            }
            Op::Hsbw => return stack.len().checked_sub(2).map(|base| stack[base + 1]),
            Op::Sbw => return stack.len().checked_sub(4).map(|base| stack[base + 2]),
            Op::Div => {
                let divisor = stack.pop()?;
                let dividend = stack.pop()?;
                stack.push(div_fix(dividend, divisor));
                large_int = false;
            }
            Op::CallSubr => {
                let index = i64::from((stack.pop()? >> 16) as i16);
                let subr = usize::try_from(index).ok().and_then(|i| subrs.get(i))?;
                if zones.len() >= MAX_CALLS {
                    return None;
                }
                zones.push((zone, at));
                (zone, at) = (subr.as_slice(), 0);
            }
            Op::Return => (zone, at) = zones.pop()?,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_from_hsbw_and_sbw() {
        // 0 500 hsbw
        assert_eq!(advance_width(&[139, 248, 136, 13], &[]), Some(500 << 16));
        // 0 0 300 0 sbw
        assert_eq!(
            advance_width(&[139, 139, 247, 192, 139, 12, 7], &[]),
            Some(300 << 16)
        );
        // 1000 3 div as the width: large integers stay unscaled until div.
        let code = [139, 255, 0, 0, 0x7D, 0x01, 142, 12, 12, 13];
        assert_eq!(advance_width(&code, &[]), Some(div_fix(32001, 3)));
        // The width in a subroutine.
        assert_eq!(
            advance_width(&[139, 10], &[vec![139, 239, 13]]),
            Some(100 << 16)
        );
    }

    #[test]
    fn failures() {
        assert_eq!(advance_width(&[139, 139, 21], &[]), None);
        assert_eq!(advance_width(&[139, 13], &[]), None);
        assert_eq!(advance_width(&[12], &[]), None);
        assert_eq!(advance_width(&[2], &[]), None);
        assert_eq!(advance_width(&[11], &[]), None);
        assert_eq!(advance_width(&[140, 10], &[]), None);
        assert_eq!(advance_width(&[139, 139], &[]), None);
    }
}
