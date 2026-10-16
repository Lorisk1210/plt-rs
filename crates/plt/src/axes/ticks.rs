//! Tick and grid appearance (matplotlib's tick parameters).

use std::str::FromStr;

use crate::colors::Color;
use crate::lines::LineStyle;
use crate::rc::{ColorOrInherit, RcParams};
use crate::ticker::AxisName;

/// Which way tick marks point relative to the axes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TickDirection {
    /// Into the axes.
    In,
    /// Out of the axes.
    #[default]
    Out,
    /// Across the axes edge.
    InOut,
}

impl TickDirection {
    /// The fraction of the tick length extending outwards (matplotlib's
    /// `get_tick_padding` factors).
    pub const fn outward_fraction(self) -> f64 {
        match self {
            Self::In => 0.0,
            Self::Out => 1.0,
            Self::InOut => 0.5,
        }
    }
}

impl FromStr for TickDirection {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, String> {
        match name {
            "in" => Ok(Self::In),
            "out" => Ok(Self::Out),
            "inout" => Ok(Self::InOut),
            other => Err(format!(
                "{other:?} is not a tick direction (in, out, inout)"
            )),
        }
    }
}

/// Which sides of an axes show something: the first side is bottom (x)
/// or left (y), the second top or right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sides {
    /// Bottom or left.
    pub first: bool,
    /// Top or right.
    pub second: bool,
}

/// How gridlines at the ticks are drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct GridStyle {
    /// Line colour.
    pub color: Color,
    /// Opacity.
    pub alpha: f64,
    /// Line width in points.
    pub linewidth: f64,
    /// Line style.
    pub linestyle: LineStyle,
}

/// The appearance of one tick level (major or minor) of one axis.
#[derive(Clone, Debug, PartialEq)]
pub struct TickStyle {
    /// Length in points.
    pub size: f64,
    /// Line width in points.
    pub width: f64,
    /// Gap between tick and label in points.
    pub pad: f64,
    /// Tick colour.
    pub color: Color,
    /// Label colour.
    pub label_color: Color,
    /// Label size in points.
    pub label_size: f64,
    /// Direction.
    pub direction: TickDirection,
    /// Sides with tick marks.
    pub ticks: Sides,
    /// Sides with tick labels.
    pub labels: Sides,
    /// Gridlines, if shown.
    pub grid: Option<GridStyle>,
}

impl TickStyle {
    /// The rc defaults for the major (`major = true`) or minor ticks of
    /// one axis, including matplotlib's `axes.grid*` rules.
    pub fn from_rc(axis: AxisName, major: bool, rc: &RcParams) -> Self {
        let x = axis == AxisName::X;
        let (size, width, pad) = match (x, major) {
            (true, true) => (
                rc.xtick_major_size,
                rc.xtick_major_width,
                rc.xtick_major_pad,
            ),
            (true, false) => (
                rc.xtick_minor_size,
                rc.xtick_minor_width,
                rc.xtick_minor_pad,
            ),
            (false, true) => (
                rc.ytick_major_size,
                rc.ytick_major_width,
                rc.ytick_major_pad,
            ),
            (false, false) => (
                rc.ytick_minor_size,
                rc.ytick_minor_width,
                rc.ytick_minor_pad,
            ),
        };
        let color = if x {
            rc.xtick_color.clone()
        } else {
            rc.ytick_color.clone()
        };
        let label_color = match if x {
            &rc.xtick_labelcolor
        } else {
            &rc.ytick_labelcolor
        } {
            ColorOrInherit::Inherit => color.clone(),
            ColorOrInherit::Color(c) => c.clone(),
        };
        let label_size = if x {
            rc.xtick_labelsize
        } else {
            rc.ytick_labelsize
        }
        .to_points(rc.font_size);
        let direction = if x {
            &rc.xtick_direction
        } else {
            &rc.ytick_direction
        }
        .parse()
        .unwrap_or_default();
        let (side1, side2, level1, level2, label1, label2) = match (x, major) {
            (true, true) => (
                rc.xtick_bottom,
                rc.xtick_top,
                rc.xtick_major_bottom,
                rc.xtick_major_top,
                rc.xtick_labelbottom,
                rc.xtick_labeltop,
            ),
            (true, false) => (
                rc.xtick_bottom,
                rc.xtick_top,
                rc.xtick_minor_bottom,
                rc.xtick_minor_top,
                rc.xtick_labelbottom,
                rc.xtick_labeltop,
            ),
            (false, true) => (
                rc.ytick_left,
                rc.ytick_right,
                rc.ytick_major_left,
                rc.ytick_major_right,
                rc.ytick_labelleft,
                rc.ytick_labelright,
            ),
            (false, false) => (
                rc.ytick_left,
                rc.ytick_right,
                rc.ytick_minor_left,
                rc.ytick_minor_right,
                rc.ytick_labelleft,
                rc.ytick_labelright,
            ),
        };
        Self {
            size,
            width,
            pad,
            color,
            label_color,
            label_size,
            direction,
            ticks: Sides {
                first: side1 && level1,
                second: side2 && level2,
            },
            labels: Sides {
                first: label1,
                second: label2,
            },
            grid: grid_from_rc(axis, major, rc),
        }
    }

    /// Distance from the axes edge to the labels in points: the pad plus
    /// the outward part of the tick (matplotlib's `Tick._pad`).
    pub fn label_offset(&self) -> f64 {
        self.pad + self.size * self.direction.outward_fraction()
    }
}

fn grid_from_rc(axis: AxisName, major: bool, rc: &RcParams) -> Option<GridStyle> {
    let which = rc.axes_grid_which.as_str();
    let level_on = if major {
        matches!(which, "major" | "both")
    } else {
        matches!(which, "minor" | "both")
    };
    let axis_on = match rc.axes_grid_axis.as_str() {
        "both" => true,
        "x" => axis == AxisName::X,
        _ => axis == AxisName::Y,
    };
    if !(rc.axes_grid && level_on && axis_on) {
        return None;
    }
    let (color, alpha, linewidth, linestyle) = if major {
        (
            &rc.grid_major_color,
            rc.grid_major_alpha,
            rc.grid_major_linewidth,
            &rc.grid_major_linestyle,
        )
    } else {
        (
            &rc.grid_minor_color,
            rc.grid_minor_alpha,
            rc.grid_minor_linewidth,
            &rc.grid_minor_linestyle,
        )
    };
    Some(GridStyle {
        color: color.clone().unwrap_or_else(|| rc.grid_color.clone()),
        alpha: alpha.unwrap_or(rc.grid_alpha),
        linewidth: linewidth.unwrap_or(rc.grid_linewidth),
        linestyle: linestyle
            .clone()
            .unwrap_or_else(|| rc.grid_linestyle.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_matplotlib() {
        let rc = RcParams::matplotlib();
        let major = TickStyle::from_rc(AxisName::X, true, &rc);
        assert_eq!((major.size, major.width, major.pad), (3.5, 0.8, 3.5));
        assert_eq!(major.label_offset(), 7.0);
        assert_eq!(
            major.ticks,
            Sides {
                first: true,
                second: false
            }
        );
        assert_eq!(
            major.labels,
            Sides {
                first: true,
                second: false
            }
        );
        assert!(major.grid.is_none());
        let minor = TickStyle::from_rc(AxisName::Y, false, &rc);
        assert_eq!((minor.size, minor.width), (2.0, 0.6));
    }

    #[test]
    fn grid_rules() {
        let mut rc = RcParams::matplotlib();
        rc.set("axes.grid", "True").expect("valid");
        rc.set("axes.grid.axis", "y").expect("valid");
        assert!(TickStyle::from_rc(AxisName::X, true, &rc).grid.is_none());
        let grid = TickStyle::from_rc(AxisName::Y, true, &rc)
            .grid
            .expect("y grid");
        assert_eq!(grid.linewidth, 0.8);
        assert!(
            TickStyle::from_rc(AxisName::Y, false, &rc).grid.is_none(),
            "minor grid needs which=minor/both"
        );
    }
}
