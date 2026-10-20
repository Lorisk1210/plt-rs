//! Image filters on artists (matplotlib's `Artist.set_agg_filter`) and
//! groups of artists filtered together.

use std::fmt;
use std::rc::Rc;

use plt_render::{FilterFn, FilterImage, Filtered};

use super::{Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, RawHandle};

/// An image filter applied to what an artist draws on raster canvases
/// (matplotlib's `agg_filter`): the artist is drawn alone, its visible
/// pixels go through the filter with the resolution in dpi, and the image
/// the filter returns is drawn in their place. Vector output filters only
/// rasterized artists, as matplotlib does.
///
/// ```
/// use plt::artist::{AggFilter, ArtistHandle, Filtered};
/// use plt::figure::Figure;
/// // Half-transparent copies, 2 pixels to the right.
/// let fade = AggFilter::new(|image, _dpi| {
///     let mut faded = image.clone();
///     faded.pixels_mut().iter_mut().for_each(|pixel| pixel[3] *= 0.5);
///     Filtered { image: faded, offset: (2.0, 0.0) }
/// });
/// let fig = Figure::new();
/// fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?.set_agg_filter(Some(fade));
/// fig.to_png()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct AggFilter(Rc<FilterFn<'static>>);

impl AggFilter {
    /// A filter running `filter`.
    pub fn new(filter: impl Fn(&FilterImage, f64) -> Filtered + 'static) -> Self {
        Self(Rc::new(filter))
    }

    /// The filter function.
    pub(crate) fn function(&self) -> &FilterFn<'static> {
        &*self.0
    }
}

impl fmt::Debug for AggFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AggFilter(..)")
    }
}

impl PartialEq for AggFilter {
    /// Filters are equal when they are the same function.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Artists drawn a second time, together, through one image filter, as
/// one rasterized layer at the group's own z-order (the agg filter
/// example's `FilteredArtistList`): glows behind texts, shadows behind
/// groups of patches. The members are still drawn on their own too.
#[derive(Debug)]
pub(crate) struct FilteredGroupArtist {
    base: ArtistBase,
    members: Vec<ArtistId>,
    filter: AggFilter,
}

impl FilteredGroupArtist {
    /// `members` drawn through `filter`, rasterized, at z-order 0.
    pub(crate) fn new(members: Vec<ArtistId>, filter: AggFilter) -> Self {
        let mut base = ArtistBase::with_zorder(0.0);
        base.rasterized = true;
        Self {
            base,
            members,
            filter,
        }
    }
}

impl Artist for FilteredGroupArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        ctx.renderer().start_filter();
        for &member in &self.members {
            ctx.draw_child(member);
        }
        ctx.renderer().stop_filter(self.filter.function());
    }
}

/// A handle to a filtered group of artists (see
/// [`Axes::add_filtered_group`](crate::axes::Axes::add_filtered_group)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilteredGroup {
    raw: RawHandle,
}

impl FilteredGroup {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }
}

impl ArtistHandle for FilteredGroup {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::figure::Figure;

    #[test]
    fn filtered_groups_send_all_their_members_through_the_filter_at_once() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let first = ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).unwrap();
        let second = ax.plot(vec![0.0, 1.0], vec![1.0, 0.0]).unwrap();
        let calls = Rc::new(Cell::new(0));
        let counted = Rc::clone(&calls);
        let filter = AggFilter::new(move |image, _dpi| {
            counted.set(counted.get() + 1);
            Filtered {
                image: image.clone(),
                offset: (0.0, 0.0),
            }
        });
        let group = ax.add_filtered_group(&[first.raw().clone(), second.raw().clone()], filter);
        assert_eq!(group.zorder(), 0.0);
        fig.to_png().unwrap();
        assert_eq!(calls.get(), 1, "one layer for the whole group");
        group.set_visible(false);
        fig.to_png().unwrap();
        assert_eq!(calls.get(), 1, "a hidden group is not drawn");
    }
}
