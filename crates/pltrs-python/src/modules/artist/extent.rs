//! Where an artist is on the canvas (matplotlib's
//! `Artist.get_window_extent` and `Artist.get_tightbbox`), in the
//! figure's current layout.

use pltrs::artist::{ArtistHandle, RawHandle};

use super::super::transforms::bbox_array;
use crate::binding::prelude::*;

/// The artist's extent in display pixels as a 2×2 array of points, `None`
/// when it occupies no space (matplotlib's `get_window_extent`).
fn window_extent(artist: RawHandle) -> PyResult<Option<NdArray>> {
    artist.window_extent().map(bbox_array).transpose()
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("window_extent", window_extent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{extension, with_python};
    use pltrs::figure::Figure;

    #[test]
    fn extents_are_display_boxes() {
        with_python(|py| {
            extension(py);
            let fig = Figure::with_size(6.4, 4.8, 100.0);
            let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]).unwrap();
            let extent = window_extent(ax.raw().clone()).unwrap().unwrap();
            assert_eq!(extent.data, [0.0, 0.0, 640.0, 480.0]);
            let artist = pltrs::artist::Custom::loose(super::super::PlainArtist::default());
            assert!(window_extent(artist.raw().clone()).unwrap().is_none());
        });
    }
}
