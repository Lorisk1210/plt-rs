//! Tick and grid appearance (matplotlib's tick parameters).

use super::grid_props::{GridLineProps, GridLinePropsOptions};
use super::grid_stroke::{GridStroke, GridStrokeOptions};
use super::which::TickLevel;
use crate::colors::Color;
use crate::fonts::{FontProperties, FontStyle, FontWeight};
use crate::lines::LineStyle;
use crate::patheffects::PathEffect;
use crate::rc::keyword_names;
use crate::rc::{ColorOrInherit, RcParams};
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use pltrs_text::{HAlign, RotationMode, VAlign};

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

pltrs_core::archive_enum!(TickDirection { In, Out, InOut });

impl TickDirection {
    const ALL: [Self; 3] = [Self::In, Self::Out, Self::InOut];

    /// The fraction of the tick length extending outwards (matplotlib's
    /// `get_tick_padding` factors).
    pub const fn outward_fraction(self) -> f64 {
        match self {
            Self::In => 0.0,
            Self::Out => 1.0,
            Self::InOut => 0.5,
        }
    }

    /// matplotlib's name (`in`, `out`, `inout`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
            Self::InOut => "inout",
        }
    }
}

keyword_names!(TickDirection, "tick direction", Self::ALL);

/// Which sides of an axes show something: the first side is bottom (x)
/// or left (y), the second top or right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sides {
    /// Bottom or left.
    pub first: bool,
    /// Top or right.
    pub second: bool,
}

pltrs_core::archive_struct!(Sides { first, second });

impl Sides {
    /// Both sides.
    pub const BOTH: Self = Self {
        first: true,
        second: true,
    };
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
    /// The z-order of the gridlines (2, a line's, by default): a tick draws
    /// its gridline first whatever its z-order, as matplotlib's `Tick.draw`
    /// does, so this is only read back (`gridline.get_zorder`).
    pub zorder: f64,
    /// Their antialiasing, caps and joins.
    pub stroke: GridStroke,
    /// Their line properties beyond the stroke (draw style, gap colour,
    /// snapping, clipping, ...).
    pub line: GridLineProps,
}

pltrs_core::archive_struct!(GridStyle {
    color,
    alpha,
    linewidth,
    linestyle,
    path_effects,
    zorder,
    stroke,
    line
});

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
        let color = color.clone().unwrap_or_else(|| rc.grid_color.clone());
        // An alpha channel in the colour beats the rc alpha.
        let (color, alpha) = match color.split_alpha() {
            Some(split) => split,
            None => (color, alpha.unwrap_or(rc.grid_alpha)),
        };
        Self {
            color,
            alpha,
            linewidth: linewidth.unwrap_or(rc.grid_linewidth),
            linestyle: linestyle
                .clone()
                .unwrap_or_else(|| rc.grid_linestyle.clone()),
            path_effects: rc.path_effects.clone(),
            zorder: 2.0,
            stroke: GridStroke::from_rc(rc),
            line: GridLineProps::default(),
        }
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
    /// The z-order of the gridlines (see [`GridStyle::zorder`]).
    pub zorder: Option<f64>,
    /// Their antialiasing, caps and joins.
    pub stroke: GridStrokeOptions,
    /// Their line properties beyond the stroke.
    pub line: GridLinePropsOptions,
}

impl GridOptions {
    /// Applies the set options to `style`; without an alpha of its own,
    /// the alpha channel of the colour, if any, sets the alpha (as
    /// matplotlib's `Tick._apply_params` does).
    pub(crate) fn restyle(&self, style: &mut GridStyle) {
        if let Some(color) = &self.color {
            match (self.alpha, color.split_alpha()) {
                (None, Some((opaque, alpha))) => {
                    style.color = opaque;
                    style.alpha = alpha;
                }
                _ => style.color = color.clone(),
            }
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
        if let Some(zorder) = self.zorder {
            style.zorder = zorder;
        }
        self.stroke.apply(&mut style.stroke);
        self.line.apply(&mut style.line);
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
    /// Font style of the labels in place of `font.style`.
    pub label_style: Option<FontStyle>,
    /// Whether the labels count in layouts and in placing the axis label
    /// and the offset text (matplotlib's `set_in_layout` on tick labels).
    pub label_in_layout: bool,
    /// The hyperlink of the labels in vector output (matplotlib's
    /// `set_url` on a tick or its labels).
    pub label_url: Option<String>,
    /// Whether gridlines are drawn at the ticks.
    pub grid_on: bool,
    /// How the gridlines are drawn; kept while they are off, like
    /// matplotlib's `grid_*` tick parameters.
    pub grid: GridStyle,
}

pltrs_core::archive_struct!(TickStyle {
    size,
    width,
    pad,
    color,
    edgecolor,
    label_color,
    label_size,
    direction,
    ticks,
    labels,
    label_rotation,
    label_rotation_auto,
    label_rotation_mode,
    label_halign,
    label_valign,
    label_multialign,
    label_family,
    label_weight,
    label_style,
    label_in_layout,
    label_url,
    grid_on,
    grid
});

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
                rc.xtick_direction,
            ),
            AxisName::Y => (
                &rc.ytick_color,
                &rc.ytick_labelcolor,
                rc.ytick_labelsize,
                rc.ytick_direction,
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
            direction,
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
            label_style: None,
            label_in_layout: true,
            label_url: None,
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
    /// label family, weight and style when set.
    pub fn label_font(&self, rc: &RcParams) -> FontProperties {
        let font = FontProperties::from_rc(rc);
        FontProperties {
            size: self.label_size,
            families: self
                .label_family
                .clone()
                .map_or_else(|| font.families.clone(), |family| vec![family]),
            weight: self.label_weight.unwrap_or(font.weight),
            style: self.label_style.unwrap_or(font.style),
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
    grid_level_on(major, rc) && rc.axes_grid_axis.includes(axis)
}

/// Whether `axes.grid` and `axes.grid.which` turn the gridlines of the
/// major (`major = true`) or minor ticks on, whichever axes
/// `axes.grid.axis` names (matplotlib's `Axis._reset_major_tick_kw` and
/// `_reset_minor_tick_kw`).
pub(crate) fn grid_level_on(major: bool, rc: &RcParams) -> bool {
    let which = rc.axes_grid_which;
    rc.axes_grid && if major { which.major() } else { which.minor() }
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
    fn the_alpha_channel_of_a_grid_colour_sets_the_alpha() {
        // matplotlib's test_grid_color_with_alpha.
        let mut style = GridStyle::from_rc(true, &RcParams::matplotlib());
        let translucent = GridOptions {
            color: Some(Color::from([0.5, 0.6, 0.7, 0.3])),
            ..GridOptions::default()
        };
        translucent.restyle(&mut style);
        assert_eq!(style.alpha, 0.3);
        assert_eq!(style.color, Color::from([0.5, 0.6, 0.7]));
        let explicit = GridOptions {
            color: Some(Color::from([0.5, 0.6, 0.7, 0.3])),
            alpha: Some(0.8),
            ..GridOptions::default()
        };
        explicit.restyle(&mut style);
        assert_eq!(style.alpha, 0.8);
        let mut rc = RcParams::matplotlib();
        rc.set("grid.color", "#00000080").expect("valid");
        let from_rc = GridStyle::from_rc(false, &rc);
        assert_eq!(from_rc.alpha, 128.0 / 255.0);
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
    fn tick_directions_round_trip_their_names() {
        for direction in TickDirection::ALL {
            assert_eq!(direction.to_string().parse(), Ok(direction));
        }
        assert!("up".parse::<TickDirection>().is_err());
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
