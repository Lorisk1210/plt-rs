//! Tick and grid appearance (matplotlib's tick parameters).

use std::str::FromStr;

use crate::colors::Color;
use crate::fonts::{FontProperties, FontWeight};
use crate::lines::LineStyle;
use crate::patheffects::PathEffect;
use crate::rc::{ColorOrInherit, RcParams};
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use plt_text::{HAlign, RotationMode, VAlign};

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

/// Where an axis draws its ticks (matplotlib's `set_ticks_position`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TickPosition {
    /// Ticks and labels on this side only; the offset text follows.
    Side(SpineSide),
    /// Ticks on both sides; labels unchanged.
    Both,
    /// No ticks; labels unchanged.
    None,
    /// Ticks on both sides, labels on the bottom or left.
    Default,
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
    /// Effects the gridlines are drawn through (`path.effects` by
    /// default, as for any line).
    pub path_effects: Vec<PathEffect>,
}

impl GridStyle {
    /// The rc style of major (`major = true`) or minor gridlines.
    pub fn from_rc(major: bool, rc: &RcParams) -> Self {
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
        Self {
            color: color.clone().unwrap_or_else(|| rc.grid_color.clone()),
            alpha: alpha.unwrap_or(rc.grid_alpha),
            linewidth: linewidth.unwrap_or(rc.grid_linewidth),
            linestyle: linestyle
                .clone()
                .unwrap_or_else(|| rc.grid_linestyle.clone()),
            path_effects: rc.path_effects.clone(),
        }
    }
}

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
    /// Whether the major ticks are included.
    pub fn major(self) -> bool {
        matches!(self, Self::Major | Self::Both)
    }

    /// Whether the minor ticks are included.
    pub fn minor(self) -> bool {
        matches!(self, Self::Minor | Self::Both)
    }
}

/// Options of [`Axes::grid_with`](super::Axes::grid_with): where gridlines
/// go and how they look (unset styles keep the current or rc ones).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GridOptions {
    /// Which tick levels get gridlines.
    pub which: TickLevel,
    /// Only this axis (`None` = both).
    pub axis: Option<AxisName>,
    /// Line colour.
    pub color: Option<Color>,
    /// Opacity.
    pub alpha: Option<f64>,
    /// Line width in points.
    pub linewidth: Option<f64>,
    /// Line style.
    pub linestyle: Option<LineStyle>,
    /// Effects the gridlines are drawn through (matplotlib's
    /// `grid(path_effects=...)`).
    pub path_effects: Option<Vec<PathEffect>>,
}

impl GridOptions {
    /// Applies the set options to `style`.
    pub(crate) fn restyle(&self, style: &mut GridStyle) {
        if let Some(color) = &self.color {
            style.color = color.clone();
        }
        if let Some(alpha) = self.alpha {
            style.alpha = alpha;
        }
        if let Some(linewidth) = self.linewidth {
            style.linewidth = linewidth;
        }
        if let Some(linestyle) = &self.linestyle {
            style.linestyle = linestyle.clone();
        }
        if let Some(effects) = &self.path_effects {
            style.path_effects.clone_from(effects);
        }
    }
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
    /// Edge colour of the tick marks when it is not the tick colour,
    /// which then only fills them (matplotlib's `markeredgecolor` of the
    /// tick lines, which `axes_grid1`'s RGB axes set).
    pub edgecolor: Option<Color>,
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
    /// Rotation of the labels in degrees.
    pub label_rotation: f64,
    /// Whether the labels of polar axes turn with their ticks, with
    /// `label_rotation` added (matplotlib's `labelrotation="auto"`).
    pub label_rotation_auto: bool,
    /// How rotated labels are aligned.
    pub label_rotation_mode: RotationMode,
    /// Horizontal label alignment replacing the side's default.
    pub label_halign: Option<HAlign>,
    /// Vertical label alignment replacing the side's default.
    pub label_valign: Option<VAlign>,
    /// Alignment of the lines of multi-line labels (`None` follows the
    /// horizontal alignment).
    pub label_multialign: Option<HAlign>,
    /// Font family of the labels in place of `font.family` (matplotlib's
    /// `labelfontfamily`).
    pub label_family: Option<String>,
    /// Font weight of the labels in place of `font.weight`.
    pub label_weight: Option<FontWeight>,
    /// Whether gridlines are drawn at the ticks.
    pub grid_on: bool,
    /// How the gridlines are drawn; kept while they are off, like
    /// matplotlib's `grid_*` tick parameters.
    pub grid: GridStyle,
}

impl TickStyle {
    /// The rc defaults for the major (`major = true`) or minor ticks of
    /// one axis, including matplotlib's `axes.grid*` rules.
    pub fn from_rc(axis: AxisName, major: bool, rc: &RcParams) -> Self {
        let (size, width, pad) = tick_dimensions(axis, major, rc);
        let (ticks, labels) = tick_sides(axis, major, rc);
        let (color, label_color, label_size, direction) = match axis {
            AxisName::X => (
                &rc.xtick_color,
                &rc.xtick_labelcolor,
                rc.xtick_labelsize,
                &rc.xtick_direction,
            ),
            AxisName::Y => (
                &rc.ytick_color,
                &rc.ytick_labelcolor,
                rc.ytick_labelsize,
                &rc.ytick_direction,
            ),
        };
        let label_color = match label_color {
            ColorOrInherit::Inherit => color.clone(),
            ColorOrInherit::Color(c) => c.clone(),
        };
        Self {
            size,
            width,
            pad,
            color: color.clone(),
            edgecolor: None,
            label_color,
            label_size: label_size.to_points(rc.font_size),
            direction: direction.parse().unwrap_or_default(),
            ticks,
            labels,
            label_rotation: 0.0,
            label_rotation_auto: false,
            label_rotation_mode: RotationMode::Default,
            label_halign: None,
            label_valign: None,
            label_multialign: None,
            label_family: None,
            label_weight: None,
            grid_on: grid_on_from_rc(axis, major, rc),
            grid: GridStyle::from_rc(major, rc),
        }
    }

    /// Whether anything of this tick level is drawn: marks, labels or
    /// gridlines.
    pub fn is_drawn(&self) -> bool {
        self.ticks.first
            || self.ticks.second
            || self.labels.first
            || self.labels.second
            || self.grid_on
    }

    /// The font of the labels: `rc`'s, at the label size and with the
    /// label family and weight when set.
    pub(crate) fn label_font(&self, rc: &RcParams) -> FontProperties {
        let font = FontProperties::from_rc(rc);
        FontProperties {
            size: self.label_size,
            families: self
                .label_family
                .clone()
                .map_or_else(|| font.families.clone(), |family| vec![family]),
            weight: self.label_weight.unwrap_or(font.weight),
            ..font
        }
    }

    /// Distance from the axes edge to the labels in points: the pad plus
    /// the outward part of the tick (matplotlib's `Tick._pad`).
    pub fn label_offset(&self) -> f64 {
        self.pad + self.size * self.direction.outward_fraction()
    }
}

/// `(size, width, pad)` of the major or minor ticks of one axis.
fn tick_dimensions(axis: AxisName, major: bool, rc: &RcParams) -> (f64, f64, f64) {
    match (axis, major) {
        (AxisName::X, true) => (
            rc.xtick_major_size,
            rc.xtick_major_width,
            rc.xtick_major_pad,
        ),
        (AxisName::X, false) => (
            rc.xtick_minor_size,
            rc.xtick_minor_width,
            rc.xtick_minor_pad,
        ),
        (AxisName::Y, true) => (
            rc.ytick_major_size,
            rc.ytick_major_width,
            rc.ytick_major_pad,
        ),
        (AxisName::Y, false) => (
            rc.ytick_minor_size,
            rc.ytick_minor_width,
            rc.ytick_minor_pad,
        ),
    }
}

/// The sides with ticks and the sides with labels: a side needs both its
/// own switch and this tick level's (matplotlib's `Axes.__init__` calls
/// `tick_params(bottom=xtick.bottom and xtick.major.bottom,
/// labelbottom=xtick.labelbottom and xtick.major.bottom, ...)`).
fn tick_sides(axis: AxisName, major: bool, rc: &RcParams) -> (Sides, Sides) {
    let (sides, levels, labels) = match (axis, major) {
        (AxisName::X, true) => (
            (rc.xtick_bottom, rc.xtick_top),
            (rc.xtick_major_bottom, rc.xtick_major_top),
            (rc.xtick_labelbottom, rc.xtick_labeltop),
        ),
        (AxisName::X, false) => (
            (rc.xtick_bottom, rc.xtick_top),
            (rc.xtick_minor_bottom, rc.xtick_minor_top),
            (rc.xtick_labelbottom, rc.xtick_labeltop),
        ),
        (AxisName::Y, true) => (
            (rc.ytick_left, rc.ytick_right),
            (rc.ytick_major_left, rc.ytick_major_right),
            (rc.ytick_labelleft, rc.ytick_labelright),
        ),
        (AxisName::Y, false) => (
            (rc.ytick_left, rc.ytick_right),
            (rc.ytick_minor_left, rc.ytick_minor_right),
            (rc.ytick_labelleft, rc.ytick_labelright),
        ),
    };
    let ticks = Sides {
        first: sides.0 && levels.0,
        second: sides.1 && levels.1,
    };
    // Like matplotlib's `Axes.clear`, a level switched off on a side
    // (`xtick.major.bottom`) loses its labels there too.
    let labels = Sides {
        first: labels.0 && levels.0,
        second: labels.1 && levels.1,
    };
    (ticks, labels)
}

/// Whether `axes.grid*` turns the gridlines of a tick level on.
fn grid_on_from_rc(axis: AxisName, major: bool, rc: &RcParams) -> bool {
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
    rc.axes_grid && level_on && axis_on
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
        assert!(!major.grid_on);
        let minor = TickStyle::from_rc(AxisName::Y, false, &rc);
        assert_eq!((minor.size, minor.width), (2.0, 0.6));
    }

    #[test]
    fn a_disabled_tick_level_hides_its_labels_too() {
        let mut rc = RcParams::matplotlib();
        rc.set("xtick.major.bottom", "False").expect("valid");
        rc.set("ytick.labelright", "True").expect("valid");
        rc.set("ytick.minor.right", "False").expect("valid");
        let x = TickStyle::from_rc(AxisName::X, true, &rc);
        assert!(!x.ticks.first && !x.labels.first);
        assert!(TickStyle::from_rc(AxisName::X, false, &rc).labels.first);
        let y_major = TickStyle::from_rc(AxisName::Y, true, &rc);
        assert!(y_major.labels.second, "major right labels stay on");
        assert!(!TickStyle::from_rc(AxisName::Y, false, &rc).labels.second);
    }

    #[test]
    fn levels_switched_off_lose_their_labels() {
        // matplotlib's test_rc_major_minor_tick.
        let mut rc = RcParams::matplotlib();
        rc.set("xtick.top", "True").expect("valid");
        rc.set("xtick.major.bottom", "False").expect("valid");
        let major = TickStyle::from_rc(AxisName::X, true, &rc);
        let both_off_but_top = Sides {
            first: false,
            second: true,
        };
        assert_eq!(major.ticks, both_off_but_top);
        assert_eq!(major.labels, Sides::default(), "labeltop is off by default");
        let minor = TickStyle::from_rc(AxisName::X, false, &rc);
        assert!(minor.ticks.first && minor.labels.first);
    }

    #[test]
    fn grid_rules() {
        let mut rc = RcParams::matplotlib();
        rc.set("axes.grid", "True").expect("valid");
        rc.set("axes.grid.axis", "y").expect("valid");
        assert!(!TickStyle::from_rc(AxisName::X, true, &rc).grid_on);
        let y = TickStyle::from_rc(AxisName::Y, true, &rc);
        assert!(y.grid_on);
        assert_eq!(y.grid.linewidth, 0.8);
        assert!(
            !TickStyle::from_rc(AxisName::Y, false, &rc).grid_on,
            "minor grid needs which=minor/both"
        );
    }
}
