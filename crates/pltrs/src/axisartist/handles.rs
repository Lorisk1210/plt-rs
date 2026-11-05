//! Handles to an axis artist and to its parts (matplotlib's `AxisArtist`
//! with its `line`, `major_ticks`, `minor_ticks`, `major_ticklabels`,
//! `minor_ticklabels` and `label` children).

use pltrs_core::Path;

use super::artist::{AxisArtistState, Level};
use super::axes::adopt_axis_artist;
use super::helper::{AxisHelper, TickInfo};
use super::line::{AxislineStyle, LineState};
use crate::artist::{ArtistHandle, LayoutContext, RawHandle, TypedHandle};
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::{Bbox, Point, Transform};

mod parts;

use self::parts::Part;
pub use self::parts::{AxisLabel, AxisLine, LabelBase, TickLabels, Ticks};

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

pltrs_core::archive_enum!(LabelDirection { Plus, Minus });

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

/// A handle to an axis artist of an [`AxisArtistAxes`](super::AxisArtistAxes) (matplotlib's
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

impl TypedHandle for AxisArtist {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<AxisArtistState>()
            .then(|| Self::from_raw(raw.clone()))
    }
}

impl AxisArtist {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// Adds an axis artist along the edge `loc` of any axes, ticked and
    /// labelled from the axes' own axis on that side, with every part
    /// shown and the tick labels and label facing `loc` (matplotlib's
    /// `ax.add_artist(AxisArtist(ax, AxisArtistHelperRectlinear.Fixed(ax,
    /// loc), axis_direction=loc))`). Unlike the edges of axisartist
    /// [`AxisArtistAxes`](super::AxisArtistAxes), the axes keeps its spines and axes; hide
    /// those as needed.
    ///
    /// ```
    /// use pltrs::axisartist::AxisArtist;
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// let right = AxisArtist::add_fixed(&ax, SpineSide::Right);
    /// assert!(right.major_ticklabels().visible() && right.label().visible());
    /// assert_eq!(right.name(), "right");
    /// ```
    pub fn add_fixed(ax: &crate::axes::Axes, loc: SpineSide) -> Self {
        adopt_axis_artist(ax, loc.name(), AxisHelper::Fixed(loc), (loc, (0.0, 0.0)))
    }

    pub(crate) fn update(&self, f: impl FnOnce(&mut AxisArtistState)) -> &Self {
        self.raw.with_mut::<AxisArtistState, _>(f);
        self
    }

    fn get<R>(&self, f: impl FnOnce(&AxisArtistState) -> R) -> R {
        self.raw.with::<AxisArtistState, _>(f)
    }

    /// What matplotlib's `AxisArtist` computes when it is made
    /// (`_init_line`, `get_line`): for a curvilinear line of constant
    /// coordinate, the grid over the axes' view limits (computed again
    /// only for other limits) and the line, which call the program's
    /// transforms of the grid.
    pub fn init_line(&self) {
        let axes = self.get(AxisArtistState::axes);
        let cell = self.raw.figure_state();
        crate::axes::Axes::from_raw(RawHandle::new(std::rc::Rc::clone(cell), axes)).unstale_view();
        let state = cell.borrow();
        if let Some(artist) = state.store.get_as::<AxisArtistState>(self.raw.id()) {
            // A line that cannot be computed fails the draw instead.
            let _ = artist.init_line(LayoutContext::new(&state));
        }
    }

    /// Its key in the axis dictionary.
    pub fn name(&self) -> String {
        self.get(|state| state.name.clone())
    }

    /// The extent of what the axis artist draws (its line, ticks, tick
    /// labels and label) in display pixels, in the figure's current layout
    /// (matplotlib's `AxisArtist.get_tightbbox`); `None` when it draws
    /// nothing or was removed.
    pub fn tight_bbox(&self) -> Option<Bbox> {
        let state = self.raw.figure_state().prepared();
        let layout = LayoutContext::new(&state);
        state.store.get(self.raw.id())?.tight_extent(&layout)
    }

    /// The major and minor ticks its helper places in the figure's
    /// current layout (matplotlib's `get_helper().get_tick_iterators(axes)`);
    /// none when the axes is gone.
    ///
    /// ```
    /// use pltrs::axisartist::AxisArtist;
    /// use pltrs::figure::Figure;
    /// use pltrs::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// let bottom = AxisArtist::add_fixed(&ax, SpineSide::Bottom);
    /// let (major, _) = bottom.helper_ticks();
    /// assert_eq!(major[0].label, "0.0");
    /// assert_eq!((major[0].angle_normal, major[0].angle_tangent), (90.0, 0.0));
    /// ```
    pub fn helper_ticks(&self) -> (Vec<TickInfo>, Vec<TickInfo>) {
        self.with_layout(AxisArtistState::helper_ticks)
    }

    /// The line its helper runs along in the figure's current layout, and
    /// the coordinates of the line (matplotlib's `get_helper().get_line`
    /// and `get_line_transform`); `None` when the axes is gone.
    pub fn helper_line(&self) -> Option<(Path, Transform)> {
        self.with_layout(AxisArtistState::helper_line)
    }

    /// Where its helper puts the label in the figure's current layout, in
    /// display pixels, and the direction of the line there in degrees
    /// (matplotlib's `get_helper().get_axislabel_pos_angle` through
    /// `get_axislabel_transform`); `None` for a line outside the axes.
    pub fn helper_label_position(&self) -> Option<(Point, f64)> {
        self.with_layout(AxisArtistState::helper_label_position)
    }

    /// The tick marks of `level` in the figure's current layout (see
    /// [`Ticks::locs_angles`]).
    fn locs_angles(&self, level: Level) -> Vec<(Point, f64)> {
        self.with_layout(|artist, layout| artist.tick_locs_angles(layout, level))
    }

    /// What `f` finds of the artist in the figure's current layout; the
    /// default when the artist is gone or `f` fails.
    fn with_layout<R: Default>(
        &self,
        f: impl FnOnce(&AxisArtistState, LayoutContext<'_>) -> Result<R, String>,
    ) -> R {
        let state = self.raw.figure_state().prepared();
        let layout = LayoutContext::new(&state);
        state
            .store
            .get_as::<AxisArtistState>(self.raw.id())
            .and_then(|artist| f(artist, layout).ok())
            .unwrap_or_default()
    }

    /// Renames the axis artist (matplotlib's `ax.axis[name] = artist`).
    pub fn set_name(&self, name: impl Into<String>) -> &Self {
        let name = name.into();
        self.update(|state| state.name = name)
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

    /// The side of the line the tick labels go (matplotlib's
    /// `_ticklabel_add_angle`, 0 for [`LabelDirection::Plus`]).
    ///
    /// ```
    /// use pltrs::axisartist::{AxisArtist, LabelDirection};
    /// use pltrs::figure::Figure;
    /// use pltrs::spines::SpineSide;
    /// let ax = Figure::new().subplot();
    /// let left = AxisArtist::add_fixed(&ax, SpineSide::Left);
    /// assert_eq!(left.ticklabel_direction(), LabelDirection::Minus);
    /// left.set_ticklabel_direction(LabelDirection::Plus);
    /// assert_eq!(left.ticklabel_direction(), LabelDirection::Plus);
    /// assert_eq!(left.axislabel_direction(), LabelDirection::Minus);
    /// ```
    pub fn ticklabel_direction(&self) -> LabelDirection {
        self.get(|state| state.ticklabel_direction)
    }

    /// The side of the line the label goes (matplotlib's
    /// `_axislabel_add_angle`).
    pub fn axislabel_direction(&self) -> LabelDirection {
        self.get(|state| state.axislabel_direction)
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

    /// Limits a floating axis line of a curvilinear grid to where its
    /// varying coordinate lies between `min` and `max` (matplotlib's
    /// `get_helper().set_extremes(min, max)`). Other axis lines ignore it.
    pub fn set_extremes(&self, min: f64, max: f64) -> &Self {
        self.update(|state| {
            if let AxisHelper::CurveFloating { line, .. } = &mut state.helper {
                line.extremes = (min, max);
            }
        })
    }

    /// Shows or hides the tick marks, tick labels and label (matplotlib's
    /// `toggle`).
    pub fn toggle(&self, toggle: Toggle) -> &Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::TickDirection;
    use crate::axisartist::AxisArtistAxes;
    use crate::figure::Figure;

    #[test]
    fn artists_are_renamed_and_measured() {
        let fig = Figure::new();
        let ax = AxisArtistAxes::new(fig.subplot());
        let bottom = ax.axis("bottom").expect("an edge");
        let extent = bottom.tight_bbox().expect("the bottom axis draws");
        let frame = ax.ax().window_extent().expect("drawn");
        assert!(extent.y0() < frame.y0(), "tick labels below the frame");
        bottom.set_name("base");
        assert_eq!(ax.axis("base"), Some(bottom.clone()));
        assert_eq!(AxisArtist::from_raw_handle(bottom.raw()), Some(bottom));
        assert_eq!(AxisArtist::from_raw_handle(ax.ax().raw()), None);
    }

    #[test]
    fn directions_turn_labels_to_the_other_side() {
        let fig = Figure::new();
        let ax = AxisArtistAxes::new(fig.subplot());
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
        let ax = AxisArtistAxes::new(fig.subplot());
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
        bottom.toggle(Toggle {
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
