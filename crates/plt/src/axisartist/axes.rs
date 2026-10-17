//! Axes drawn with axis artists, and handles to their axis artists
//! (matplotlib's `axisartist.axislines.Axes`, `AxesZero` and the `axis`
//! dictionary).

use std::rc::Rc;

use super::artist::AxisArtistState;
use super::helper::AxisHelper;
use super::line::AxislineStyle;
use crate::artist::{ArtistHandle, ArtistId, RawHandle};
use crate::axes::AxesState;
use crate::spines::SpineSide;
use crate::ticker::AxisName;

/// An axes whose axis lines, ticks and labels are drawn by axis artists,
/// one per entry of its axis dictionary, instead of its spines and axes
/// (matplotlib's `mpl_toolkits.axisartist.Axes`). Besides the four edges,
/// floating axis lines can run through the data.
///
/// ```
/// use plt::axisartist;
/// use plt::prelude::*;
/// use plt::axes::AxisName;
/// let fig = Figure::new();
/// let ax = axisartist::Axes::new(fig.subplot());
/// let through_zero = ax.new_floating_axis("y=0", AxisName::X, 0.0);
/// ax.axis("top").expect("an edge").set_visible(false);
/// assert_eq!(through_zero.name(), "y=0");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Axes {
    ax: crate::axes::Axes,
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

impl Axes {
    /// Turns `ax` into axes drawn by axis artists: its spines and axes are
    /// hidden and artists along the four edges take over, those at the top
    /// and right without labels (matplotlib's `axisartist.Axes`).
    pub fn new(ax: crate::axes::Axes) -> Self {
        let spines = ax.spines();
        for spine in [&spines.left, &spines.right, &spines.bottom, &spines.top] {
            spine.set_visible(false);
        }
        ax.xaxis().set_visible(false);
        ax.yaxis().set_visible(false);
        let axes = Self { ax };
        for side in [
            SpineSide::Bottom,
            SpineSide::Top,
            SpineSide::Left,
            SpineSide::Right,
        ] {
            let name = match side {
                SpineSide::Bottom => "bottom",
                SpineSide::Top => "top",
                SpineSide::Left => "left",
                SpineSide::Right => "right",
            };
            let artist = axes.insert(name, AxisHelper::Fixed(side), side);
            if side.is_high() {
                artist.toggle(&Toggle {
                    ticklabels: Some(false),
                    label: Some(false),
                    ..Toggle::default()
                });
            }
        }
        axes
    }

    /// [`Axes::new`] plus hidden axis lines through the data origin, `xzero`
    /// along `y = 0` and `yzero` along `x = 0`, clipped to the axes
    /// (matplotlib's `AxesZero`).
    pub fn zero(ax: crate::axes::Axes) -> Self {
        let axes = Self::new(ax);
        for (name, nth, direction) in [
            ("xzero", AxisName::X, SpineSide::Bottom),
            ("yzero", AxisName::Y, SpineSide::Left),
        ] {
            let artist = axes.insert(name, AxisHelper::Floating { nth, value: 0.0 }, direction);
            artist.update(|state| state.line_clipped = true);
            artist.set_visible(false);
        }
        axes
    }

    /// The underlying axes.
    pub fn ax(&self) -> &crate::axes::Axes {
        &self.ax
    }

    /// The axis artist under `name` in the axis dictionary (`ax.axis[name]`).
    pub fn axis(&self, name: &str) -> Option<AxisArtist> {
        let figure = self.ax.raw().figure_state();
        let state = figure.borrow();
        let children = state
            .store
            .get_as::<AxesState>(self.ax.id())?
            .children
            .clone();
        children
            .into_iter()
            .find(|&id| {
                state
                    .store
                    .get_as::<AxisArtistState>(id)
                    .is_some_and(|artist| artist.name == name)
            })
            .map(|id| AxisArtist {
                raw: RawHandle::new(Rc::clone(figure), id),
            })
    }

    /// Adds an axis line where data coordinate `nth` varies and the other
    /// one equals `value`, under `name` in place of any artist there
    /// (`ax.axis[name] = ax.new_floating_axis(nth_coord, value)`). Its
    /// tick labels face down (matplotlib's default `axis_direction`) and
    /// its line is clipped to the axes.
    pub fn new_floating_axis(&self, name: &str, nth: AxisName, value: f64) -> AxisArtist {
        let artist = self.insert(name, AxisHelper::Floating { nth, value }, SpineSide::Bottom);
        artist.update(|state| state.line_clipped = true);
        artist
    }

    /// Adds an axis artist under `name`, replacing any artist there.
    fn insert(&self, name: &str, helper: AxisHelper, direction: SpineSide) -> AxisArtist {
        if let Some(existing) = self.axis(name) {
            existing.remove();
        }
        let figure = self.ax.raw().figure_state();
        let rc = self
            .ax
            .raw()
            .with::<AxesState, _>(|state| Rc::clone(&state.rc));
        let artist = AxisArtistState::new(name, self.ax.id(), helper, direction, rc);
        let id: ArtistId = figure.borrow_mut().adopt(self.ax.id(), Box::new(artist));
        AxisArtist {
            raw: RawHandle::new(Rc::clone(figure), id),
        }
    }
}

/// A handle to an axis artist of an [`Axes`] (matplotlib's `AxisArtist`).
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
    fn update(&self, f: impl FnOnce(&mut AxisArtistState)) -> &Self {
        self.raw.with_mut::<AxisArtistState, _>(f);
        self
    }

    /// Its key in the axis dictionary.
    pub fn name(&self) -> String {
        self.raw
            .with::<AxisArtistState, _>(|state| state.name.clone())
    }

    /// Draws the line with an arrow head past its end, or plainly with
    /// `None` (matplotlib's `set_axisline_style`). The new line is not
    /// clipped.
    pub fn set_axisline_style(&self, style: Option<AxislineStyle>) -> &Self {
        self.update(|state| {
            state.line_style = style;
            state.line_clipped = false;
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
                state.major_labels.visible = visible;
                state.minor_labels.visible = visible;
            }
            if let Some(visible) = label {
                state.label_visible = visible;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn axis_artists_replace_spines_and_axes() {
        let fig = Figure::new();
        let ax = Axes::new(fig.subplot());
        assert!(!ax.ax().spines().left.visible() && !ax.ax().xaxis().visible());
        let names = ["bottom", "top", "left", "right"];
        assert!(names.iter().all(|name| ax.axis(name).is_some()));
        let top = ax.axis("top").expect("an edge");
        top.raw().with::<AxisArtistState, _>(|state| {
            assert!(!state.major_labels.visible && !state.label_visible);
            assert!(state.major_ticks.visible);
        });
        assert!(ax.axis("bottom").and_then(|a| a.window_extent()).is_some());
    }

    #[test]
    fn floating_axes_replace_entries_of_the_same_name() {
        let fig = Figure::new();
        let ax = Axes::zero(fig.subplot());
        let xzero = ax.axis("xzero").expect("an axis along y = 0");
        assert!(!xzero.visible());
        let replacement = ax.new_floating_axis("xzero", AxisName::X, 0.5);
        assert!(!xzero.raw().is_alive());
        assert_eq!(ax.axis("xzero"), Some(replacement.clone()));
        replacement
            .set_axisline_style(Some(AxislineStyle::SimpleArrow { size: 1.0 }))
            .toggle(&Toggle {
                all: Some(false),
                ticks: Some(true),
                ..Toggle::default()
            });
        replacement.raw().with::<AxisArtistState, _>(|state| {
            assert!(!state.line_clipped && state.major_ticks.visible);
            assert!(!state.major_labels.visible && !state.label_visible);
        });
    }
}
