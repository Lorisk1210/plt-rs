//! A contour set in its figure: the artist keeping its levels, colour
//! mapping and labels, the [`ContourSet`] handle to it, and recolouring
//! when the colour mapping changes (matplotlib's `ContourSet.changed`).

use std::rc::Rc;

use plt_core::{Path, Rgba};
use plt_render::Dashes;

use super::ContourError;
use super::label::ContourLabel;
use super::levels;
use super::set::{ContourState, ContourWarning};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ArtistStore, DrawContext, RawHandle,
};
use crate::axes::Axes;
use crate::collections::{Collection, CollectionArtist, EdgeColors};
use crate::colorizer::{ColorMapped, MappingChange, change_mapping};
use crate::colors::{Color, Colormap, Extend, Norm, NormError, ScalarMappable};
use crate::image::normalize_scalars;
use crate::lines::LineStyle;
use crate::text::{Text, TextArtist};
use crate::transforms::{ResolvedTransform, Transform};

/// The state of a contour set, kept in its axes next to the collections
/// that draw it. It draws nothing itself.
#[derive(Debug)]
pub(crate) struct ContourSetArtist {
    base: ArtistBase,
    pub(crate) state: ContourState,
    /// The collections drawing the contours.
    pub(crate) collections: Vec<ArtistId>,
}

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

    /// Recolours the faces (fills) or edges (lines) of the collections,
    /// one colour per path.
    fn paint_collections(&self, others: &mut ArtistStore) {
        let colors: Vec<Color> = self.state.colors.iter().map(|&c| Color::from(c)).collect();
        let per_band = self.collections.len() > 1;
        for (i, &id) in self.collections.iter().enumerate() {
            let Some(collection) = others.get_as_mut::<CollectionArtist>(id) else {
                continue;
            };
            let colors = if per_band {
                colors.get(i).cloned().into_iter().collect()
            } else {
                colors.clone()
            };
            if self.state.filled {
                collection.facecolors = Some(colors);
            } else {
                collection.edgecolors = EdgeColors::Colors(colors);
            }
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
        let cvalues: Vec<Option<f64>> = state.cvalues.iter().copied().map(Some).collect();
        let normalized = normalize_scalars(mapping, &cvalues, state.cvalue_kind)?;
        let cmap = mapping.cmap();
        state.colors = normalized
            .into_iter()
            .map(|n| cmap.lookup(n, None))
            .collect();
        self.paint_collections(others);
        self.paint_labels(others);
        Ok(())
    }
}

/// A contour plot: what [`Axes::contour`], [`Axes::contourf`],
/// [`Axes::tricontour`] and [`Axes::tricontourf`] return (matplotlib's
/// `ContourSet`). Clones refer to the same contours.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContourSet {
    axes: Axes,
    raw: RawHandle,
    collections: Vec<Collection>,
}

impl ContourSet {
    /// The handle to the contour set artist behind `raw`, drawn in `axes`.
    pub(crate) fn from_raw(axes: Axes, raw: RawHandle) -> Self {
        let collections = raw.with::<ContourSetArtist, _>(|set| set.collections.clone());
        let figure = raw.figure_state();
        let collections = collections
            .into_iter()
            .map(|id| Collection::from_raw(RawHandle::new(Rc::clone(figure), id)))
            .collect();
        Self {
            axes,
            raw,
            collections,
        }
    }

    /// The id of the contour set artist.
    pub(crate) fn id(&self) -> ArtistId {
        self.raw.id()
    }

    /// Whether the contour set is still in its figure.
    pub(crate) fn is_attached(&self) -> bool {
        self.raw.is_attached()
    }

    /// Removes the contours from the figure: the set, its collections and
    /// its labels (matplotlib's `ContourSet.remove`). Like the handles of
    /// every removed artist, the set stays usable.
    pub fn remove(&self) {
        let labels: Vec<ArtistId> =
            self.read(|state| state.labels.iter().map(|label| label.artist).collect());
        let mut figure = self.raw.figure_state().borrow_mut();
        let collections = self.collections.iter().map(ArtistHandle::id);
        for id in collections.chain(labels).chain([self.id()]) {
            figure.remove_artist(id);
        }
    }

    /// Reads the levels, colours and labels.
    pub(crate) fn read<R>(&self, f: impl FnOnce(&ContourState) -> R) -> R {
        self.raw.with::<ContourSetArtist, _>(|set| f(&set.state))
    }

    /// Changes the levels, colours and labels.
    pub(crate) fn update<R>(&self, f: impl FnOnce(&mut ContourState) -> R) -> R {
        self.raw
            .with_mut::<ContourSetArtist, _>(|set| f(&mut set.state))
    }

    /// The axes the contours are drawn in.
    pub fn axes(&self) -> &Axes {
        &self.axes
    }

    /// The transform from the coordinates of the contour lines to the
    /// display (matplotlib's `get_transform`): the axes' data transform,
    /// unless the set was made with another.
    pub fn transform(&self) -> Transform {
        self.collections
            .first()
            .and_then(|c| c.raw().with_base(|base| base.transform.clone()))
            .unwrap_or_else(|| self.axes.trans_data())
    }

    /// [`ContourSet::transform`] resolved in the axes' current view.
    pub(crate) fn display_transform(&self) -> Result<ResolvedTransform, ContourError> {
        self.axes.unstale_view();
        let figure = self.axes.raw().figure_state().borrow();
        Ok(self.transform().resolve(&*figure)?)
    }

    /// The collections drawing the contours: one, or one per band for
    /// hatched filled contours.
    pub fn collections(&self) -> &[Collection] {
        &self.collections
    }

    /// Whether these are filled contours.
    pub fn filled(&self) -> bool {
        self.read(|state| state.filled)
    }

    /// The contour levels.
    pub fn levels(&self) -> Vec<f64> {
        self.read(|state| state.levels.clone())
    }

    /// The value each level (lines) or band (fills) is coloured by: the
    /// levels, or the band midpoints (with the `extend` sentinels
    /// `±1e250`; geometric when log-scaled).
    pub fn layers(&self) -> Vec<f64> {
        self.read(|state| state.layers.clone())
    }

    /// The smallest unmasked z (`None` when all z is masked).
    pub fn zmin(&self) -> Option<f64> {
        self.read(|state| state.zmin)
    }

    /// The largest unmasked z.
    pub fn zmax(&self) -> Option<f64> {
        self.read(|state| state.zmax)
    }

    /// Whether levels and colours are log-scaled.
    pub fn logscale(&self) -> bool {
        self.read(|state| state.logscale)
    }

    /// Which ends are extended.
    pub fn extend(&self) -> Extend {
        self.read(|state| state.extend)
    }

    /// The norm's `(vmin, vmax)`: autoscaled to the levels unless set.
    pub fn norm_limits(&self) -> (Option<f64>, Option<f64>) {
        self.read(|state| state.norm_limits)
    }

    /// The z interval `(lower, upper]` of every band of a filled contour:
    /// consecutive levels including the `extend` sentinels, the lowest
    /// bound lowered when it equals [`ContourSet::zmin`] so the minimum is
    /// included (matplotlib's `_get_lowers_and_uppers`).
    pub fn bands(&self) -> Vec<(f64, f64)> {
        self.read(|state| levels::bands(&state.extended, state.zmin, state.logscale))
    }

    /// One colour per level (lines) or band (fills), alpha applied.
    pub fn colors(&self) -> Vec<Rgba> {
        self.read(|state| state.colors.iter().map(|&c| state.with_alpha(c)).collect())
    }

    /// The colormap the colours come from (`None` when no colour is
    /// mapped); explicit `colors` make a listed colormap.
    pub fn cmap(&self) -> Option<Colormap> {
        self.read(|state| state.mapping.as_ref().map(|m| m.cmap().clone()))
    }

    /// The line width of every level in points (empty for fills).
    pub fn linewidths(&self) -> Vec<f64> {
        self.read(|state| state.linewidths.clone())
    }

    /// The line style of every level (empty for fills).
    pub fn linestyles(&self) -> Vec<LineStyle> {
        self.read(|state| state.linestyles.clone())
    }

    /// The dash pattern of every level, scaled by its line width
    /// (matplotlib's `get_linestyle`; solid lines have an empty pattern).
    pub fn dash_patterns(&self) -> Vec<Dashes> {
        self.read(|state| {
            state
                .linestyles
                .iter()
                .zip(&state.linewidths)
                .map(|(style, &width)| style.scaled_dashes(width, &state.rc))
                .collect()
        })
    }

    /// One compound path per level (lines) or band (fills), as currently
    /// drawn (inline labels cut the lines).
    pub fn paths(&self) -> Vec<Path> {
        self.collections
            .iter()
            .flat_map(|c| c.raw().with::<CollectionArtist, _>(|c| c.paths.clone()))
            .collect()
    }

    /// What matplotlib would have warned about.
    pub fn warnings(&self) -> Vec<ContourWarning> {
        self.read(|state| state.warnings.clone())
    }

    /// The labels added by [`ContourSet::clabel`], in creation order.
    pub fn labels(&self) -> Vec<ContourLabel> {
        let figure = self.raw.figure_state();
        self.read(|state| {
            state
                .labels
                .iter()
                .map(|record| {
                    record.label(Text::from_raw(RawHandle::new(
                        Rc::clone(figure),
                        record.artist,
                    )))
                })
                .collect()
        })
    }

    /// Replaces the colormap (matplotlib's `set_cmap`): the contours, the
    /// labels taking the colour of their level and the colorbars of the
    /// set are recoloured. Explicit `colors` are indexed in the new
    /// colormap, as matplotlib's `NoNorm` does.
    pub fn set_cmap(&self, cmap: Colormap) -> &Self {
        change_mapping::<ContourSetArtist>(&self.raw, MappingChange::Cmap(Box::new(cmap)));
        self
    }

    /// Replaces the norm (matplotlib's `set_norm`), autoscaled to the
    /// levels unless its limits are set, and recolours as
    /// [`ContourSet::set_cmap`] does.
    pub fn set_norm(&self, norm: Box<dyn Norm>) -> &Self {
        change_mapping::<ContourSetArtist>(&self.raw, MappingChange::Norm(norm));
        self
    }

    /// Sets the data values mapped to the ends of the colormap
    /// (matplotlib's `set_clim`); `None` leaves a limit unchanged. Explicit
    /// `colors`, indexed through `NoNorm`, stay as they are.
    pub fn set_clim(&self, vmin: Option<f64>, vmax: Option<f64>) -> &Self {
        change_mapping::<ContourSetArtist>(&self.raw, MappingChange::Clim(vmin, vmax));
        self
    }

    /// Sets the line widths in points, cycled over the levels (matplotlib's
    /// `set_linewidth` on a line contour set); colorbars and legends made
    /// afterwards show them. Filled contours, which are not stroked, and
    /// an empty `widths` are left alone.
    pub fn set_linewidths(&self, widths: &[f64]) -> &Self {
        if widths.is_empty() || self.filled() {
            return self;
        }
        let widths = self.update(|state| {
            let cycled = widths.iter().copied().cycle();
            state.linewidths = cycled.take(state.linewidths.len()).collect();
            state.linewidths.clone()
        });
        self.restyle_lines(|c| c.linewidths = Some(widths.clone()));
        self
    }

    /// Sets the line styles, cycled over the levels (matplotlib's
    /// `set_linestyle` on a line contour set), replacing the dashes of
    /// negative levels; filled contours and an empty `styles` are left
    /// alone.
    pub fn set_linestyles(&self, styles: &[LineStyle]) -> &Self {
        if styles.is_empty() || self.filled() {
            return self;
        }
        let styles = self.update(|state| {
            let cycled = styles.iter().cloned().cycle();
            state.linestyles = cycled.take(state.linestyles.len()).collect();
            state.linestyles.clone()
        });
        self.restyle_lines(|c| c.linestyles.clone_from(&styles));
        self
    }

    /// Applies `f` to the collections drawing the lines.
    fn restyle_lines(&self, f: impl Fn(&mut CollectionArtist)) {
        for collection in &self.collections {
            collection.raw().with_mut::<CollectionArtist, _>(&f);
        }
    }

    /// Replaces the paths, one per level (lines) or band (fills), keeping
    /// their colours and styles (matplotlib's `set_paths`).
    pub fn set_paths(&self, paths: Vec<Path>) -> &Self {
        if let [only] = self.collections.as_slice() {
            only.raw()
                .with_mut::<CollectionArtist, _>(|c| c.paths = paths);
            return self;
        }
        for (collection, path) in self.collections.iter().zip(paths) {
            collection
                .raw()
                .with_mut::<CollectionArtist, _>(|c| c.paths = vec![path]);
        }
        self
    }

    /// Sets the edge colours, cycled over the levels or bands
    /// (matplotlib's `set_edgecolors`): filled contours then get outlines,
    /// and hatches take the colour of their band's edge when
    /// `hatch.color` is `edge`.
    pub fn set_edgecolors(&self, colors: Vec<Color>) -> &Self {
        if let [only] = self.collections.as_slice() {
            only.set_edgecolors(EdgeColors::Colors(colors));
            return self;
        }
        for (collection, color) in self.collections.iter().zip(colors.iter().cycle()) {
            collection.set_edgecolor(color.clone());
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{Normalize, colormaps};
    use crate::contour::{ClabelOptions, ContourOptions, Levels, QuadField};
    use crate::figure::Figure;
    use crate::numeric::Grid;

    /// Lines of z = row + column on a 4 × 5 grid.
    fn lines(options: ContourOptions) -> ContourSet {
        let field = QuadField::new(Grid::from_fn(4, 5, |i, j| (i + j) as f64));
        Figure::new().subplot().contour(&field, options).unwrap()
    }

    fn edge_colors(cs: &ContourSet) -> Vec<Rgba> {
        cs.collections()[0]
            .raw()
            .with::<CollectionArtist, _>(|c| c.resolved_colors().unwrap().1)
    }

    #[test]
    fn colormaps_and_limits_recolour_lines_and_labels() {
        let cs = lines(ContourOptions {
            levels: Levels::Values(vec![1.0, 3.0, 5.0]),
            ..ContourOptions::default()
        });
        cs.clabel(ClabelOptions::default()).unwrap();
        let gray = colormaps::get("gray").unwrap();
        cs.set_cmap(gray.clone());
        assert_eq!(cs.cmap().map(|c| c.name().to_owned()), Some("gray".into()));
        let expected = [gray.sample(0.0), gray.sample(0.5), gray.sample(1.0)];
        assert_eq!(cs.colors(), expected);
        assert_eq!(edge_colors(&cs), expected);
        assert!(cs.labels().iter().all(|l| expected.contains(&l.color)));
        cs.set_clim(Some(1.0), Some(9.0));
        assert_eq!(cs.norm_limits(), (Some(1.0), Some(9.0)));
        assert_eq!(cs.colors()[2], gray.sample(0.5));
        cs.set_norm(Box::new(Normalize::default()));
        assert_eq!(
            cs.norm_limits(),
            (Some(1.0), Some(5.0)),
            "autoscaled to the levels"
        );
    }

    #[test]
    fn line_widths_and_styles_cycle_over_the_levels() {
        let cs = lines(ContourOptions {
            levels: Levels::Values(vec![1.0, 3.0, 5.0]),
            colors: Some(vec![Color::from("k")]),
            ..ContourOptions::default()
        });
        cs.set_linewidths(&[1.0, 4.0]);
        assert_eq!(cs.linewidths(), [1.0, 4.0, 1.0]);
        cs.set_linestyles(&[LineStyle::Dashed]);
        assert_eq!(cs.linestyles(), [const { LineStyle::Dashed }; 3]);
        let (widths, styles) = cs.collections()[0]
            .raw()
            .with::<CollectionArtist, _>(|c| (c.linewidths.clone(), c.linestyles.clone()));
        assert_eq!(widths, Some(vec![1.0, 4.0, 1.0]));
        assert_eq!(styles, cs.linestyles());
        cs.set_linewidths(&[]);
        assert_eq!(cs.linewidths(), [1.0, 4.0, 1.0], "nothing to cycle");
    }

    #[test]
    fn filled_contours_have_no_lines_to_style() {
        let field = QuadField::new(Grid::from_fn(4, 5, |i, j| (i + j) as f64));
        let cs = Figure::new()
            .subplot()
            .contourf(&field, ContourOptions::default())
            .unwrap();
        cs.set_linewidths(&[3.0])
            .set_linestyles(&[LineStyle::Dotted]);
        assert!(cs.linewidths().is_empty() && cs.linestyles().is_empty());
    }

    #[test]
    fn explicit_colors_index_the_new_colormap_and_ignore_limits() {
        let cs = lines(ContourOptions {
            levels: Levels::Values(vec![1.0, 3.0]),
            colors: Some(vec![Color::from("r"), Color::from("b")]),
            ..ContourOptions::default()
        });
        let before = cs.colors();
        cs.set_clim(Some(0.0), Some(100.0));
        assert_eq!(cs.colors(), before, "NoNorm ignores the limits");
        let viridis = colormaps::get("viridis").unwrap();
        cs.set_cmap(viridis.clone());
        assert_eq!(cs.colors(), [viridis.lut()[0], viridis.lut()[1]]);
    }

    #[test]
    fn edge_colours_cycle_over_hatched_bands() {
        let field = QuadField::new(Grid::from_fn(4, 5, |i, j| (i + j) as f64));
        let options = ContourOptions {
            levels: Levels::Values(vec![0.0, 2.0, 4.0, 7.0]),
            hatches: vec![Some("/".to_owned())],
            ..ContourOptions::default()
        };
        let cs = Figure::new().subplot().contourf(&field, options).unwrap();
        assert_eq!(
            cs.collections().len(),
            3,
            "hatched bands are drawn one by one"
        );
        cs.set_edgecolors(vec![Color::from("r"), Color::from("b")]);
        let edges: Vec<Rgba> = cs
            .collections()
            .iter()
            .map(|c| {
                c.raw()
                    .with::<CollectionArtist, _>(|c| c.resolved_colors().unwrap().1[0])
            })
            .collect();
        let (red, blue) = (
            Color::from("r").resolve().unwrap(),
            Color::from("b").resolve().unwrap(),
        );
        assert_eq!(edges, [red, blue, red]);
        let lines = lines(ContourOptions::default());
        lines.set_edgecolors(vec![Color::from("r")]);
        assert!(edge_colors(&lines).iter().all(|&c| c == red));
    }

    #[test]
    fn replaced_paths_keep_the_styles() {
        let cs = lines(ContourOptions {
            levels: Levels::Values(vec![1.0, 3.0]),
            ..ContourOptions::default()
        });
        let colors = cs.colors();
        let other = lines(ContourOptions {
            levels: Levels::Values(vec![2.0, 5.0]),
            ..ContourOptions::default()
        });
        cs.set_paths(other.paths());
        assert_eq!(cs.paths(), other.paths());
        assert_eq!(cs.colors(), colors);
    }

    #[test]
    fn clones_share_the_contours() {
        let cs = lines(ContourOptions::default());
        let copy = cs.clone();
        copy.set_cmap(colormaps::get("hot").unwrap());
        assert_eq!(cs.cmap().map(|c| c.name().to_owned()), Some("hot".into()));
        assert_eq!(cs, copy);
    }
}
