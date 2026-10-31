//! Numbers written with the conventions of a locale: its decimal point and
//! its grouping of the digits before it (Python's `locale.format_string`
//! with `grouping=True`, which matplotlib's `ScalarFormatter` writes its
//! labels with when `useLocale` is set).

/// One size of the digit groups of a locale, from the right (an entry of
/// Python's `localeconv()["grouping"]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grouping {
    /// A group of this many digits.
    Size(usize),
    /// The previous size, for every group left (C's `0`).
    RepeatLast,
    /// No more groups (C's `CHAR_MAX`).
    Stop,
}

pltrs_core::archive_enum!(Grouping {
    Size(v0),
    RepeatLast,
    Stop,
});

/// How a locale writes numbers (the numeric part of Python's
/// `locale.localeconv()`).
///
/// ```
/// use pltrs::ticker::{Grouping, NumericLocale};
/// let english = NumericLocale {
///     decimal_point: ".".into(),
///     thousands_sep: ",".into(),
///     grouping: vec![Grouping::Size(3), Grouping::RepeatLast],
/// };
/// assert_eq!(english.localize("-1234567.25"), "-1,234,567.25");
/// let german = NumericLocale { decimal_point: ",".into(), thousands_sep: ".".into(), ..english };
/// assert_eq!(german.localize("1234.5e+06"), "1.234,5e+06");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumericLocale {
    /// The decimal point (`decimal_point`).
    pub decimal_point: String,
    /// What separates the groups of digits (`thousands_sep`).
    pub thousands_sep: String,
    /// The sizes of the groups of digits, from the right (`grouping`); no
    /// sizes, no grouping.
    pub grouping: Vec<Grouping>,
}

pltrs_core::archive_struct!(NumericLocale {
    decimal_point,
    thousands_sep,
    grouping
});

impl NumericLocale {
    /// `formatted`, a number written with `%` (`"%1.3f"`, `"%d"`,
    /// `"%1.10e"`), as the locale writes it: the digits before the point
    /// grouped and the point the locale's (Python's `locale._localize`
    /// with grouping).
    pub fn localize(&self, formatted: &str) -> String {
        let (integer, fraction) = match formatted.split_once('.') {
            Some((integer, fraction)) => (integer, Some(fraction)),
            None => (formatted, None),
        };
        let (grouped, separators) = self.group(integer);
        let localized = match fraction {
            Some(fraction) => format!("{grouped}{}{fraction}", self.decimal_point),
            None => grouped,
        };
        strip_padding(&localized, separators)
    }

    /// `digits` grouped from the right and the length of the separators
    /// added (Python's `locale._group`).
    fn group(&self, digits: &str) -> (String, usize) {
        if self.grouping.is_empty() {
            return (digits.to_owned(), 0);
        }
        let trimmed = digits.trim_end_matches(' ');
        let right_spaces = &digits[trimmed.len()..];
        let mut rest = trimmed;
        let mut left_spaces = "";
        let mut groups = Vec::new();
        for size in self.intervals() {
            if !rest.ends_with(|c: char| c.is_ascii_digit()) {
                // Only the sign or spaces are left.
                left_spaces = rest;
                rest = "";
                break;
            }
            let start = rest.len().saturating_sub(size);
            groups.push(&rest[start..]);
            rest = &rest[..start];
        }
        if !rest.is_empty() {
            groups.push(rest);
        }
        groups.reverse();
        let separators = self.thousands_sep.len() * groups.len().saturating_sub(1);
        (
            format!(
                "{left_spaces}{}{right_spaces}",
                groups.join(&self.thousands_sep)
            ),
            separators,
        )
    }

    /// The successive group sizes (Python's `locale._grouping_intervals`):
    /// a repeat without a previous size, where Python refuses the
    /// grouping, and a size of no digits end grouping.
    fn intervals(&self) -> impl Iterator<Item = usize> + '_ {
        let mut entries = self.grouping.iter();
        let mut last = None;
        let mut repeating = false;
        std::iter::from_fn(move || {
            if !repeating {
                match entries.next()? {
                    Grouping::Size(size) => last = Some(*size).filter(|&size| size > 0),
                    Grouping::RepeatLast => repeating = true,
                    Grouping::Stop => return None,
                }
            }
            last
        })
    }
}

/// `text` with up to `amount` spaces taken from its ends, left first
/// (Python's `locale._strip_padding`).
fn strip_padding(text: &str, amount: usize) -> String {
    let leading = text.len() - text.trim_start_matches(' ').len();
    let from_left = leading.min(amount);
    let rest = &text[from_left..];
    let trailing = rest.len() - rest.trim_end_matches(' ').len();
    let from_right = trailing.min(amount - from_left);
    rest[..rest.len() - from_right].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn english() -> NumericLocale {
        NumericLocale {
            decimal_point: ".".into(),
            thousands_sep: ",".into(),
            grouping: vec![Grouping::Size(3), Grouping::Size(3), Grouping::RepeatLast],
        }
    }

    #[test]
    fn digits_group_as_python_groups_them() {
        let locale = english();
        assert_eq!(locale.localize("10000000"), "10,000,000");
        assert_eq!(locale.localize("123"), "123");
        assert_eq!(locale.localize("-1000.5"), "-1,000.5");
        assert_eq!(locale.localize("1.0000002"), "1.0000002");
        assert_eq!(locale.localize("1.5000000000e+07"), "1.5000000000e+07");
    }

    #[test]
    fn stops_and_no_grouping_leave_the_rest_alone() {
        let stopped = NumericLocale {
            grouping: vec![Grouping::Size(3), Grouping::Stop],
            ..english()
        };
        assert_eq!(stopped.localize("1234567"), "1234,567");
        let c = NumericLocale {
            decimal_point: ".".into(),
            thousands_sep: String::new(),
            grouping: Vec::new(),
        };
        assert_eq!(c.localize("1234567.5"), "1234567.5");
        let indian = NumericLocale {
            grouping: vec![Grouping::Size(3), Grouping::Size(2), Grouping::RepeatLast],
            ..english()
        };
        assert_eq!(indian.localize("12345678"), "1,23,45,678");
        let invalid = NumericLocale {
            grouping: vec![Grouping::RepeatLast],
            ..english()
        };
        assert_eq!(invalid.localize("1234"), "1234");
    }
}
