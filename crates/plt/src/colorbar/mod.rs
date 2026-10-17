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
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod artists;
mod blocks;
mod build;
mod error;
mod mappable;
mod options;
mod placement;
mod ticking;

pub use error::ColorbarError;
pub use mappable::Mappable;
pub use options::{
    ColorbarAxes, ColorbarFormat, ColorbarOptions, ColorbarTicks, ExtendFrac, ParentAnchor, Spacing,
};

use crate::artist::ArtistHandle;
use crate::axes::{Axes, Axis, AxisName, GridOptions, LabelLoc, TickLevel};
use crate::collections::Collection;
use crate::colors::{Color, Extend, ScalarMappable};
use crate::contour::ContourSet;
use crate::figure::Figure;
use crate::layout::LayoutEngine;
use crate::orientation::Orientation;
use artists::Painter;
use blocks::Blocks;

/// A colorbar drawn in its own axes (matplotlib's `Colorbar`).
#[derive(Debug)]
pub struct Colorbar {
    ax: Axes,
    orientation: Orientation,
    extend: Extend,
    /// A copy of the mappable's colormap and norm.
    mapping: ScalarMappable,
    blocks: Blocks,
    /// The block edges along the long axis (matplotlib's `_y`).
    edges: Vec<f64>,
    alpha: Option<f64>,
    drawedges: bool,
    dividers: Option<Collection>,
    lines: Vec<Collection>,
}

/// The axes a colorbar takes space from.
enum Parents {
    /// One axes, placed through its grid cell when it has one.
    One(Axes),
    /// The bounding box of several axes.
    Many(Vec<Axes>),
}

impl Figure {
    /// Adds a colorbar showing the colour mapping of `mappable`
    /// (matplotlib's `Figure.colorbar`): in the axes `options.axes` names
    /// with [`ColorbarAxes::Cax`], otherwise in a new axes that takes
    /// space from the parent axes.
    ///
    /// # Errors
    ///
    /// When there is no axes to take space from, the options contradict
    /// each other, or the norm cannot map the colorbar's values.
    pub fn colorbar<'a>(
        &self,
        mappable: impl Into<Mappable<'a>>,
        mut options: ColorbarOptions,
    ) -> Result<Colorbar, ColorbarError> {
        let source = mappable.into().source()?;
        let parents = match std::mem::take(&mut options.axes) {
            ColorbarAxes::Cax(cax) => return Colorbar::new(cax, source, &options, None),
            ColorbarAxes::FromMappable => {
                Parents::One(source.axes.clone().ok_or(ColorbarError::NoAxes)?)
            }
            ColorbarAxes::Parent(ax) => Parents::One(ax),
            ColorbarAxes::Parents(axes) if axes.is_empty() => return Err(ColorbarError::NoAxes),
            ColorbarAxes::Parents(axes) => Parents::Many(axes),
        };
        let engine = self.layout_engine();
        let use_gridspec = options.use_gridspec && engine.is_none_or(|e| e.colorbar_gridspec());
        let constrained = matches!(engine, Some(LayoutEngine::Constrained(_)));
        // The new colorbar axes becomes current; matplotlib restores the
        // axes that was current before.
        let current = self.current_axes();
        let placement = match parents {
            Parents::One(parent) => match parent.subplot_spec() {
                Some(spec) if use_gridspec => {
                    placement::make_axes_gridspec(self, parent, &spec, &options)?
                }
                _ => placement::make_axes(self, &[parent], &options, constrained)?,
            },
            Parents::Many(parents) => placement::make_axes(self, &parents, &options, constrained)?,
        };
        if let Some(current) = current {
            self.sca(&current);
        }
        placement.cax.grid_with(
            false,
            &GridOptions {
                which: TickLevel::Both,
                ..GridOptions::default()
            },
        );
        Colorbar::new(placement.cax.clone(), source, &options, Some(&placement))
    }
}

impl Colorbar {
    fn painter(&self) -> Painter<'_> {
        Painter {
            ax: &self.ax,
            orientation: self.orientation,
            alpha: self.alpha,
        }
    }

    /// The axes the colorbar is drawn in.
    pub fn ax(&self) -> &Axes {
        &self.ax
    }

    /// The colorbar's orientation.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Which ends have extensions.
    pub fn extend(&self) -> Extend {
        self.extend
    }

    /// The axis along the colorbar, which carries its ticks and label.
    pub fn long_axis(&self) -> Axis {
        self.ax.axis_of(long_axis(self.orientation))
    }

    /// The axis across the colorbar.
    fn short_axis(&self) -> Axis {
        self.ax.axis_of(match self.orientation {
            Orientation::Vertical => AxisName::X,
            Orientation::Horizontal => AxisName::Y,
        })
    }

    /// The edges of the colour blocks, extensions included.
    pub fn boundaries(&self) -> &[f64] {
        &self.blocks.boundaries
    }

    /// The value coloured in each block.
    pub fn values(&self) -> &[f64] {
        &self.blocks.values
    }

    /// Labels the long axis, at `loc` along it (`None` = the rc default)
    /// (matplotlib's `set_label`).
    pub fn set_label(&self, text: impl Into<String>, loc: Option<LabelLoc>) -> &Self {
        set_label(&self.ax, self.orientation, text.into(), loc);
        self
    }

    /// Puts the major ticks exactly at `ticks` (matplotlib's `set_ticks`).
    pub fn set_ticks(&self, ticks: Vec<f64>) -> &Self {
        self.long_axis().set_ticks(ticks);
        self
    }

    /// Turns minor ticks on along the colorbar (matplotlib's
    /// `minorticks_on`).
    pub fn minorticks_on(&self) -> &Self {
        self.ax.minorticks_on();
        self.short_axis().minorticks_off();
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
    pub fn add_lines(
        &mut self,
        contours: &ContourSet,
        erase: bool,
    ) -> Result<&mut Self, ColorbarError> {
        if contours.filled() {
            return Err(ColorbarError::FilledContours);
        }
        let colors = contours.colors().into_iter().map(Color::from).collect();
        self.add_level_lines(
            contours.levels(),
            colors,
            contours.linewidths().to_vec(),
            erase,
        )
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
        &mut self,
        levels: &[f64],
        colors: Vec<Color>,
        linewidths: Vec<f64>,
        erase: bool,
    ) -> Result<&mut Self, ColorbarError> {
        let y = self
            .blocks
            .locate(self.mapping.norm_mut(), &self.edges, levels)?;
        let (first, last) = (self.edges[0], self.edges[self.edges.len() - 1]);
        let rtol = (last - first) * 1e-10;
        let keep: Vec<usize> = (0..y.len())
            .filter(|&i| y[i] < last + rtol && y[i] > first - rtol)
            .collect();
        let positions: Vec<f64> = keep.iter().map(|&i| y[i]).collect();
        let colors = per_level(colors, &keep, y.len());
        let linewidths = per_level(linewidths, &keep, y.len());
        if erase {
            for lines in self.lines.drain(..) {
                lines.remove();
            }
        }
        let lines = self.painter().lines(&positions, colors, linewidths);
        self.lines.push(lines);
        Ok(self)
    }
}

/// The entries of a per-level list at the `keep` indices; lists of
/// another length are cycled over the levels and kept whole.
fn per_level<T: Clone>(items: Vec<T>, keep: &[usize], levels: usize) -> Vec<T> {
    if items.len() == levels {
        keep.iter().map(|&i| items[i].clone()).collect()
    } else {
        items
    }
}

/// The axis along a colorbar of `orientation`.
fn long_axis(orientation: Orientation) -> AxisName {
    match orientation {
        Orientation::Vertical => AxisName::Y,
        Orientation::Horizontal => AxisName::X,
    }
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
