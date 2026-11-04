//! Whether a collection's per-item alphas fit what they apply to, with
//! matplotlib's errors when they do not.

use super::style::{MappedSides, unfilled};
use super::{CollectionArtist, EdgeColors};
use crate::colors::Color;

impl CollectionArtist {
    /// matplotlib's `ValueError` message for per-item alphas that do not fit
    /// the colours the collection was given, as setting them raises it
    /// (numpy's broadcasting in `to_rgba_array`): the faces, the edges and
    /// the gaps of dashed edges, each a single colour, as many colours as
    /// alphas, or any number of colours for a single alpha. `None` when they
    /// fit.
    pub(crate) fn alpha_colors_problem(&self) -> Option<String> {
        let alphas = self.alphas.as_ref()?.len();
        let faces = self.face_spec(self.rc());
        let edges = match &self.edgecolors {
            EdgeColors::Colors(colors) => Some(colors.as_slice()),
            EdgeColors::Default | EdgeColors::Face => None,
        };
        [Some(faces.as_slice()), edges, self.gapcolors.as_deref()]
            .into_iter()
            .flatten()
            .find_map(|colors| broadcast_problem(colors, alphas))
    }

    /// matplotlib's `ValueError` message for per-item alphas that do not fit
    /// the collection, as its draw raises it: first those of a mapping (its
    /// `update_scalarmappable`), as many alphas as values, then
    /// [`alpha_colors_problem`](Self::alpha_colors_problem)'s.
    pub(crate) fn alpha_problem(&self) -> Option<String> {
        let alphas = self.alphas.as_ref()?;
        let values = self
            .mapping
            .as_ref()
            .filter(|_| MappedSides::of(self) != MappedSides::default())
            .map(|mapped| mapped.source.values.len());
        match values {
            Some(values) if values != alphas.len() => {
                let data = self
                    .mesh_value_shape()
                    .map_or_else(|| vec![values], |(rows, cols)| vec![rows, cols]);
                let given = self
                    .alpha_shape
                    .clone()
                    .unwrap_or_else(|| vec![alphas.len()]);
                Some(format!(
                    "Data array shape, {} is incompatible with alpha array shape, {}. This \
                     can occur with the deprecated behavior of the \"flat\" shading option, \
                     in which a row and/or column of the data array is dropped.",
                    python_shape(&data),
                    python_shape(&given)
                ))
            }
            _ => self.alpha_colors_problem(),
        }
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
