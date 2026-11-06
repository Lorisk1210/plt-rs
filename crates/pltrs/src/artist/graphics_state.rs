//! The state a draw's graphics contexts start from.
//!
//! An artist sets in its [`GraphicsContext`] what it has a property for
//! (colour, width, dashes, the joins of a line or a patch) and leaves the
//! rest as the renderer starts it ([`DrawContext::base_gc`]). matplotlib's
//! renderers start from `GraphicsContextBase`, which joins lines with
//! rounds; its Cairo renderer (the `cairo` backend, and the GUI backends
//! drawing with Cairo) hands each artist a Cairo context, whose own state
//! applies where the artist sets nothing, and Cairo joins lines with
//! miters. So a collection given no `joinstyle` has round joins under
//! Agg and mitred ones under Cairo, like a patch's. A canvas drawing as
//! matplotlib's Cairo renderer does draws its figure in
//! [`GraphicsState::Cairo`], with [`drawing_with_state`].
//!
//! [`DrawContext::base_gc`]: super::DrawContext::base_gc

use std::cell::Cell;

use pltrs_render::{GraphicsContext, JoinStyle};

thread_local! {
    /// The state the figures this thread draws start from.
    static STATE: Cell<GraphicsState> = const { Cell::new(GraphicsState::Matplotlib) };
}

/// Whose state an artist's graphics context starts from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GraphicsState {
    /// matplotlib's `GraphicsContextBase` (Agg and the vector backends):
    /// round joins.
    #[default]
    Matplotlib,
    /// A Cairo context's, as matplotlib's Cairo renderer gives it: mitred
    /// joins.
    Cairo,
}

impl GraphicsState {
    /// The graphics context an artist drawn in this state starts from.
    pub fn context(self) -> GraphicsContext {
        let join = match self {
            Self::Matplotlib => JoinStyle::Round,
            Self::Cairo => JoinStyle::Miter,
        };
        GraphicsContext {
            join,
            ..GraphicsContext::default()
        }
    }
}

/// Puts the previous state back when dropped (also on panics).
struct Restore(GraphicsState);

impl Drop for Restore {
    fn drop(&mut self) {
        STATE.set(self.0);
    }
}

/// Runs `draw` with the figures this thread draws meanwhile starting
/// their artists' graphics contexts from `state`; the state before is
/// back afterwards.
pub fn drawing_with_state<R>(state: GraphicsState, draw: impl FnOnce() -> R) -> R {
    let _restore = Restore(STATE.replace(state));
    draw()
}

/// The state the figures this thread draws now start from.
pub(crate) fn current() -> GraphicsState {
    STATE.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::{CollectionProps, CollectionSpec};
    use crate::figure::Figure;
    use pltrs_core::{Path, Point};

    #[test]
    fn states_differ_in_their_joins_only() {
        assert_eq!(
            GraphicsState::Matplotlib.context(),
            GraphicsContext::default()
        );
        let cairo = GraphicsState::Cairo.context();
        assert_eq!(cairo.join, JoinStyle::Miter);
        assert_eq!(
            GraphicsContext {
                join: JoinStyle::Round,
                ..cairo
            },
            GraphicsContext::default()
        );
    }

    #[test]
    fn the_state_is_scoped() {
        assert_eq!(current(), GraphicsState::Matplotlib);
        let inner = drawing_with_state(GraphicsState::Cairo, || {
            let nested = drawing_with_state(GraphicsState::Matplotlib, current);
            (current(), nested)
        });
        assert_eq!(inner, (GraphicsState::Cairo, GraphicsState::Matplotlib));
        assert_eq!(current(), GraphicsState::Matplotlib);
        let unwound = std::panic::catch_unwind(|| {
            drawing_with_state(GraphicsState::Cairo, || panic!("drawing failed"))
        });
        assert!(unwound.is_err());
        assert_eq!(current(), GraphicsState::Matplotlib);
    }

    /// The pixels of a figure with a thick zigzag drawn by a collection
    /// of the join `join` (none: the state's), in `state`.
    fn zigzag(join: Option<JoinStyle>, state: GraphicsState) -> Vec<u8> {
        let fig = Figure::with_size(2.0, 2.0, 50.0);
        let ax = fig.subplot();
        let path = Path::polyline(vec![
            Point::new(0.1, 0.1),
            Point::new(0.5, 0.9),
            Point::new(0.9, 0.1),
        ]);
        let mut props = CollectionProps::default().linewidth(12.0).facecolor("none");
        if let Some(join) = join {
            props = props.joinstyle(join);
        }
        ax.add_collection(CollectionSpec::paths(vec![path]).with_props(props));
        drawing_with_state(state, || fig.render_raster())
            .expect("drawn")
            .into_image()
            .into_data()
    }

    #[test]
    fn collections_without_a_join_take_the_states() {
        let round = zigzag(Some(JoinStyle::Round), GraphicsState::Matplotlib);
        let miter = zigzag(Some(JoinStyle::Miter), GraphicsState::Matplotlib);
        assert_ne!(round, miter);
        assert_eq!(zigzag(None, GraphicsState::Matplotlib), round);
        assert_eq!(zigzag(None, GraphicsState::Cairo), miter);
        assert_eq!(zigzag(Some(JoinStyle::Round), GraphicsState::Cairo), round);
    }
}
