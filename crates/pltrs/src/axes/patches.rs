//! Adding patches to an axes.

use super::children::DataUpdate;
use super::lines::cycle_color;
use super::{Axes, AxesState};
use crate::artist::{Artist, ArtistHandle};
use crate::colors::Color;
use crate::patches::{FancyArrow, IntoPatchShape, Patch, PatchArtist, PatchProps};

impl Axes {
    /// Adds a patch in data coordinates, clipped to the axes, and extends
    /// the data limits to cover it without autoscaling the view
    /// (matplotlib's `add_patch`). A shape given in plotting values, such
    /// as a [`DataRectangle`](crate::patches::DataRectangle), is converted
    /// first, and again when an axis changes unit.
    pub fn add_patch<S: IntoPatchShape>(&self, shape: S) -> Patch<S::Shape> {
        self.add_patch_with(shape, PatchProps::default())
    }

    /// [`add_patch`](Self::add_patch) for a patch styled by `props` before
    /// it is added, as matplotlib adds a patch made with its keyword
    /// arguments: a `transform` places it (and its data limits), a
    /// `clip_path` replaces the axes clip box.
    pub fn add_patch_with<S: IntoPatchShape>(
        &self,
        shape: S,
        props: PatchProps,
    ) -> Patch<S::Shape> {
        let unit_source = shape.unit_source();
        let mut patch = PatchArtist::new(shape.into_shape(self), crate::rc::params());
        patch.unit_source = unit_source;
        props.apply(&mut patch);
        // Like matplotlib's `_set_artist_props`, keep a transform the
        // shape already chose (a shadow inherits its source's).
        let base = patch.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        // matplotlib's add_patch extends the data limits without
        // autoscaling the view.
        let update = DataUpdate::extending(patch.limit_points(self.id()));
        Patch::from_raw(self.adopt_in_autoscale(patch, &update))
    }

    /// Adds an arrow polygon in data coordinates, extends the data limits
    /// to cover it and autoscales the view (matplotlib's `arrow`, whose
    /// keyword arguments are the fields of [`FancyArrow`] plus patch
    /// properties set through the returned handle).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::patches::FancyArrow;
    /// let ax = Figure::new().subplot();
    /// let arrow = ax.arrow(FancyArrow { head_width: Some(0.05), ..FancyArrow::new(0.0, 0.0, 0.5, 0.5) });
    /// arrow.set_facecolor("k");
    /// assert!(ax.xlim().1 > 0.5, "the view covers the head");
    /// ```
    pub fn arrow(&self, arrow: FancyArrow) -> Patch<FancyArrow> {
        let patch = self.add_patch(arrow);
        self.update_data(&DataUpdate {
            points: Vec::new(),
            update: (false, false),
            autoscale: (true, true),
        });
        patch
    }

    /// The next colour of the patch property cycle (matplotlib's
    /// `_get_patches_for_fill.get_next_color`), black when the cycle has
    /// no colours; the cycle advances only when it has.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert_ne!(ax.next_patch_color(), ax.next_patch_color());
    /// ```
    pub fn next_patch_color(&self) -> Color {
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
