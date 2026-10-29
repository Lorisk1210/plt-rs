//! Where an axes sits in its figure: the grid cell it was placed on and
//! its position in figure fractions (matplotlib's `get_position` and
//! `set_position`).

use std::rc::Rc;

use pltrs_core::Bbox;

use super::{Axes, AxesState};
use crate::artist::{Artist, ArtistHandle, RawHandle};
use crate::gridspec::{GridSpecError, SubplotParams, SubplotSpec};

/// Which position of an axes [`Axes::set_position_which`] sets
/// (matplotlib's `which`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WhichPosition {
    /// The original and the active position.
    #[default]
    Both,
    /// The position before any aspect adjustment.
    Original,
    /// The position drawn, until the aspect is next applied.
    Active,
}

impl Axes {
    /// The grid cell the axes occupies, if it was placed on a grid.
    pub fn subplot_spec(&self) -> Option<SubplotSpec> {
        self.raw.with::<AxesState, _>(|a| a.subplot_spec.clone())
    }

    /// Moves the axes to the grid cell `spec`, its twins with it
    /// (matplotlib's `set_subplotspec`); unlike
    /// [`set_position`](Self::set_position), the layout engine goes on
    /// placing the axes.
    ///
    /// ```
    /// use pltrs::gridspec::GridSpec;
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let grid = GridSpec::new(2, 2)?;
    /// let ax = fig.add_subplot_spec(&grid.cell(0, 0)?)?;
    /// let reference = fig.add_subplot_spec(&grid.cell(1, 1)?)?;
    /// ax.set_subplot_spec(grid.cell(1, 1)?)?;
    /// assert_eq!(ax.subplot_spec(), Some(grid.cell(1, 1)?));
    /// assert_eq!(ax.position(), reference.position());
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the grid cannot be laid out with the figure's subplot
    /// parameters.
    pub fn set_subplot_spec(&self, spec: SubplotSpec) -> Result<&Self, GridSpecError> {
        let figure = self.raw.figure_state();
        let frame = self.raw.with::<AxesState, _>(|a| a.base().parent);
        let params = frame
            .and_then(|frame| figure.borrow().subplot_params_of(frame))
            .unwrap_or_else(|| SubplotParams::from_rc(&crate::rc::params()));
        let position = spec.position(&params)?;
        for twin in self.twins() {
            twin.raw.with_mut::<AxesState, _>(|a| {
                a.position = position;
                a.aspect.set_active(a.position, position);
            });
        }
        self.raw
            .with_mut::<AxesState, _>(|a| a.subplot_spec = Some(spec));
        Ok(self)
    }

    /// Placement as a fraction of the figure (or subfigure), after the
    /// aspect adjustment (matplotlib's `get_position`). Insets and
    /// secondary axes, which are placed in their host axes, are reported
    /// in the host's figure fractions too.
    pub fn position(&self) -> Bbox {
        let host = self.host().map(|host| host.position());
        self.apply_aspect();
        let active = self
            .raw
            .with::<AxesState, _>(|a| a.aspect.active_position(&a.position));
        host.map_or(active, |host| {
            active.transformed(&crate::transforms::unit_to_bbox(&host))
        })
    }

    /// The axes this one is placed in (insets, secondary axes).
    fn host(&self) -> Option<Self> {
        let figure = self.raw.figure_state();
        let parent = self.raw.with::<AxesState, _>(|a| a.base().parent)?;
        let is_axes = figure.borrow().store.get_as::<AxesState>(parent).is_some();
        is_axes.then(|| Self::from_raw(RawHandle::new(Rc::clone(figure), parent)))
    }

    /// Moves the axes and its twins to `[left, bottom, width, height]` in
    /// figure fractions, out of the layout engine's control (matplotlib's
    /// `set_position`).
    pub fn set_position(&self, rect: [f64; 4]) -> &Self {
        self.set_position_which(rect, WhichPosition::Both)
    }

    /// Moves the axes and its twins to `[left, bottom, width, height]` in
    /// figure fractions: the position before any aspect adjustment, the
    /// one drawn until the aspect is next applied, or both (matplotlib's
    /// `set_position(pos, which=...)`). The layout engine leaves this axes
    /// alone from now on.
    pub fn set_position_which(&self, rect: [f64; 4], which: WhichPosition) -> &Self {
        self.set_in_layout(false);
        let [left, bottom, width, height] = rect;
        let position = Bbox::from_bounds(left, bottom, width, height);
        for twin in self.twins() {
            twin.raw.with_mut::<AxesState, _>(|a| {
                if which != WhichPosition::Active {
                    a.position = position;
                }
                if which != WhichPosition::Original {
                    a.aspect.set_active(a.position, position);
                }
            });
        }
        self
    }
}
