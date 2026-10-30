//! The handlers of matplotlib's default handler map, each drawing the
//! handles of one kind of artist or container (`HandlerLine2D`,
//! `HandlerPatch`, `HandlerStepPatch`, `HandlerLineCollection`,
//! `HandlerPathCollection`, `HandlerPolyCollection`, `HandlerErrorbar`,
//! `HandlerStem` and `HandlerTuple`). A handler given a handle of another
//! kind fails with [`LegendError::UnsupportedHandle`], as matplotlib's
//! raise; a handle whose artist was removed draws nothing.

use super::{
    HandleBox, HandlerArtist, HandlerContext, HandlerEntry, LegendHandler, collection, create,
    errorbar, line, patch, stem,
};
use crate::collections::CollectionArtist;
use crate::container::ContainerIds;
use crate::legend::HandlerTuple;
use crate::legend::LegendError;
use crate::legend::handle::{Proxy, Source};
use crate::lines::LineArtist;
use crate::patches::{PatchView, StepPatch};

/// Draws lines: a line across the handle box in the original's style,
/// with the legend's `numpoints` markers (matplotlib's `HandlerLine2D`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandlerLine2D;

pltrs_core::archive_struct!(HandlerLine2D);

impl LegendHandler for HandlerLine2D {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        let line = match entry.source {
            Source::Artist(id) => entry.ctx.store.get_as::<LineArtist>(*id),
            Source::Proxy(Proxy::Line(line)) => Some(line.as_ref()),
            _ => None,
        };
        match line {
            Some(line) => Ok(line::line2d(entry.ctx, line, &entry.area)),
            None => unsupported(entry, "HandlerLine2D"),
        }
    }
}

/// Draws patches and bars: a rectangle filling the handle box in the
/// style of the patch, or of the first bar of a bar container
/// (matplotlib's `HandlerPatch`, and its `update_from_first_child` for
/// bar containers).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandlerPatch;

pltrs_core::archive_struct!(HandlerPatch);

impl LegendHandler for HandlerPatch {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        let original = patch::original_patch(entry.ctx.store, entry.source);
        // A bar container without bars still gets a rectangle.
        let bars = matches!(entry.source, Source::Container(ContainerIds::Bar { .. }));
        if original.is_some() || bars {
            Ok(patch::patch(entry.ctx, original, &entry.area))
        } else {
            unsupported(entry, "HandlerPatch")
        }
    }
}

/// Draws step patches: a rectangle like [`HandlerPatch`] when filled or
/// hatched, else a line in the edge's colour, width and style (matplotlib's
/// `HandlerStepPatch`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandlerStepPatch;

pltrs_core::archive_struct!(HandlerStepPatch);

impl LegendHandler for HandlerStepPatch {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        let artist = match entry.source {
            Source::Artist(id) => entry.ctx.store.get(*id),
            _ => None,
        };
        let Some(artist) = artist else {
            return unsupported(entry, "HandlerStepPatch");
        };
        match artist.as_patch() {
            Some(view @ PatchView { shape, .. }) if shape.is::<StepPatch>() => {
                patch::step_patch(entry.ctx, artist.base(), view, &entry.area)
            }
            Some(view) => Ok(patch::patch(
                entry.ctx,
                Some((artist.base(), view)),
                &entry.area,
            )),
            None => unsupported(entry, "HandlerStepPatch"),
        }
    }
}

/// The collection behind the handle of `entry`, if it is one.
fn collection_of<'a>(entry: &HandlerEntry<'a>) -> Option<&'a CollectionArtist> {
    match entry.source {
        Source::Artist(id) => entry.ctx.store.get_as::<CollectionArtist>(*id),
        _ => None,
    }
}

/// Draws line collections: a line in the colour, width and dash pattern
/// of their first line (matplotlib's `HandlerLineCollection`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandlerLineCollection;

pltrs_core::archive_struct!(HandlerLineCollection);

impl LegendHandler for HandlerLineCollection {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        match collection_of(entry) {
            Some(source) => line::line_collection(entry.ctx, source, &entry.area),
            None => unsupported(entry, "HandlerLineCollection"),
        }
    }
}

/// Draws scatter plots: the legend's `scatterpoints` markers of their
/// first shape, sized between their smallest and largest markers
/// (matplotlib's `HandlerPathCollection`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandlerPathCollection;

pltrs_core::archive_struct!(HandlerPathCollection);

impl LegendHandler for HandlerPathCollection {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        match collection_of(entry) {
            Some(source) => collection::path_collection(entry.ctx, source, &entry.area),
            None => unsupported(entry, "HandlerPathCollection"),
        }
    }
}

/// Draws polygon and patch collections: a rectangle filling the handle
/// box in their first face, edge and hatch colour (matplotlib's
/// `HandlerPolyCollection`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandlerPolyCollection;

pltrs_core::archive_struct!(HandlerPolyCollection);

impl LegendHandler for HandlerPolyCollection {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        match collection_of(entry) {
            Some(source) => patch::poly_collection(entry.ctx, source, &entry.area),
            None => unsupported(entry, "HandlerPolyCollection"),
        }
    }
}

/// Draws error bar containers: the data line and its markers with error
/// bars and caps (matplotlib's `HandlerErrorbar`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandlerErrorbar;

pltrs_core::archive_struct!(HandlerErrorbar);

impl LegendHandler for HandlerErrorbar {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        match entry.source {
            Source::Container(ContainerIds::Errorbar(ids)) => {
                errorbar::errorbar(entry.ctx, ids, &entry.area)
            }
            _ => unsupported(entry, "HandlerErrorbar"),
        }
    }
}

/// Draws stem containers: stems from a baseline to their heads
/// (matplotlib's `HandlerStem`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandlerStem;

pltrs_core::archive_struct!(HandlerStem);

impl LegendHandler for HandlerStem {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        match entry.source {
            Source::Container(ContainerIds::Stem {
                markerline,
                stemlines,
                baseline,
                ..
            }) => stem::stem(entry.ctx, [*markerline, *stemlines, *baseline], &entry.area),
            _ => unsupported(entry, "HandlerStem"),
        }
    }
}

/// Draws the handles of a tuple side by side in `ndivide` parts of the
/// box, cycling, each with its own handler (matplotlib's
/// `HandlerTuple.create_artists`).
impl LegendHandler for HandlerTuple {
    fn create_artists(&self, entry: &HandlerEntry<'_>) -> Result<Vec<HandlerArtist>, LegendError> {
        let Source::Tuple(sources) = entry.source else {
            return unsupported(entry, "HandlerTuple");
        };
        tuple_parts(entry.ctx, sources, *self, &entry.area)
    }
}

/// What `handler` makes of a handle it cannot draw: nothing when the
/// handle's artist was removed, else [`LegendError::UnsupportedHandle`].
fn unsupported(
    entry: &HandlerEntry<'_>,
    handler: &'static str,
) -> Result<Vec<HandlerArtist>, LegendError> {
    match entry.source {
        Source::Artist(id) if entry.ctx.store.get(*id).is_none() => Ok(Vec::new()),
        _ => Err(LegendError::UnsupportedHandle(handler)),
    }
}

/// The handles of `sources` side by side in `layout.ndivide` parts of
/// `area`, cycling.
fn tuple_parts(
    ctx: &HandlerContext<'_>,
    sources: &[Source],
    layout: HandlerTuple,
    area: &HandleBox,
) -> Result<Vec<HandlerArtist>, LegendError> {
    let parts = layout.ndivide.unwrap_or(sources.len()).max(1);
    let pad = layout.pad.unwrap_or(ctx.settings.borderpad) * ctx.settings.fontsize;
    let width = if parts > 1 {
        (area.width - pad * (parts - 1) as f64) / parts as f64
    } else {
        area.width
    };
    let mut made = Vec::new();
    for (i, source) in sources.iter().enumerate() {
        let part = HandleBox {
            xdescent: area.xdescent - (width + pad) * (i % parts) as f64,
            width,
            ..*area
        };
        if let Some(artists) = create(ctx, source, &part)? {
            made.extend(artists);
        }
    }
    Ok(made)
}
