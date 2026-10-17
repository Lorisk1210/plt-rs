//! Adding patches to an axes.

use super::children::DataUpdate;
use super::lines::cycle_color;
use super::{Axes, AxesState};
use crate::artist::{Artist, ArtistHandle};
use crate::colors::Color;
use crate::patches::{Patch, PatchArtist, PatchShape};

impl Axes {
    /// Adds a patch in data coordinates, clipped to the axes, and extends
    /// the data limits to cover it without autoscaling the view
    /// (matplotlib's `add_patch`).
    pub fn add_patch<S: PatchShape>(&self, shape: S) -> Patch<S> {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        // Like matplotlib's `_set_artist_props`, keep a transform the
        // shape already chose (a shadow inherits its source's).
        let base = patch.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        // matplotlib's add_patch extends the data limits without
        // autoscaling the view.
        let update = DataUpdate {
            points: patch.data_points(),
            update: (true, true),
            autoscale: (false, false),
        };
        Patch::from_raw(self.adopt(patch, &update))
    }

    /// The next colour of the patch property cycle (matplotlib's
    /// `_get_patches_for_fill.get_next_color`).
    pub(crate) fn next_patch_color(&self) -> Color {
        self.raw().with_mut::<AxesState, _>(|axes| {
            let color = cycle_color(&axes.prop_cycle, axes.patch_cycle);
            // Like matplotlib, only a cycle that provides colours advances.
            if color.is_some() {
                axes.patch_cycle += 1;
            }
            color.unwrap_or_else(|| Color::from("k"))
        })
    }
}
