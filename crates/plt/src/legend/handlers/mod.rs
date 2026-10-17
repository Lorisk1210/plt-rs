//! Legend handlers: the artists drawn as an entry's handle, made from the
//! artists the entry stands for (`matplotlib.legend_handler`).
//!
//! Each handler builds new artists sized in points inside the entry's
//! handle box and copies the look of the original artists, as matplotlib's
//! handlers do with `update_from`. The legend places the box at layout
//! time.

mod collection;
mod errorbar;
mod line;
mod patch;
mod stem;

use std::rc::Rc;

use super::LegendError;
use super::handle::Source;
use super::settings::Settings;
use crate::artist::{Artist, ArtistBase, ArtistStore};
use crate::collections::{CollectionArtist, CollectionKind, EdgeColors};
use crate::colors::Color;
use crate::container::ContainerIds;
use crate::lines::LineArtist;
use crate::numeric::linspace;
use crate::patches::StepPatch;
use crate::rc::RcParams;

/// The box a handle is drawn in, in points relative to the box's origin
/// (matplotlib's handle box `DrawingArea`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HandleBox {
    pub(crate) xdescent: f64,
    pub(crate) ydescent: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
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

/// One artist of a handle and how it follows the handle box.
#[derive(Debug)]
pub(crate) struct HandleArtist {
    pub(crate) artist: Box<dyn Artist>,
    pub(crate) attach: Attach,
}

impl HandleArtist {
    fn placed(artist: impl Artist) -> Self {
        Self {
            artist: Box::new(artist),
            attach: Attach::Transform,
        }
    }
}

/// The artists of one handle, in drawing order; the first one is the
/// legend handle proper (matplotlib's `legend_handles` entry).
#[derive(Debug)]
pub(crate) struct HandleArtists {
    pub(crate) artists: Vec<HandleArtist>,
    /// Whether the first artist maps data values to colours (matplotlib's
    /// `get_array() is not None`).
    pub(crate) mapped: bool,
}

impl HandleArtists {
    fn unmapped(artists: Vec<HandleArtist>) -> Self {
        Self {
            artists,
            mapped: false,
        }
    }
}

/// What handlers read: the figure's artists and the legend's settings.
pub(crate) struct HandlerContext<'a> {
    pub(crate) store: &'a ArtistStore,
    pub(crate) settings: &'a Settings,
    pub(crate) rc: &'a Rc<RcParams>,
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

/// The handle artists of `source` in `area`, or `None` when legends do not
/// support it (matplotlib's `get_legend_handler` + `legend_artist`).
pub(crate) fn create(
    ctx: &HandlerContext<'_>,
    source: &Source,
    area: &HandleBox,
) -> Result<Option<HandleArtists>, LegendError> {
    match source {
        Source::Artist(id) => {
            let Some(artist) = ctx.store.get(*id) else {
                return Ok(None);
            };
            if let Some(line) = ctx.store.get_as::<LineArtist>(*id) {
                return Ok(Some(line::line2d(ctx, line, area)));
            }
            if let Some(view) = artist.as_patch() {
                let base = artist.base();
                return Ok(Some(match view.shape.downcast_ref::<StepPatch>() {
                    Some(_) => patch::step_patch(ctx, base, view, area)?,
                    None => patch::patch(ctx, Some((base, view)), area),
                }));
            }
            let Some(collection) = ctx.store.get_as::<CollectionArtist>(*id) else {
                return Ok(None);
            };
            match collection.kind {
                CollectionKind::Line => line::line_collection(ctx, collection, area),
                CollectionKind::Path => collection::path_collection(ctx, collection, area),
                CollectionKind::Poly => patch::poly_collection(ctx, collection, area).map(Some),
                CollectionKind::Generic | CollectionKind::QuadMesh => Ok(None),
            }
        }
        Source::Container(ContainerIds::Bar { patches, .. }) => {
            let first = patches
                .first()
                .and_then(|&id| ctx.store.get(id))
                .and_then(|a| Some((a.base(), a.as_patch()?)));
            Ok(Some(patch::patch(ctx, first, area)))
        }
        Source::Container(ContainerIds::Errorbar(ids)) => errorbar::errorbar(ctx, ids, area),
        Source::Container(ContainerIds::Stem {
            markerline,
            stemlines,
            baseline,
            ..
        }) => stem::stem(ctx, [*markerline, *stemlines, *baseline], area),
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
