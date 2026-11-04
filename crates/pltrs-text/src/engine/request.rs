//! What a text asks of the fonts: the query, the generic-family lists
//! that resolve it and how its glyphs are rendered.

use pltrs_font::{FontDatabase, FontId, FontQuery, GenericFamilies, Hinting};

use crate::error::TextError;
use crate::lookups::{self, FontLookup, LookupKind};

/// A font query plus the generic-family lists (`font.sans-serif`, ...)
/// in effect when it is resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct FontRequest {
    /// Families, style, weight, stretch and size.
    pub query: FontQuery,
    /// Preference lists for generic family names.
    pub generics: GenericFamilies,
    /// Whether characters no family covers come from the Last Resort font
    /// rather than the first family's missing-glyph box (matplotlib's
    /// `font.enable_last_resort`).
    pub last_resort: bool,
    /// How glyphs are grid-fitted when the text is laid out and rasterized
    /// (matplotlib's `text.hinting` for the Agg backend; vector output and
    /// text paths are never hinted).
    pub hinting: Hinting,
    /// The keyword the weight was given as (`"bold"`, ...), if it was: the
    /// PGF backend sets only weights named bold in bold, as matplotlib's
    /// does.
    pub weight_keyword: Option<&'static str>,
    /// A face to set the text in instead of matching the query
    /// (matplotlib's `FontProperties(fname=...)`).
    pub file: Option<FontId>,
}

impl FontRequest {
    /// A request resolving generic families with matplotlib's defaults,
    /// unhinted.
    pub fn new(query: FontQuery) -> Self {
        Self {
            query,
            generics: GenericFamilies::default(),
            last_resort: true,
            hinting: Hinting::None,
            weight_keyword: None,
            file: None,
        }
    }

    /// The fonts the text is set in, in fallback order: the requested
    /// file's face, else the best match of each family, or of the default
    /// family when none has one (`_find_fonts_by_props`). A lookup
    /// matplotlib logs about is recorded (see [`take_font_lookups`]).
    ///
    /// [`take_font_lookups`]: crate::take_font_lookups
    pub fn fonts(&self, db: &FontDatabase) -> Result<Vec<FontId>, TextError> {
        let (fonts, weights) = self.fallback_chain(db)?;
        if let Some(weights) = weights {
            self.note(LookupKind::Fonts, &weights);
        }
        Ok(fonts)
    }

    /// The first font of [`fonts`](Self::fonts), whose metrics give the
    /// height of the text's lines, without recording the lookup: the line
    /// layouts of a text record it, and an empty text, which matplotlib
    /// never looks fonts up for, has none.
    pub(crate) fn primary_font(&self, db: &FontDatabase) -> Result<FontId, TextError> {
        let (fonts, _) = self.fallback_chain(db)?;
        Ok(fonts[0])
    }

    /// The fonts of [`fonts`](Self::fonts), never empty, and for a lookup
    /// by families the weight of the best face of each (`None` for a
    /// family without one).
    #[expect(
        clippy::type_complexity,
        reason = "the chain and the weights the lookup found, used together once"
    )]
    fn fallback_chain(
        &self,
        db: &FontDatabase,
    ) -> Result<(Vec<FontId>, Option<Vec<Option<u16>>>), TextError> {
        if let Some(face) = self.file {
            return Ok((vec![face], None));
        }
        let matches = db.family_matches(&self.query, &self.generics);
        let weights = matches
            .iter()
            .map(|entry| entry.as_ref().map(|entry| entry.weight))
            .collect();
        let found: Vec<FontId> = matches.into_iter().flatten().map(|e| e.font).collect();
        let fonts = if found.is_empty() {
            vec![db.find_font_with(&self.query, &self.generics)?]
        } else {
            found
        };
        Ok((fonts, Some(weights)))
    }

    /// The font of the text: the requested file's face, else the best
    /// match (`findfont`). A lookup matplotlib logs about is recorded (see
    /// [`take_font_lookups`](crate::take_font_lookups)).
    pub fn font(&self, db: &FontDatabase) -> Result<FontId, TextError> {
        if let Some(face) = self.file {
            return Ok(face);
        }
        let best = db
            .best_entry(&self.query, &self.generics, |_| true)
            .filter(|(_, score)| *score < 10.0);
        self.note(
            LookupKind::Font,
            &[best.as_ref().map(|(entry, _)| entry.weight)],
        );
        match best {
            Some((entry, _)) => Ok(entry.font),
            None => Ok(db.find_font_with(&self.query, &self.generics)?),
        }
    }

    /// Records this request's lookup if matplotlib logs about it, given
    /// the weights of the faces found (`None` for a family without one).
    fn note(&self, kind: LookupKind, weights: &[Option<u16>]) {
        lookups::note(
            FontLookup {
                kind,
                query: self.query.clone(),
                generics: self.generics.clone(),
                weight_keyword: self.weight_keyword,
            },
            weights,
        );
    }

    /// `DejaVu Sans` at `size` points, the request most unit tests make.
    #[cfg(test)]
    pub(crate) fn dejavu_sans(size: f64) -> Self {
        Self::new(FontQuery {
            size,
            ..FontQuery::families(["DejaVu Sans"])
        })
    }
}
