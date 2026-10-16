//! Named colour sequences (`matplotlib.color_sequences`).

use plt_core::Rgba;

use super::sequence_data::SEQUENCES;

/// The colours of a built-in sequence such as `"tab10"` or `"Set1"`.
pub fn color_sequence(name: &str) -> Option<Vec<Rgba>> {
    SEQUENCES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, colors)| {
            colors
                .iter()
                .map(|&[r, g, b]| Rgba::opaque(r, g, b))
                .collect()
        })
}

/// Names of all built-in colour sequences, in matplotlib's order.
pub fn color_sequence_names() -> impl Iterator<Item = &'static str> {
    SEQUENCES.iter().map(|(name, _)| *name)
}
