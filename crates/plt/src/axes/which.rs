//! Which tick levels and which axes a setting applies to (matplotlib's
//! `which` and `axis` arguments, and `axes.grid.which` and
//! `axes.grid.axis`).

use crate::rc::keyword_names;
use crate::ticker::AxisName;

/// Which tick levels a setting applies to (matplotlib's `which`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TickLevel {
    /// The major ticks.
    #[default]
    Major,
    /// The minor ticks.
    Minor,
    /// Major and minor ticks.
    Both,
}

impl TickLevel {
    const ALL: [Self; 3] = [Self::Major, Self::Minor, Self::Both];

    /// Whether the major ticks are included.
    pub fn major(self) -> bool {
        matches!(self, Self::Major | Self::Both)
    }

    /// Whether the minor ticks are included.
    pub fn minor(self) -> bool {
        matches!(self, Self::Minor | Self::Both)
    }

    /// matplotlib's name (`major`, `minor` or `both`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Both => "both",
        }
    }
}

keyword_names!(TickLevel, "tick level", Self::ALL);

/// Which axes of an axes a setting applies to (matplotlib's `axis`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WhichAxis {
    /// Both axes.
    #[default]
    Both,
    /// The x axis only.
    X,
    /// The y axis only.
    Y,
}

impl WhichAxis {
    const ALL: [Self; 3] = [Self::Both, Self::X, Self::Y];

    /// Whether the setting applies to `axis`.
    pub fn includes(self, axis: AxisName) -> bool {
        match self {
            Self::Both => true,
            Self::X => axis == AxisName::X,
            Self::Y => axis == AxisName::Y,
        }
    }

    /// matplotlib's name (`both`, `x` or `y`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::X => "x",
            Self::Y => "y",
        }
    }
}

keyword_names!(WhichAxis, "axis", Self::ALL);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_levels_round_trip_their_names() {
        for level in TickLevel::ALL {
            assert_eq!(level.to_string().parse(), Ok(level));
        }
        assert!(TickLevel::Both.major() && TickLevel::Both.minor());
        assert!(!TickLevel::Minor.major());
        assert!("Major".parse::<TickLevel>().is_err(), "case-sensitive");
    }

    #[test]
    fn axes_round_trip_their_names() {
        for axis in WhichAxis::ALL {
            assert_eq!(axis.to_string().parse(), Ok(axis));
        }
        assert!(WhichAxis::Both.includes(AxisName::Y));
        assert!(WhichAxis::X.includes(AxisName::X));
        assert!(!WhichAxis::X.includes(AxisName::Y));
        assert_eq!(
            "z".parse::<WhichAxis>().map_err(|err| err.to_string()),
            Err("\"z\" is not a valid axis".to_owned())
        );
    }
}
