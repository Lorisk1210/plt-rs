//! Which converter dates get (`date.converter`): matplotlib's
//! `DateConverter` or its `ConciseDateConverter`.

use std::fmt;
use std::str::FromStr;

use crate::image::UnknownName;

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

impl fmt::Display for DateConverter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for DateConverter {
    type Err = UnknownName;

    /// Parses matplotlib's name ([`DateConverter::name`]).
    fn from_str(name: &str) -> Result<Self, UnknownName> {
        [Self::Auto, Self::Concise]
            .into_iter()
            .find(|converter| converter.name() == name)
            .ok_or_else(|| UnknownName {
                kind: "date converter",
                name: name.to_owned(),
            })
    }
}

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
