//! `text.hinting`: matplotlib's names of `FreeType`'s load flags, kept as
//! spelled, and the hinting each selects.

use std::fmt;
use std::str::FromStr;

use plt_text::Hinting;

use crate::image::UnknownName;

/// The glyph hinting `text.hinting` names: `FreeType`'s load flags by their
/// own names (`default`, `no_autohint`, ...) or by matplotlib's shorter
/// ones (`either`, `native`, ...), kept as spelled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextHinting {
    /// `default`: `FreeType`'s default.
    #[default]
    Default,
    /// `no_autohint`: native hinting only.
    NoAutohint,
    /// `force_autohint`: the autohinter for every font.
    ForceAutohint,
    /// `no_hinting`: no grid fitting.
    NoHinting,
    /// `auto`: the same as `force_autohint`.
    Auto,
    /// `native`: the same as `no_autohint`.
    Native,
    /// `either`: the same as `default`.
    Either,
    /// `none`: the same as `no_hinting`.
    None,
}

impl TextHinting {
    const ALL: [Self; 8] = [
        Self::Default,
        Self::NoAutohint,
        Self::ForceAutohint,
        Self::NoHinting,
        Self::Auto,
        Self::Native,
        Self::Either,
        Self::None,
    ];

    /// matplotlib's name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::NoAutohint => "no_autohint",
            Self::ForceAutohint => "force_autohint",
            Self::NoHinting => "no_hinting",
            Self::Auto => "auto",
            Self::Native => "native",
            Self::Either => "either",
            Self::None => "none",
        }
    }

    /// The hinting glyphs are loaded with (matplotlib's Agg backend's
    /// `get_hinting_flag`).
    pub const fn hinting(self) -> Hinting {
        match self {
            Self::Default | Self::Either => Hinting::Default,
            Self::NoAutohint | Self::Native => Hinting::Native,
            Self::ForceAutohint | Self::Auto => Hinting::Auto,
            Self::NoHinting | Self::None => Hinting::None,
        }
    }
}

impl fmt::Display for TextHinting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for TextHinting {
    type Err = UnknownName;

    /// Parses matplotlib's name ([`TextHinting::name`]).
    fn from_str(name: &str) -> Result<Self, UnknownName> {
        Self::ALL
            .into_iter()
            .find(|hinting| hinting.name() == name)
            .ok_or_else(|| UnknownName {
                kind: "text hinting",
                name: name.to_owned(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_aliases_share_a_hinting() {
        for hinting in TextHinting::ALL {
            assert_eq!(hinting.to_string().parse(), Ok(hinting));
        }
        assert_eq!(TextHinting::Either.hinting(), Hinting::Default);
        assert_eq!(TextHinting::NoAutohint.hinting(), Hinting::Native);
        assert_eq!(TextHinting::Auto.hinting(), Hinting::Auto);
        assert_eq!(TextHinting::None.hinting(), Hinting::None);
        assert!("AUTO".parse::<TextHinting>().is_err(), "case-sensitive");
    }
}
