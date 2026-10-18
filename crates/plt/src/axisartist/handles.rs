//! Handles to an axis artist and to its parts (matplotlib's `AxisArtist`
//! with its `line`, `major_ticks`, `minor_ticks`, `major_ticklabels`,
//! `minor_ticklabels` and `label` children).

use std::ops::Deref;

use crate::text::{HorizontalAlignment, VerticalAlignment};

use super::artist::{AxisArtistState, Level};
use super::helper::AxisHelper;
use super::labels::LabelState;
use super::line::{AxislineStyle, LineState};
use super::ticks::TicksState;
use crate::artist::{ArtistHandle, RawHandle};
use crate::axes::TickDirection;
use crate::colors::Color;
use crate::spines::SpineSide;
use crate::ticker::AxisName;

/// Which side of the line, relative to the direction of the increasing
/// coordinate, labels go (matplotlib's `"+"` and `"-"`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LabelDirection {
    /// To the right of the increasing coordinate (`"+"`): below a
    /// horizontal line, right of a vertical one.
    #[default]
    Plus,
    /// To the left (`"-"`).
    Minus,
}

impl LabelDirection {
    /// The turn from the right of the line, in degrees.
    pub(crate) fn add_angle(self) -> f64 {
        match self {
            Self::Plus => 0.0,
            Self::Minus => 180.0,
        }
    }

    /// The other side.
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Self::Plus => Self::Minus,
            Self::Minus => Self::Plus,
        }
    }
}

/// Which parts of an axis artist to show or hide (matplotlib's
/// `AxisArtist.toggle`); `all` applies first, the others override it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Toggle {
    /// Every part.
    pub all: Option<bool>,
    /// The tick marks.
    pub ticks: Option<bool>,
    /// The tick labels.
    pub ticklabels: Option<bool>,
    /// The axis label.
    pub label: Option<bool>,
}

/// A handle to an axis artist of an [`Axes`](super::Axes) (matplotlib's
/// `AxisArtist`).
#[derive(Clone, Debug, PartialEq)]
pub struct AxisArtist {
    raw: RawHandle,
}

impl ArtistHandle for AxisArtist {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl AxisArtist {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    pub(crate) fn update(&self, f: impl FnOnce(&mut AxisArtistState)) -> &Self {
        self.raw.with_mut::<AxisArtistState, _>(f);
        self
    }

    fn get<R>(&self, f: impl FnOnce(&AxisArtistState) -> R) -> R {
        self.raw.with::<AxisArtistState, _>(f)
    }

    /// Its key in the axis dictionary.
    pub fn name(&self) -> String {
        self.get(|state| state.name.clone())
    }

    /// Adjusts the side, angle and alignment of the major tick labels and
    /// the label to matplotlib's conventions for an axis on `direction`
    /// (matplotlib's `set_axis_direction`): left and top put them on the
    /// `-` side of the line, bottom and right on the `+` side.
    pub fn set_axis_direction(&self, direction: SpineSide) -> &Self {
        self.update(|state| state.set_axis_direction(direction))
    }

    /// Puts the tick labels on `direction`'s side of the line
    /// (matplotlib's `set_ticklabel_direction`).
    pub fn set_ticklabel_direction(&self, direction: LabelDirection) -> &Self {
        self.update(|state| state.ticklabel_direction = direction)
    }

    /// Puts the label on `direction`'s side of the line (matplotlib's
    /// `set_axislabel_direction`).
    pub fn set_axislabel_direction(&self, direction: LabelDirection) -> &Self {
        self.update(|state| state.axislabel_direction = direction)
    }

    /// Moves the tick labels to the other side of the line, with the
    /// alignment of the opposite direction (matplotlib's
    /// `invert_ticklabel_direction`).
    pub fn invert_ticklabel_direction(&self) -> &Self {
        self.update(|state| {
            state.ticklabel_direction = state.ticklabel_direction.opposite();
            for labels in [&mut state.major_ticklabels, &mut state.minor_ticklabels] {
                labels.set_axis_direction(opposite(labels.direction));
            }
        })
    }

    /// Draws the line with an arrow head past its end, or plainly with
    /// `None` (matplotlib's `set_axisline_style`). Like matplotlib, this
    /// makes a new line: its colour, width and clipping are reset.
    pub fn set_axisline_style(&self, style: Option<AxislineStyle>) -> &Self {
        self.update(|state| {
            state.line = LineState {
                style,
                ..LineState::default()
            };
        })
    }

    /// The axis line style (matplotlib's `get_axisline_style`).
    pub fn axisline_style(&self) -> Option<AxislineStyle> {
        self.get(|state| state.line.style.clone())
    }

    /// The gap between the tick labels and the label, in points
    /// (matplotlib's `LABELPAD`).
    pub fn labelpad(&self) -> f64 {
        self.get(|state| state.label.pad)
    }

    /// Sets the gap between the tick labels and the label (matplotlib's
    /// `LABELPAD = ...`).
    pub fn set_labelpad(&self, pad: f64) -> &Self {
        self.update(|state| state.label.pad = pad)
    }

    /// Labels the ticks of curvilinear coordinate `nth` on an edge of a
    /// curvilinear grid; the other coordinate's ticks stay unlabelled
    /// (matplotlib's `get_helper().nth_coord_ticks = ...`). Other axis
    /// lines ignore it.
    pub fn set_nth_coord_ticks(&self, nth: AxisName) -> &Self {
        self.update(|state| {
            if let AxisHelper::CurveFixed { nth_ticks, .. } = &mut state.helper {
                *nth_ticks = nth;
            }
        })
    }

    /// Shows or hides the tick marks, tick labels and label (matplotlib's
    /// `toggle`).
    pub fn toggle(&self, toggle: &Toggle) -> &Self {
        let ticks = toggle.ticks.or(toggle.all);
        let ticklabels = toggle.ticklabels.or(toggle.all);
        let label = toggle.label.or(toggle.all);
        self.update(|state| {
            if let Some(visible) = ticks {
                state.major_ticks.visible = visible;
                state.minor_ticks.visible = visible;
            }
            if let Some(visible) = ticklabels {
                state.major_ticklabels.visible = visible;
                state.minor_ticklabels.visible = visible;
            }
            if let Some(visible) = label {
                state.label.visible = visible;
            }
        })
    }

    /// The major tick marks (matplotlib's `major_ticks`).
    pub fn major_ticks(&self) -> Ticks {
        self.ticks(Level::Major)
    }

    /// The minor tick marks (matplotlib's `minor_ticks`).
    pub fn minor_ticks(&self) -> Ticks {
        self.ticks(Level::Minor)
    }

    fn ticks(&self, level: Level) -> Ticks {
        Ticks {
            artist: self.clone(),
            level,
        }
    }

    /// The major tick labels (matplotlib's `major_ticklabels`).
    pub fn major_ticklabels(&self) -> TickLabels {
        TickLabels(self.label_base(Part::TickLabels(Level::Major)))
    }

    /// The minor tick labels (matplotlib's `minor_ticklabels`).
    pub fn minor_ticklabels(&self) -> TickLabels {
        TickLabels(self.label_base(Part::TickLabels(Level::Minor)))
    }

    /// The axis label (matplotlib's `label`).
    pub fn label(&self) -> AxisLabel {
        AxisLabel(self.label_base(Part::Label))
    }

    fn label_base(&self, part: Part) -> LabelBase {
        LabelBase {
            artist: self.clone(),
            part,
        }
    }

    /// The axis line (matplotlib's `line`).
    pub fn line(&self) -> AxisLine {
        AxisLine {
            artist: self.clone(),
        }
    }
}

/// The opposite axis direction.
fn opposite(direction: SpineSide) -> SpineSide {
    SpineSide::of(direction.axis(), !direction.is_high())
}

/// The tick marks of one level of an axis artist (matplotlib's `Ticks`).
#[derive(Clone, Debug, PartialEq)]
pub struct Ticks {
    artist: AxisArtist,
    level: Level,
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

    /// Sets the opacity of the marks.
    pub fn set_alpha(&self, alpha: Option<f64>) -> &Self {
        self.update(|ticks| ticks.alpha = alpha)
    }
}

/// The texts of an axis artist: its tick labels of one level or its
/// label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    TickLabels(Level),
    Label,
}

/// What tick labels and the axis label have in common (matplotlib's
/// `LabelBase` and the `AxisLabel` setters).
#[derive(Clone, Debug, PartialEq)]
pub struct LabelBase {
    artist: AxisArtist,
    part: Part,
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
    pub fn set_ha(&self, halign: HorizontalAlignment) -> &Self {
        self.update(|label| label.halign = halign)
    }

    /// Sets the vertical alignment (matplotlib's `set_va`).
    pub fn set_va(&self, valign: VerticalAlignment) -> &Self {
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
}

/// The tick labels of one level of an axis artist (matplotlib's
/// `TickLabels`); the common settings come from [`LabelBase`].
#[derive(Clone, Debug, PartialEq)]
pub struct TickLabels(LabelBase);

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
}

/// The label of an axis artist (matplotlib's `AxisLabel`); the common
/// settings come from [`LabelBase`].
#[derive(Clone, Debug, PartialEq)]
pub struct AxisLabel(LabelBase);

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
    artist: AxisArtist,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axisartist::Axes;
    use crate::figure::Figure;

    #[test]
    fn directions_turn_labels_to_the_other_side() {
        let fig = Figure::new();
        let ax = Axes::new(fig.subplot());
        let left = ax.axis("left").expect("an edge");
        left.raw().with::<AxisArtistState, _>(|state| {
            assert_eq!(state.ticklabel_direction, LabelDirection::Minus);
            assert_eq!(state.axislabel_direction, LabelDirection::Minus);
        });
        left.set_axis_direction(SpineSide::Right);
        assert_eq!(left.major_ticklabels().axis_direction(), SpineSide::Right);
        assert_eq!(left.minor_ticklabels().axis_direction(), SpineSide::Left);
        left.invert_ticklabel_direction();
        assert_eq!(left.major_ticklabels().axis_direction(), SpineSide::Left);
        left.raw().with::<AxisArtistState, _>(|state| {
            assert_eq!(state.ticklabel_direction, LabelDirection::Minus);
            assert_eq!(state.axislabel_direction, LabelDirection::Plus);
        });
        left.set_axislabel_direction(LabelDirection::Minus);
        left.raw().with::<AxisArtistState, _>(|state| {
            assert_eq!(state.axislabel_direction, LabelDirection::Minus);
        });
    }

    #[test]
    fn parts_are_styled_separately() {
        let fig = Figure::new();
        let ax = Axes::new(fig.subplot());
        let bottom = ax.axis("bottom").expect("an edge");
        bottom.major_ticks().set_tick_out(true).set_ticksize(8.0);
        bottom.minor_ticks().set_visible(false);
        bottom.major_ticklabels().set_rotation(-30.0);
        bottom.label().set_text("time").set_pad(12.0);
        assert_eq!(bottom.major_ticks().tick_direction(), TickDirection::Out);
        assert_eq!(bottom.major_ticks().ticksize(), 8.0);
        assert!(bottom.major_ticks().visible() && !bottom.minor_ticks().visible());
        assert_eq!(bottom.major_ticklabels().rotation(), 330.0);
        assert_eq!(bottom.label().text().as_deref(), Some("time"));
        assert_eq!(bottom.labelpad(), 12.0);
        bottom.toggle(&Toggle {
            all: Some(false),
            ticks: Some(true),
            ..Toggle::default()
        });
        assert!(bottom.major_ticks().visible() && !bottom.major_ticklabels().visible());
        bottom.line().set_visible(false);
        assert!(!bottom.line().visible());
        bottom.set_axisline_style(Some(AxislineStyle::SimpleArrow { size: 1.0 }));
        assert!(bottom.line().visible(), "a new line");
    }
}
