//! Archived figures: a figure written to bytes and read back, as pickling
//! a figure does (see [`pltrs_core::archive`]).

use std::rc::Rc;

use pltrs_core::archive::{self, ArchiveError, External, ProgramKinds};

use super::{Figure, FigureCell, FigureState, StateRole};
use crate::artist::RawHandle;

impl Figure {
    /// Archives the figure with everything in it, removed artists
    /// included, under their ids: the bytes, and the externals they refer
    /// to, which `program` made of what it archives itself. What only
    /// lives while a figure is shown (its canvas, callbacks, timers and
    /// tools) is left out.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs_core::archive::NoProgram;
    /// let fig = Figure::new();
    /// fig.subplot().plot([0.0, 1.0], [1.0, 3.0])?;
    /// let (bytes, externals) = fig.to_archive(&NoProgram)?;
    /// let back = Figure::from_archive(&bytes, &externals, &NoProgram)?;
    /// assert_eq!(back.to_png()?, fig.to_png()?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`ArchiveError::Unsupported`] when the figure holds what cannot be
    /// archived, such as a function given without an origin.
    pub fn to_archive(
        &self,
        program: &dyn ProgramKinds,
    ) -> Result<(Vec<u8>, Vec<External>), ArchiveError> {
        self.root.archive_figure(program)
    }

    /// The figure archived by [`Figure::to_archive`], read back with the
    /// externals it returned (as the program restored them) and the same
    /// kind of `program`.
    pub fn from_archive(
        bytes: &[u8],
        externals: &[External],
        program: &dyn ProgramKinds,
    ) -> Result<Self, ArchiveError> {
        let state: FigureState = archive::from_bytes(bytes, externals, program)?;
        if state.role != StateRole::Figure {
            return Err(ArchiveError::invalid("the archive holds no figure"));
        }
        let root = state.root;
        let state = Rc::new(FigureCell::new(state));
        let figure = Self {
            root: RawHandle::new(Rc::clone(&state), root),
            state,
        };
        crate::artist::reconnect_drags(&figure.root);
        Ok(figure)
    }
}

#[cfg(test)]
mod tests;
