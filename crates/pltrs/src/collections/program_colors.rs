//! Colours a colour mapping of the program's computed for the items of a
//! collection (matplotlib's `_mapped_colors`, the colours `to_rgba` gives
//! for the collection's array): the bivariate and multivariate mappings
//! of a `Colorizer`, whose norms and colormaps are the program's objects.

use pltrs_core::Rgba;

use super::{Collection, CollectionArtist};

impl Collection {
    /// Colours the items `colors`, one per item (cycled), as the values of
    /// a colour mapping of the program's map them, alpha applied: the
    /// sides the collection's mapping colours (the faces, or the edges of
    /// unfilled items, as for [`Collection::set_array`]) take them in
    /// place of the colours of its own mapping. `None` gives the sides
    /// back to its own mapping, if any.
    ///
    /// ```
    /// use pltrs::collections::CollectionSpec;
    /// use pltrs::prelude::*;
    /// use pltrs::colors::Rgba;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let corner = vec![Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(1.0, 1.0)];
    /// let polygons = ax.add_collection(CollectionSpec::polygons(vec![corner.clone(), corner]));
    /// let red = Rgba::new(1.0, 0.0, 0.0, 0.5);
    /// polygons.set_program_colors(Some(vec![red, red]));
    /// assert_eq!(polygons.colors()?.faces, vec![red, red]);
    /// polygons.set_program_colors(None);
    /// assert_ne!(polygons.colors()?.faces, vec![red, red]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_program_colors(&self, colors: Option<Vec<Rgba>>) -> &Self {
        self.update(|c: &mut CollectionArtist| c.program_colors = colors)
    }
}
