//! Handles to the parts of an axis artist: its tick marks, tick labels,
//! label and line (matplotlib's `Ticks`, `TickLabels`, `AxisLabel` and the
//! axis line).

use std::ops::Deref;

use pltrs_core::{Bbox, Point};

use super::{AxisArtist, opposite};
use crate::axes::TickDirection;
use crate::axisartist::artist::Level;
use crate::axisartist::labels::{LabelState, TextLook};
use crate::axisartist::line::LineState;
use crate::axisartist::ticks::TicksState;
use crate::colors::Color;
use crate::spines::SpineSide;
use crate::text::{HAlign, VAlign};

/// The tick marks of one level of an axis artist (matplotlib's `Ticks`).
#[derive(Clone, Debug, PartialEq)]
pub struct Ticks {
    pub(super) artist: AxisArtist,
    pub(super) level: Level,
}

impl Ticks {
    fn update(&self, f: impl FnOnce(&mut TicksState)) -> &Self {
        let level = self.level;
        self.artist.update(|state| f(state.ticks_mut(level)));
        self
    }

    fn get<R>(&self, f: impl FnOnce(&TicksState) -> R) -> R {
        self.artist.get(|state| {
            f(match self.level {
                Level::Major => &state.major_ticks,
                Level::Minor => &state.minor_ticks,
            })
        })
    }

    /// Sets the length of the marks in points (matplotlib's
    /// `set_ticksize`).
    pub fn set_ticksize(&self, size: f64) -> &Self {
        self.update(|ticks| ticks.size = size)
    }

    /// The length of the marks in points.
    pub fn ticksize(&self) -> f64 {
        self.get(|ticks| ticks.size)
    }

    /// Points the marks into the axes, out of it or across the line
    /// (matplotlib's `set_tick_direction`).
    pub fn set_tick_direction(&self, direction: TickDirection) -> &Self {
        self.update(|ticks| ticks.direction = direction)
    }

    /// Where the marks point.
    pub fn tick_direction(&self) -> TickDirection {
        self.get(|ticks| ticks.direction)
    }

    /// Points the marks out of the axes, or into it (matplotlib's
    /// `set_tick_out`).
    pub fn set_tick_out(&self, out: bool) -> &Self {
        self.set_tick_direction(if out {
            TickDirection::Out
        } else {
            TickDirection::In
        })
    }

    /// Shows or hides the marks.
    pub fn set_visible(&self, visible: bool) -> &Self {
        self.update(|ticks| ticks.visible = visible)
    }

    /// Whether the marks are drawn.
    pub fn visible(&self) -> bool {
        self.get(|ticks| ticks.visible)
    }

    /// Colours the marks; `None` follows the reference axis' ticks
    /// (matplotlib's `set_color`, `"auto"` for `None`).
    pub fn set_color(&self, color: Option<Color>) -> &Self {
        self.update(|ticks| ticks.color = color)
    }

    /// Sets the line width of the marks in points; `None` follows the
    /// reference axis' ticks (matplotlib's `set_markeredgewidth`).
    pub fn set_markeredgewidth(&self, width: Option<f64>) -> &Self {
        self.update(|ticks| ticks.width = width)
    }

    /// The colour of the marks; `None` when they follow the reference
    /// axis' ticks (matplotlib's `"auto"`).
    pub fn color(&self) -> Option<Color> {
        self.get(|ticks| ticks.color.clone())
    }

    /// The line width of the marks in points; `None` when it follows the
    /// reference axis' ticks (matplotlib's `"auto"`).
    pub fn markeredgewidth(&self) -> Option<f64> {
        self.get(|ticks| ticks.width)
    }

    /// Sets the opacity of the marks.
    pub fn set_alpha(&self, alpha: Option<f64>) -> &Self {
        self.update(|ticks| ticks.alpha = alpha)
    }

    /// Where the marks are in the figure's current layout (laid out
    /// first): their locations in the coordinates of the helper's tick
    /// transform and their angles in degrees (matplotlib's `locs_angles`
    /// after a draw); empty once the artist was removed.
    ///
    /// ```
    /// use pltrs::axisartist::AxisArtist;
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// let bottom = AxisArtist::add_fixed(&ax, SpineSide::Bottom);
    /// let marks = bottom.major_ticks().locs_angles();
    /// assert!(!marks.is_empty() && marks.iter().all(|&(_, angle)| angle == 90.0));
    /// ```
    pub fn locs_angles(&self) -> Vec<(Point, f64)> {
        self.artist.locs_angles(self.level)
    }
}

/// The texts of an axis artist: its tick labels of one level or its
/// label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    TickLabels(Level),
    Label,
}

/// What tick labels and the axis label have in common (matplotlib's
/// `LabelBase` and the `AxisLabel` setters).
#[derive(Clone, Debug, PartialEq)]
pub struct LabelBase {
    pub(super) artist: AxisArtist,
    pub(super) part: Part,
}

impl LabelBase {
    fn update(&self, f: impl FnOnce(&mut LabelState)) -> &Self {
        let part = self.part;
        self.artist.update(|state| {
            f(match part {
                Part::TickLabels(level) => state.ticklabels_mut(level),
                Part::Label => &mut state.label,
            });
        });
        self
    }

    fn get<R>(&self, f: impl FnOnce(&LabelState) -> R) -> R {
        self.artist.get(|state| {
            f(match self.part {
                Part::TickLabels(Level::Major) => &state.major_ticklabels,
                Part::TickLabels(Level::Minor) => &state.minor_ticklabels,
                Part::Label => &state.label,
            })
        })
    }

    /// Adopts matplotlib's angle and alignment for an axis on `direction`
    /// (matplotlib's `set_axis_direction`).
    pub fn set_axis_direction(&self, direction: SpineSide) -> &Self {
        self.update(|label| label.set_axis_direction(direction))
    }

    /// The axis direction.
    pub fn axis_direction(&self) -> SpineSide {
        self.get(|label| label.direction)
    }

    /// Sets the pad in points: from the ticks for tick labels, from the
    /// tick labels for the label (matplotlib's `set_pad`).
    pub fn set_pad(&self, pad: f64) -> &Self {
        self.update(|label| label.pad = pad)
    }

    /// The pad in points.
    pub fn pad(&self) -> f64 {
        self.get(|label| label.pad)
    }

    /// Shows or hides the text(s).
    pub fn set_visible(&self, visible: bool) -> &Self {
        self.update(|label| label.visible = visible)
    }

    /// Whether the text(s) are drawn.
    pub fn visible(&self) -> bool {
        self.get(|label| label.visible)
    }

    /// Colours the text(s); `None` returns to the default (`text.color`
    /// for tick labels, the axis label's colour for the label).
    pub fn set_color(&self, color: Option<Color>) -> &Self {
        self.update(|label| label.color = color)
    }

    /// The colour of the text(s); `None` for the default (matplotlib's
    /// `"auto"`).
    pub fn color(&self) -> Option<Color> {
        self.get(|label| label.color.clone())
    }

    /// Sets the text angle in degrees, relative to 90° plus the direction
    /// from the line to the text (matplotlib's `set_rotation`).
    pub fn set_rotation(&self, degrees: f64) -> &Self {
        self.update(|label| label.rotation = degrees.rem_euclid(360.0))
    }

    /// The text angle in degrees.
    pub fn rotation(&self) -> f64 {
        self.get(|label| label.rotation)
    }

    /// Sets the horizontal alignment (matplotlib's `set_ha`).
    pub fn set_ha(&self, halign: HAlign) -> &Self {
        self.update(|label| label.halign = halign)
    }

    /// Sets the vertical alignment (matplotlib's `set_va`).
    pub fn set_va(&self, valign: VAlign) -> &Self {
        self.update(|label| label.valign = valign)
    }

    /// Sets the font size in points (matplotlib's `set_size`).
    pub fn set_size(&self, size: f64) -> &Self {
        self.update(|label| label.size = size)
    }

    /// The font size in points.
    pub fn size(&self) -> f64 {
        self.get(|label| label.size)
    }

    /// Typesets the text(s) as `look` says: in its font at their own
    /// size, their lines aligned and spaced as it says, rotated in its
    /// rotation mode (matplotlib's `set_fontproperties`, `set_family`,
    /// `set_linespacing`, `set_multialignment`, `set_rotation_mode`, ...).
    ///
    /// ```
    /// use pltrs::axisartist::AxisArtist;
    /// use pltrs::figure::Figure;
    /// use pltrs::fonts::FontWeight;
    /// use pltrs::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// let label = AxisArtist::add_fixed(&ax, SpineSide::Left).label();
    /// let mut look = label.look();
    /// look.font = look.font.with_weight(FontWeight::BOLD);
    /// label.set_look(look.clone());
    /// assert_eq!(label.look(), look);
    /// ```
    pub fn set_look(&self, look: TextLook) -> &Self {
        self.update(|label| label.look = look)
    }

    /// How the text(s) are typeset beyond their size, colour, angle and
    /// alignment.
    pub fn look(&self) -> TextLook {
        self.get(|label| label.look.clone())
    }
}

/// The tick labels of one level of an axis artist (matplotlib's
/// `TickLabels`); the common settings come from [`LabelBase`].
#[derive(Clone, Debug, PartialEq)]
pub struct TickLabels(pub(super) LabelBase);

impl Deref for TickLabels {
    type Target = LabelBase;

    fn deref(&self) -> &LabelBase {
        &self.0
    }
}

impl TickLabels {
    /// Adopts the angle and alignment of the opposite axis direction
    /// (matplotlib's `invert_axis_direction`).
    pub fn invert_axis_direction(&self) -> &Self {
        self.0
            .update(|label| label.set_axis_direction(opposite(label.direction)));
        self
    }

    /// The level of the tick labels.
    fn level(&self) -> Level {
        match self.0.part {
            Part::TickLabels(level) => level,
            Part::Label => Level::Major,
        }
    }

    /// The display boxes of the tick labels in the figure's current
    /// layout, none when they are hidden (matplotlib's
    /// `get_window_extents`).
    ///
    /// ```
    /// use pltrs::axisartist::AxisArtist;
    /// use pltrs::figure::Figure;
    /// use pltrs::spines::SpineSide;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let left = AxisArtist::add_fixed(&ax, SpineSide::Left);
    /// assert!(!left.major_ticklabels().window_extents().is_empty());
    /// assert_eq!(
    ///     left.major_ticklabels().window_extents().len(),
    ///     left.major_ticklabels().texts_widths_heights_descents().len()
    /// );
    /// ```
    pub fn window_extents(&self) -> Vec<Bbox> {
        let level = self.level();
        self.0
            .artist
            .with_layout(|artist, layout| artist.ticklabel_extents(layout, level))
    }

    /// The width, height and descent in pixels of each non-empty tick
    /// label, in the figure's current layout (matplotlib's
    /// `get_texts_widths_heights_descents`).
    pub fn texts_widths_heights_descents(&self) -> Vec<(f64, f64, f64)> {
        let level = self.level();
        self.0
            .artist
            .with_layout(|artist, layout| artist.ticklabel_metrics(layout, level))
    }
}

/// The label of an axis artist (matplotlib's `AxisLabel`); the common
/// settings come from [`LabelBase`].
#[derive(Clone, Debug, PartialEq)]
pub struct AxisLabel(pub(super) LabelBase);

impl Deref for AxisLabel {
    type Target = LabelBase;

    fn deref(&self) -> &LabelBase {
        &self.0
    }
}

impl AxisLabel {
    /// Sets the label text; by default the label shows that of the axes'
    /// x or y axis (matplotlib's `set_text`).
    pub fn set_text(&self, text: impl Into<String>) -> &Self {
        let text = text.into();
        self.0.artist.update(|state| state.label_text = Some(text));
        self
    }

    /// The label text set on the artist, if any.
    pub fn text(&self) -> Option<String> {
        self.0.artist.get(|state| state.label_text.clone())
    }
}

/// The line of an axis artist (matplotlib's `AxisArtist.line`).
#[derive(Clone, Debug, PartialEq)]
pub struct AxisLine {
    pub(super) artist: AxisArtist,
}

impl AxisLine {
    fn update(&self, f: impl FnOnce(&mut LineState)) -> &Self {
        self.artist.update(|state| f(&mut state.line));
        self
    }

    /// Colours the line; `None` returns to the default.
    pub fn set_color(&self, color: Option<Color>) -> &Self {
        self.update(|line| line.color = color)
    }

    /// Sets the line width in points; `None` returns to the default.
    pub fn set_linewidth(&self, width: Option<f64>) -> &Self {
        self.update(|line| line.linewidth = width)
    }

    /// Shows or hides the line.
    pub fn set_visible(&self, visible: bool) -> &Self {
        self.update(|line| line.visible = visible)
    }

    /// Whether the line is drawn.
    pub fn visible(&self) -> bool {
        self.artist.get(|state| state.line.visible)
    }
}
