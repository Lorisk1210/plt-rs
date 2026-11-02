//! What a colorbar remembers, kept in its axes so that it can follow its
//! mappable (matplotlib's `Colorbar` attributes, `update_normal` and the
//! `'changed'` callback connecting them).

use std::cell::RefCell;
use std::rc::Rc;

use super::blocks::{BlockSpec, Blocks};
use super::mappable::{ContourLevels, Mappable, MappableLink, Source};
use super::ticking::{self, Tickers};
use super::{Colorbar, ColorbarError, ExtendFrac, Spacing, long_axis};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ArtistStore, DrawContext, DrawError, RawHandle,
};
use crate::axes::{Axes, AxesState};
use crate::colors::{Color, Extend, Norm, NormKind, ScalarMappable};
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

pltrs_core::archive_struct!(BlockSettings {
    boundaries,
    values,
    spacing,
    extendfrac,
    extendrect,
    drawedges,
    aspect
});

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

pltrs_core::archive_struct!(Drawn {
    outline,
    extensions,
    solids,
    solids_patches,
    dividers,
    lines
});

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

pltrs_core::archive_struct!(ColorbarState {
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
    drawn
});

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
    pub(super) fn follow(
        &mut self,
        ax: &Axes,
        source: Source,
        reset: bool,
    ) -> Result<(), ColorbarError> {
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

pltrs_core::archive_struct!(ColorbarArtist { base, state });

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
    /// Shows the colour mapping of `mappable` from now on (matplotlib's
    /// `update_normal`): its colormap, norm, opacity and levels; the
    /// colorbar then follows the mappable's changes as it follows those of
    /// the one it was made for. The long axis gets the norm's scale and
    /// default ticks anew when `mappable` is another artist than the one
    /// followed, or a bare norm and colormap with a norm of another kind
    /// than the colorbar's (matplotlib: a norm that is another object);
    /// otherwise it keeps its ticks.
    ///
    /// ```
    /// use pltrs::colorbar::ColorbarOptions;
    /// use pltrs::colors::{LogNorm, Normalize, ScalarMappable, colormaps};
    /// use pltrs::prelude::*;
    /// use pltrs::scale::ScaleName;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let mut sm = ScalarMappable::with_norm(colormaps::get("viridis")?, Box::new(Normalize::new(0.0, 1.0)));
    /// let colorbar = fig.colorbar(&sm, ColorbarOptions::beside(&ax))?;
    /// sm.set_norm(Box::new(LogNorm::new(1.0, 100.0)));
    /// colorbar.update_normal(&sm)?;
    /// assert_eq!(colorbar.ax().yscale(), ScaleName::Log);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the norm cannot map the colorbar's values, or the mappable was
    /// removed from its figure.
    pub fn update_normal<'a>(
        &self,
        mappable: impl Into<Mappable<'a>>,
    ) -> Result<&Self, ColorbarError> {
        let mappable = mappable.into();
        let source = mappable.source()?;
        let link = mappable
            .link()
            .filter(|(figure, _)| Rc::ptr_eq(figure, self.raw.figure_state()))
            .map(|(_, link)| link);
        let reset = match (self.read(|state| state.link), link) {
            (Some(old), Some(new)) => old.id() != new.id(),
            (None, None) => {
                self.read(|state| norm_replaced(state.mapping.norm(), source.mapping.norm()))
            }
            _ => true,
        };
        self.edit(|state| {
            state.link = link;
            state.follow(&self.ax, source, reset)
        })?;
        Ok(self)
    }

    /// Follows the current colour mapping of the mappable, if the colorbar
    /// has one that is still in the figure.
    fn update(&self, reset: bool) -> Result<(), ColorbarError> {
        let Some(link) = self.read(|state| state.link) else {
            return Ok(());
        };
        let source = link.source(&*self.raw.figure_state().borrow());
        match source {
            None => Ok(()),
            Some(source) => {
                let source = source?;
                self.edit(|state| state.follow(&self.ax, source, reset))
            }
        }
    }
}

/// Whether `new` is a norm of another kind than `old`: another family,
/// or a norm made from another scale.
fn norm_replaced(old: &dyn Norm, new: &dyn Norm) -> bool {
    match (old.kind(), new.kind()) {
        (NormKind::Scaled(old), NormKind::Scaled(new)) => old.name() != new.name(),
        (old, new) => std::mem::discriminant(&old) != std::mem::discriminant(&new),
    }
}

/// Lets the colorbars following `mappable` take its new colour mapping;
/// `reset` when its norm was replaced. Problems are reported with the next
/// draw.
pub(crate) fn follow(figure: &Rc<RefCell<FigureState>>, mappable: ArtistId, reset: bool) {
    for id in following(figure, mappable) {
        let Some(colorbar) = Colorbar::from_raw(RawHandle::new(Rc::clone(figure), id)) else {
            continue;
        };
        if let Err(err) = colorbar.update(reset) {
            report(figure, id, &err);
        }
    }
}

/// The colorbars following `mappable`, oldest first.
fn following(figure: &Rc<RefCell<FigureState>>, mappable: ArtistId) -> Vec<ArtistId> {
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
}

/// The colorbar made last for `mappable` that still follows it, if any
/// (matplotlib's `mappable.colorbar`).
pub(crate) fn colorbar_of(mappable: Mappable<'_>) -> Option<Colorbar> {
    let (figure, link) = mappable.link()?;
    let id = following(figure, link.id()).pop()?;
    Colorbar::from_raw(RawHandle::new(Rc::clone(figure), id))
}

/// Lets the colorbar drawn in `ax`, if any, follow a change of the view
/// limits (matplotlib's `xlim_changed` and `ylim_changed` callbacks).
/// Problems are reported with the next draw.
pub(crate) fn follow_view(ax: &Axes) {
    let figure = ax.raw().figure_state();
    let Some(id) = colorbar_id(&figure.borrow().store, ax.id()) else {
        return;
    };
    let colorbar = Colorbar {
        ax: ax.clone(),
        raw: RawHandle::new(Rc::clone(figure), id),
    };
    if let Err(err) = colorbar.edit(|state| state.follow_view(ax)) {
        report(figure, id, &err);
    }
}

/// The id of the colorbar drawn in `axes`, if any.
pub(super) fn colorbar_id(store: &ArtistStore, axes: ArtistId) -> Option<ArtistId> {
    store
        .get_as::<AxesState>(axes)?
        .children
        .iter()
        .copied()
        .find(|&id| store.get_as::<ColorbarArtist>(id).is_some())
}

/// Reports a problem of the colorbar `artist` with the next draw.
fn report(figure: &Rc<RefCell<FigureState>>, artist: ArtistId, err: &ColorbarError) {
    figure
        .borrow_mut()
        .pending_errors
        .push(DrawError::new(artist, err.to_string()));
}
