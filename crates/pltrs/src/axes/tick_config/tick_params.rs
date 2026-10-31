//! Tick parameters (matplotlib's `tick_params`): direction, size, colour,
//! label and gridline style, and the sides ticks and labels are drawn on.

use crate::axes::axis::TickOverrides;
use crate::axes::ticks::{GridOptions, TickDirection, TickStyle};
use crate::axes::{Axes, Axis, AxisName, TickLevel};
use crate::colors::Color;
use crate::fonts::FontSize;
use crate::lines::LineStyle;
use crate::patheffects::PathEffect;
use crate::rc::RcParams;
use crate::text::RotationMode;

/// Options of [`Axes::tick_params`] (matplotlib's `tick_params`); unset
/// fields keep their current value. The side switches only apply to the
/// axis drawn on that side (`bottom` and `top` to x, `left` and `right`
/// to y).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickParams {
    /// Only this axis (`None` = both).
    pub axis: Option<AxisName>,
    /// Which tick levels change.
    pub which: TickLevel,
    /// Restore the rc defaults before applying the rest.
    pub reset: bool,
    /// Tick direction.
    pub direction: Option<TickDirection>,
    /// Tick length in points.
    pub length: Option<f64>,
    /// Tick width in points.
    pub width: Option<f64>,
    /// Tick colour.
    pub color: Option<Color>,
    /// Gap between tick and label in points.
    pub pad: Option<f64>,
    /// Label font size.
    pub labelsize: Option<FontSize>,
    /// Label colour.
    pub labelcolor: Option<Color>,
    /// Tick and label colour (overrides `color` and `labelcolor`).
    pub colors: Option<Color>,
    /// Label rotation in degrees.
    pub labelrotation: Option<f64>,
    /// Whether the labels of polar axes turn with their ticks, with
    /// `labelrotation` (default 0) added: matplotlib's
    /// `labelrotation="auto"` (or `("auto", angle)`) for `Some(true)` and
    /// `"default"` for `Some(false)`. A `labelrotation` given alone is a
    /// fixed rotation.
    pub labelrotation_auto: Option<bool>,
    /// How rotated labels are aligned.
    pub labelrotation_mode: Option<RotationMode>,
    /// Label font family (`labelfontfamily`).
    pub labelfontfamily: Option<String>,
    /// Ticks on the bottom side.
    pub bottom: Option<bool>,
    /// Ticks on the top side.
    pub top: Option<bool>,
    /// Ticks on the left side.
    pub left: Option<bool>,
    /// Ticks on the right side.
    pub right: Option<bool>,
    /// Labels on the bottom side.
    pub labelbottom: Option<bool>,
    /// Labels on the top side.
    pub labeltop: Option<bool>,
    /// Labels on the left side.
    pub labelleft: Option<bool>,
    /// Labels on the right side.
    pub labelright: Option<bool>,
    /// Gridline colour (`grid_color`).
    pub grid_color: Option<Color>,
    /// Gridline opacity (`grid_alpha`).
    pub grid_alpha: Option<f64>,
    /// Gridline width in points (`grid_linewidth`).
    pub grid_linewidth: Option<f64>,
    /// Gridline style (`grid_linestyle`).
    pub grid_linestyle: Option<LineStyle>,
    /// Effects the gridlines are drawn through (`grid_path_effects`).
    pub grid_path_effects: Option<Vec<PathEffect>>,
}

impl TickParams {
    /// Applies the parameters to the style of one tick level of axis
    /// `name`.
    pub(crate) fn apply(&self, name: AxisName, major: bool, style: &mut TickStyle, rc: &RcParams) {
        if self.reset {
            *style = TickStyle::from_rc(name, major, rc);
        }
        let set = |slot: &mut f64, value: Option<f64>| {
            if let Some(value) = value {
                *slot = value;
            }
        };
        set(&mut style.size, self.length);
        set(&mut style.width, self.width);
        set(&mut style.pad, self.pad);
        if self.labelrotation.is_some() || self.labelrotation_auto.is_some() {
            style.label_rotation = self.labelrotation.unwrap_or(0.0);
            style.label_rotation_auto = self.labelrotation_auto.unwrap_or(false);
        }
        if let Some(direction) = self.direction {
            style.direction = direction;
        }
        if let Some(size) = self.labelsize {
            style.label_size = size.to_points(rc.font_size);
        }
        if let Some(mode) = self.labelrotation_mode {
            style.label_rotation_mode = mode;
        }
        if let Some(family) = &self.labelfontfamily {
            style.label_family = Some(family.clone());
        }
        let (color, labelcolor) = match &self.colors {
            Some(both) => (Some(both), Some(both)),
            None => (self.color.as_ref(), self.labelcolor.as_ref()),
        };
        if let Some(color) = color {
            style.color = color.clone();
        }
        if let Some(color) = labelcolor {
            style.label_color = color.clone();
        }
        GridOptions {
            color: self.grid_color.clone(),
            alpha: self.grid_alpha,
            linewidth: self.grid_linewidth,
            linestyle: self.grid_linestyle.clone(),
            path_effects: self.grid_path_effects.clone(),
            ..GridOptions::default()
        }
        .restyle(&mut style.grid);
        let (low, high, label_low, label_high) = match name {
            AxisName::X => (self.bottom, self.top, self.labelbottom, self.labeltop),
            AxisName::Y => (self.left, self.right, self.labelleft, self.labelright),
        };
        for (slot, value) in [
            (&mut style.ticks.first, low),
            (&mut style.ticks.second, high),
            (&mut style.labels.first, label_low),
            (&mut style.labels.second, label_high),
        ] {
            if let Some(value) = value {
                *slot = value;
            }
        }
    }
}

impl TickParams {
    /// Drops what single ticks have of their own among the properties
    /// these parameters set, all of it on a reset: matplotlib applies tick
    /// parameters to every existing tick (`Tick._apply_params`).
    fn clear_overridden(&self, name: AxisName, overrides: &mut TickOverrides) {
        if self.reset {
            *overrides = TickOverrides::default();
            return;
        }
        let colors = self.colors.is_some();
        let (low, high, label_low, label_high) = match name {
            AxisName::X => (self.bottom, self.top, self.labelbottom, self.labeltop),
            AxisName::Y => (self.left, self.right, self.labelleft, self.labelright),
        };
        for label in &mut overrides.labels {
            clear_if(&mut label.rotation, self.labelrotation.is_some());
            clear_if(&mut label.rotation_mode, self.labelrotation_mode.is_some());
            clear_if(&mut label.color, colors || self.labelcolor.is_some());
            clear_if(&mut label.fontsize, self.labelsize.is_some());
            clear_if(&mut label.family, self.labelfontfamily.is_some());
            clear_if(&mut label.label1_visible, label_low.is_some());
            clear_if(&mut label.label2_visible, label_high.is_some());
        }
        for line in &mut overrides.lines {
            let color = colors || self.color.is_some();
            clear_if(&mut line.color, color);
            clear_if(&mut line.markeredgecolor, color);
            clear_if(&mut line.size, self.length.is_some());
            clear_if(&mut line.width, self.width.is_some());
            clear_if(&mut line.tick1_visible, low.is_some());
            clear_if(&mut line.tick2_visible, high.is_some());
            clear_if(&mut line.grid_color, self.grid_color.is_some());
            clear_if(&mut line.grid_alpha, self.grid_alpha.is_some());
            clear_if(&mut line.grid_linewidth, self.grid_linewidth.is_some());
            clear_if(&mut line.grid_linestyle, self.grid_linestyle.is_some());
        }
    }
}

/// Empties `slot` when `set`.
fn clear_if<T>(slot: &mut Option<T>, set: bool) {
    if set {
        *slot = None;
    }
}

impl Axis {
    /// Changes the tick parameters of this axis (matplotlib's
    /// `Axis.set_tick_params`); `params.axis` is ignored. The properties
    /// set apply to every tick of the levels changed, those of single
    /// ticks included.
    pub fn set_tick_params(&self, params: TickParams) -> &Self {
        let rc = crate::rc::params();
        let name = self.name();
        self.update(|axis| {
            if params.which.major() {
                params.clear_overridden(name, &mut axis.major_overrides);
                params.apply(name, true, &mut axis.major_style, &rc);
                // The tick-space heuristic follows the major label size of
                // the tick parameters (matplotlib's `_get_tick_label_size`).
                if params.reset {
                    axis.label_size = None;
                }
                if params.labelsize.is_some() {
                    axis.label_size = Some(axis.major_style.label_size);
                }
            }
            if params.which.minor() {
                params.clear_overridden(name, &mut axis.minor_overrides);
                params.apply(name, false, &mut axis.minor_style, &rc);
            }
        });
        self
    }
}

impl Axes {
    /// Changes the tick parameters of one or both axes (matplotlib's
    /// `tick_params`).
    pub fn tick_params(&self, params: TickParams) -> &Self {
        for name in [AxisName::X, AxisName::Y] {
            if params.axis.is_none_or(|only| only == name) {
                self.axis_of(name).set_tick_params(params.clone());
            }
        }
        self
    }
}
