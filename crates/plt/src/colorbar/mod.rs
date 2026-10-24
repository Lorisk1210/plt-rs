//! Colorbars: the mapping from data values to colours drawn in an axes
//! of its own (matplotlib.colorbar).
//!
//! [`Figure::colorbar`] makes room for a colorbar next to one or several
//! axes, or draws into a given axes, and fills it:
//!
//! * the colour blocks, as a mesh, one per colour of the colormap or per
//!   interval of a `BoundaryNorm` or contour set, spaced uniformly or in
//!   proportion to their data intervals;
//! * triangular or rectangular extensions for values beyond the range;
//! * an outline, optional dividers between colours and lines at contour
//!   levels;
//! * a long axis whose scale, ticks and labels follow the norm.
//!
//! The colorbar follows its mappable: changing the colormap, norm or
//! limits of the image, collection, triangle mesh or contour set redraws
//! it (matplotlib's `update_normal`). A new norm also brings its own scale
//! and ticks; new limits or colormaps keep the ticks.
//!
//! ```
//! use plt::axes::ImshowOptions;
//! use plt::colorbar::ColorbarOptions;
//! use plt::numeric::Grid;
//! use plt::prelude::*;
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let image = ax.imshow(Grid::from_fn(4, 4, |r, c| (r * c) as f64), ImshowOptions::default())?;
//! let colorbar = fig.colorbar(&image, ColorbarOptions::default())?;
//! assert_eq!(colorbar.ax().ylim(), (0.0, 9.0));
//! assert_eq!(fig.axes().len(), 2);
//! image.set_clim(Some(1.0), Some(5.0));
//! assert_eq!(colorbar.ax().ylim(), (1.0, 5.0));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod artists;
mod blocks;
mod build;
mod dividers;
mod error;
mod mappable;
mod navigation;
mod options;
mod placement;
mod remove;
mod state;
mod ticking;

pub use error::ColorbarError;
pub use mappable::Mappable;
pub use options::{
    ColorbarAxes, ColorbarFormat, ColorbarOptions, ColorbarTicks, ExtendFrac, ParentAnchor, Spacing,
};
pub(crate) use remove::release;
pub(crate) use state::{follow, follow_view};

use std::cell::RefCell;
use std::rc::Rc;

use plt_core::{Bbox, Path};

use crate::artist::{ArtistHandle, ArtistId, ArtistStore, LayoutContext, RawHandle};
use crate::axes::{Axes, AxesState, Axis, AxisName, GridOptions, LabelLoc, TickLevel};
use crate::colors::{Color, Extend};
use crate::contour::ContourSet;
use crate::figure::{Figure, FigureState};
use crate::layout::LayoutEngine;
use crate::orientation::Orientation;
use crate::patches::{Patch, PathShape};
use mappable::MappableLink;
use placement::Parent;
pub(crate) use state::ColorbarArtist;
use state::{ColorbarState, colorbar_id};

/// A colorbar drawn in its own axes (matplotlib's `Colorbar`): a handle
/// to what the colorbar keeps in its axes. Clones refer to the same
/// colorbar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colorbar {
    ax: Axes,
    raw: RawHandle,
}

/// The axes a colorbar takes space from.
enum Parents {
    /// One axes, placed through its grid cell when it has one.
    One(Parent),
    /// The bounding box of several axes.
    Many(Vec<Parent>),
}

impl Figure {
    /// Adds a colorbar showing the colour mapping of `mappable`
    /// (matplotlib's `Figure.colorbar`): in the axes `options.axes` names
    /// with [`ColorbarAxes::Cax`], otherwise in a new axes that takes
    /// space from the parent axes, in their figure (which, as in
    /// matplotlib, may be another one than this). The colorbar follows
    /// later changes of the mappable's colour mapping, unless the
    /// mappable is a bare [`ScalarMappable`](crate::colors::ScalarMappable)
    /// or is drawn in another figure than the colorbar.
    ///
    /// # Errors
    ///
    /// When there is no axes to take space from, the options contradict
    /// each other, the norm cannot map the colorbar's values, or the
    /// mappable was removed from its figure.
    pub fn colorbar<'a>(
        &self,
        mappable: impl Into<Mappable<'a>>,
        mut options: ColorbarOptions,
    ) -> Result<Colorbar, ColorbarError> {
        let mappable = mappable.into();
        let source = mappable.source()?;
        let parents = match std::mem::take(&mut options.axes) {
            ColorbarAxes::Cax(cax) => {
                let link = link_in(&mappable, cax.raw().figure_state());
                return Colorbar::new(cax, source, link, &options, None);
            }
            ColorbarAxes::FromMappable => {
                Parents::One(Parent::of(mappable.axes().ok_or(ColorbarError::NoAxes)?))
            }
            ColorbarAxes::Parent(ax) => Parents::One(Parent::Axes(ax)),
            ColorbarAxes::Parents(axes) if axes.is_empty() => return Err(ColorbarError::NoAxes),
            ColorbarAxes::Parents(axes) => {
                Parents::Many(axes.into_iter().map(Parent::Axes).collect())
            }
        };
        // matplotlib makes room in the figure of the (first) parent, even
        // when asked through another one.
        let host = Some(parents.first().figure_state())
            .filter(|state| !Rc::ptr_eq(state, self.raw().figure_state()))
            .map(Self::root_of);
        let fig = host.as_ref().unwrap_or(self);
        let link = link_in(&mappable, fig.raw().figure_state());
        let engine = fig.layout_engine();
        let use_gridspec = options.use_gridspec && engine.is_none_or(|e| e.colorbar_gridspec());
        let constrained = matches!(engine, Some(LayoutEngine::Constrained(_)));
        // The new colorbar axes becomes current; matplotlib restores the
        // axes that was current before.
        let current = fig.current_axes();
        let placement = match parents {
            Parents::One(parent) => match parent.subplot_spec() {
                Some(spec) if use_gridspec => {
                    placement::make_axes_gridspec(fig, parent, &spec, &options)?
                }
                _ => placement::make_axes(fig, &[parent], &options, constrained)?,
            },
            Parents::Many(parents) => placement::make_axes(fig, &parents, &options, constrained)?,
        };
        if let Some(current) = current {
            fig.sca(&current);
        }
        placement.cax.grid_with(
            false,
            GridOptions {
                which: TickLevel::Both,
                ..GridOptions::default()
            },
        );
        Colorbar::new(
            placement.cax.clone(),
            source,
            link,
            &options,
            Some(&placement),
        )
    }
}

impl Parents {
    /// The first parent.
    fn first(&self) -> &Parent {
        match self {
            Self::One(parent) => parent,
            Self::Many(parents) => &parents[0],
        }
    }
}

/// The artist a colorbar drawn in the figure owning `figure` follows:
/// the mappable, when it is drawn in that figure.
fn link_in(mappable: &Mappable<'_>, figure: &Rc<RefCell<FigureState>>) -> Option<MappableLink> {
    mappable
        .link()
        .filter(|(state, _)| Rc::ptr_eq(state, figure))
        .map(|(_, link)| link)
}

impl Colorbar {
    /// The colorbar whose state is the artist behind `raw`, drawn in the
    /// axes that is its parent; `None` once that is gone.
    fn from_raw(raw: RawHandle) -> Option<Self> {
        let parent = {
            let figure = raw.figure_state().borrow();
            let parent = figure.store.get(raw.id())?.base().parent?;
            figure.store.get_as::<AxesState>(parent).map(|_| parent)
        }?;
        let ax = Axes::from_raw(RawHandle::new(Rc::clone(raw.figure_state()), parent));
        Some(Self { ax, raw })
    }

    fn read<R>(&self, f: impl FnOnce(&ColorbarState) -> R) -> R {
        self.raw
            .with::<ColorbarArtist, _>(|colorbar| f(&colorbar.state))
    }

    /// Runs `f` on a copy of the state, while `f` may draw in the axes,
    /// and keeps the copy even when `f` fails part way (it records the
    /// artists drawn so far).
    fn edit<R>(
        &self,
        f: impl FnOnce(&mut ColorbarState) -> Result<R, ColorbarError>,
    ) -> Result<R, ColorbarError> {
        let mut state = self.read(Clone::clone);
        let result = f(&mut state);
        self.raw
            .with_mut::<ColorbarArtist, _>(|colorbar| colorbar.state = state);
        result
    }

    /// The axes the colorbar is drawn in.
    pub fn ax(&self) -> &Axes {
        &self.ax
    }

    /// The colorbar's orientation.
    pub fn orientation(&self) -> Orientation {
        self.read(|state| state.orientation)
    }

    /// Which ends have extensions.
    pub fn extend(&self) -> Extend {
        self.read(|state| state.extend)
    }

    /// The axis along the colorbar, which carries its ticks and label.
    pub fn long_axis(&self) -> Axis {
        self.ax.axis_of(long_axis(self.orientation()))
    }

    /// The edges of the colour blocks, extensions included.
    pub fn boundaries(&self) -> Vec<f64> {
        self.read(|state| state.blocks.boundaries.clone())
    }

    /// The value coloured in each block.
    pub fn values(&self) -> Vec<f64> {
        self.read(|state| state.blocks.values.clone())
    }

    /// The lowest block edge inside the extensions (matplotlib's `vmin`).
    pub fn vmin(&self) -> f64 {
        self.read(|state| state.blocks.limits().0)
    }

    /// The highest block edge inside the extensions (matplotlib's `vmax`).
    pub fn vmax(&self) -> f64 {
        self.read(|state| state.blocks.limits().1)
    }

    /// The opacity of the colours: the mappable's, unless it has none or
    /// one per item (matplotlib's `alpha`).
    pub fn alpha(&self) -> Option<f64> {
        self.read(|state| state.alpha)
    }

    /// The outline around the colours and the extensions (matplotlib's
    /// `outline`); it keeps the style given to it when the colorbar is
    /// redrawn.
    pub fn outline(&self) -> Patch<PathShape> {
        let id = self.read(|state| state.drawn.outline);
        Patch::from_raw(self.handle(id.expect("a colorbar is drawn with its outline")))
    }

    /// The patches filling the extensions, the one at the lower end of the
    /// long axis first (matplotlib's `_extend_patches`); inverting the
    /// axis swaps their ends.
    pub fn extend_patches(&self) -> Vec<Patch<PathShape>> {
        self.read(|state| state.drawn.extensions.clone())
            .into_iter()
            .map(|id| Patch::from_raw(self.handle(id)))
            .collect()
    }

    /// With `drawedges`, the lines between the colours as drawn for the
    /// current view: across the colorbar at every block edge inside the
    /// view, and at the ends of the view the colorbar extends
    /// (matplotlib's `dividers.get_segments()`); empty without.
    pub fn divider_segments(&self) -> Vec<Path> {
        let view = match long_axis(self.orientation()) {
            AxisName::X => self.ax.xlim(),
            AxisName::Y => self.ax.ylim(),
        };
        self.read(|state| {
            if state.settings.drawedges {
                state.dividers(&self.ax).segments(view)
            } else {
                Vec::new()
            }
        })
    }

    /// A handle to the artist `id` of the colorbar's figure.
    fn handle(&self, id: ArtistId) -> RawHandle {
        RawHandle::new(Rc::clone(self.raw.figure_state()), id)
    }

    /// Labels the long axis, at `loc` along it (`None` = the rc default)
    /// (matplotlib's `set_label`).
    pub fn set_label(&self, text: impl Into<String>, loc: Option<LabelLoc>) -> &Self {
        set_label(&self.ax, self.orientation(), text.into(), loc);
        self
    }

    /// Puts the major ticks exactly at `ticks` (matplotlib's `set_ticks`);
    /// they stay when the mappable's limits or colormap change.
    pub fn set_ticks(&self, ticks: Vec<f64>) -> &Self {
        self.long_axis().set_ticks(ticks);
        self
    }

    /// Turns minor ticks on along the colorbar (matplotlib's
    /// `minorticks_on`).
    pub fn minorticks_on(&self) -> &Self {
        minorticks_on(&self.ax, self.orientation());
        self
    }

    /// Removes the minor ticks along the colorbar (matplotlib's
    /// `minorticks_off`).
    pub fn minorticks_off(&self) -> &Self {
        self.long_axis().minorticks_off();
        self
    }

    /// Draws the levels of line contours across the colorbar in their
    /// colours and widths (matplotlib's `add_lines(contour_set)`);
    /// `erase` removes previously added lines.
    ///
    /// # Errors
    ///
    /// When the contours are filled.
    pub fn add_lines(&self, contours: &ContourSet, erase: bool) -> Result<&Self, ColorbarError> {
        if contours.filled() {
            return Err(ColorbarError::FilledContours);
        }
        let colors = contours.colors().into_iter().map(Color::from).collect();
        self.add_level_lines(&contours.levels(), colors, contours.linewidths(), erase)
    }

    /// Draws lines across the colorbar at the data values `levels`, with
    /// one colour and width per level (or a list cycled over them),
    /// leaving out levels beyond the colorbar (matplotlib's
    /// `add_lines(levels, colors, linewidths)`).
    ///
    /// # Errors
    ///
    /// When the norm cannot map the levels.
    pub fn add_level_lines(
        &self,
        levels: &[f64],
        colors: Vec<Color>,
        linewidths: Vec<f64>,
        erase: bool,
    ) -> Result<&Self, ColorbarError> {
        self.edit(|state| state.add_level_lines(&self.ax, levels, colors, linewidths, erase))?;
        Ok(self)
    }
}

/// The axis along a colorbar of `orientation`.
fn long_axis(orientation: Orientation) -> AxisName {
    match orientation {
        Orientation::Vertical => AxisName::Y,
        Orientation::Horizontal => AxisName::X,
    }
}

/// The axis across a colorbar of `orientation` drawn in `ax`.
fn short_axis(ax: &Axes, orientation: Orientation) -> Axis {
    ax.axis_of(match orientation {
        Orientation::Vertical => AxisName::X,
        Orientation::Horizontal => AxisName::Y,
    })
}

/// Turns minor ticks on along a colorbar of `orientation` drawn in `ax`.
fn minorticks_on(ax: &Axes, orientation: Orientation) {
    ax.minorticks_on();
    short_axis(ax, orientation).minorticks_off();
}

/// The colorbar drawn in `axes`, if any.
fn colorbar_in(store: &ArtistStore, axes: ArtistId) -> Option<&ColorbarArtist> {
    store.get_as::<ColorbarArtist>(colorbar_id(store, axes)?)
}

/// Whether `ax` is the axes of a colorbar (matplotlib's axes labelled
/// `<colorbar>`).
pub(crate) fn is_colorbar_axes(ax: &Axes) -> bool {
    colorbar_id(&ax.raw().figure_state().borrow().store, ax.id()).is_some()
}

/// The extent of the outline of the colorbar drawn in `axes`, if it
/// holds one (matplotlib's `spines["outline"]`, which the title and the
/// y offset text clear).
pub(crate) fn outline_extent(layout: &LayoutContext<'_>, axes: ArtistId) -> Option<Bbox> {
    let store = layout.store();
    let outline = colorbar_in(store, axes)?.state.drawn.outline?;
    store.get(outline)?.window_extent(layout)
}

/// Labels the long axis of a colorbar at `loc`, by default the
/// `xaxis.labellocation` or `yaxis.labellocation` rc parameter.
fn set_label(ax: &Axes, orientation: Orientation, text: String, loc: Option<LabelLoc>) {
    let rc = crate::rc::params();
    let default = match orientation {
        Orientation::Vertical => {
            ax.set_ylabel(text);
            &rc.yaxis_labellocation
        }
        Orientation::Horizontal => {
            ax.set_xlabel(text);
            &rc.xaxis_labellocation
        }
    };
    let loc = loc.unwrap_or_else(|| LabelLoc::from_rc(default));
    ax.axis_of(long_axis(orientation)).set_label_loc(loc);
}

#[cfg(test)]
mod tests;
