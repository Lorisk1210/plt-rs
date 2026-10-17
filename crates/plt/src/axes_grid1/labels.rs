//! Which axes of a grid keep their tick labels and axis labels
//! (matplotlib's `Grid.set_label_mode` and `SimpleAxisArtist.toggle`).

use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxesState, AxisName, TickLevel, TickParams};
use crate::spines::SpineSide;

/// Which axes of a grid are labelled (matplotlib's `label_mode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelMode {
    /// The left column gets y labels, the bottom row x labels (`"L"`).
    #[default]
    L,
    /// Only the lower-left axes is labelled (`"1"`).
    One,
    /// Every axes is labelled (`"all"`).
    All,
    /// The labels are left as they are (`"keep"`).
    Keep,
}

/// Shows or hides the bottom and left tick labels and axis labels of the
/// axes in `rows` (top to bottom) according to `mode`.
pub(super) fn apply_label_mode(rows: &[Vec<Axes>], mode: LabelMode) {
    let nrows = rows.len();
    for (i, row) in rows.iter().enumerate() {
        for (j, ax) in row.iter().enumerate() {
            let lower_left = i + 1 == nrows && j == 0;
            let (bottom, left) = match mode {
                LabelMode::Keep => return,
                LabelMode::L => (i + 1 == nrows, j == 0),
                LabelMode::One => (lower_left, lower_left),
                LabelMode::All => (true, true),
            };
            toggle(ax, SpineSide::Bottom, bottom);
            toggle(ax, SpineSide::Left, left);
        }
    }
}

/// Shows or hides the major tick labels and the axis label on `side`
/// (matplotlib's `SimpleAxisArtist.toggle(ticklabels=..., label=...)`).
fn toggle(ax: &Axes, side: SpineSide, shown: bool) {
    let name = side.axis();
    let params = match name {
        AxisName::X => TickParams {
            labelbottom: Some(shown),
            ..TickParams::default()
        },
        AxisName::Y => TickParams {
            labelleft: Some(shown),
            ..TickParams::default()
        },
    };
    ax.tick_params(&TickParams {
        axis: Some(name),
        which: TickLevel::Major,
        ..params
    });
    let axis = ax.axis_of(name);
    let on_side = axis.label_position() == side;
    if shown {
        set_label_visible(ax, name, true);
        axis.set_label_position(side)
            .expect("the label goes on a side of its own axis");
    } else if on_side {
        set_label_visible(ax, name, false);
    }
}

/// Shows or hides the label of axis `name`.
fn set_label_visible(ax: &Axes, name: AxisName, visible: bool) {
    ax.raw()
        .with_mut::<AxesState, _>(|state| state.axis_mut(name).label.visible = visible);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn l_mode_labels_the_outer_axes() {
        let fig = Figure::new();
        let axs = fig.subplots(2, 2).unwrap();
        let rows: Vec<Vec<Axes>> = (0..2)
            .map(|r| (0..2).map(|c| axs[(r, c)].clone()).collect())
            .collect();
        for ax in axs.as_slice() {
            ax.set_xlabel("x").set_ylabel("y");
        }
        apply_label_mode(&rows, LabelMode::L);
        let label = |ax: &Axes, name: AxisName| {
            ax.raw()
                .with::<AxesState, _>(|state| state.axis(name).label.visible)
        };
        assert!(!label(&axs[(0, 0)], AxisName::X) && label(&axs[(0, 0)], AxisName::Y));
        assert!(label(&axs[(1, 0)], AxisName::X) && label(&axs[(1, 0)], AxisName::Y));
        assert!(label(&axs[(1, 1)], AxisName::X) && !label(&axs[(1, 1)], AxisName::Y));
    }
}
