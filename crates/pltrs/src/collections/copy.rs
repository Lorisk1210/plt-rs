//! Copying the look of one collection to another (matplotlib's
//! `Collection.update_from`).

use super::{Collection, CollectionArtist, ColorMapping, EdgeColors, HatchColors, Mapped};
use crate::artist::ArtistHandle;
use crate::colorizer;
use crate::colors::Color;
use crate::lines::LineStyle;

/// What `Collection.update_from` copies of a collection besides the
/// common artist properties.
struct Look {
    facecolors: Option<Vec<Color>>,
    edgecolors: EdgeColors,
    linewidths: Option<Vec<f64>>,
    linestyles: Vec<LineStyle>,
    alphas: Option<Vec<f64>>,
    antialiased: Option<Vec<bool>>,
    hatch: Option<String>,
    hatchcolors: HatchColors,
    pickradius: f64,
    mapping: Option<ColorMapping>,
}

impl Look {
    fn of(c: &CollectionArtist) -> Self {
        Self {
            facecolors: c.facecolors.clone(),
            edgecolors: c.edgecolors.clone(),
            linewidths: c.linewidths.clone(),
            linestyles: c.linestyles.clone(),
            alphas: c.alphas.clone(),
            antialiased: c.antialiased.clone(),
            hatch: c.hatch.clone(),
            hatchcolors: c.hatchcolors.clone(),
            pickradius: c.pickradius,
            mapping: c.mapping.as_ref().map(|mapped| ColorMapping {
                values: mapped.source.values.clone(),
                mappable: mapped.source.mappable.clone(),
            }),
        }
    }

    fn give(self, c: &mut CollectionArtist) {
        c.facecolors = self.facecolors;
        c.edgecolors = self.edgecolors;
        c.linewidths = self.linewidths;
        c.linestyles = self.linestyles;
        c.alphas = self.alphas;
        c.antialiased = self.antialiased;
        c.hatch = self.hatch;
        c.hatchcolors = self.hatchcolors;
        c.pickradius = self.pickradius;
        c.mapping = self.mapping.map(Mapped::new);
    }
}

impl CollectionArtist {
    /// Takes the look of `other` (matplotlib's `Collection.update_from`
    /// but for the common artist properties): its colours as given, with
    /// its colour mapping, which the collection maps when it is drawn, and
    /// the colours its getters report until then, those `other` reports
    /// (matplotlib copies its last mapped `_facecolors`).
    pub(crate) fn take_look(&mut self, other: &Self) {
        Look::of(other).give(self);
        self.mapped_shown.set(other.mapped_shown.get());
        let shown = other.mapping_shown.borrow().clone();
        *self.mapping_shown.get_mut() = shown.or_else(|| other.mapping.clone());
    }

    /// Takes what the collections drawing the bands of a hatched contour
    /// set share with the first (matplotlib's set is one collection,
    /// drawing its bands one by one): its artist properties but its label,
    /// and its face colours, line widths and styles, alphas,
    /// antialiasing, hatch colours and width, joins, caps, links and pick
    /// radius; each band's paths, hatch, edge colours and colour mapping
    /// stay its own.
    pub(crate) fn share_band_look(&mut self, first: &Self) {
        let base = &mut self.base;
        let from = &first.base;
        base.visible = from.visible;
        base.alpha = from.alpha;
        base.zorder = from.zorder;
        base.transform.clone_from(&from.transform);
        base.clip_on = from.clip_on;
        base.clip_box = from.clip_box;
        base.clip_path.clone_from(&from.clip_path);
        base.rasterized = from.rasterized;
        base.animated = from.animated;
        base.in_layout = from.in_layout;
        base.snap = from.snap;
        base.sketch = from.sketch;
        base.path_effects.clone_from(&from.path_effects);
        base.gid.clone_from(&from.gid);
        base.url.clone_from(&from.url);
        base.picker.clone_from(&from.picker);
        base.mouseover = from.mouseover;
        base.agg_filter.clone_from(&from.agg_filter);
        self.facecolors.clone_from(&first.facecolors);
        self.linewidths.clone_from(&first.linewidths);
        self.linestyles.clone_from(&first.linestyles);
        self.alphas.clone_from(&first.alphas);
        self.antialiased.clone_from(&first.antialiased);
        self.hatchcolors.clone_from(&first.hatchcolors);
        self.hatch_linewidth = first.hatch_linewidth;
        self.capstyle = first.capstyle;
        self.joinstyle = first.joinstyle;
        self.urls.clone_from(&first.urls);
        self.pickradius = first.pickradius;
    }
}

impl Collection {
    /// Gives the collection the common artist properties of `other`
    /// ([`RawHandle::update_from`](crate::artist::RawHandle::update_from))
    /// and its look: face and edge colours, widths, line styles, alphas,
    /// antialiasing, hatch and hatch colours, pick radius and colour
    /// mapping (matplotlib's `Collection.update_from`); whether the
    /// transform and the clip regions were copied too.
    ///
    /// ```
    /// use pltrs::collections::CollectionSpec;
    /// use pltrs::colors::Color;
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let square = vec![Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(1.0, 1.0)];
    /// let source = ax.add_collection(CollectionSpec::polygons(vec![square.clone()]));
    /// source.set_facecolor(Color::parse("red")?).set_linewidths(vec![3.0]);
    /// let target = ax.add_collection(CollectionSpec::polygons(vec![square]));
    /// assert!(target.update_from(&source));
    /// assert_eq!(target.style(), source.style());
    /// # Ok::<(), pltrs::colors::ColorError>(())
    /// ```
    pub fn update_from(&self, other: &Collection) -> bool {
        let references = self.raw().update_from(other.raw());
        let look = other.raw().with::<CollectionArtist, _>(Look::of);
        self.update(|c| look.give(c));
        colorizer::notify(self.raw(), true);
        references
    }
}

#[cfg(test)]
mod tests {
    use crate::collections::CollectionSpec;
    use crate::figure::Figure;
    use crate::transforms::Point;

    #[test]
    fn the_colour_mapping_is_copied() {
        let ax = Figure::new().subplot();
        let source = ax.scatter(vec![0.0, 1.0], vec![0.0, 1.0]).expect("dots");
        source
            .set_array(vec![Some(0.0), Some(2.0)])
            .expect("values")
            .set_pickradius(7.0);
        let target = ax.add_collection(CollectionSpec::polygons(vec![vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
        ]]));
        assert!(target.update_from(&source));
        assert_eq!(target.array(), Some(vec![Some(0.0), Some(2.0)]));
        assert_eq!(target.pickradius(), 7.0);
        assert_eq!(
            target.colors().expect("colours"),
            source.colors().expect("colours")
        );
    }
}
