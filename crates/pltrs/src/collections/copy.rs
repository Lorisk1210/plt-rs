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
