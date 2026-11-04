//! The glyph buffer as AAT subtables use it (HarfBuzz's `hb_buffer_t`
//! output side and `hb_aat_apply_context_t`'s glyph bookkeeping).
//!
//! Subtables that change the number of glyphs (ligatures, insertions)
//! read glyphs from the buffer and write them to an output that replaces
//! it at the end; the others change glyphs in place. The buffer also
//! tracks every glyph id it has held since substitution started, which
//! decides whether a subtable can act at all, and the deleted glyphs, which
//! are removed once substitution is over.

use std::collections::HashSet;

use crate::aat::lookup::DELETED_GLYPH;
use crate::layout::buffer::{Buffer, GlyphInfo, GlyphPos, gdef_props, merge_clusters};
use crate::layout::gdef::Gdef;
use crate::types::GlyphId;

/// The glyph AAT tables replace a deleted one with.
pub(crate) const DELETED: GlyphId = GlyphId(DELETED_GLYPH);

/// Buffers shorter than this are searched glyph by glyph instead of
/// keeping a set of their glyphs (HarfBuzz's threshold).
const GLYPH_SET_MIN_LEN: usize = 4;

/// A [`Buffer`] being transformed by AAT subtables.
#[derive(Debug)]
pub(crate) struct AatBuffer<'b, 'g> {
    /// The glyphs; `buffer.idx` is the cursor.
    pub(crate) buffer: &'b mut Buffer,
    /// The glyphs written so far, while a subtable writes an output.
    out: Option<Vec<GlyphInfo>>,
    /// Every glyph id the buffer has held, when it is long enough to keep
    /// them.
    glyph_set: Option<HashSet<u16>>,
    /// The face's glyph classes, given to new glyphs.
    gdef: &'g Gdef<'g>,
    /// Whether the glyphs are in reverse order.
    reversed: bool,
    /// Whether a limit was hit; the subtable stops then.
    failed: bool,
}

impl<'b, 'g> AatBuffer<'b, 'g> {
    /// Starts transforming `buffer` of a face with classes `gdef`
    /// (HarfBuzz's `setup_buffer_glyph_set`).
    pub(crate) fn new(buffer: &'b mut Buffer, gdef: &'g Gdef<'g>) -> Self {
        let glyph_set = (buffer.len() >= GLYPH_SET_MIN_LEN)
            .then(|| buffer.info.iter().map(|info| info.glyph.0).collect());
        Self {
            buffer,
            out: None,
            glyph_set,
            gdef,
            reversed: false,
            failed: false,
        }
    }

    /// Whether any glyph the buffer holds (or, for a long buffer, ever
    /// held) passes `test` (HarfBuzz's `buffer_intersects_machine`).
    pub(crate) fn intersects(&self, test: impl Fn(u16) -> bool) -> bool {
        match &self.glyph_set {
            Some(set) => set.iter().any(|&glyph| test(glyph)),
            None => self.buffer.info.iter().any(|info| test(info.glyph.0)),
        }
    }

    /// Number of input glyphs.
    pub(crate) fn len(&self) -> usize {
        self.buffer.len()
    }

    /// The cursor.
    pub(crate) const fn idx(&self) -> usize {
        self.buffer.idx
    }

    /// The glyph under the cursor.
    pub(crate) fn cur(&self) -> &GlyphInfo {
        &self.buffer.info[self.buffer.idx]
    }

    /// Number of glyphs written to the output.
    pub(crate) fn out_len(&self) -> usize {
        self.out.as_ref().map_or(self.buffer.idx, Vec::len)
    }

    /// Whether a limit stopped the work.
    pub(crate) const fn failed(&self) -> bool {
        self.failed
    }

    /// Spends `count` operations of the buffer's budget; `false` (and the
    /// work stops) once it is exhausted.
    pub(crate) fn spend(&mut self, count: usize) -> bool {
        if (0..count).all(|_| self.buffer.spend_op()) {
            true
        } else {
            self.failed = true;
            false
        }
    }

    /// Reverses the glyphs (and remembers the order).
    pub(crate) fn reverse(&mut self) {
        self.buffer.reverse();
        self.reversed = !self.reversed;
    }

    /// Whether the glyphs are in reverse order.
    pub(crate) const fn is_reversed(&self) -> bool {
        self.reversed
    }

    /// Starts writing an output (HarfBuzz's `clear_output`).
    pub(crate) fn start_output(&mut self) {
        self.buffer.idx = 0;
        self.out = Some(Vec::with_capacity(self.buffer.len()));
    }

    /// Ends the output: it becomes the buffer, followed by the glyphs not
    /// read (HarfBuzz's `sync`).
    pub(crate) fn finish_output(&mut self) {
        if let Some(mut out) = self.out.take() {
            out.extend_from_slice(&self.buffer.info[self.buffer.idx..]);
            self.buffer.pos = vec![GlyphPos::default(); out.len()];
            self.buffer.info = out;
        }
        self.buffer.idx = 0;
    }

    /// Notes `glyph` as held by the buffer.
    fn note(&mut self, glyph: u16) {
        if let Some(set) = &mut self.glyph_set {
            set.insert(glyph);
        }
    }

    /// Gives `info` the glyph `glyph` and its class (HarfBuzz's glyph
    /// property update, done only for fonts with glyph classes).
    fn assign(gdef: &Gdef<'_>, info: &mut GlyphInfo, glyph: GlyphId) {
        info.glyph = glyph;
        if glyph != DELETED && gdef.has_glyph_classes() {
            info.props = gdef_props(gdef, glyph);
        }
    }

    /// Whether the output can take `count` more glyphs.
    fn has_room(&mut self, count: usize) -> bool {
        if self
            .buffer
            .can_grow(self.out_len().saturating_sub(self.buffer.idx) + count)
        {
            true
        } else {
            self.failed = true;
            false
        }
    }

    /// Advances the cursor, copying the glyph to the output if one is
    /// being written (HarfBuzz's `next_glyph`).
    pub(crate) fn next_glyph(&mut self) {
        let idx = self.buffer.idx;
        if let Some(out) = &mut self.out {
            out.push(self.buffer.info[idx]);
        }
        self.buffer.idx = idx + 1;
    }

    /// Copies the glyph under the cursor to the output without advancing
    /// (HarfBuzz's `copy_glyph`).
    pub(crate) fn copy_glyph(&mut self) -> bool {
        if !self.has_room(1) {
            return false;
        }
        let info = self.buffer.info[self.buffer.idx];
        self.out.get_or_insert_default().push(info);
        true
    }

    /// Advances the cursor without output (HarfBuzz's `skip_glyph`).
    pub(crate) fn skip_glyph(&mut self) {
        self.buffer.idx += 1;
    }

    /// Writes a copy of the glyph under the cursor (or of the last glyph
    /// written, at the end) carrying each of `glyphs` (HarfBuzz's
    /// `output_glyphs` of the AAT context).
    pub(crate) fn output_glyphs(&mut self, glyphs: &[GlyphId]) -> bool {
        if !self.has_room(glyphs.len()) {
            return false;
        }
        let idx = self.buffer.idx;
        let template = match (self.buffer.info.get(idx), &self.out) {
            (Some(info), _) => *info,
            (None, Some(out)) => match out.last().or_else(|| self.buffer.info.first()) {
                Some(info) => *info,
                None => return false,
            },
            (None, None) => return false,
        };
        for &glyph in glyphs {
            self.note(glyph.0);
            let mut info = template;
            Self::assign(self.gdef, &mut info, glyph);
            info.aat_deleted = glyph == DELETED;
            self.out.get_or_insert_default().push(info);
        }
        true
    }

    /// Replaces the glyph under the cursor by `glyph` in the output and
    /// advances (HarfBuzz's `replace_glyph` of the AAT context).
    pub(crate) fn replace_glyph(&mut self, glyph: GlyphId) -> bool {
        if !self.has_room(1) {
            return false;
        }
        self.note(glyph.0);
        let idx = self.buffer.idx;
        let mut info = self.buffer.info[idx];
        if glyph == DELETED {
            info.aat_deleted = true;
        }
        Self::assign(self.gdef, &mut info, glyph);
        self.out.get_or_insert_default().push(info);
        self.buffer.idx = idx + 1;
        true
    }

    /// Deletes the glyph under the cursor: it is written as the deleted
    /// glyph, removed after substitution (HarfBuzz's `delete_glyph` of the
    /// AAT context).
    pub(crate) fn delete_glyph(&mut self) -> bool {
        if !self.has_room(1) {
            return false;
        }
        let idx = self.buffer.idx;
        let mut info = self.buffer.info[idx];
        info.aat_deleted = true;
        info.glyph = DELETED;
        self.out.get_or_insert_default().push(info);
        self.buffer.idx = idx + 1;
        true
    }

    /// Replaces the glyph at `index` of the buffer in place (HarfBuzz's
    /// `replace_glyph_inplace`).
    pub(crate) fn replace_in_place(&mut self, index: usize, glyph: GlyphId) {
        self.note(glyph.0);
        Self::assign(self.gdef, &mut self.buffer.info[index], glyph);
    }

    /// Moves the cursor so that `target` glyphs have been written,
    /// writing glyphs ahead or taking written ones back into the input
    /// (HarfBuzz's `move_to`).
    pub(crate) fn move_to(&mut self, target: usize) -> bool {
        let Some(out) = &mut self.out else {
            self.buffer.idx = target.min(self.buffer.len());
            return true;
        };
        let idx = self.buffer.idx;
        let written = out.len();
        if written < target {
            let count = target - written;
            let Some(ahead) = self.buffer.info.get(idx..idx + count) else {
                self.failed = true;
                return false;
            };
            out.extend_from_slice(ahead);
            self.buffer.idx = idx + count;
        } else if written > target {
            let count = written - target;
            let mut idx = idx;
            if idx < count {
                // HarfBuzz's `shift_forward`: room before the cursor.
                let room = count - idx;
                if !self.buffer.can_grow(room) {
                    self.failed = true;
                    return false;
                }
                let filler = out[target];
                self.buffer
                    .info
                    .splice(idx..idx, std::iter::repeat_n(filler, room));
                idx += room;
            }
            idx -= count;
            self.buffer.info[idx..idx + count].copy_from_slice(&out[target..]);
            out.truncate(target);
            self.buffer.idx = idx;
        }
        true
    }

    /// Merges the clusters of the output glyphs `start..end`, extending
    /// into the input when the range reaches the end of the output
    /// (HarfBuzz's `merge_out_clusters`).
    pub(crate) fn merge_out_clusters(&mut self, start: usize, end: usize) {
        let Some(out) = &mut self.out else {
            self.buffer.merge_clusters(start, end);
            return;
        };
        if end.saturating_sub(start) < 2 || end > out.len() {
            return;
        }
        let cluster = out[start..end]
            .iter()
            .map(|info| info.cluster)
            .min()
            .unwrap_or_default();
        let mut start = start;
        while start > 0 && out[start - 1].cluster == out[start].cluster {
            start -= 1;
        }
        let mut end = end;
        while end < out.len() && out[end - 1].cluster == out[end].cluster {
            end += 1;
        }
        if end == out.len() {
            let last = out[end - 1].cluster;
            for info in self.buffer.info[self.buffer.idx..]
                .iter_mut()
                .take_while(|info| info.cluster == last)
            {
                info.cluster = cluster;
            }
        }
        for info in &mut out[start..end] {
            info.cluster = cluster;
        }
    }

    /// Merges the clusters of the input glyphs `start..end`.
    pub(crate) fn merge_clusters(&mut self, start: usize, end: usize) {
        self.buffer.merge_clusters(start, end);
    }
}

/// Removes the glyphs AAT deleted, giving their clusters to their
/// neighbours (HarfBuzz's `delete_glyphs_inplace` with
/// `is_deleted_glyph`).
pub(crate) fn remove_deleted_glyphs(buffer: &mut Buffer) {
    if !buffer.info.iter().any(|info| info.aat_deleted) {
        return;
    }
    let count = buffer.len();
    let mut kept = 0;
    for i in 0..count {
        let info = buffer.info[i];
        if info.aat_deleted {
            let cluster = info.cluster;
            if i + 1 < count && cluster == buffer.info[i + 1].cluster {
                continue;
            }
            if kept > 0 {
                let old = buffer.info[kept - 1].cluster;
                if cluster < old {
                    for k in (0..kept).rev() {
                        if buffer.info[k].cluster != old {
                            break;
                        }
                        buffer.info[k].cluster = cluster;
                    }
                }
                continue;
            }
            if i + 1 < count {
                merge_clusters(&mut buffer.info, i, i + 2, |info| &mut info.cluster);
            }
            continue;
        }
        buffer.info[kept] = buffer.info[i];
        buffer.pos[kept] = buffer.pos[i];
        kept += 1;
    }
    buffer.info.truncate(kept);
    buffer.pos.truncate(kept);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::buffer::CharProps;

    fn buffer(glyphs: &[u16]) -> Buffer {
        Buffer::new(
            glyphs
                .iter()
                .zip(0..)
                .map(|(&g, cluster)| GlyphInfo::new(GlyphId(g), cluster, CharProps::default(), 1))
                .collect(),
        )
    }

    fn glyphs(buffer: &Buffer) -> Vec<u16> {
        buffer.info.iter().map(|info| info.glyph.0).collect()
    }

    #[test]
    fn output_moves_back_and_forth() {
        let gdef = Gdef::default();
        let mut glyphs_in = buffer(&[1, 2, 3, 4]);
        let mut aat = AatBuffer::new(&mut glyphs_in, &gdef);
        aat.start_output();
        aat.next_glyph();
        aat.next_glyph();
        assert_eq!((aat.out_len(), aat.idx()), (2, 2));
        // Back to the first glyph, which becomes a ligature of the two.
        assert!(aat.move_to(0));
        assert_eq!((aat.out_len(), aat.idx(), aat.cur().glyph.0), (0, 0, 1));
        assert!(aat.replace_glyph(GlyphId(9)));
        assert!(aat.delete_glyph());
        aat.merge_out_clusters(0, 2);
        assert!(aat.move_to(3));
        assert!(aat.output_glyphs(&[GlyphId(7)]));
        aat.finish_output();
        assert_eq!(glyphs(aat.buffer), [9, DELETED_GLYPH, 3, 7, 4]);
        assert_eq!(aat.buffer.pos.len(), 5);
        remove_deleted_glyphs(aat.buffer);
        assert_eq!(glyphs(&glyphs_in), [9, 3, 7, 4]);
        assert_eq!(glyphs_in.info[0].cluster, 0);
    }

    #[test]
    fn rewinding_past_the_cursor_makes_room() {
        let gdef = Gdef::default();
        let mut glyphs_in = buffer(&[1, 2]);
        let mut aat = AatBuffer::new(&mut glyphs_in, &gdef);
        aat.start_output();
        aat.next_glyph();
        assert!(aat.output_glyphs(&[GlyphId(5), GlyphId(6)]));
        assert_eq!((aat.out_len(), aat.idx()), (3, 1));
        assert!(aat.move_to(0));
        assert_eq!(aat.idx(), 0);
        assert_eq!(glyphs(aat.buffer), [1, 5, 6, 2]);
        aat.finish_output();
        assert_eq!(glyphs(&glyphs_in), [1, 5, 6, 2]);
    }

    #[test]
    fn glyph_sets_remember_replaced_glyphs() {
        let gdef = Gdef::default();
        let mut short = buffer(&[1, 2]);
        let mut aat = AatBuffer::new(&mut short, &gdef);
        aat.replace_in_place(0, GlyphId(8));
        assert!(!aat.intersects(|g| g == 1) && aat.intersects(|g| g == 8));
        let mut long = buffer(&[1, 2, 3, 4]);
        let mut aat = AatBuffer::new(&mut long, &gdef);
        aat.replace_in_place(0, GlyphId(8));
        assert!(aat.intersects(|g| g == 1) && aat.intersects(|g| g == 8));
    }

    #[test]
    fn deleted_glyphs_give_their_clusters_away() {
        let mut deleted = buffer(&[1, 2, 3]);
        deleted.info[0].aat_deleted = true;
        deleted.info[2].aat_deleted = true;
        deleted.info[1].cluster = 5;
        remove_deleted_glyphs(&mut deleted);
        assert_eq!(glyphs(&deleted), [2]);
        assert_eq!(deleted.info[0].cluster, 0);
    }
}
