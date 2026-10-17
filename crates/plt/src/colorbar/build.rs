//! Filling a colorbar axes (matplotlib's `Colorbar.__init__`,
//! `_draw_all`, `_do_extends` and `_ColorbarAxesLocator`).

use std::rc::Rc;

use plt_core::{Bbox, Rgba};

use super::artists::Extensions;
use super::blocks::{BlockSpec, Blocks};
use super::mappable::{ContourLevels, Source};
use super::placement::{Placement, default_ticklocation, orientation_of};
use super::ticking::{self, Tickers};
use super::{Colorbar, ColorbarError, ColorbarOptions, ColorbarTicks, long_axis, set_label};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxesLocator, AxesState, AxisName, TickPosition};
use crate::colors::{Color, Extend, NormKind, ScalarMappable};
use crate::orientation::Orientation;
use crate::spines::SpineSide;
use crate::ticker::FixedLocator;

impl Colorbar {
    /// Draws a colorbar of `source` into `ax`; `placement` describes the
    /// axes when the colorbar made it.
    pub(super) fn new(
        ax: Axes,
        source: Source,
        options: &ColorbarOptions,
        placement: Option<&Placement>,
    ) -> Result<Self, ColorbarError> {
        let (orientation, ticklocation) = match placement {
            Some(placement) => (placement.orientation, placement.location),
            None => resolve_orientation(options)?,
        };
        let Source {
            mut mapping,
            alpha,
            contours,
            ..
        } = source;
        let extend = match (&contours, options.extend) {
            (Some(levels), _) => levels.extend,
            (None, Some(extend)) => extend,
            (None, None) => default_extend(&mapping),
        };
        let long = ax.axis_of(long_axis(orientation));
        let spec = BlockSpec {
            ncolors: mapping.cmap().n(),
            boundaries: contours
                .as_ref()
                .map(|levels| levels.extended.as_slice())
                .or(options.boundaries.as_deref()),
            values: contours
                .as_ref()
                .map(|levels| levels.cvalues.as_slice())
                .or(options.values.as_deref()),
            extend,
            inverted: long.inverted(),
            extendfrac: options.extendfrac,
            spacing: options.spacing,
        };
        let blocks = Blocks::new(&spec, mapping.norm_mut())?;
        set_label(&ax, orientation, options.label.clone(), None);
        let scale =
            ticking::long_axis_scale(&blocks, mapping.norm(), options.spacing, contours.is_some());
        ax.set_scale(long.name(), scale);
        let ticks = options.ticks.clone().or_else(|| {
            contours.as_ref().map(|levels| {
                ColorbarTicks::Locator(Rc::new(
                    FixedLocator::new(levels.levels.clone()).with_nbins(10),
                ))
            })
        });
        let tickers = Tickers::choose(
            &blocks,
            mapping.norm(),
            ticks.as_ref(),
            options.format.as_ref(),
        )?;
        let edges = blocks.edges(mapping.norm_mut())?;
        let mut colorbar = Self {
            ax,
            orientation,
            extend,
            mapping,
            blocks,
            edges,
            alpha,
            drawedges: options.drawedges,
            dividers: None,
            lines: Vec::new(),
        };
        colorbar.draw_all(
            ticklocation,
            &tickers,
            options.extendrect,
            contours.as_ref(),
        )?;
        if let Some(levels) = contours.filter(|levels| !levels.filled) {
            let colors = levels.colors.iter().map(|&c| Color::from(c)).collect();
            colorbar.add_level_lines(&levels.levels, colors, levels.linewidths, true)?;
        }
        colorbar.install_locator(placement.map(|_| options.aspect))?;
        Ok(colorbar)
    }

    /// matplotlib's `_draw_all`: ticks, outline and extensions, limits,
    /// then the colour blocks.
    fn draw_all(
        &mut self,
        ticklocation: SpineSide,
        tickers: &Tickers,
        extendrect: bool,
        contours: Option<&ContourLevels>,
    ) -> Result<(), ColorbarError> {
        let rc = crate::rc::params();
        let minor_visible = match self.orientation {
            Orientation::Vertical => rc.ytick_minor_visible,
            Orientation::Horizontal => rc.xtick_minor_visible,
        };
        if minor_visible {
            self.minorticks_on();
        }
        let long = self.long_axis();
        long.set_label_position(ticklocation)
            .and_then(|axis| axis.set_ticks_position(TickPosition::Side(ticklocation)))
            .map_err(|_| ColorbarError::TickLocation {
                ticklocation,
                orientation: self.orientation,
            })?;
        let short = self.short_axis();
        short.set_ticks(Vec::<f64>::new());
        short.set_minor_ticks(Vec::<f64>::new());
        let filled = contours.is_none_or(|levels| levels.filled);
        self.do_extends(extendrect, filled)?;
        let (lower, upper) = self.blocks.limits();
        let (lower, upper) = if long.inverted() {
            (upper, lower)
        } else {
            (lower, upper)
        };
        self.ax.set_limits(short.name(), 0.0, 1.0);
        self.ax.set_limits(long.name(), lower, upper);
        tickers.install(&long);
        if filled {
            let skip_low = usize::from(self.blocks.lower);
            let end = self.blocks.values.len() - usize::from(self.blocks.upper);
            let values = &self.blocks.values[skip_low..end];
            self.painter().solids(&self.edges, values, &self.mapping)?;
            self.update_dividers();
        }
        Ok(())
    }

    /// matplotlib's `_do_extends`: the outline and the extension patches.
    fn do_extends(&mut self, rect: bool, filled: bool) -> Result<(), ColorbarError> {
        let (_, lengths) = self.blocks.proportional_y(self.mapping.norm_mut())?;
        let extensions = Extensions {
            lower: if self.blocks.lower { lengths[0] } else { 0.0 },
            upper: if self.blocks.upper { lengths[1] } else { 0.0 },
            rect,
        };
        self.painter().outline(&extensions);
        if filled {
            let last = self.blocks.values.len() - 1;
            let (low, high) = if self.long_axis().inverted() {
                (last, 0)
            } else {
                (0, last)
            };
            let lower = self.end_color(self.blocks.lower, low)?;
            let upper = self.end_color(self.blocks.upper, high)?;
            self.painter().extension_patches(&extensions, lower, upper);
        }
        self.update_dividers();
        Ok(())
    }

    /// The colour of block `index` when the end it fills is extended.
    fn end_color(&mut self, extended: bool, index: usize) -> Result<Option<Rgba>, ColorbarError> {
        if !extended {
            return Ok(None);
        }
        let value = self.blocks.values[index];
        Ok(Some(self.mapping.to_rgba(&[Some(value)], None)?[0]))
    }

    /// matplotlib's `_update_dividers`: with `drawedges`, lines between
    /// the colours inside the view and at the ends of the extensions.
    fn update_dividers(&mut self) {
        if !self.drawedges {
            return;
        }
        let (lo, hi) = match self.long_axis().name() {
            AxisName::X => self.ax.xlim(),
            AxisName::Y => self.ax.ylim(),
        };
        let mut positions: Vec<f64> = self
            .edges
            .iter()
            .copied()
            .filter(|&y| lo < y && y < hi)
            .collect();
        if self.blocks.lower {
            positions.insert(0, lo);
        }
        if self.blocks.upper {
            positions.push(hi);
        }
        let painter = self.painter();
        match &self.dividers {
            Some(dividers) => painter.move_dividers(dividers, &positions),
            None => self.dividers = Some(painter.dividers(&positions)),
        }
    }

    /// Shrinks the axes to leave room for the extensions (matplotlib's
    /// `_ColorbarAxesLocator`); `aspect` is the long-to-short ratio of a
    /// colorbar that made its own axes, whose box aspect follows the
    /// shrinking.
    fn install_locator(&self, aspect: Option<f64>) -> Result<(), ColorbarError> {
        if self.extend == Extend::Neither {
            return Ok(());
        }
        let mut norm = self.mapping.norm().clone_box();
        let (_, lengths) = self.blocks.proportional_y(norm.as_mut())?;
        let lower = if self.blocks.lower { lengths[0] } else { 0.0 };
        let upper = if self.blocks.upper { lengths[1] } else { 0.0 };
        let total = lower + upper + 1.0;
        let (shrink, offset) = (1.0 / total, lower / total);
        let locator = match self.orientation {
            Orientation::Vertical => AxesLocator {
                fraction: Bbox::from_bounds(0.0, offset, 1.0, shrink),
                box_aspect: aspect.map(|a| a * shrink),
            },
            Orientation::Horizontal => AxesLocator {
                fraction: Bbox::from_bounds(offset, 0.0, shrink, 1.0),
                box_aspect: aspect.map(|a| 1.0 / (a * shrink)),
            },
        };
        self.ax
            .raw()
            .with_mut::<AxesState, _>(|axes| axes.aspect.locator = Some(locator));
        Ok(())
    }
}

/// The orientation and tick side of a colorbar drawn into a given axes.
fn resolve_orientation(
    options: &ColorbarOptions,
) -> Result<(Orientation, SpineSide), ColorbarError> {
    let orientation = match (options.location, options.orientation) {
        (Some(location), Some(orientation)) if orientation_of(location) != orientation => {
            return Err(ColorbarError::LocationOrientation {
                location,
                orientation,
            });
        }
        (location, orientation) => orientation
            .or(location.map(orientation_of))
            .unwrap_or_default(),
    };
    let ticklocation = options
        .ticklocation
        .or(options.location)
        .unwrap_or_else(|| default_ticklocation(orientation));
    Ok((orientation, ticklocation))
}

/// The extensions a mappable asks for: its colormap's, else its
/// `BoundaryNorm`'s.
fn default_extend(mapping: &ScalarMappable) -> Extend {
    if let Some(extend) = mapping.cmap().colorbar_extend() {
        return extend;
    }
    match mapping.norm().kind() {
        NormKind::Boundary(norm) => norm.extend(),
        _ => Extend::Neither,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_comes_from_location_or_defaults() {
        let options = ColorbarOptions::default();
        assert_eq!(
            resolve_orientation(&options),
            Ok((Orientation::Vertical, SpineSide::Right))
        );
        let top = ColorbarOptions {
            location: Some(SpineSide::Top),
            ..ColorbarOptions::default()
        };
        assert_eq!(
            resolve_orientation(&top),
            Ok((Orientation::Horizontal, SpineSide::Top))
        );
        let horizontal = ColorbarOptions {
            orientation: Some(Orientation::Horizontal),
            ..ColorbarOptions::default()
        };
        assert_eq!(
            resolve_orientation(&horizontal),
            Ok((Orientation::Horizontal, SpineSide::Bottom))
        );
        let clash = ColorbarOptions {
            location: Some(SpineSide::Left),
            orientation: Some(Orientation::Horizontal),
            ..ColorbarOptions::default()
        };
        assert!(resolve_orientation(&clash).is_err());
    }
}
