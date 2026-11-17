//! Whether a collection's per-item alphas fit what they apply to, with
//! matplotlib's errors when they do not, and the colours its getters
//! report meanwhile.

use pltrs_core::Rgba;

use super::style::{HatchColors, MappedSides, Sided, unfilled};
use super::{CollectionArtist, EdgeColors};
use crate::colors::Color;

/// The colours of the sides of a collection that matplotlib did not
/// resolve again when per-item alphas, or colours, that do not fit each
/// other were set. matplotlib resolves a collection's colours when they
/// or its alpha are set, side by side (faces, then edges), and stops at
/// the first that fails: that side and those after it keep the colours
/// they had, which its getters report and its draws draw, until that
/// side's colours are set again or a draw resolves them again
/// ([`CollectionArtist::draw_resolves_colors`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct KeptColors {
    faces: Option<Vec<Rgba>>,
    edges: Option<Vec<Rgba>>,
}

pltrs_core::archive_struct!(KeptColors { faces, edges });

impl KeptColors {
    /// The colours `faces` and `edges` reported before a change that
    /// failed on `side`: kept from that side on, those that could be
    /// resolved.
    pub(crate) fn from_side(side: Side, faces: Sided, edges: Sided) -> Self {
        Self {
            faces: faces.ok().filter(|_| side == Side::Faces),
            edges: edges.ok().filter(|_| side <= Side::Edges),
        }
    }

    /// The faces reported: those kept, else `faces` as resolved now.
    pub(super) fn faces(&self, faces: Sided) -> Sided {
        Self::or_now(self.faces.as_ref(), faces)
    }

    /// The edges reported: those kept, else `edges` as resolved now.
    pub(super) fn edges(&self, edges: Sided) -> Sided {
        Self::or_now(self.edges.as_ref(), edges)
    }

    fn or_now(kept: Option<&Vec<Rgba>>, now: Sided) -> Sided {
        kept.map_or(now, |kept| Ok(kept.clone()))
    }

    /// Forgets the faces kept, once they are set again.
    pub(crate) fn forget_faces(&mut self) {
        self.faces = None;
    }

    /// Forgets the edges kept, once they are set again.
    pub(crate) fn forget_edges(&mut self) {
        self.edges = None;
    }

    /// Whether edges are kept.
    pub(crate) fn holds_edges(&self) -> bool {
        self.edges.is_some()
    }

    /// Keeps `faces` as the faces reported, set again with colours that
    /// do not fit the alphas.
    pub(crate) fn keep_faces(&mut self, faces: Sided) {
        self.faces = faces.ok();
    }

    /// Keeps `edges` as the edges reported, set again with colours that
    /// do not fit the alphas.
    pub(crate) fn keep_edges(&mut self, edges: Sided) {
        self.edges = edges.ok();
    }
}

/// The colours of a collection's items per-item alphas apply to, in the
/// order matplotlib's `set_alpha` resolves them again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Side {
    /// The face colours.
    Faces,
    /// The edge colours.
    Edges,
    /// The hatch colours.
    Hatches,
    /// The colours of the gaps of dashed edges.
    Gaps,
}

impl CollectionArtist {
    /// The first side whose colours, as given, the per-item alphas cannot
    /// be broadcast onto, with numpy's `ValueError` message, in the order
    /// matplotlib's `set_alpha` resolves them (faces, edges, hatches, the
    /// gaps of dashed edges): each side fits with a single colour, as many
    /// colours as alphas, or any number of colours for a single alpha.
    /// `None` when they fit.
    pub(crate) fn alpha_colors_problem(&self) -> Option<(Side, String)> {
        [Side::Faces, Side::Edges, Side::Hatches, Side::Gaps]
            .into_iter()
            .find_map(|side| self.side_problem(side).map(|problem| (side, problem)))
    }

    /// numpy's message when the per-item alphas cannot be broadcast onto
    /// the colours given to `side`.
    pub(crate) fn side_problem(&self, side: Side) -> Option<String> {
        let alphas = self.alphas.as_ref()?.len();
        let face_spec;
        let colors = match side {
            Side::Faces => {
                face_spec = self.face_spec(&self.colors_rc());
                face_spec.as_slice()
            }
            Side::Edges => match &self.edgecolors {
                EdgeColors::Colors(colors) => colors.as_slice(),
                EdgeColors::Default | EdgeColors::Face => return None,
            },
            Side::Hatches => match &self.hatchcolors {
                HatchColors::Colors(colors) => colors.as_slice(),
                HatchColors::Edge => return None,
            },
            Side::Gaps => self.gapcolors.as_deref()?,
        };
        broadcast_problem(colors, alphas)
    }

    /// matplotlib's `ValueError` message for per-item alphas that do not
    /// fit the collection, as a draw that resolves its colours again
    /// raises it (its `update_scalarmappable`), with the side it fails on:
    /// first values to map, as many alphas as values (the faces, nothing
    /// being set yet), then the faces and the edges the values do not
    /// colour.
    pub(crate) fn draw_alpha_problem(&self) -> Option<(Side, String)> {
        let alphas = self.alphas.as_ref()?;
        let values = self
            .mapping
            .as_ref()
            .map(|mapped| mapped.source.values.len());
        if let Some(values) = values.filter(|&values| values != alphas.len()) {
            let data = self
                .mesh_value_shape()
                .map_or_else(|| vec![values], |(rows, cols)| vec![rows, cols]);
            let given = self
                .alpha_shape
                .clone()
                .unwrap_or_else(|| vec![alphas.len()]);
            let problem = format!(
                "Data array shape, {} is incompatible with alpha array shape, {}. This \
                 can occur with the deprecated behavior of the \"flat\" shading option, \
                 in which a row and/or column of the data array is dropped.",
                python_shape(&data),
                python_shape(&given)
            );
            return Some((Side::Faces, problem));
        }
        let mapped = MappedSides::of(self);
        [(Side::Faces, mapped.faces), (Side::Edges, mapped.edges)]
            .into_iter()
            .filter(|&(_, mapped)| !mapped)
            .find_map(|(side, _)| self.side_problem(side).map(|problem| (side, problem)))
    }
}

/// numpy's message when `alphas` alphas cannot be broadcast onto `colors`
/// (a lone `"none"`, or no colour at all, takes any number).
fn broadcast_problem(colors: &[Color], alphas: usize) -> Option<String> {
    let count = colors.len();
    let fits = unfilled(colors) || count == 1 || alphas == 1 || count == alphas;
    (!fits).then(|| {
        format!("could not broadcast input array from shape ({alphas},) into shape ({count},)")
    })
}

/// `shape` written as Python writes a tuple of sizes: `(3,)`, `(3, 4)`.
fn python_shape(shape: &[usize]) -> String {
    match shape {
        [one] => format!("({one},)"),
        sizes => {
            let sizes: Vec<String> = sizes.iter().map(usize::to_string).collect();
            format!("({})", sizes.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_are_written_as_python_tuples() {
        assert_eq!(python_shape(&[3]), "(3,)");
        assert_eq!(python_shape(&[3, 4]), "(3, 4)");
    }

    #[test]
    fn alphas_broadcast_as_numpy_does() {
        let two = [Color::from("r"), Color::from("g")];
        assert_eq!(broadcast_problem(&two, 2), None);
        assert_eq!(broadcast_problem(&two, 1), None);
        assert_eq!(broadcast_problem(&two[..1], 5), None);
        assert_eq!(broadcast_problem(&[], 5), None);
        assert_eq!(broadcast_problem(&[Color::none()], 5), None);
        assert_eq!(
            broadcast_problem(&two, 3).as_deref(),
            Some("could not broadcast input array from shape (3,) into shape (2,)")
        );
    }
}
