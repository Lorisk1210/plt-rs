//! What a 3-D axis draws its gridlines, tick marks and label with, as a
//! program reads and writes it (matplotlib's `axis._axinfo`, whose
//! `grid`, `tick` and `label` entries its draws read every time).

use pltrs_text::{HAlign, RotationMode, VAlign};

use super::coords::Axis3;
use super::handle::Axes3D;
use crate::colors::Color;
use crate::lines::LineStyle;

/// The look of a 3-D axis that matplotlib's draws read from `_axinfo`.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisInfo {
    /// Gridline colour (`_axinfo['grid']['color']`).
    pub grid_color: Color,
    /// Gridline width in points (`['grid']['linewidth']`).
    pub grid_width: f64,
    /// Gridline style (`['grid']['linestyle']`).
    pub grid_style: LineStyle,
    /// Width of the major tick marks in points (`['tick']['linewidth'][True]`).
    pub major_tick_width: f64,
    /// Width of the minor tick marks in points (`['tick']['linewidth'][False]`).
    pub minor_tick_width: f64,
    /// How far the tick marks reach inwards, as a fraction of the tick
    /// offset (`['tick']['inward_factor']`).
    pub tick_inward: f64,
    /// How far they reach outwards (`['tick']['outward_factor']`).
    pub tick_outward: f64,
    /// The label's horizontal alignment (`['label']['ha']`).
    pub label_halign: HAlign,
    /// The label's vertical alignment (`['label']['va']`).
    pub label_valign: VAlign,
    /// How the label turns (`['label']['rotation_mode']`).
    pub label_rotation_mode: RotationMode,
}

impl Axes3D {
    /// The look of `axis` that matplotlib's `_axinfo` holds.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// let ax = Figure::new().subplot_3d();
    /// let mut info = ax.axis_info(Axis3::Y);
    /// assert_eq!((info.tick_inward, info.tick_outward), (0.2, 0.1));
    /// info.grid_width = 3.0;
    /// ax.set_axis_info(Axis3::Y, info);
    /// assert_eq!(ax.axis_info(Axis3::Y).grid_width, 3.0);
    /// ```
    pub fn axis_info(&self, axis: Axis3) -> AxisInfo {
        self.with(|axes| {
            let style = &axes.axis(axis).style;
            AxisInfo {
                grid_color: style.grid_color.clone(),
                grid_width: style.grid_width,
                grid_style: style.grid_style.clone(),
                major_tick_width: style.major_tick_width,
                minor_tick_width: style.minor_tick_width,
                tick_inward: style.tick_inward,
                tick_outward: style.tick_outward,
                label_halign: style.label_halign,
                label_valign: style.label_valign,
                label_rotation_mode: style.label_rotation_mode,
            }
        })
    }

    /// Draws `axis` with `info` from the next draw on (a program's writes
    /// to matplotlib's `_axinfo`, which its draws read).
    pub fn set_axis_info(&self, axis: Axis3, info: AxisInfo) -> &Self {
        self.with_mut(|axes| {
            let style = &mut axes.axis_mut(axis).style;
            style.grid_color = info.grid_color;
            style.grid_width = info.grid_width;
            style.grid_style = info.grid_style;
            style.major_tick_width = info.major_tick_width;
            style.minor_tick_width = info.minor_tick_width;
            style.tick_inward = info.tick_inward;
            style.tick_outward = info.tick_outward;
            style.label_halign = info.label_halign;
            style.label_valign = info.label_valign;
            style.label_rotation_mode = info.label_rotation_mode;
        });
        self
    }
}
