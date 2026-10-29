//! Changing the colour mapping of an artist and propagating the change
//! (matplotlib's `ColorizingArtist.changed`, which processes the
//! `'changed'` callbacks colorbars connect to).
//!
//! Images, collections, triangle meshes and contour sets keep their norm
//! and colormap in the figure. Their handles change them through
//! [`change_mapping`], which re-derives the artist's colours and then
//! lets the colorbars showing the artist follow. Colorbars refer to their
//! mappable by id, so nothing keeps a figure alive from inside itself.

use crate::artist::{Artist, ArtistStore, DrawError, RawHandle};
use crate::colors::{Colormap, Norm, NormError, ScalarMappable};

/// An artist state that colours data through a norm and a colormap.
pub(crate) trait ColorMapped: Artist {
    /// The norm and colormap, mutably; `None` when the artist maps no
    /// colours.
    fn mapping_mut(&mut self) -> Option<&mut ScalarMappable>;

    /// Re-derives the colours after the mapping changed (the body of
    /// matplotlib's `changed`): fills unset limits of the norm from the
    /// data and recolours. `others` holds the rest of the figure's
    /// artists, which contour sets recolour too.
    ///
    /// # Errors
    ///
    /// When the norm cannot map the values of an artist whose colours are
    /// computed now rather than when drawn.
    fn remap(&mut self, others: &mut ArtistStore) -> Result<(), NormError>;
}

/// A change to a colour mapping.
#[derive(Debug)]
pub(crate) enum MappingChange {
    /// A new colormap (`set_cmap`).
    Cmap(Box<Colormap>),
    /// New limits, `None` leaving a limit unchanged (`set_clim`).
    Clim(Option<f64>, Option<f64>),
    /// A new norm (`set_norm`); its unset limits come from the data.
    Norm(Box<dyn Norm>),
}

impl MappingChange {
    /// Whether the norm is replaced, which resets the scale and tickers of
    /// the colorbars following the artist (matplotlib compares the norm
    /// objects in `Colorbar.update_normal`).
    fn replaces_norm(&self) -> bool {
        matches!(self, Self::Norm(_))
    }

    fn apply(self, mapping: &mut ScalarMappable) {
        match self {
            Self::Cmap(cmap) => mapping.set_cmap(*cmap),
            Self::Clim(vmin, vmax) => mapping.set_clim(vmin, vmax),
            Self::Norm(norm) => mapping.set_norm(norm),
        };
    }
}

/// Applies `change` to the colour mapping of the `T` behind `raw`,
/// recolours it and lets its colorbars follow; artists without a colour
/// mapping are left alone. A norm that cannot map the artist's values is
/// reported with the next draw.
pub(crate) fn change_mapping<T: ColorMapped>(raw: &RawHandle, change: MappingChange) {
    let reset = change.replaces_norm();
    let remapped = raw.with_others::<T, _>(|artist, others| {
        let mapping = artist.mapping_mut()?;
        change.apply(mapping);
        Some(artist.remap(others))
    });
    match remapped {
        None => {}
        Some(Ok(())) => notify(raw, reset),
        Some(Err(err)) => raw
            .figure_state()
            .borrow_mut()
            .pending_errors
            .push(DrawError {
                artist: raw.id(),
                message: err.to_string(),
            }),
    }
}

/// Lets the colorbars showing the artist behind `raw` follow a change of
/// its colour mapping (the `'changed'` callbacks); `reset` when its norm
/// was replaced.
pub(crate) fn notify(raw: &RawHandle, reset: bool) {
    crate::colorbar::follow(raw.figure_state(), raw.id(), reset);
}
