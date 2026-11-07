//! Whether the links between a figure's artists end: following an
//! artist's container (its parent, or the host axes an inset is placed
//! in) or what it owns (its children) from any artist must reach an
//! artist without such links, as it does in every figure a program makes.
//! The links of a figure read back from an archive may point anywhere,
//! and placing or drawing artists linked in a cycle would recurse without
//! end.

use std::collections::HashSet;

use super::FigureState;
use crate::artist::{Artist, ArtistId, ArtistLookup, downcast_ref};
use crate::axes::AxesState;

/// Whether no artist of the figure, removed ones included, is its own
/// container or owner, however indirectly.
pub(super) fn links_end(state: &FigureState) -> bool {
    let ids: Vec<ArtistId> = state.store.ids().chain(state.detached.ids()).collect();
    let linked = |id: ArtistId, links: fn(&dyn Artist) -> Vec<ArtistId>| {
        state.artist(id).map(links).unwrap_or_default()
    };
    acyclic(&ids, |id| linked(id, containers)) && acyclic(&ids, |id| linked(id, Artist::children))
}

/// What contains `artist`: its parent and, for axes, the axes they are
/// placed in.
fn containers(artist: &dyn Artist) -> Vec<ArtistId> {
    let home = downcast_ref::<AxesState>(artist).and_then(|axes| axes.home);
    artist.base().parent.into_iter().chain(home).collect()
}

/// Whether the graph over `ids` with the edges `next` holds no cycle: a
/// depth-first search without recursion, as a chain of links can be as
/// long as the figure has artists.
fn acyclic(ids: &[ArtistId], next: impl Fn(ArtistId) -> Vec<ArtistId>) -> bool {
    let mut done = HashSet::new();
    let mut open = HashSet::new();
    for &start in ids {
        if done.contains(&start) {
            continue;
        }
        open.insert(start);
        let mut path = vec![(start, next(start).into_iter())];
        while let Some((id, successors)) = path.last_mut() {
            let id = *id;
            match successors.next() {
                Some(successor) if open.contains(&successor) => return false,
                Some(successor) if !done.contains(&successor) => {
                    open.insert(successor);
                    path.push((successor, next(successor).into_iter()));
                }
                Some(_) => {}
                None => {
                    open.remove(&id);
                    done.insert(id);
                    path.pop();
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(index: u32) -> ArtistId {
        ArtistId::from_index(index)
    }

    /// The graph of `edges` over the artists `0..count`.
    fn graph(count: u32, edges: &[(u32, u32)]) -> bool {
        let ids: Vec<ArtistId> = (0..count).map(id).collect();
        acyclic(&ids, |from| {
            edges
                .iter()
                .filter(|&&(start, _)| id(start) == from)
                .map(|&(_, end)| id(end))
                .collect()
        })
    }

    #[test]
    fn trees_and_shared_descendants_are_acyclic() {
        assert!(graph(0, &[]));
        assert!(graph(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]));
        // A link to an artist the figure does not hold ends there.
        assert!(graph(2, &[(0, 1), (1, 7)]));
    }

    #[test]
    fn loops_and_cycles_are_found() {
        assert!(!graph(1, &[(0, 0)]));
        assert!(!graph(3, &[(0, 1), (1, 2), (2, 0)]));
        assert!(!graph(4, &[(0, 1), (1, 2), (2, 3), (3, 1)]));
    }

    #[test]
    fn a_long_chain_needs_no_deep_stack() {
        let edges: Vec<(u32, u32)> = (0..200_000).map(|i| (i, i + 1)).collect();
        let ids: Vec<ArtistId> = (0..=200_000).map(id).collect();
        assert!(acyclic(&ids, |from| {
            let index = from.index();
            edges
                .get(index as usize)
                .map(|&(_, end)| vec![id(end)])
                .unwrap_or_default()
        }));
    }
}
