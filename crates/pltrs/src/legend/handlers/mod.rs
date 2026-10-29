//! Legend handlers: the artists drawn as an entry's handle, made from the
//! artists the entry stands for (`matplotlib.legend_handler`).
//!
//! A handler is a [`LegendHandler`]: given an entry ([`HandlerEntry`]:
//! the original handle, the handle box and the legend's font size), it
//! returns the [`HandlerArtist`]s to draw, sized in points inside the
//! handle box; the legend places the box at layout time. A
//! [`HandlerMap`] says which handler draws which kind of handle
//! ([`HandlerKey`]). Legends start from the default map
//! ([`Legend::default_handler_map`](super::Legend::default_handler_map)),
//! whose built-in handlers copy the look of the original artists into
//! new ones, as matplotlib's do with `update_from`; the
//! [`handler_map`](super::LegendOptions::handler_map) of a legend adds to
//! it, and a handle may carry a handler of its own
//! ([`LegendHandle::with_handler`](super::LegendHandle::with_handler)).

mod builtin;
mod collection;
mod errorbar;
mod line;
mod map;
mod patch;
mod stem;

use std::any::Any;
use std::rc::Rc;

pub use builtin::{
    HandlerErrorbar, HandlerLine2D, HandlerLineCollection, HandlerPatch, HandlerPathCollection,
    HandlerPolyCollection, HandlerStem, HandlerStepPatch,
};
pub use map::{HandlerKey, HandlerMap, TypeKey};
pub(crate) use map::{
    default_handlers, set_default_handlers, update_default_handlers, with_defaults,
};

use super::LegendError;
use super::handle::{LegendHandle, Source};
use super::settings::{Settings, handle_points};
use crate::artist::{Artist, ArtistBase, ArtistStore};
use crate::collections::{CollectionArtist, EdgeColors};
use crate::colors::Color;
use crate::lines::{LineArtist, LineProps};
use crate::numeric::linspace;
use crate::patches::{PatchArtist, PatchProps, PatchShape};
use crate::rc::RcParams;

/// The box a handle is drawn in, in points (matplotlib's handle box
/// `DrawingArea`): handle artists span `-xdescent..width - xdescent`
/// across and `-ydescent..height - ydescent` up, as matplotlib's
/// handlers place them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandleBox {
    /// How far the box reaches left of its origin.
    pub xdescent: f64,
    /// How far the box reaches below its origin.
    pub ydescent: f64,
    /// The width of the box.
    pub width: f64,
    /// The height of the box.
    pub height: f64,
}

/// How a handle artist follows the box it is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Attach {
    /// Its coordinates are points in the box: its transform is the box's.
    Transform,
    /// Its offsets are points in the box: a collection's offset transform
    /// is the box's (its shapes keep their own transform).
    Offsets,
}

/// One artist of a legend handle, in points in the handle box (matplotlib's
/// artists of `create_artists`, drawn with the handle box's transform).
#[derive(Debug)]
pub struct HandlerArtist {
    pub(crate) artist: Box<dyn Artist>,
    pub(crate) attach: Attach,
    /// Whether the artist maps data values to colours (matplotlib's
    /// `get_array() is not None`), which label colours follow.
    pub(crate) mapped: bool,
}

impl HandlerArtist {
    /// A patch of `shape` styled by `props`, with the defaults of the rc
    /// parameters (matplotlib's `Patch` made in `create_artists`).
    pub fn patch<S: PatchShape>(shape: S, props: PatchProps) -> Self {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        props.apply(&mut patch);
        Self::placed(patch)
    }

    /// A line through `(x, y)` styled by `props`, with the defaults of the
    /// rc parameters (matplotlib's `Line2D` made in `create_artists`).
    pub fn line(x: Vec<f64>, y: Vec<f64>, props: LineProps) -> Self {
        let mut line = LineArtist::new(x, y, crate::rc::params());
        props.apply(&mut line);
        Self::placed(line)
    }

    /// `artist`, placed by the box's transform.
    pub(crate) fn placed(artist: impl Artist) -> Self {
        Self {
            artist: Box::new(artist),
            attach: Attach::Transform,
            mapped: false,
        }
    }
}

/// Draws the handles of legend entries (matplotlib's `HandlerBase`, whose
/// `legend_artist` adds the artists of `create_artists` to the handle
/// box).
///
/// A handler runs while its legend is being made, and gets what it needs
/// from the [`HandlerEntry`]. It is `Send`, so it cannot keep a handle to
/// the figure and use it meanwhile.
///
/// matplotlib's legend guide draws an ellipse for a circle
/// (`HandlerEllipse`):
///
/// ```
/// use pltrs::legend::{HandlerArtist, HandlerEntry, HandlerKey, LegendError, LegendHandle, LegendHandler, LegendOptions};
/// use pltrs::patches::{Circle, Ellipse, PatchProps};
/// use pltrs::prelude::*;
///
/// #[derive(Debug)]
/// struct HandlerEllipse;
///
/// impl LegendHandler for HandlerEllipse {
///     fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
///         let area = entry.area();
///         let center = (0.5 * area.width - 0.5 * area.xdescent, 0.5 * area.height - 0.5 * area.ydescent);
///         let ellipse = Ellipse::new(center, area.width + area.xdescent, area.height + area.ydescent);
///         Ok(vec![entry.patch_like_original(ellipse)])
///     }
/// }
///
/// let ax = Figure::new().subplot();
/// let c = ax.add_patch_with(Circle::new((0.5, 0.5), 0.25), PatchProps::default().facecolor("green"));
/// ax.legend(
///     LegendOptions::default()
///         .handles([&c])
///         .labels(["An ellipse, not a rectangle"])
///         .handler(HandlerKey::of::<Circle>(), HandlerEllipse),
/// )?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait LegendHandler: std::fmt::Debug + Send {
    /// The artists drawing the handle of `entry` in its handle box
    /// (matplotlib's `create_artists`); none leaves the entry out of the
    /// legend.
    ///
    /// # Errors
    ///
    /// When the look of the original artists cannot be resolved.
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError>;
}

/// One entry of a legend being made, as its handler sees it
/// (matplotlib's `legend`, `orig_handle`, `fontsize` and `handlebox`
/// arguments).
pub struct HandlerEntry<'a> {
    pub(crate) ctx: &'a HandlerContext<'a>,
    pub(crate) source: &'a Source,
    pub(crate) area: HandleBox,
}

impl std::fmt::Debug for HandlerEntry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerEntry")
            .field("source", self.source)
            .field("area", &self.area)
            .finish_non_exhaustive()
    }
}

impl HandlerEntry<'_> {
    /// The handle box.
    pub fn area(&self) -> HandleBox {
        self.area
    }

    /// The legend's font size in points.
    pub fn fontsize(&self) -> f64 {
        self.ctx.settings.fontsize
    }

    /// The handle the entry was made for (matplotlib's `orig_handle`).
    pub fn handle(&self) -> LegendHandle {
        LegendHandle(self.source.clone())
    }

    /// The object of a handle made by
    /// [`LegendHandle::object`](super::LegendHandle::object), if it is a
    /// `T`.
    pub fn object<T: Any>(&self) -> Option<&T> {
        match self.source {
            Source::Object(object) => object.value.downcast_ref(),
            _ => None,
        }
    }

    /// A patch of `shape` drawn like the entry's original patch, or the
    /// first bar of a bar container: its face, edge, hatch and line style,
    /// visibility, alpha and label (matplotlib's `HandlerPatch.update_prop`,
    /// which calls `update_from`); a patch in the rc style when the
    /// original is no patch.
    pub fn patch_like_original<S: PatchShape>(&self, shape: S) -> HandlerArtist {
        let mut patch = PatchArtist::new(shape, Rc::clone(self.ctx.rc));
        patch::copy_original(&mut patch, self.ctx, self.source);
        HandlerArtist::placed(patch)
    }
}

/// What handlers read: the figure's artists, the legend's settings and
/// the handler map of the legend being made.
pub(crate) struct HandlerContext<'a> {
    pub(crate) store: &'a ArtistStore,
    pub(crate) settings: &'a Settings,
    pub(crate) rc: &'a Rc<RcParams>,
    pub(crate) handlers: &'a HandlerMap,
}

impl HandlerContext<'_> {
    /// matplotlib's `HandlerNpoints.get_xdata`: the x of the line through
    /// the handle and of its markers, `numpoints` markers spread inside a
    /// pad of 0.3 font sizes, or one in the middle of a full-width line.
    fn xdata(&self, area: &HandleBox, numpoints: usize) -> (Vec<f64>, Vec<f64>) {
        let start = -area.xdescent;
        if numpoints > 1 {
            let pad = 0.3 * self.settings.fontsize;
            let xdata = linspace(start + pad, start + area.width - pad, numpoints);
            (xdata.clone(), xdata)
        } else {
            (
                vec![start, start + area.width],
                vec![start + 0.5 * area.width],
            )
        }
    }

    /// A new line through `(x, y)` with the default style.
    fn line(&self, x: Vec<f64>, y: Vec<f64>) -> LineArtist {
        LineArtist::new(x, y, Rc::clone(self.rc))
    }
}

/// The handle artists of `source` in `area`, or `None` when no handler
/// draws it (matplotlib's `get_legend_handler` + `legend_artist`).
pub(crate) fn create(
    ctx: &HandlerContext<'_>,
    source: &Source,
    area: &HandleBox,
) -> Result<Option<Vec<HandlerArtist>>, LegendError> {
    match ctx.handlers.handler_for(source, ctx.store) {
        Some(handler) => draw(ctx, handler.as_ref(), source, area),
        None => Ok(None),
    }
}

/// The artists `handler` makes for `source` in `area`, `None` when it
/// makes none. A handle's own handler and number of markers are applied
/// here, so that handlers see the handle itself.
fn draw(
    ctx: &HandlerContext<'_>,
    handler: &dyn LegendHandler,
    source: &Source,
    area: &HandleBox,
) -> Result<Option<Vec<HandlerArtist>>, LegendError> {
    match source {
        Source::Handled(inner, _) => draw(ctx, handler, inner, area),
        Source::Npoints(inner, numpoints) => {
            let settings = Settings {
                numpoints: handle_points(*numpoints)?,
                ..ctx.settings.clone()
            };
            let ctx = HandlerContext {
                settings: &settings,
                ..*ctx
            };
            draw(&ctx, handler, inner, area)
        }
        _ => {
            let entry = HandlerEntry {
                ctx,
                source,
                area: *area,
            };
            let artists = handler.create_artists(&entry)?;
            Ok((!artists.is_empty()).then_some(artists))
        }
    }
}

/// Copies the properties matplotlib's `Artist.update_from` copies, except
/// the transform and clipping, which legend handles reset.
fn copy_base(dst: &mut ArtistBase, src: &ArtistBase) {
    dst.visible = src.visible;
    dst.alpha = src.alpha;
    dst.clip_on = src.clip_on;
    dst.label.clone_from(&src.label);
    dst.sketch = src.sketch;
    dst.path_effects.clone_from(&src.path_effects);
}

/// Copies the style of a line (matplotlib's `Line2D.update_from`).
fn copy_line(dst: &mut LineArtist, src: &LineArtist) {
    copy_base(dst.base_mut(), src.base());
    dst.linestyle = src.linestyle.clone();
    dst.linewidth = src.linewidth;
    dst.color = src.color.clone();
    dst.gapcolor = src.gapcolor.clone();
    dst.markersize = src.markersize;
    dst.markerfacecolor = src.markerfacecolor.clone();
    dst.markerfacecoloralt = src.markerfacecoloralt.clone();
    dst.markeredgecolor = src.markeredgecolor.clone();
    dst.markeredgewidth = src.markeredgewidth;
    dst.dash_capstyle = src.dash_capstyle;
    dst.dash_joinstyle = src.dash_joinstyle;
    dst.solid_capstyle = src.solid_capstyle;
    dst.solid_joinstyle = src.solid_joinstyle;
    dst.marker = src.marker.clone();
    dst.fillstyle = src.fillstyle;
    dst.marker_transform = src.marker_transform;
    dst.marker_capstyle = src.marker_capstyle;
    dst.marker_joinstyle = src.marker_joinstyle;
    dst.drawstyle = src.drawstyle;
    dst.recache();
}

/// Copies the look of a collection (matplotlib's `Collection.update_from`):
/// its colours as resolved (colour mapping and alpha applied), widths,
/// line styles, anti-aliasing and hatch.
fn copy_collection(dst: &mut CollectionArtist, src: &CollectionArtist) -> Result<(), LegendError> {
    copy_base(dst.base_mut(), src.base());
    let (faces, edges) = src.resolved_colors().map_err(LegendError::HandleColors)?;
    dst.facecolors = Some(if faces.is_empty() {
        vec![Color::none()]
    } else {
        faces.into_iter().map(Color::from).collect()
    });
    dst.edgecolors = EdgeColors::Colors(edges.into_iter().map(Color::from).collect());
    dst.linewidths.clone_from(&src.linewidths);
    dst.linestyles.clone_from(&src.linestyles);
    dst.antialiased.clone_from(&src.antialiased);
    dst.hatch.clone_from(&src.hatch);
    Ok(())
}

#[cfg(test)]
mod tests;
