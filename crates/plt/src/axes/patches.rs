//! Adding patches to an axes.

use std::rc::Rc;

use super::{Axes, AxesState};
use crate::artist::{Artist, ArtistHandle, RawHandle};
use crate::patches::{Patch, PatchArtist, PatchShape};
use crate::transforms::Transform;

impl Axes {
    /// Adds a patch in data coordinates, clipped to the axes, and extends
    /// the data limits to cover it (matplotlib's `add_patch`).
    pub fn add_patch<S: PatchShape>(&self, shape: S) -> Patch<S> {
        let figure = self.raw().figure_state();
        let axes_id = self.id();
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        let base = patch.base_mut();
        base.transform = Some(Transform::Data(axes_id));
        base.clip_box = Some(axes_id);
        let points = patch.data_points();
        let mut state = figure.borrow_mut();
        let id = state.adopt(axes_id, Box::new(patch));
        if let Some(axes) = state.store.get_as_mut::<AxesState>(axes_id) {
            axes.add_data(&points, true, true);
        }
        Patch::from_raw(RawHandle::new(Rc::clone(figure), id))
    }
}
