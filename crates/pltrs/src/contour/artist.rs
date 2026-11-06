//! The artist of a contour set in its figure: its levels, colour mapping
//! and labels, and recolouring when the colour mapping changes
//! (matplotlib's `ContourSet.changed`).

use pltrs_core::Rgba;

use super::set::ContourState;
use crate::artist::{Artist, ArtistBase, ArtistId, ArtistStore, DrawContext};
use crate::collections::CollectionArtist;
use crate::colorizer::ColorMapped;
use crate::colors::{Color, NormError, Normalized, ScalarMappable};
use crate::image::normalize_scalars;
use crate::text::TextArtist;

/// The state of a contour set, kept in its axes next to the collections
/// that draw it. It draws nothing itself.
#[derive(Clone, Debug)]
pub(crate) struct ContourSetArtist {
    base: ArtistBase,
    pub(crate) state: ContourState,
    /// The collections drawing the contours.
    pub(crate) collections: Vec<ArtistId>,
}

pltrs_core::archive_struct!(ContourSetArtist {
    base,
    state,
    collections
});

impl ContourSetArtist {
    /// The artist remembering `state` for `collections`.
    pub(crate) fn new(state: ContourState, collections: Vec<ArtistId>) -> Self {
        let mut base = ArtistBase::default();
        "_nolegend_".clone_into(&mut base.label);
        base.in_layout = false;
        Self {
            base,
            state,
            collections,
        }
    }

    /// Maps the values of the paths of the collections again, `normalized`
    /// being the set's normalised `cvalues`: one value per path (see
    /// [`ContourState::color`]).
    fn paint_collections(&self, others: &mut ArtistStore, normalized: &[Normalized]) {
        let colors: Vec<Color> = self.state.colors.iter().map(|&c| Color::from(c)).collect();
        let per_band = self.collections.len() > 1;
        for (i, &id) in self.collections.iter().enumerate() {
            let Some(collection) = others.get_as_mut::<CollectionArtist>(id) else {
                continue;
            };
            let paths = if per_band {
                i..i + 1
            } else {
                0..self.state.cvalues.len()
            };
            self.state.color(collection, &colors, normalized, paths);
        }
    }

    /// Recolours the labels that take the colour of their level. As in
    /// matplotlib, which pairs the labels with one alpha per colour, only
    /// the first labels, as many as there are colours, are updated.
    fn paint_labels(&mut self, others: &mut ArtistStore) {
        let state = &mut self.state;
        let count = state.colors.len();
        for k in 0..state.labels.len().min(count) {
            let label = &state.labels[k];
            if !label.follows {
                continue;
            }
            let color = state
                .colors
                .get(label.index)
                .map_or(Rgba::TRANSPARENT, |&c| state.with_alpha(c));
            if let Some(text) = others.get_as_mut::<TextArtist>(label.artist) {
                text.color = Color::from(color);
            }
            state.labels[k].color = color;
        }
    }
}

impl Artist for ContourSetArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, _ctx: &mut DrawContext<'_>) {}
}

impl ColorMapped for ContourSetArtist {
    fn mapping_mut(&mut self) -> Option<&mut ScalarMappable> {
        self.state.mapping.as_mut()
    }

    /// matplotlib's `ContourSet.changed`: limits from the levels, then
    /// the colours of the `cvalues` on the paths and the labels.
    fn remap(&mut self, others: &mut ArtistStore) -> Result<(), NormError> {
        let state = &mut self.state;
        let Some(mapping) = &mut state.mapping else {
            return Ok(());
        };
        let levels: Vec<Option<f64>> = state.levels.iter().copied().map(Some).collect();
        mapping.norm_mut().autoscale_none(&levels);
        state.norm_limits = mapping.clim();
        let cvalues = super::set::mapped_values(&state.cvalues);
        let normalized = normalize_scalars(mapping, &cvalues, state.cvalue_kind)?;
        let cmap = mapping.cmap();
        state.colors = normalized.iter().map(|&n| cmap.lookup(n, None)).collect();
        self.paint_collections(others, &normalized);
        self.paint_labels(others);
        Ok(())
    }
}
