//! Artists kept by a figure, shared with the draw drawing them.
//!
//! Drawing a figure runs the program's code: a path effect, a norm, a
//! transform. matplotlib lets that code read and change the figure while it
//! is drawn, the artist being drawn included (a path effect recolouring its
//! artist), and the rest of the draw sees the change. So a draw does not
//! borrow the artists it draws from their figure: it holds each one it
//! draws, shared with the figure ([`ArtistStore::shared`]), and the figure
//! stays free to change meanwhile. A change made to an artist a draw holds
//! is made to a copy ([`ArtistClone`]), which the figure keeps from then
//! on, while the draw finishes that artist as it found it.
//!
//! [`ArtistStore::shared`]: super::ArtistStore::shared

use std::rc::Rc;

use pltrs_core::archive::{Archive, ArchiveError, ArchiveRc, Decoder, Encoder};

use super::Artist;

/// Copying an artist, as a figure does to change one a draw holds (see
/// [`Artist`]): every artist that is `Clone` has it.
pub trait ArtistClone {
    /// A copy of the artist.
    fn clone_artist(&self) -> Box<dyn Artist>;
}

impl<T: Artist + Clone> ArtistClone for T {
    fn clone_artist(&self) -> Box<dyn Artist> {
        Box::new(self.clone())
    }
}

/// An artist a figure keeps (in its store or among its removed artists),
/// shared with the draw drawing it, if one does.
#[derive(Clone, Debug)]
pub(crate) struct KeptArtist(Rc<dyn Artist>);

impl KeptArtist {
    /// `artist`, kept.
    pub(crate) fn new(artist: impl Artist) -> Self {
        Self(Rc::new(artist))
    }

    /// The artist, to read.
    pub(crate) fn get(&self) -> &dyn Artist {
        &*self.0
    }

    /// The artist, to change: a copy of it, kept from now on, when a draw
    /// holds it.
    pub(crate) fn get_mut(&mut self) -> &mut (dyn Artist + 'static) {
        if Rc::get_mut(&mut self.0).is_none() {
            self.0 = Rc::from(self.0.clone_artist());
        }
        Rc::get_mut(&mut self.0).expect("a copy no draw holds")
    }

    /// The artist, shared, for a draw to hold while it draws it.
    pub(crate) fn share(&self) -> Rc<dyn Artist> {
        Rc::clone(&self.0)
    }
}

impl From<Box<dyn Artist>> for KeptArtist {
    fn from(artist: Box<dyn Artist>) -> Self {
        Self(Rc::from(artist))
    }
}

impl<T: Artist> From<Box<T>> for KeptArtist {
    fn from(artist: Box<T>) -> Self {
        Self::new(*artist)
    }
}

/// Archived as the artist alone, as a `Box<dyn Artist>` is: the sharing
/// with a draw is not kept.
impl Archive for KeptArtist {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        ArchiveRc::encode_rc(&*self.0, encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        Box::<dyn Artist>::decode(decoder).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::{ArtistBase, DrawContext};

    #[derive(Clone, Debug, Default)]
    struct Plain {
        base: ArtistBase,
    }

    impl Artist for Plain {
        fn base(&self) -> &ArtistBase {
            &self.base
        }

        fn base_mut(&mut self) -> &mut ArtistBase {
            &mut self.base
        }

        fn draw(&self, _ctx: &mut DrawContext<'_>) {}
    }

    #[test]
    fn a_change_while_a_draw_holds_the_artist_is_made_to_a_copy() {
        let mut kept = KeptArtist::from(Box::new(Plain::default()) as Box<dyn Artist>);
        kept.get_mut().base_mut().zorder = 1.0;
        let held = kept.share();
        kept.get_mut().base_mut().zorder = 2.0;
        assert_eq!(held.base().zorder, 1.0, "the draw finishes as it found it");
        assert_eq!(kept.get().base().zorder, 2.0);
        drop(held);
        let before = Rc::as_ptr(&kept.0);
        kept.get_mut().base_mut().zorder = 3.0;
        assert!(
            std::ptr::addr_eq(before, Rc::as_ptr(&kept.0)),
            "no copy once nothing holds it"
        );
    }
}
