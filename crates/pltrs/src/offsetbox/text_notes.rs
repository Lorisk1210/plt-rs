//! How many times matplotlib lays out the texts of an offset box, for the
//! font lookups each layout makes (see `text::font_notes`).
//!
//! matplotlib measures a box by measuring its children again, each time
//! it is asked: a text area lays its text out (`Text._get_layout`) on
//! every `get_bbox`. Drawing an anchored or an annotation box asks the
//! whole tree twice (for its window extent, then for its offset or its
//! frame), a legend once; then each packer and padded box measures its
//! children once more as it draws them (`_get_bbox_and_child_offsets`),
//! and the text lays itself out as it is drawn. A measure asks the tree a
//! number of times that depends on the box. pltrs lays a tree out once
//! for all that, where and when it needs (also to measure a print it
//! draws), so its layouts count for nothing
//! (`font_notes::layouts_counted`), and the boxes' draws and measures
//! note matplotlib's with these functions.

use super::{ArtistRef, OffsetBox};
use crate::artist::{ArtistId, ArtistLookup, LayoutContext};
use crate::text::{TextArtist, font_notes};

/// The texts of the visible text areas of `tree`, each with the number of
/// packers and padded boxes above it (each of which measures it once more
/// as it is drawn).
fn texts<A: ArtistRef>(tree: &OffsetBox<A>) -> Vec<(ArtistId, usize)> {
    fn visit<A: ArtistRef>(tree: &OffsetBox<A>, depth: usize, found: &mut Vec<(ArtistId, usize)>) {
        if !tree.is_visible() {
            return;
        }
        let below = match tree {
            OffsetBox::TextArea(area) => {
                found.push((area.text.artist_id(), depth));
                return;
            }
            OffsetBox::HPacker(_) | OffsetBox::VPacker(_) | OffsetBox::PaddedBox(_) => depth + 1,
            _ => depth,
        };
        for child in tree.children() {
            visit(child, below, found);
        }
    }
    let mut found = Vec::new();
    visit(tree, 0, &mut found);
    found
}

/// Notes the layouts of `text` that `times` of matplotlib's stand for.
fn note_text_layouts(ctx: LayoutContext<'_>, text: ArtistId, times: usize) {
    if times == 0 {
        return;
    }
    if let Some(artist) = ctx.figure_state().artist_as::<TextArtist>(text) {
        // What cannot be laid out is reported where the box is drawn.
        let _ = font_notes::text_area_layouts(times, || artist.layout(ctx));
    }
}

/// Notes the layouts of the texts of `tree` that matplotlib's draw of the
/// box holding it makes before the texts draw themselves: `measures` of
/// the whole tree by the box, then one for each packer or padded box
/// above a text.
pub(crate) fn note_drawn<A: ArtistRef>(
    ctx: LayoutContext<'_>,
    tree: &OffsetBox<A>,
    measures: usize,
) {
    for (text, depth) in texts(tree) {
        note_text_layouts(ctx, text, measures + depth);
    }
}

/// Notes the layouts of the texts of `tree` that `times` measures of the
/// whole tree make (one each, whatever the depth: a box measures its
/// children in the same pass).
pub(crate) fn note_measured<A: ArtistRef>(
    ctx: LayoutContext<'_>,
    tree: &OffsetBox<A>,
    times: usize,
) {
    for (text, _) in texts(tree) {
        note_text_layouts(ctx, text, times);
    }
}
