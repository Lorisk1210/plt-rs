//! Filling a colorbar axes (matplotlib's `Colorbar.__init__`,
//! `_draw_all`, `_do_extends`, `add_lines` and `_ColorbarAxesLocator`).

use std::rc::Rc;

use pltrs_core::Rgba;

use super::artists::{Block, Extensions, Painter, SolidFill};
use super::blocks::Blocks;
use super::dividers::Dividers;
use super::mappable::{ContourLevels, MappableLink, Source};
use super::placement::{ColorbarCax, default_ticklocation, orientation_of};
use super::state::{BlockSettings, ColorbarArtist, ColorbarState, Drawn};
use super::ticking::{self, Tickers};
use super::{
    Colorbar, ColorbarError, ColorbarOptions, ColorbarTicks, long_axis, minorticks_on, set_label,
    short_axis,
};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxesLocator, AxesState, DataUpdate, TickPosition};
use crate::colors::{Color, Extend, NormKind, ScalarMappable};
use crate::image::{ScalarKind, normalize_scalars};
use crate::orientation::Orientation;
use crate::spines::SpineSide;
use crate::ticker::FixedLocator;

impl Colorbar {
    /// Draws a colorbar of `source` into `ax`, following the artist
    /// `link` when given; `placement` describes the axes when the colorbar
    /// made it.
    pub(super) fn new(
        ax: Axes,
        source: Source,
        link: Option<MappableLink>,
        options: &ColorbarOptions,
        placement: Option<&ColorbarCax>,
    ) -> Result<Self, ColorbarError> {
        let (orientation, ticklocation) = match placement {
            Some(placement) => (placement.orientation, placement.location),
            None => resolve_orientation(options)?,
        };
        let Source {
            mut mapping,
            alpha,
            contours,
            data,
        } = source;
        let alpha = if link.is_some() { alpha } else { options.alpha };
        let extend = match (&contours, options.extend) {
            (Some(levels), _) => levels.extend,
            (None, Some(extend)) => extend,
            (None, None) => default_extend(&mapping),
        };
        let settings = BlockSettings {
            boundaries: options.boundaries.clone(),
            values: options.values.clone(),
            spacing: options.spacing,
            extendfrac: options.extendfrac,
            extendrect: options.extendrect,
            drawedges: options.drawedges,
            aspect: placement.map(|_| options.aspect),
        };
        let long = ax.axis_of(long_axis(orientation));
        let limits = |mapping: &ScalarMappable| (mapping.norm().vmin(), mapping.norm().vmax());
        let given = limits(&mapping);
        let ends = (extend, long.inverted());
        let blocks = settings.blocks(contours.as_ref(), ends, &mut mapping, data.as_deref())?;
        if let (Some(link), (Some(vmin), Some(vmax))) = (link, limits(&mapping))
            && limits(&mapping) != given
        {
            link.share_limits(ax.raw().figure_state(), vmin, vmax);
        }
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
        // matplotlib's `_draw_all` processes the values again (as
        // `_reset_locator_formatter_scale` did), the program's norm seeing
        // it, before it meshes them.
        let blocks = settings.blocks(contours.as_ref(), ends, &mut mapping, data.as_deref())?;
        let edges = blocks.edges(mapping.norm_mut())?;
        let mut state = ColorbarState {
            link,
            orientation,
            ticklocation,
            extend,
            mapping,
            alpha,
            contours,
            settings,
            blocks,
            edges,
            drawn: Drawn::default(),
        };
        state.drawn.dividers = Some(state.painter(&ax).dividers());
        state.redraw(&ax, Some(&tickers))?;
        let raw = ax.adopt(ColorbarArtist::new(state), &DataUpdate::default());
        let colorbar = Self { ax, raw };
        if !colorbar.navigable() {
            colorbar.ax.set_navigate(false);
        }
        Ok(colorbar)
    }
}

impl ColorbarState {
    fn painter<'a>(&self, ax: &'a Axes) -> Painter<'a> {
        Painter {
            ax,
            orientation: self.orientation,
            alpha: self.alpha,
        }
    }

    /// matplotlib's `_draw_all`: ticks, outline and extensions, limits,
    /// then the colour blocks.
    pub(super) fn draw_all(
        &mut self,
        ax: &Axes,
        tickers: Option<&Tickers>,
    ) -> Result<(), ColorbarError> {
        let rc = crate::rc::params();
        let minor_visible = match self.orientation {
            Orientation::Vertical => rc.ytick_minor_visible,
            Orientation::Horizontal => rc.xtick_minor_visible,
        };
        if minor_visible {
            minorticks_on(ax, self.orientation);
        }
        let long = ax.axis_of(long_axis(self.orientation));
        long.set_label_position(self.ticklocation)
            .and_then(|axis| axis.set_ticks_position(TickPosition::Side(self.ticklocation)))
            .map_err(|_| ColorbarError::TickLocation {
                ticklocation: self.ticklocation,
                orientation: self.orientation,
            })?;
        let short = short_axis(ax, self.orientation);
        short.set_ticks(Vec::<f64>::new());
        short.set_minor_ticks(Vec::<f64>::new());
        let filled = self.contours.as_ref().is_none_or(|levels| levels.filled);
        self.do_extends(ax, filled)?;
        let (lower, upper) = self.blocks.limits();
        let (lower, upper) = if long.inverted() {
            (upper, lower)
        } else {
            (lower, upper)
        };
        ax.set_view_limits(short.name(), 0.0, 1.0)?;
        ax.set_view_limits(long.name(), lower, upper)?;
        // matplotlib's limit callbacks place the dividers for the new view.
        self.update_dividers(ax);
        if let Some(tickers) = tickers {
            tickers.install(&long);
        }
        if filled {
            let skip_low = usize::from(self.blocks.lower);
            let end = self.blocks.values.len() - usize::from(self.blocks.upper);
            let painter = self.painter(ax);
            painter.remove(self.drawn.solids.take());
            painter.remove(std::mem::take(&mut self.drawn.solids_patches));
            if self.contours.as_ref().is_some_and(ContourLevels::hatched) {
                let blocks = (skip_low..end)
                    .map(|index| self.block(index))
                    .collect::<Result<Vec<_>, _>>()?;
                self.drawn.solids_patches = painter.solids_patches(&self.edges, blocks);
            } else if self.blocks.kind == ScalarKind::Integer {
                let colors = (skip_low..end)
                    .map(|index| self.block_color(index))
                    .collect::<Result<Vec<_>, _>>()?;
                self.drawn.solids = Some(painter.solids(&self.edges, SolidFill::Colors(&colors))?);
            } else {
                let values = &self.blocks.values[skip_low..end];
                let fill = SolidFill::Mapped(values, &self.mapping);
                self.drawn.solids = Some(painter.solids(&self.edges, fill)?);
            }
            self.update_dividers(ax);
        }
        Ok(())
    }

    /// The colour and hatch of block `index`, counting the extensions.
    fn block(&mut self, index: usize) -> Result<Block, ColorbarError> {
        Ok(Block {
            color: self.block_color(index)?,
            hatch: self.band_hatch(index),
        })
    }

    /// The colour of the value of block `index`.
    fn block_color(&mut self, index: usize) -> Result<Rgba, ColorbarError> {
        let value = self.blocks.values[index];
        let normalized = normalize_scalars(&mut self.mapping, &[Some(value)], self.blocks.kind)?;
        Ok(self.mapping.cmap().lookup(normalized[0], None))
    }

    /// The hatch of contour band `band`, if the bands are hatched.
    fn band_hatch(&self, band: usize) -> Option<String> {
        self.contours
            .as_ref()
            .and_then(|levels| levels.hatches.get(band).cloned().flatten())
    }

    /// matplotlib's `_do_extends`: the outline and the extension patches.
    fn do_extends(&mut self, ax: &Axes, filled: bool) -> Result<(), ColorbarError> {
        let (_, lengths) = self.blocks.proportional_y(self.mapping.norm_mut())?;
        let extensions = Extensions {
            lower: if self.blocks.lower { lengths[0] } else { 0.0 },
            upper: if self.blocks.upper { lengths[1] } else { 0.0 },
            rect: self.settings.extendrect,
        };
        let painter = self.painter(ax);
        painter.remove(std::mem::take(&mut self.drawn.extensions));
        self.drawn.outline = Some(painter.outline(&extensions, self.drawn.outline));
        if filled {
            let last = self.blocks.values.len() - 1;
            let (low, high) = if ax.axis_of(long_axis(self.orientation)).inverted() {
                (last, 0)
            } else {
                (0, last)
            };
            // The extensions take the colours of the end values and the
            // hatches of the end bands.
            let lower = self.end_block(self.blocks.lower, low, 0)?;
            let upper = self.end_block(self.blocks.upper, high, last)?;
            self.drawn.extensions = painter.extension_patches(&extensions, lower, upper);
        }
        self.update_dividers(ax);
        Ok(())
    }

    /// When the end it fills is extended, the block coloured by value
    /// `index` and hatched as band `band`.
    fn end_block(
        &mut self,
        extended: bool,
        index: usize,
        band: usize,
    ) -> Result<Option<Block>, ColorbarError> {
        if !extended {
            return Ok(None);
        }
        Ok(Some(Block {
            color: self.extension_color(index)?,
            hatch: self.band_hatch(band),
        }))
    }

    /// The colour of the value of block `index` normalised alone, as
    /// matplotlib colours an extension (`cmap(norm(value))` of a scalar).
    fn extension_color(&mut self, index: usize) -> Result<Rgba, ColorbarError> {
        let floats = matches!(self.blocks.kind, ScalarKind::Float | ScalarKind::Float32);
        if !floats && self.mapping.norm().is_identity() {
            return self.block_color(index);
        }
        let value = self.blocks.values[index];
        let normalized = self.mapping.norm_mut().normalize_scalar(value)?;
        Ok(self.mapping.cmap().lookup(normalized, None))
    }

    /// matplotlib's `_update_dividers`: with `drawedges`, lines between
    /// the colours inside the current view and at the ends of the
    /// extensions; without, none.
    fn update_dividers(&self, ax: &Axes) {
        let Some(dividers) = self.drawn.dividers else {
            return;
        };
        let placement = self.settings.drawedges.then(|| Dividers {
            orientation: self.orientation,
            edges: self.edges.clone(),
            lower: self.blocks.lower,
            upper: self.blocks.upper,
        });
        self.painter(ax)
            .place_dividers(dividers, placement.as_ref());
    }

    /// matplotlib's `_do_extends` as the view limits change: once the
    /// long axis was inverted or put back, the extensions swap ends, take
    /// the colours there and move the room left for them.
    pub(super) fn follow_view(&mut self, ax: &Axes) -> Result<(), ColorbarError> {
        let inverted = ax.axis_of(long_axis(self.orientation)).inverted();
        (self.blocks.lower, self.blocks.upper) = Blocks::extended_ends(self.extend, inverted);
        let filled = self.contours.as_ref().is_none_or(|levels| levels.filled);
        self.do_extends(ax, filled)?;
        self.install_locator(ax)
    }

    /// matplotlib's `add_lines(levels, colors, linewidths)`: lines across
    /// the colorbar at the data values `levels`, leaving out those beyond
    /// it; `erase` removes the lines added before.
    pub(super) fn add_level_lines(
        &mut self,
        ax: &Axes,
        levels: &[f64],
        colors: Vec<Color>,
        linewidths: Vec<f64>,
        erase: bool,
    ) -> Result<(), ColorbarError> {
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
        let painter = self.painter(ax);
        if erase {
            painter.remove(std::mem::take(&mut self.drawn.lines));
        }
        self.drawn
            .lines
            .push(painter.lines(&positions, colors, linewidths));
        Ok(())
    }

    /// Shrinks the axes to leave room for the extensions (matplotlib's
    /// `_ColorbarAxesLocator`); a colorbar that made its own axes keeps
    /// its long-to-short ratio on the shrunk axes.
    pub(super) fn install_locator(&self, ax: &Axes) -> Result<(), ColorbarError> {
        if self.extend == Extend::Neither {
            return Ok(());
        }
        let (lower, upper) = self.extension_lengths()?;
        let total = lower + upper + 1.0;
        let (shrink, offset) = (1.0 / total, lower / total);
        let aspect = self.settings.aspect;
        let locator = match self.orientation {
            Orientation::Vertical => AxesLocator {
                shrink: (1.0, shrink),
                offset: (0.0, offset),
                box_aspect: aspect.map(|a| a * shrink),
            },
            Orientation::Horizontal => AxesLocator {
                shrink: (shrink, 1.0),
                offset: (offset, 0.0),
                box_aspect: aspect.map(|a| 1.0 / (a * shrink)),
            },
        };
        ax.raw()
            .with_mut::<AxesState, _>(|axes| axes.aspect.locator = Some(locator));
        Ok(())
    }

    /// The lengths of the lower and upper extensions as fractions of the
    /// interior, 0 for an end without one (matplotlib's `_proportional_y`
    /// extension lengths, zeroed by `_extend_lower` and `_extend_upper`).
    pub(super) fn extension_lengths(&self) -> Result<(f64, f64), ColorbarError> {
        let mut norm = self.mapping.norm().clone_box();
        let lengths = self.blocks.extension_lengths(norm.as_mut())?;
        let lower = if self.blocks.lower { lengths[0] } else { 0.0 };
        let upper = if self.blocks.upper { lengths[1] } else { 0.0 };
        Ok((lower, upper))
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

    #[test]
    fn per_level_lists_keep_the_levels_inside() {
        assert_eq!(per_level(vec![1, 2, 3], &[0, 2], 3), [1, 3]);
        assert_eq!(
            per_level(vec![7], &[0, 2], 3),
            [7],
            "cycled lists stay whole"
        );
    }

    /// A colorbar beside a new axes of the linear mapping of `cmap` to
    /// `[0, 1]`, made with `options`.
    fn colorbar(cmap: crate::colors::Colormap, options: ColorbarOptions) -> Colorbar {
        let fig = crate::figure::Figure::new();
        let ax = fig.subplot();
        let mappable =
            ScalarMappable::with_norm(cmap, Box::new(crate::colors::Normalize::new(0.0, 1.0)));
        fig.colorbar(
            &mappable,
            ColorbarOptions {
                axes: crate::colorbar::ColorbarAxes::Parent(ax),
                ..options
            },
        )
        .unwrap()
    }

    #[test]
    fn extensions_swap_ends_with_the_long_axis() {
        // matplotlib's test_colorbar_extension_inverted_axis: the minimum's
        // extension keeps the under colour and moves to the top.
        let cmap = crate::colors::colormaps::get("viridis")
            .and_then(|cmap| cmap.with_under(Rgba::BLACK))
            .unwrap();
        let options = ColorbarOptions {
            extend: Some(Extend::Min),
            ..ColorbarOptions::default()
        };
        let colorbar = colorbar(cmap, options);
        let extension = || {
            let patches = colorbar.extend_patches();
            assert_eq!(patches.len(), 1);
            let ys: Vec<f64> = patches[0]
                .shape()
                .0
                .vertices()
                .iter()
                .map(|p| p.y)
                .collect();
            (patches[0].colors().unwrap().face, ys)
        };
        let (face, ys) = extension();
        assert_eq!(face, Rgba::BLACK);
        assert!(ys.iter().all(|&y| y <= 0.0), "below the axes: {ys:?}");
        colorbar.long_axis().set_inverted(true);
        let (face, ys) = extension();
        assert_eq!(face, Rgba::BLACK);
        assert!(ys.iter().all(|&y| y >= 1.0), "above the axes: {ys:?}");
    }

    #[test]
    fn the_outline_keeps_its_style_when_redrawn() {
        let viridis = crate::colors::colormaps::get("viridis").unwrap();
        let colorbar = colorbar(viridis.clone(), ColorbarOptions::default());
        colorbar.outline().set_edgecolor("red");
        let log =
            ScalarMappable::with_norm(viridis, Box::new(crate::colors::LogNorm::new(1.0, 100.0)));
        colorbar.update_normal(&log).unwrap();
        let red = Color::from("red").resolve().unwrap();
        assert_eq!(colorbar.outline().colors().unwrap().edge, red);
    }
}
