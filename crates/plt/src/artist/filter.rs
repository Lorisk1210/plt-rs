//! Image filters on artists (matplotlib's `Artist.set_agg_filter`).

use std::fmt;
use std::rc::Rc;

use plt_render::{FilterFn, FilterImage, Filtered};

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
