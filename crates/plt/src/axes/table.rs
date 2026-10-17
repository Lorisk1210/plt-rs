//! Tables (`Axes::table`).

use std::rc::Rc;

use super::Axes;
use crate::artist::{ArtistHandle, LayoutContext, RawHandle};
use crate::table::{Table, TableArtist, TableError, TableOptions};

impl Axes {
    /// Adds a table of texts (matplotlib's `table`). Its rows are sized for
    /// a 10-point font in the axes' current height, so call this after
    /// creating the axes layout but before resizing it, as in matplotlib.
    pub fn table(&self, options: &TableOptions) -> Result<Table, TableError> {
        let figure = self.raw().figure_state();
        let (height, dpi) = {
            let state = figure.borrow();
            let layout = LayoutContext::new(&state);
            let height = layout
                .axes_extent(self.id())
                .map_or(1.0, |extent| extent.height());
            (height, layout.dpi())
        };
        let table = TableArtist::new(self.id(), options, (height, dpi), crate::rc::params())?;
        let id = figure.borrow_mut().adopt(self.id(), Box::new(table));
        Ok(Table::from_raw(RawHandle::new(Rc::clone(figure), id)))
    }
}
