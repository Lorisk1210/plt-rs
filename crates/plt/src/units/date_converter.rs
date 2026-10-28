//! Which converter dates get (`date.converter`): matplotlib's
//! `DateConverter` or its `ConciseDateConverter`.

use crate::rc::keyword_names;

/// The converter of date axes (`date.converter`), which decides their
/// default tick label formatter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DateConverter {
    /// `auto`: an `AutoDateFormatter` (matplotlib's `DateConverter`).
    #[default]
    Auto,
    /// `concise`: a `ConciseDateFormatter` (matplotlib's
    /// `ConciseDateConverter`).
    Concise,
}

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
