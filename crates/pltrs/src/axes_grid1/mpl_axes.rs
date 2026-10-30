//! The axis dictionary of `axes_grid1` axes (matplotlib's
//! `axes_grid1.mpl_axes`): one handle per side of an axes, acting together
//! on the spine of that side and on what the x or y axis draws there.

use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxisName, TickLevel, TickParams};
use crate::axisartist::Toggle;
use crate::spines::{Spine, SpineSide};

/// One side of an axes: its spine, and the major ticks, major tick labels
/// and label its axis draws on that side (matplotlib's `SimpleAxisArtist`,
/// `ax.axis["right"]`).
///
/// ```
/// use pltrs::axes_grid1::SimpleAxisArtist;
/// use pltrs::axisartist::Toggle;
/// use pltrs::prelude::*;
/// use pltrs::spines::SpineSide;
/// let ax = Figure::new().subplot();
/// ax.set_ylabel("y");
/// let right = SimpleAxisArtist::new(&ax, SpineSide::Right);
/// right.toggle(Toggle { all: Some(true), ..Toggle::default() });
/// assert_eq!(ax.yaxis().label_position(), SpineSide::Right);
/// right.set_visible(false);
/// assert!(!ax.spines().right.visible());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleAxisArtist {
    axes: Axes,
    side: SpineSide,
}

impl SimpleAxisArtist {
    /// The `side` of `axes`.
    pub fn new(axes: &Axes, side: SpineSide) -> Self {
        Self {
            axes: axes.clone(),
            side,
        }
    }

    /// The side this handle acts on.
    pub fn side(&self) -> SpineSide {
        self.side
    }

    /// The spine of this side (matplotlib's `line`).
    pub fn line(&self) -> Spine {
        let spines = self.axes.spines();
        match self.side {
            SpineSide::Left => spines.left,
            SpineSide::Right => spines.right,
            SpineSide::Bottom => spines.bottom,
            SpineSide::Top => spines.top,
        }
    }

    /// Shows or hides everything on this side: the spine, the major ticks
    /// and tick labels, and the label if it is or comes here; the axis
    /// itself stays visible (matplotlib's `SimpleAxisArtist.set_visible`).
    pub fn set_visible(&self, visible: bool) -> &Self {
        self.toggle(Toggle {
            all: Some(visible),
            ..Toggle::default()
        });
        self.line().set_visible(visible);
        self.axes.axis_of(self.side.axis()).set_visible(true);
        self
    }

    /// Shows or hides the major ticks, the major tick labels and the
    /// label on this side; `all` applies first and the others override it
    /// (matplotlib's `SimpleAxisArtist.toggle`). Showing the label moves
    /// it here; hiding it only hides it when it is here.
    pub fn toggle(&self, toggle: Toggle) -> &Self {
        let ticks = toggle.ticks.or(toggle.all);
        let ticklabels = toggle.ticklabels.or(toggle.all);
        let label = toggle.label.or(toggle.all);
        if ticks.is_some() || ticklabels.is_some() {
            self.axes.tick_params(self.side_params(ticks, ticklabels));
        }
        match label {
            Some(true) => {
                self.set_label_visible(true);
                self.axes
                    .axis_of(self.side.axis())
                    .set_label_position(self.side)
                    .expect("a side always belongs to its own axis");
            }
            Some(false) if self.axes.axis_of(self.side.axis()).label_position() == self.side => {
                self.set_label_visible(false);
            }
            Some(false) | None => {}
        }
        self
    }

    /// Shows or hides the major tick labels on this side (matplotlib's
    /// `major_ticklabels.set_visible`).
    pub fn set_ticklabels_visible(&self, visible: bool) -> &Self {
        self.toggle(Toggle {
            ticklabels: Some(visible),
            ..Toggle::default()
        })
    }

    /// Shows or hides the label of this side's axis, wherever it is
    /// (matplotlib's `label.set_visible`).
    pub fn set_label_visible(&self, visible: bool) -> &Self {
        self.axes
            .axis_of(self.side.axis())
            .set_label_visible(visible);
        self
    }

    /// Sets the text of this side's axis label (matplotlib's `set_label`).
    pub fn set_label(&self, text: impl Into<String>) -> &Self {
        match self.side.axis() {
            AxisName::X => self.axes.set_xlabel(text),
            AxisName::Y => self.axes.set_ylabel(text),
        };
        self
    }

    /// Tick parameters showing or hiding the major ticks and tick labels
    /// on this side only.
    fn side_params(&self, ticks: Option<bool>, labels: Option<bool>) -> TickParams {
        let params = TickParams {
            axis: Some(self.side.axis()),
            which: TickLevel::Major,
            ..TickParams::default()
        };
        match self.side {
            SpineSide::Left => TickParams {
                left: ticks,
                labelleft: labels,
                ..params
            },
            SpineSide::Right => TickParams {
                right: ticks,
                labelright: labels,
                ..params
            },
            SpineSide::Bottom => TickParams {
                bottom: ticks,
                labelbottom: labels,
                ..params
            },
            SpineSide::Top => TickParams {
                top: ticks,
                labeltop: labels,
                ..params
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::testing::major_tick_sides;

    #[test]
    fn hiding_a_side_keeps_the_axis_and_the_other_side() {
        let ax = Figure::new().subplot();
        SimpleAxisArtist::new(&ax, SpineSide::Bottom).set_visible(false);
        assert_eq!(
            major_tick_sides(&ax, AxisName::X),
            ((false, false), (false, false))
        );
        assert!(ax.xaxis().visible());
        assert!(!ax.spines().bottom.visible());
        assert!(ax.spines().top.visible());
        SimpleAxisArtist::new(&ax, SpineSide::Top).set_visible(true);
        assert_eq!(
            major_tick_sides(&ax, AxisName::X),
            ((false, true), (false, true))
        );
        assert_eq!(ax.xaxis().label_position(), SpineSide::Top);
    }

    #[test]
    fn hiding_the_label_elsewhere_leaves_it() {
        let ax = Figure::new().subplot();
        let label_visible = |ax: &Axes| ax.yaxis().label_visible();
        SimpleAxisArtist::new(&ax, SpineSide::Right).toggle(Toggle {
            label: Some(false),
            ..Toggle::default()
        });
        assert!(label_visible(&ax), "the label is on the left");
        SimpleAxisArtist::new(&ax, SpineSide::Left).set_ticklabels_visible(false);
        assert_eq!(major_tick_sides(&ax, AxisName::Y).1, (false, false));
        SimpleAxisArtist::new(&ax, SpineSide::Left).toggle(Toggle {
            all: Some(false),
            ticks: Some(true),
            ..Toggle::default()
        });
        assert!(!label_visible(&ax));
        assert_eq!(major_tick_sides(&ax, AxisName::Y).0, (true, false));
    }
}
