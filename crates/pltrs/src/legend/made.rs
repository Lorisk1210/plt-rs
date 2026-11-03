//! Legend entries drawn by artists made beforehand: the handlers of a
//! program that make the artists of an entry themselves (matplotlib's
//! handlers whose `legend_artist` adds the artists it makes to the handle
//! box), see [`LegendHandle::made`].

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use pltrs_core::archive::{Archive, ArchiveError, Decoder, Encoder};

use super::LegendHandle;
use super::handle::Source;
use super::handlers::{Attach, HandlerArtist};
use crate::artist::{Artist, AttachError, RawHandle, downcast_ref};
use crate::collections::CollectionArtist;

/// The artists of a made entry, until the legend showing the entry takes
/// them; handles compare equal when they share them.
#[derive(Clone)]
pub(crate) struct MadeArtists(Rc<RefCell<Vec<HandlerArtist>>>);

impl MadeArtists {
    /// The artists, which move into the legend showing the entry: a
    /// second legend gets none.
    pub(crate) fn take(&self) -> Vec<HandlerArtist> {
        std::mem::take(&mut *self.0.borrow_mut())
    }
}

impl PartialEq for MadeArtists {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for MadeArtists {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MadeArtists")
            .field(&self.0.borrow().len())
            .finish()
    }
}

/// A made entry lives until its legend is made, which keeps its artists:
/// the handle itself is never archived.
impl Archive for MadeArtists {
    fn encode(&self, _encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        Err(ArchiveError::unsupported("a legend handle of made artists"))
    }

    fn decode(_decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        Err(ArchiveError::invalid("a legend handle of made artists"))
    }
}

impl LegendHandle {
    /// A handle drawn by `artists`, made beforehand, each placed as its
    /// [`Attach`] says in the handle box (matplotlib's handlers of the
    /// program's, whose `legend_artist` makes the artists of an entry and
    /// adds them to the handle box: those whose transform is the box's are
    /// [`Attach::Transform`]). Each artist is a loose artist
    /// ([`Patch::loose`](crate::patches::Patch::loose), ...) or one taken
    /// into a figure that does not draw it
    /// ([`Figure::take_artist`](crate::figure::Figure::take_artist)); it
    /// moves into the legend that shows the handle, which draws it.
    ///
    /// ```
    /// use pltrs::legend::{Attach, LegendHandle, LegendOptions};
    /// use pltrs::patches::{PatchProps, Rectangle};
    /// use pltrs::prelude::*;
    /// use pltrs::patches::Patch;
    /// let ax = Figure::new().subplot();
    /// let mark = Patch::loose(Rectangle::new((0.0, 0.0), 20.0, 7.0), PatchProps::default().facecolor("red"));
    /// let handle = LegendHandle::made([(mark.raw().clone(), Attach::Transform)])?;
    /// let legend = ax.legend(LegendOptions::default().handles([handle]).labels(["made"]))?;
    /// assert_eq!(legend.entry_artists()[0].len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AttachError`] when an artist is drawn by a figure or an artist
    /// (as [`Figure::attach`](crate::figure::Figure::attach) refuses it)
    /// or owns other artists.
    pub fn made(
        artists: impl IntoIterator<Item = (RawHandle, Attach)>,
    ) -> Result<Self, AttachError> {
        let artists = artists
            .into_iter()
            .map(|(raw, attach)| {
                let artist = take_waiting(&raw)?;
                let mapped = downcast_ref::<CollectionArtist>(artist.as_ref())
                    .is_some_and(|collection| collection.mapping.is_some());
                Ok(HandlerArtist {
                    artist,
                    attach,
                    mapped,
                })
            })
            .collect::<Result<_, AttachError>>()?;
        Ok(Self(Source::Made(MadeArtists(Rc::new(RefCell::new(
            artists,
        ))))))
    }
}

/// Takes the artist behind `raw` out of where it waits: its loose holder,
/// or the figure it was taken into or removed from; it must own no other
/// artist.
fn take_waiting(raw: &RawHandle) -> Result<Box<dyn Artist>, AttachError> {
    let mut state = raw.figure_state().borrow_mut();
    if !state.is_removed(raw.id()) {
        return Err(if state.detached.contains(raw.id()) {
            AttachError::OwnedByRemoved
        } else {
            AttachError::NotRemoved
        });
    }
    let owns_others = state
        .detached
        .get(raw.id())
        .is_some_and(|artist| !artist.children().is_empty());
    if owns_others {
        return Err(AttachError::Misplaced);
    }
    state.detached.take(raw.id()).ok_or(AttachError::NotRemoved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::legend::LegendOptions;
    use crate::lines::{Line2D, LineProps};

    #[test]
    fn made_artists_move_into_the_legend_once() {
        let ax = Figure::new().subplot();
        let line = Line2D::loose(vec![0.0, 20.0], vec![3.5, 3.5], LineProps::default());
        let handle =
            LegendHandle::made([(line.raw().clone(), Attach::Transform)]).expect("a loose line");
        let legend = ax
            .legend(
                LegendOptions::default()
                    .handles([handle.clone()])
                    .labels(["made"]),
            )
            .expect("a legend");
        assert_eq!(legend.entry_artists()[0].len(), 1);
        let again = ax
            .legend(LegendOptions::default().handles([handle]).labels(["again"]))
            .expect("a legend");
        assert!(again.entry_artists()[0].is_empty(), "taken by the first");
    }

    #[test]
    fn artists_drawn_somewhere_are_refused() {
        let ax = Figure::new().subplot();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("a line");
        assert_eq!(
            LegendHandle::made([(line.raw().clone(), Attach::Own)]),
            Err(AttachError::NotRemoved)
        );
    }
}
