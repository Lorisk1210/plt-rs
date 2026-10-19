//! The order in which a figure draws its artists, for what matplotlib
//! measures while drawing and so sees as drawn before or not yet.

use super::frame::{FigureFrame, listing_order};
use crate::artist::{ArtistId, ArtistStore};

/// `id` and its containers, from the root figure down to `id`.
fn lineage(store: &ArtistStore, id: ArtistId) -> Vec<ArtistId> {
    let mut lineage = vec![id];
    let mut next = store.get(id).and_then(|artist| artist.base().parent);
    while let Some(parent) = next {
        lineage.push(parent);
        next = store.get(parent).and_then(|artist| artist.base().parent);
    }
    lineage.reverse();
    lineage
}

/// The children of `container` in the order it draws them: by z-order,
/// ties in the order a figure lists them (matplotlib's `get_children`)
/// or, in other containers, in the order they were added.
fn drawing_order(store: &ArtistStore, container: ArtistId) -> Vec<ArtistId> {
    let Some(artist) = store.get(container) else {
        return Vec::new();
    };
    let mut children = artist.children();
    if store.get_as::<FigureFrame>(container).is_some() {
        children = listing_order(store, &children);
    }
    let zorder = |id: &ArtistId| store.get(*id).map_or(0.0, |child| child.base().zorder);
    children.sort_by(|a, b| zorder(a).total_cmp(&zorder(b)));
    children
}

/// Whether drawing the figure draws `first` before `second`: the closest
/// container holding both draws the branch of `first` earlier, or
/// `first` is a container of `second`.
pub(crate) fn draws_before(store: &ArtistStore, first: ArtistId, second: ArtistId) -> bool {
    let (from_first, from_second) = (lineage(store, first), lineage(store, second));
    let shared = from_first
        .iter()
        .zip(&from_second)
        .take_while(|(a, b)| a == b)
        .count();
    let (Some(&branch_first), Some(&branch_second)) =
        (from_first.get(shared), from_second.get(shared))
    else {
        return from_first.len() < from_second.len();
    };
    let Some(&container) = shared.checked_sub(1).and_then(|i| from_first.get(i)) else {
        return false;
    };
    let order = drawing_order(store, container);
    let position = |id: ArtistId| order.iter().position(|&child| child == id);
    matches!(
        (position(branch_first), position(branch_second)),
        (Some(i), Some(j)) if i < j
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::patches::Rectangle;

    /// `draws_before` on the store of `fig`.
    fn before(fig: &Figure, first: ArtistId, second: ArtistId) -> bool {
        draws_before(&fig.raw().figure_state().borrow().store, first, second)
    }

    #[test]
    fn siblings_draw_by_z_order_then_as_added() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let low = ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0)).id();
        let high = ax.text(0.5, 0.5, "above").id();
        let later = ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0)).id();
        assert!(before(&fig, low, high) && !before(&fig, high, low));
        assert!(before(&fig, later, high), "a lower z-order draws first");
        assert!(before(&fig, low, later) && !before(&fig, later, low));
        assert!(!before(&fig, low, low));
    }

    #[test]
    fn branches_draw_with_their_containers() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let inside = ax.text(0.5, 0.5, "in the axes").id();
        inside_raised(&fig, inside);
        let figure_patch = fig.add_patch(Rectangle::new((0.0, 0.0), 0.1, 0.1)).id();
        assert!(
            before(&fig, inside, figure_patch),
            "the axes (z-order 0) draws before the figure's patch (z-order 1)"
        );
        assert!(before(&fig, ax.id(), inside), "containers draw first");
        assert!(!before(&fig, inside, ax.id()));
    }

    /// Raises artist `id` above every figure-level artist, which must not
    /// change its order with them: its axes decides.
    fn inside_raised(fig: &Figure, id: ArtistId) {
        let mut state = fig.raw().figure_state().borrow_mut();
        if let Some(artist) = state.store.get_mut(id) {
            artist.base_mut().zorder = 100.0;
        }
    }

    #[test]
    fn figures_list_added_artists_before_texts() {
        let fig = Figure::new();
        let text = fig.text(0.5, 0.5, "listed after artists").id();
        let patch = fig.add_patch(Rectangle::new((0.0, 0.0), 0.1, 0.1));
        patch.set_zorder(3.0);
        assert!(before(&fig, patch.id(), text) && !before(&fig, text, patch.id()));
    }
}
