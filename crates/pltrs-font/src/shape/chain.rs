//! Multi-font layout: matplotlib's font fallback plus FreeType-style text
//! extents (`FT2Font.set_text` / `get_width_height` / `get_descent`).

use crate::autofit::Autohinter;
use crate::db::{FontDatabase, FontId};
use crate::error::Result;
use crate::face::Face;
use crate::hinting::{HintedFace, Hinter, Hinting};
use crate::scale::{Scaler, Size};
use crate::shape::{ShapeOptions, shape_text};
use crate::types::{GlyphId, Rect};

/// An ordered list of faces used like matplotlib's `FT2Font` fallback list
/// (whose last entry is usually the Last Resort font): text is shaped with
/// the first face, and every cluster that comes out as `.notdef` moves on
/// to the next face.
#[derive(Clone, Debug)]
pub struct FontChain<'a> {
    faces: Vec<Face<'a>>,
    /// The database the faces come from, with their ids, whose cached
    /// autohinter analyses layout reuses.
    database: Option<(&'a FontDatabase, Vec<FontId>)>,
    /// The autohinter analyses of faces outside a database that their
    /// owner keeps, by face (see [`with_autohinters`](Self::with_autohinters)).
    kept: Vec<Option<&'a Autohinter>>,
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
    /// Left edge of that union (matplotlib's `get_bitmap_offset`).
    pub x_min: i32,
}

impl<'a> FontChain<'a> {
    /// Builds a chain; the first face is the primary one.
    pub fn new(faces: Vec<Face<'a>>) -> Self {
        Self {
            faces,
            database: None,
            kept: Vec::new(),
        }
    }

    /// The chain using `analyses`, the autohinter's analysis of each face
    /// where its owner keeps one (made with [`Autohinter::new`] from that
    /// face), instead of analysing the face anew for each layout that
    /// autohints it.
    #[must_use]
    pub fn with_autohinters(mut self, analyses: Vec<Option<&'a Autohinter>>) -> Self {
        self.kept = analyses;
        self
    }

    /// A chain over the faces `ids` of `database`.
    pub(crate) fn from_database(
        faces: Vec<Face<'a>>,
        database: &'a FontDatabase,
        ids: Vec<FontId>,
    ) -> Self {
        Self {
            faces,
            database: Some((database, ids)),
            kept: Vec::new(),
        }
    }

    /// The autohinter analysis of face `index` kept from one layout to the
    /// next: the database's, or the one its owner keeps.
    fn cached_autohinter(&self, index: usize) -> Option<&'a Autohinter> {
        match &self.database {
            Some((database, ids)) => database.autohinter(*ids.get(index)?).ok(),
            None => self.kept.get(index).copied().flatten(),
        }
    }

    /// The faces in fallback order.
    pub fn faces(&self) -> &[Face<'a>] {
        &self.faces
    }

    /// The database font of face `face`, for chains made by
    /// [`FontDatabase::font_chain`] (matplotlib's `ft_object` of a laid-out
    /// glyph).
    pub fn font_id(&self, face: usize) -> Option<FontId> {
        self.database.as_ref()?.1.get(face).copied()
    }

    /// Lays out one line of `text` at `points` / `dpi` like matplotlib's
    /// `FT2Font.set_text(text, 0, flags, features, language)` with the load
    /// flags of `hinting` and the features and languages of `options`:
    /// runs split by bidi level, script, font and language are shaped
    /// separately and placed one after the other in visual order. Faces
    /// `hinting` hints (by bytecode or the autohinter) get hinted advances
    /// and extents.
    pub fn layout(
        &self,
        text: &str,
        points: f64,
        dpi: f64,
        options: &ShapeOptions,
        hinting: Hinting,
    ) -> Result<TextLayout> {
        let sizes: Vec<Size> = self
            .faces
            .iter()
            .map(|face| Size::new(face, points, dpi))
            .collect();
        self.lay_out(text, &sizes, options, hinting, false)
    }

    /// Lays out one line of `text` like [`layout`](Self::layout) for a
    /// font whose size was never set (matplotlib's `FT2Font.set_text`
    /// before `set_size`): FreeType then loads the glyphs unscaled and
    /// unhinted ("undefined scale means no scale"), so that positions,
    /// advances and boxes are in font units, and HarfBuzz scales every
    /// positioning value by the font's scale, which is zero: no kerning,
    /// no mark offsets.
    ///
    /// ```
    /// use pltrs_font::{Face, FontChain, Hinting, ShapeOptions, bundled_font_data};
    /// let data = bundled_font_data("DejaVuSans.ttf").expect("bundled");
    /// let face = Face::parse(data, 0)?;
    /// let chain = FontChain::new(vec![face.clone()]);
    /// let unscaled = chain.layout_unscaled("AV", &ShapeOptions::default())?;
    /// let a = face.glyph_index('A').expect("mapped");
    /// assert_eq!(unscaled.glyphs[1].x, i32::from(face.glyph_advance(a)));
    /// let points = f64::from(face.units_per_em()) / 64.0;
    /// let kerned = chain.layout("AV", points, 72.0, &ShapeOptions::default(), Hinting::None)?;
    /// assert!(kerned.glyphs[1].x < unscaled.glyphs[1].x);
    /// # Ok::<(), pltrs_font::FontError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`layout`](Self::layout).
    pub fn layout_unscaled(&self, text: &str, options: &ShapeOptions) -> Result<TextLayout> {
        let sizes: Vec<Size> = self
            .faces
            .iter()
            .map(|face| Size::new(face, f64::from(face.units_per_em()) / 64.0, 72.0))
            .collect();
        self.lay_out(text, &sizes, options, Hinting::None, true)
    }

    /// The layout of [`layout`](Self::layout) at the size `sizes` gives
    /// each face, positioned with the font's scale unless `unscaled`.
    fn lay_out(
        &self,
        text: &str,
        sizes: &[Size],
        options: &ShapeOptions,
        hinting: Hinting,
        unscaled: bool,
    ) -> Result<TextLayout> {
        let mut layout = TextLayout::default();
        if self.faces.is_empty() {
            return Ok(layout);
        }
        let text: Vec<(usize, char)> = text.char_indices().collect();
        let assignment = self.assign_faces(&text, options);
        // Only the faces some character falls to are hinted, as FreeType
        // hints only the glyphs it loads: the analysis of a fallback face
        // no character reaches (the last resort's, which maps every code
        // point) would cost time and memory for nothing.
        let mut used = vec![false; self.faces.len()];
        for &face in &assignment {
            used[face] = true;
        }
        let hinters: Vec<Option<Hinter>> = self
            .faces
            .iter()
            .zip(&used)
            .map(|(face, &used)| used.then(|| hinting.hinter(face, true)))
            .collect();
        // Faces with no analysis kept get theirs for this call.
        let analyses: Vec<Option<Autohinter>> = self
            .faces
            .iter()
            .zip(&hinters)
            .enumerate()
            .map(|(index, (face, &hinter))| {
                (hinter == Some(Hinter::Autohinter) && self.cached_autohinter(index).is_none())
                    .then(|| Autohinter::new(face))
            })
            .collect();
        let hinted: Vec<Option<HintedFace<'_, 'a>>> = self
            .faces
            .iter()
            .zip(sizes)
            .zip(&hinters)
            .enumerate()
            .map(|(index, ((face, size), hinter))| match hinter {
                None | Some(Hinter::Unhinted) => None,
                Some(Hinter::Bytecode) => Some(HintedFace::bytecode(face, size)),
                Some(Hinter::Autohinter) => {
                    let analysis = self
                        .cached_autohinter(index)
                        .or(analyses[index].as_ref())
                        .expect("autohinted faces have an analysis");
                    Some(HintedFace::autohinted(face, size, analysis))
                }
            })
            .collect();
        let scaler = |face: usize| match &hinted[face] {
            _ if unscaled => Scaler::Unscaled(sizes[face]),
            Some(hinted) => Scaler::Hinted(sizes[face], hinted),
            None => Scaler::Pixels(sizes[face]),
        };
        let mut bbox: Option<Rect> = None;
        let mut pen = 0i32;
        for (face_index, glyph) in shape_text(&text, &self.faces, &assignment, options, &scaler) {
            // Positions saturate rather than overflow for absurdly long or
            // large text (FreeType's pen is 64-bit).
            let (x, y) = (pen.saturating_add(glyph.x_offset), glyph.y_offset);
            let cbox = match &hinted[face_index] {
                Some(hinted) => hinted.control_box(glyph.glyph_id)?,
                None => sizes[face_index].glyph_cbox(&self.faces[face_index], glyph.glyph_id)?,
            }
            .map_or(Rect::default(), |r| Rect {
                x_min: r.x_min.saturating_add(x),
                y_min: r.y_min.saturating_add(y),
                x_max: r.x_max.saturating_add(x),
                y_max: r.y_max.saturating_add(y),
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
            pen = pen.saturating_add(glyph.x_advance);
        }
        let bbox = bbox.unwrap_or_default();
        layout.width = pen;
        layout.height = bbox.height();
        layout.descent = -bbox.y_min;
        layout.x_min = bbox.x_min;
        Ok(layout)
    }

    /// matplotlib's font fallback: every cluster shaped to `.notdef` moves
    /// on to the next face, until all clusters render or reach the last
    /// face. Returns the face index of each character.
    fn assign_faces(&self, text: &[(usize, char)], options: &ShapeOptions) -> Vec<usize> {
        let last = self.faces.len() - 1;
        let mut assignment = vec![0; text.len()];
        loop {
            let glyphs = shape_text(text, &self.faces, &assignment, options, &|_| Scaler::Units);
            let mut clusters: Vec<usize> = glyphs.iter().map(|(_, g)| g.cluster).collect();
            clusters.sort_unstable();
            clusters.dedup();
            let mut changed = false;
            for &(face, glyph) in &glyphs {
                if glyph.glyph_id != GlyphId::NOTDEF || face == last {
                    continue;
                }
                // The cluster's characters, found by bisection: scanning
                // the whole text for each missing glyph made long lines of
                // fallback characters quadratic.
                let next = clusters
                    .get(clusters.partition_point(|&c| c <= glyph.cluster))
                    .copied()
                    .unwrap_or(usize::MAX);
                let first = text.partition_point(|&(offset, _)| offset < glyph.cluster);
                for (index, &(offset, _)) in text.iter().enumerate().skip(first) {
                    if offset >= next {
                        break;
                    }
                    if assignment[index] == face {
                        assignment[index] = face + 1;
                        changed = true;
                    }
                }
            }
            if !changed {
                return assignment;
            }
        }
    }
}

fn union(a: Rect, b: Rect) -> Rect {
    Rect {
        x_min: a.x_min.min(b.x_min),
        y_min: a.y_min.min(b.y_min),
        x_max: a.x_max.max(b.x_max),
        y_max: a.y_max.max(b.y_max),
    }
}
