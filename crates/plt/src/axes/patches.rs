//! Adding patches to an axes.

use super::Axes;
use super::children::DataUpdate;
use crate::artist::Artist;
use crate::patches::{Patch, PatchArtist, PatchShape};

impl Axes {
    /// Adds a patch in data coordinates, clipped to the axes, and extends
    /// the data limits to cover it without autoscaling the view
    /// (matplotlib's `add_patch`).
    pub fn add_patch<S: PatchShape>(&self, shape: S) -> Patch<S> {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        patch.base_mut().transform = Some(self.trans_data());
        // matplotlib's add_patch extends the data limits without
        // autoscaling the view.
        let update = DataUpdate {
            points: patch.data_points(),
            update: (true, true),
            autoscale: (false, false),
        };
        Patch::from_raw(self.adopt(patch, &update))
    }
}
