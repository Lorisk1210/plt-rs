//! FreeType's PostScript number conversions (`psconv.c`) and the field
//! readers of its Type 1 parser (`psobjs.c`), applied to the source text
//! of dictionary values.

/// PostScript white space.
const fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\r' | b'\n' | b'\t' | b'\x0c' | b'\0')
}

/// The value of a digit in bases up to 36.
const fn digit_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some((byte - b'0') as u32),
        b'a'..=b'z' => Some((byte - b'a') as u32 + 10),
        b'A'..=b'Z' => Some((byte - b'A') as u32 + 10),
        _ => None,
    }
}

/// A cursor over source text.
#[derive(Clone, Copy, Debug)]
struct Cursor<'a> {
    text: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.at).copied()
    }

    /// `PS_Conv_Strtol`: an integer in `base`, saturating at
    /// `0x7FFF_FFFF`; 0 without digits.
    fn read_strtol(&mut self, base: i64) -> i64 {
        if self.at >= self.text.len() || !(2..=36).contains(&base) {
            return 0;
        }
        let mut negative = false;
        if let Some(sign @ (b'-' | b'+')) = self.peek() {
            negative = sign == b'-';
            self.at += 1;
            if matches!(self.peek(), None | Some(b'-' | b'+')) {
                return 0;
            }
        }
        let (limit, last) = (0x7FFF_FFFF / base, 0x7FFF_FFFF % base);
        let mut number = 0i64;
        let mut overflow = false;
        while let Some(byte) = self.peek() {
            if is_space(byte) || byte >= 0x80 {
                break;
            }
            let Some(digit) = digit_value(byte).map(i64::from).filter(|&d| d < base) else {
                break;
            };
            if number > limit || (number == limit && digit > last) {
                overflow = true;
            } else {
                number = number * base + digit;
            }
            self.at += 1;
        }
        if overflow {
            number = 0x7FFF_FFFF;
        }
        if negative { -number } else { number }
    }

    /// `PS_Conv_ToInt`: a decimal integer, optionally `base#digits`.
    fn read_int(&mut self) -> i64 {
        let start = self.at;
        let number = self.read_strtol(10);
        if self.at == start {
            return 0;
        }
        if self.peek() == Some(b'#') {
            let mut radix = *self;
            radix.at += 1;
            let digits = radix.at;
            let number = radix.read_strtol(number);
            if radix.at == digits {
                return 0;
            }
            *self = radix;
            return number;
        }
        number
    }

    /// `PS_Conv_ToFixed`: a real number as 16.16 fixed point, scaled by
    /// `10^power_ten`.
    fn read_fixed(&mut self, mut power_ten: i64) -> i64 {
        const LIMIT: i64 = 0xCCC_CCCC;
        let mut p = *self;
        let mut integral = 0i64;
        let (mut decimal, mut divider) = (0i64, 1i64);
        let (mut overflow, mut underflow) = (false, false);
        if p.at >= p.text.len() {
            return 0;
        }
        let mut negative = false;
        if let Some(sign @ (b'-' | b'+')) = p.peek() {
            negative = sign == b'-';
            p.at += 1;
            if matches!(p.peek(), None | Some(b'-' | b'+')) {
                return 0;
            }
        }
        if p.peek() != Some(b'.') {
            let start = p.at;
            integral = p.read_int();
            if p.at == start {
                return 0;
            }
            if integral > 0x7FFF {
                overflow = true;
            } else {
                integral = i64::from(((integral as u32) << 16) as i32);
            }
        }
        if p.peek() == Some(b'.') {
            p.at += 1;
            while let Some(byte) = p.peek() {
                if is_space(byte) || byte >= 0x80 {
                    break;
                }
                let Some(digit) = digit_value(byte).map(i64::from).filter(|&d| d < 10) else {
                    break;
                };
                if divider < LIMIT && decimal < LIMIT {
                    decimal = decimal * 10 + digit;
                    if integral == 0 && power_ten > 0 {
                        power_ten -= 1;
                    } else {
                        divider *= 10;
                    }
                }
                p.at += 1;
            }
        }
        if p.at + 1 < p.text.len() && matches!(p.peek(), Some(b'e' | b'E')) {
            p.at += 1;
            let start = p.at;
            let exponent = p.read_int();
            if p.at == start {
                return 0;
            }
            if exponent > 1000 {
                overflow = true;
            } else if exponent < -1000 {
                underflow = true;
            } else {
                power_ten += exponent;
            }
        }
        *self = p;
        if integral == 0 && decimal == 0 {
            return 0;
        }
        let value = if overflow {
            Some(0x7FFF_FFFF)
        } else if underflow {
            None
        } else {
            scale(integral, decimal, divider, power_ten)
        };
        match value {
            Some(value) if negative => -value,
            Some(value) => value,
            None => 0,
        }
    }
}

/// The tail of `PS_Conv_ToFixed`: applies the power of ten to the
/// integral part and the decimal fraction; `None` on underflow.
fn scale(mut integral: i64, mut decimal: i64, mut divider: i64, mut power_ten: i64) -> Option<i64> {
    const LIMIT: i64 = 0xCCC_CCCC;
    while power_ten > 0 {
        if integral >= LIMIT {
            return Some(0x7FFF_FFFF);
        }
        integral *= 10;
        if decimal >= LIMIT {
            if divider == 1 {
                return Some(0x7FFF_FFFF);
            }
            divider /= 10;
        } else {
            decimal *= 10;
        }
        power_ten -= 1;
    }
    while power_ten < 0 {
        integral /= 10;
        if divider < LIMIT {
            divider *= 10;
        } else {
            decimal /= 10;
        }
        if integral == 0 && decimal == 0 {
            return None;
        }
        power_ten += 1;
    }
    if decimal != 0 {
        integral += crate::fixed::div_fix(decimal, divider);
    }
    Some(integral)
}

/// `PS_Conv_ToInt` of the start of `text`.
pub(crate) fn to_int(text: &str) -> i64 {
    Cursor {
        text: text.as_bytes(),
        at: 0,
    }
    .read_int()
}

/// `PS_Conv_ToFixed` of the start of `text`.
pub(crate) fn to_fixed(text: &str, power_ten: i64) -> i64 {
    Cursor {
        text: text.as_bytes(),
        at: 0,
    }
    .read_fixed(power_ten)
}

/// `ps_tofixedarray`: up to `max` numbers of a `[...]` or `{...}` array
/// (or a single number) as 16.16 fixed point scaled by `10^power_ten`;
/// `None` where FreeType finds something that is not a number.
pub(crate) fn to_fixed_array(text: &str, max: usize, power_ten: i64) -> Option<Vec<i64>> {
    let mut cursor = Cursor {
        text: text.as_bytes(),
        at: 0,
    };
    let ender = match cursor.peek() {
        Some(b'[') => Some(b']'),
        Some(b'{') => Some(b'}'),
        _ => None,
    };
    if ender.is_some() {
        cursor.at += 1;
    }
    let mut values = Vec::new();
    while cursor.at < cursor.text.len() {
        skip_spaces(&mut cursor);
        let Some(byte) = cursor.peek() else {
            break;
        };
        if Some(byte) == ender {
            break;
        }
        if values.len() >= max {
            break;
        }
        let start = cursor.at;
        let value = cursor.read_fixed(power_ten);
        if cursor.at == start {
            return None;
        }
        values.push(value);
        if ender.is_none() {
            break;
        }
    }
    Some(values)
}

/// Skips white space and `%` comments.
fn skip_spaces(cursor: &mut Cursor<'_>) {
    while let Some(byte) = cursor.peek() {
        if byte == b'%' {
            while cursor.peek().is_some_and(|b| b != b'\r' && b != b'\n') {
                cursor.at += 1;
            }
        } else if is_space(byte) {
            cursor.at += 1;
        } else {
            break;
        }
    }
}

/// The integers of a `[...]` or `{...}` array, at most `max` of them,
/// as FreeType reads a `T1_FIELD_NUM_TABLE` entry (`PS_Conv_ToInt` of
/// each element); empty for other values.
pub(crate) fn to_int_array(text: &str, max: usize) -> Vec<i64> {
    let inner = text
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .or_else(|| text.strip_prefix('{')?.strip_suffix('}'));
    inner.map_or_else(Vec::new, |inner| {
        inner
            .split(|c: char| c.is_ascii() && is_space(c as u8))
            .filter(|element| !element.is_empty())
            .take(max)
            .map(to_int)
            .collect()
    })
}

/// `ps_tobool`: whether the text starts with `true`.
pub(crate) fn to_bool(text: &str) -> bool {
    text.starts_with("true")
}

/// The string FreeType reads for a `T1_FIELD_STRING` or `T1_FIELD_KEY`
/// entry: a name without its slash or a string without its parentheses
/// (escapes untouched), up to the first NUL; `None` for other values.
pub(crate) fn to_string(text: &str) -> Option<String> {
    let inner = if let Some(name) = text.strip_prefix('/') {
        name
    } else {
        text.strip_prefix('(')?.strip_suffix(')')?
    };
    Some(inner.split('\0').next().unwrap_or_default().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers() {
        assert_eq!(to_int("-100"), -100);
        assert_eq!(to_int("-97.5"), -97);
        assert_eq!(to_int("16#ff"), 255);
        assert_eq!(to_int("16#"), 0);
        assert_eq!(to_int("x"), 0);
        assert_eq!(to_int("--1"), 0);
        assert_eq!(to_int("99999999999"), 0x7FFF_FFFF);
    }

    #[test]
    fn fixed_point_numbers() {
        assert_eq!(to_fixed("0.001", 3), 0x10000);
        assert_eq!(to_fixed("0.0005", 3), 0x8000);
        assert_eq!(to_fixed("0.000167", 3), 10945);
        assert_eq!(to_fixed("-9.4809", 0), -621_340);
        assert_eq!(to_fixed("1e3", 0), 1000 << 16);
        assert_eq!(to_fixed("40000", 0), 0x7FFF_FFFF);
        assert_eq!(to_fixed("1e-9", 0), 0);
        assert_eq!(to_fixed(".5", 0), 0x8000);
        assert_eq!(to_fixed("abc", 0), 0);
    }

    #[test]
    fn arrays_and_fields() {
        assert_eq!(
            to_fixed_array("[0.001 0 0 0.001 0 0 ]", 6, 3),
            Some(vec![0x10000, 0, 0, 0x10000, 0, 0])
        );
        assert_eq!(
            to_fixed_array("{-40 -250 1009 750 }", 4, 0),
            Some(vec![-40 << 16, -250 << 16, 1009 << 16, 750 << 16])
        );
        assert_eq!(
            to_fixed_array("[1 2 3]", 2, 0),
            Some(vec![1 << 16, 2 << 16])
        );
        assert_eq!(to_fixed_array("[1 x]", 4, 0), None);
        assert_eq!(to_fixed_array("7", 4, 0), Some(vec![7 << 16]));
        assert!(to_bool("true") && !to_bool("false"));
        assert_eq!(to_int_array("[-22 0 683 705 ]", 14), vec![-22, 0, 683, 705]);
        assert_eq!(to_int_array("{1 2 3}", 2), vec![1, 2]);
        assert_eq!(to_int_array("5", 14), Vec::<i64>::new());
        assert_eq!(
            to_string("(Computer Modern)").as_deref(),
            Some("Computer Modern")
        );
        assert_eq!(to_string("/CMR10").as_deref(), Some("CMR10"));
        assert_eq!(to_string("(a\\050b)").as_deref(), Some("a\\050b"));
        assert_eq!(to_string("12"), None);
    }
}
