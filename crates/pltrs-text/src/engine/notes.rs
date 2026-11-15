//! What the engine's callers note of the texts they lay out, measure and
//! draw: the characters no font has and the lookups of fonts matplotlib's
//! font manager logs about (see [`crate::take_missing_glyphs`] and
//! [`crate::take_font_notes`]).

use pltrs_font::AfmDirectory;

use super::{FontRequest, TextEngine};
use crate::error::TextError;
use crate::lookups::LookupKind;
use crate::memo;
use crate::missing::{self, MissingGlyph};
use crate::run::{GlyphChars, TextRun};

impl TextEngine {
    /// Records the characters of `run`, laid out from `text` in
    /// `request`, that no font of the request has: those drawn from the
    /// Last Resort font, or as the missing-glyph box (see
    /// [`take_missing_glyphs`](crate::take_missing_glyphs)). matplotlib
    /// warns about them where its renderers measure a line, which its
    /// cache of line metrics spares them while a line is cached
    /// (`get_text_width_height_descent`), so the caller notes them there.
    ///
    /// # Errors
    ///
    /// When the request's fonts cannot be looked up.
    pub fn note_missing_glyphs(
        &self,
        run: &TextRun,
        text: &str,
        request: &FontRequest,
    ) -> Result<(), TextError> {
        let resort = self.db.last_resort();
        if !run
            .glyphs
            .iter()
            .any(|g| g.glyph.0 == 0 || Some(g.font) == resort)
        {
            return Ok(());
        }
        let fonts = request.fonts(&self.db)?;
        let mut families: Vec<String> = Vec::new();
        for &font in &fonts {
            if let Some(name) = self.db.face(font)?.family_name()
                && !families.contains(&name)
            {
                families.push(name);
            }
        }
        missing::record(
            run.glyphs
                .iter()
                .filter(|g| g.glyph.0 == 0 || !fonts.contains(&g.font))
                .filter_map(|g| match g.chars {
                    GlyphChars::Cluster { start, .. } => text.get(start..)?.chars().next(),
                    GlyphChars::Unknown | GlyphChars::Code(_) => None,
                })
                .map(|character| MissingGlyph {
                    character,
                    families: families.clone(),
                }),
        );
        Ok(())
    }

    /// Records the lookup of `request`'s fonts if matplotlib's font
    /// manager logs about it (a family no font has, a face of another
    /// weight; see [`take_font_notes`](crate::take_font_notes)).
    /// matplotlib's renderers look the fonts of a text up
    /// (`_find_fonts_by_props`) whenever they lay it out, measure one of
    /// its lines that is not cached and draw one of its lines, so the
    /// caller notes the lookup there.
    ///
    /// # Errors
    ///
    /// When the request's fonts cannot be looked up.
    pub fn note_font_lookup(&self, request: &FontRequest) -> Result<(), TextError> {
        let weights = self.chains.get_or_compute(
            memo::fingerprint(request, ()),
            |stored| stored == request,
            || request.clone(),
            || request.chain_weights(&self.db),
        )?;
        if let Some(weights) = weights {
            request.note(LookupKind::Fonts, &weights);
        }
        Ok(())
    }

    /// Records the lookup of `request`'s font among the AFM fonts of
    /// `directory`, which matplotlib's PostScript and PDF renderers make
    /// for each text they measure and draw in them (see
    /// [`LookupKind::Afm`]): its font manager logs about a face of another
    /// weight the first time it looks up fonts of those properties.
    ///
    /// ```
    /// use pltrs_text::{AfmDirectory, FontNote, FontQuery, FontRequest, LookupKind, TextEngine};
    /// let request = FontRequest::new(FontQuery::default());
    /// drop(pltrs_text::take_font_notes());
    /// TextEngine::global().note_afm_lookup(&request, AfmDirectory::PostScript);
    /// let noted = pltrs_text::take_font_notes();
    /// assert!(matches!(
    ///     &noted[0],
    ///     FontNote::Lookup(run) if run.lookup.kind == LookupKind::Afm(AfmDirectory::PostScript)
    /// ));
    /// ```
    pub fn note_afm_lookup(&self, request: &FontRequest, directory: AfmDirectory) {
        request.note(LookupKind::Afm(directory), &[]);
    }
}
