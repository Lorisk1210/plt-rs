//! Multi-font layout: matplotlib's font fallback plus FreeType-style text
//! extents (`FT2Font.set_text` / `get_width_height` / `get_descent`).

use crate::error::Result;
use crate::face::Face;
use crate::scale::{Scaler, Size};
use crate::shape::{Script, ShapeOptions, script_runs, shape_run};
use crate::types::{GlyphId, Rect};

/// An ordered list of faces used like matplotlib's `FT2Font` fallback list
/// (whose last entry is usually the Last Resort font): text is shaped with
/// the first face, and every cluster that comes out as `.notdef` moves on
/// to the next face.
#[derive(Clone, Debug)]
pub struct FontChain<'a> {
    faces: Vec<Face<'a>>,
}

/// One glyph placed by [`FontChain::layout`]; positions in 26.6 pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LaidOutGlyph {
    /// Index of the face in the chain.
    pub face: usize,
    /// The glyph.
    pub glyph_id: GlyphId,
    /// Byte offset in the source text of the glyph's cluster.
    pub cluster: usize,
    /// Horizontal glyph origin (pen position plus offset).
    pub x: i32,
    /// Vertical glyph origin (y up).
    pub y: i32,
    /// Horizontal advance.
    pub x_advance: i32,
}

/// Result of laying out a string, all in 26.6 pixels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextLayout {
    /// Placed glyphs in visual order.
    pub glyphs: Vec<LaidOutGlyph>,
    /// Total advance (matplotlib's text width).
    pub width: i32,
    /// Height of the union of glyph control boxes (and the origin).
    pub height: i32,
    /// Depth of that union below the baseline.
    pub descent: i32,
}

impl<'a> FontChain<'a> {
    /// Builds a chain; the first face is the primary one.
    pub fn new(faces: Vec<Face<'a>>) -> Self {
        Self { faces }
    }

    /// The faces in fallback order.
    pub fn faces(&self) -> &[Face<'a>] {
        &self.faces
    }

    /// Lays out one line of `text` at `points` / `dpi` like matplotlib's
    /// `FT2Font.set_text(text, 0, LoadFlags.NO_HINTING)`: runs split by
    /// script and font are shaped separately and placed one after the other.
    pub fn layout(
        &self,
        text: &str,
        points: f64,
        dpi: f64,
        options: &ShapeOptions,
    ) -> Result<TextLayout> {
        let mut layout = TextLayout::default();
        if self.faces.is_empty() {
            return Ok(layout);
        }
        let sizes: Vec<Size> = self
            .faces
            .iter()
            .map(|face| Size::new(face, points, dpi))
            .collect();
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let runs = script_runs(&chars, options.script);
        let assignment = self.assign_faces(&chars, &runs, options);
        let mut bbox: Option<Rect> = None;
        let mut pen = 0i32;
        for &(start, end, script) in &runs {
            for (segment_start, segment_end) in segments(&assignment, start, end) {
                let face_index = assignment[segment_start];
                let (face, size) = (&self.faces[face_index], &sizes[face_index]);
                let segment = &chars[segment_start..segment_end];
                for glyph in shape_run(face, segment, script, options, Scaler::Pixels(*size)) {
                    let (x, y) = (pen + glyph.x_offset, glyph.y_offset);
                    let cbox =
                        size.glyph_cbox(face, glyph.glyph_id)?
                            .map_or(Rect::default(), |r| Rect {
                                x_min: r.x_min + x,
                                y_min: r.y_min + y,
                                x_max: r.x_max + x,
                                y_max: r.y_max + y,
                            });
                    bbox = Some(bbox.map_or(cbox, |b| union(b, cbox)));
                    layout.glyphs.push(LaidOutGlyph {
                        face: face_index,
                        glyph_id: glyph.glyph_id,
                        cluster: glyph.cluster,
                        x,
                        y,
                        x_advance: glyph.x_advance,
                    });
                    pen += glyph.x_advance;
                }
            }
        }
        let bbox = bbox.unwrap_or_default();
        layout.width = pen;
        layout.height = bbox.height();
        layout.descent = -bbox.y_min;
        Ok(layout)
    }

    /// matplotlib's font fallback: every cluster shaped to `.notdef` moves
    /// on to the next face, until all clusters render or reach the last
    /// face. Returns the face index of each character.
    fn assign_faces(
        &self,
        chars: &[(usize, char)],
        runs: &[(usize, usize, Script)],
        options: &ShapeOptions,
    ) -> Vec<usize> {
        let last = self.faces.len() - 1;
        let mut assignment = vec![0; chars.len()];
        loop {
            let mut changed = false;
            for &(start, end, script) in runs {
                for (segment_start, segment_end) in segments(&assignment, start, end) {
                    let face_index = assignment[segment_start];
                    if face_index == last {
                        continue;
                    }
                    let segment = &chars[segment_start..segment_end];
                    let glyphs = shape_run(
                        &self.faces[face_index],
                        segment,
                        script,
                        options,
                        Scaler::Units,
                    );
                    let mut clusters: Vec<usize> = glyphs.iter().map(|g| g.cluster).collect();
                    clusters.sort_unstable();
                    clusters.dedup();
                    for glyph in glyphs.iter().filter(|g| g.glyph_id == GlyphId::NOTDEF) {
                        let next = clusters
                            .iter()
                            .copied()
                            .find(|&c| c > glyph.cluster)
                            .unwrap_or(usize::MAX);
                        for (index, &(offset, _)) in chars
                            .iter()
                            .enumerate()
                            .take(segment_end)
                            .skip(segment_start)
                        {
                            if (glyph.cluster..next).contains(&offset)
                                && assignment[index] == face_index
                            {
                                assignment[index] = face_index + 1;
                                changed = true;
                            }
                        }
                    }
                }
            }
            if !changed {
                return assignment;
            }
        }
    }
}

/// Maximal `(start, end)` ranges of `start..end` with one face assignment.
fn segments(assignment: &[usize], start: usize, end: usize) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for index in start..end {
        match out.last_mut() {
            Some(segment) if assignment[segment.0] == assignment[index] => segment.1 = index + 1,
            _ => out.push((index, index + 1)),
        }
    }
    out
}

fn union(a: Rect, b: Rect) -> Rect {
    Rect {
        x_min: a.x_min.min(b.x_min),
        y_min: a.y_min.min(b.y_min),
        x_max: a.x_max.max(b.x_max),
        y_max: a.y_max.max(b.y_max),
    }
}
