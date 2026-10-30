//! The kinds of converter of dates: matplotlib's `DateConverter` and its
//! `ConciseDateConverter`, which `date.converter` chooses from for the
//! dates plotted on an axis, and which an axis can be given.

use crate::rc::keyword_names;

/// A kind of converter of dates, which decides the default tick label
/// formatter of their axis: the one `date.converter` names for the
/// dates plotted on an axis, or the kind of the converter an axis is
/// given with [`Axis::set_converter`]`(kind)` (a
/// [`UnitConverter::DatesBy`], which a kind converts into).
///
/// [`Axis::set_converter`]: crate::axes::Axis::set_converter
/// [`UnitConverter::DatesBy`]: crate::units::UnitConverter::DatesBy
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DateConverter {
    /// `auto`: an `AutoDateFormatter` (matplotlib's `DateConverter`).
    #[default]
    Auto,
    /// `concise`: a `ConciseDateFormatter` (matplotlib's
    /// `ConciseDateConverter`).
    Concise,
}

pltrs_core::archive_enum!(DateConverter { Auto, Concise });

impl DateConverter {
    /// matplotlib's name (`auto` or `concise`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Concise => "concise",
        }
    }
}

keyword_names!(DateConverter, "date converter", [Self::Auto, Self::Concise]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converters_round_trip_their_names() {
        for converter in [DateConverter::Auto, DateConverter::Concise] {
            assert_eq!(converter.to_string().parse(), Ok(converter));
        }
        assert!("Concise".parse::<DateConverter>().is_err());
    }
}
