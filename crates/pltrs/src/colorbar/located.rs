//! matplotlib's `_ColorbarAxesLocator` measuring the extensions of a
//! colorbar wherever the aspect of its axes is applied (a figure's draw,
//! the axes' draw, a tight box): it calls `_proportional_y`, which
//! normalises the block boundaries through the colorbar's norm. pltrs
//! places the axes from the extension lengths it keeps; a norm of the
//! program's is called at these sites, as matplotlib calls it.

use super::state::{ColorbarArtist, colorbar_id};
use crate::artist::ArtistId;
use crate::colors::Extend;
use crate::figure::FigureState;

impl FigureState {
    /// The locator of the colorbar drawn in `axes`, if any, measuring its
    /// extensions through the colorbar's norm when that is the program's,
    /// the figure lent meanwhile.
    pub(crate) fn colorbar_located(&mut self, axes: ArtistId) {
        let Some(colorbar) = colorbar_id(&self.store, axes)
            .and_then(|id| self.store.shared_as::<ColorbarArtist>(id))
        else {
            return;
        };
        let state = &colorbar.state;
        if state.extend == Extend::Neither || !state.mapping.norm().calls_program() {
            return;
        }
        let mut norm = state.mapping.norm().clone_box();
        // What the program's norm gives stands for nothing pltrs keeps: the
        // extension lengths do not depend on it (see
        // `Blocks::extension_lengths`); a failure is the norm's own.
        self.lend(|_| drop(state.blocks.proportional_y(norm.as_mut())));
    }
}
