//! The axes a new axes of a projection shares its axes with as it is made
//! (matplotlib's `sharex=` and `sharey=` given to an Axes' `__init__`).
//!
//! matplotlib joins them in the first clear its `__init__` makes, and a
//! polar Axes clears a second time (`PolarAxes.__init__`): each clear
//! sets the scale of every other axes sharing an axis anew, and the polar
//! clear sets that of each axis it does not share once more, each time
//! with a default `ScalarFormatter` (see `scale::note_default_formatter`).
//! So polar axes join before their projection sets them up, and the
//! second clear is counted with the groups as they then are.

use super::ProjectionError;
use crate::artist::{ArtistHandle, ArtistLookup};
use crate::axes::{Axes, AxesState};
use crate::mplot3d::SharedAxes;
use crate::scale::{ScaleName, note_default_formatter};
use crate::ticker::AxisName;

/// The axes a new axes shares its x and y axes with as it is made; either
/// may be an axes of either kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InitialShares {
    /// The axes whose x axis the new one shares.
    pub x: Option<SharedAxes>,
    /// The axes whose y axis the new one shares.
    pub y: Option<SharedAxes>,
}

impl InitialShares {
    /// Has `axes` share the axes given; axes that cannot share them (a
    /// projection fixing its limits) are removed.
    pub(crate) fn join(&self, axes: &Axes) -> Result<(), ProjectionError> {
        for (name, other) in [(AxisName::X, &self.x), (AxisName::Y, &self.y)] {
            if let Some(other) = other
                && let Err(err) = axes.share_with(name, other)
            {
                axes.remove();
                return Err(ProjectionError::Share(err));
            }
        }
        Ok(())
    }
}

/// Counts the default formatters of the clear matplotlib's
/// `PolarAxes.__init__` makes after the first: each axis is cleared, then
/// set linear again where the axes does not share it; where it does, the
/// clear sets the scale of every other axes of the group instead.
pub(crate) fn note_second_polar_clear(axes: &Axes) {
    let id = axes.id();
    let state = axes.raw().figure_state().borrow();
    for name in [AxisName::X, AxisName::Y] {
        let shared = state.artist_as::<AxesState>(id).is_some_and(|axes| {
            match name {
                AxisName::X => axes.shares.0,
                AxisName::Y => axes.shares.1,
            }
            .is_some()
        });
        let set_anew = if shared {
            let siblings = state.shared.siblings(name, id);
            siblings
                .into_iter()
                .filter(|&sibling| sibling != id)
                .count()
        } else {
            1
        };
        // The axis' own clear, then the scales set anew.
        for _ in 0..=set_anew {
            note_default_formatter(ScaleName::Linear);
        }
    }
}
