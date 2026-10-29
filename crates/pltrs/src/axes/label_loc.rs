//! Where an axis label sits along its axis: on any axis ([`LabelLoc`]),
//! and as matplotlib names it for the x and y axis ([`XLabelLoc`],
//! [`YLabelLoc`]: the `loc` of `set_xlabel` and `set_ylabel`, and
//! `xaxis.labellocation` and `yaxis.labellocation`).
//!
//! [`Axis::set_label_loc`](super::Axis::set_label_loc) takes either: the
//! per-axis names convert into a [`LabelLoc`], and keep matplotlib's
//! spelling of each rcParam, so that `rc::get` gives back what `rc::set`
//! took.

use pltrs_text::HAlign;

use crate::rc::keyword_names;

/// Where an axis label sits along its axis (matplotlib's `loc` of
/// `set_xlabel`/`set_ylabel`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LabelLoc {
    /// At the left (x) or bottom (y) end, aligned to it.
    Start,
    /// Centred.
    #[default]
    Center,
    /// At the right (x) or top (y) end, aligned to it.
    End,
}

impl LabelLoc {
    /// The position along the axis as a fraction of its length.
    pub(crate) fn fraction(self) -> f64 {
        match self {
            Self::Start => 0.0,
            Self::Center => 0.5,
            Self::End => 1.0,
        }
    }

    /// The label's alignment along the axis: the text's horizontal
    /// alignment, before any rotation.
    pub(crate) fn halign(self) -> HAlign {
        match self {
            Self::Start => HAlign::Left,
            Self::Center => HAlign::Center,
            Self::End => HAlign::Right,
        }
    }
}

/// Where the label of an x axis sits (`xaxis.labellocation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum XLabelLoc {
    /// At the left end.
    Left,
    /// Centred.
    #[default]
    Center,
    /// At the right end.
    Right,
}

impl XLabelLoc {
    const ALL: [Self; 3] = [Self::Left, Self::Center, Self::Right];

    /// matplotlib's name (`left`, `center` or `right`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }
}

impl From<XLabelLoc> for LabelLoc {
    fn from(loc: XLabelLoc) -> Self {
        match loc {
            XLabelLoc::Left => Self::Start,
            XLabelLoc::Center => Self::Center,
            XLabelLoc::Right => Self::End,
        }
    }
}

keyword_names!(XLabelLoc, "x label location", Self::ALL);

/// Where the label of a y axis sits (`yaxis.labellocation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum YLabelLoc {
    /// At the bottom end.
    Bottom,
    /// Centred.
    #[default]
    Center,
    /// At the top end.
    Top,
}

impl YLabelLoc {
    const ALL: [Self; 3] = [Self::Bottom, Self::Center, Self::Top];

    /// matplotlib's name (`bottom`, `center` or `top`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bottom => "bottom",
            Self::Center => "center",
            Self::Top => "top",
        }
    }
}

impl From<YLabelLoc> for LabelLoc {
    fn from(loc: YLabelLoc) -> Self {
        match loc {
            YLabelLoc::Bottom => Self::Start,
            YLabelLoc::Center => Self::Center,
            YLabelLoc::Top => Self::End,
        }
    }
}

keyword_names!(YLabelLoc, "y label location", Self::ALL);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_locations_name_the_ends_left_and_right() {
        for loc in XLabelLoc::ALL {
            assert_eq!(loc.to_string().parse(), Ok(loc));
        }
        assert_eq!(LabelLoc::from(XLabelLoc::Left), LabelLoc::Start);
        assert_eq!(LabelLoc::from(XLabelLoc::Right), LabelLoc::End);
        assert!("top".parse::<XLabelLoc>().is_err());
    }

    #[test]
    fn y_locations_name_the_ends_bottom_and_top() {
        for loc in YLabelLoc::ALL {
            assert_eq!(loc.to_string().parse(), Ok(loc));
        }
        assert_eq!(LabelLoc::from(YLabelLoc::Bottom), LabelLoc::Start);
        assert_eq!(LabelLoc::from(YLabelLoc::Top), LabelLoc::End);
        assert_eq!(LabelLoc::from(YLabelLoc::Center).fraction(), 0.5);
        assert!("left".parse::<YLabelLoc>().is_err());
    }
}
