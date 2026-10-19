//! What a colorbar remembers, kept in its axes so that it can follow its
//! mappable (matplotlib's `Colorbar` attributes, `update_normal` and the
//! `'changed'` callback connecting them).

use std::cell::RefCell;
use std::rc::Rc;

use super::blocks::{BlockSpec, Blocks};
use super::mappable::{ContourLevels, MappableLink, Source};
use super::ticking::{self, Tickers};
use super::{Colorbar, ColorbarError, ExtendFrac, Spacing, long_axis};
use crate::artist::{Artist, ArtistBase, ArtistId, DrawContext, DrawError, RawHandle};
use crate::axes::Axes;
use crate::colors::{Color, Extend, ScalarMappable};
use crate::figure::FigureState;
use crate::image::ScalarKind;
use crate::orientation::Orientation;
use crate::spines::SpineSide;

/// How the colour blocks are laid out, as the colorbar was asked to.
#[derive(Clone, Debug)]
pub(crate) struct BlockSettings {
    /// Explicit block edges (`boundaries`).
    pub(crate) boundaries: Option<Vec<f64>>,
    /// Explicit block values (`values`).
    pub(crate) values: Option<Vec<f64>>,
    /// The spacing of discrete colorbars.
    pub(crate) spacing: Spacing,
    /// The length of the extensions.
    pub(crate) extendfrac: ExtendFrac,
    /// Rectangular extensions.
    pub(crate) extendrect: bool,
    /// Lines between the colours.
    pub(crate) drawedges: bool,
    /// The long-to-short ratio of a colorbar that made its own axes.
    pub(crate) aspect: Option<f64>,
}

/// The artists a colorbar drew in its axes, replaced when it is redrawn.
#[derive(Clone, Debug, Default)]
pub(crate) struct Drawn {
    pub(crate) outline: Option<ArtistId>,
    pub(crate) extensions: Vec<ArtistId>,
    pub(crate) solids: Option<ArtistId>,
    /// The hatched blocks drawn instead of `solids` for hatched contours.
    pub(crate) solids_patches: Vec<ArtistId>,
    pub(crate) dividers: Option<ArtistId>,
    pub(crate) lines: Vec<ArtistId>,
}

/// Everything a colorbar needs to redraw itself.
#[derive(Clone, Debug)]
pub(crate) struct ColorbarState {
    /// The artist followed; `None` for a bare norm and colormap, or a
    /// mappable in another figure, which the colorbar shows as it was.
    pub(crate) link: Option<MappableLink>,
    pub(crate) orientation: Orientation,
    /// The side of the ticks and label.
    pub(crate) ticklocation: SpineSide,
    pub(crate) extend: Extend,
    /// The colormap and norm drawn (matplotlib's `cmap` and `norm`).
    pub(crate) mapping: ScalarMappable,
    /// The opacity of the colours.
    pub(crate) alpha: Option<f64>,
    /// The levels of a contour set.
    pub(crate) contours: Option<ContourLevels>,
    pub(crate) settings: BlockSettings,
    pub(crate) blocks: Blocks,
    /// The block edges along the long axis (matplotlib's `_y`).
    pub(crate) edges: Vec<f64>,
    pub(crate) drawn: Drawn,
}

impl BlockSettings {
    /// matplotlib's `_process_values` for `mapping`: contour levels, or
    /// the explicit boundaries and values, or blocks from the norm (whose
    /// limits are made non-singular).
    pub(crate) fn blocks(
        &self,
        contours: Option<&ContourLevels>,
        extend: Extend,
        inverted: bool,
        mapping: &mut ScalarMappable,
    ) -> Result<Blocks, ColorbarError> {
        let spec = BlockSpec {
            ncolors: mapping.cmap().n(),
            boundaries: contours
                .map(|levels| levels.extended.as_slice())
                .or(self.boundaries.as_deref()),
            values: contours
                .map(|levels| levels.cvalues.as_slice())
                .or(self.values.as_deref()),
            value_kind: contours.map_or(ScalarKind::Float, |levels| levels.cvalue_kind),
            extend,
            inverted,
            extendfrac: self.extendfrac,
            spacing: self.spacing,
        };
        Blocks::new(&spec, mapping.norm_mut())
    }
}

impl ColorbarState {
    /// matplotlib's `update_normal`: takes the mappable's colormap, norm,
    /// opacity and levels; when the norm was replaced (`reset`) the long
    /// axis gets the norm's scale and tickers anew, else they are kept.
    /// Then everything is redrawn.
    fn follow(&mut self, ax: &Axes, source: Source, reset: bool) -> Result<(), ColorbarError> {
        self.mapping = source.mapping;
        self.alpha = source.alpha;
        self.contours = source.contours;
        let long = ax.axis_of(long_axis(self.orientation));
        self.blocks = self.settings.blocks(
            self.contours.as_ref(),
            self.extend,
            long.inverted(),
            &mut self.mapping,
        )?;
        let contours = self.contours.is_some();
        let scale = ticking::long_axis_scale(
            &self.blocks,
            self.mapping.norm(),
            self.settings.spacing,
            contours,
        );
        let tickers = if reset {
            ax.set_scale(long.name(), scale);
            Some(Tickers::choose(
                &self.blocks,
                self.mapping.norm(),
                None,
                None,
            )?)
        } else {
            // matplotlib's scales made from a norm call the norm itself,
            // so they follow its new limits.
            if ticking::scale_follows_limits(&self.blocks, self.mapping.norm()) {
                ax.adopt_scale(long.name(), scale);
            }
            None
        };
        self.edges = self.blocks.edges(self.mapping.norm_mut())?;
        self.redraw(ax, tickers.as_ref())
    }

    /// Draws the colorbar with `tickers` on the long axis (`None` keeps
    /// those it has), the lines of line contours and the room for the
    /// extensions.
    pub(crate) fn redraw(
        &mut self,
        ax: &Axes,
        tickers: Option<&Tickers>,
    ) -> Result<(), ColorbarError> {
        self.draw_all(ax, tickers)?;
        if let Some(levels) = self.contours.clone().filter(|levels| !levels.filled) {
            let colors = levels.colors.iter().map(|&c| Color::from(c)).collect();
            self.add_level_lines(ax, &levels.levels, colors, levels.linewidths, true)?;
        }
        self.install_locator(ax)
    }
}

/// The state of a colorbar in its axes. The colour blocks, outline and
/// lines are artists of the axes; this one draws nothing.
#[derive(Debug)]
pub(crate) struct ColorbarArtist {
    base: ArtistBase,
    pub(crate) state: ColorbarState,
}

impl ColorbarArtist {
    pub(crate) fn new(state: ColorbarState) -> Self {
        let mut base = ArtistBase::default();
        "_nolegend_".clone_into(&mut base.label);
        base.in_layout = false;
        Self { base, state }
    }
}

impl Artist for ColorbarArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, _ctx: &mut DrawContext<'_>) {}
}

impl Colorbar {
    /// Follows the current colour mapping of the mappable, if the colorbar
    /// has one that is still in the figure.
    fn update(&self, reset: bool) -> Result<(), ColorbarError> {
        let Some(link) = self.read(|state| state.link) else {
            return Ok(());
        };
        let source = link.source(&self.raw.figure_state().borrow().store);
        match source {
            None => Ok(()),
            Some(source) => {
                let source = source?;
                self.edit(|state| state.follow(&self.ax, source, reset))
            }
        }
    }
}

/// Lets the colorbars following `mappable` take its new colour mapping;
/// `reset` when its norm was replaced. Problems are reported with the next
/// draw.
pub(crate) fn follow(figure: &Rc<RefCell<FigureState>>, mappable: ArtistId, reset: bool) {
    let colorbars: Vec<ArtistId> = {
        let state = figure.borrow();
        state
            .store
            .ids()
            .filter(|&id| {
                state
                    .store
                    .get_as::<ColorbarArtist>(id)
                    .and_then(|colorbar| colorbar.state.link)
                    .is_some_and(|link| link.id() == mappable)
            })
            .collect()
    };
    for id in colorbars {
        let Some(colorbar) = Colorbar::from_raw(RawHandle::new(Rc::clone(figure), id)) else {
            continue;
        };
        if let Err(err) = colorbar.update(reset) {
            figure.borrow_mut().pending_errors.push(DrawError {
                artist: id,
                message: err.to_string(),
            });
        }
    }
}
