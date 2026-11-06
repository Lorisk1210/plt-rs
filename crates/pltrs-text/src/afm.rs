//! Text measured with Adobe Font Metrics: what matplotlib's PDF and
//! PostScript backends report for text that is not mathtext when they
//! draw it in standard PostScript fonts (`pdf.use14corefonts`,
//! `ps.useafm`; `RendererPDFPSBase.get_text_width_height_descent` and
//! `_get_font_height_metrics`).

use pltrs_font::{AfmDirectory, AfmFont, AfmMetrics};

use crate::block::{FontHeightMetrics, LineMetrics};
use crate::engine::FontRequest;
use crate::error::TextError;

/// AFM dimensions are in 1/1000 of the font size.
const UNITS_PER_EM: f64 = 1000.0;

impl FontRequest {
    /// The AFM font of `directory` text in this font is drawn with
    /// (`_get_font_afm`: `findfont(prop, fontext="afm", directory=...)`).
    ///
    /// # Errors
    ///
    /// Never for matplotlib's directories, where `Helvetica` matches.
    pub fn afm_font(&self, directory: AfmDirectory) -> Result<&'static AfmFont, TextError> {
        Ok(AfmFont::find_in(&self.query, &self.generics, directory)?)
    }

    /// The AFM font of `directory` for every family of the request that
    /// has one (`_find_fonts_by_props(prop, fontext="afm", directory=...)`,
    /// which the PDF backend registers fonts with); the first is
    /// [`afm_font`](Self::afm_font).
    ///
    /// # Errors
    ///
    /// Never for matplotlib's directories, where `Helvetica` matches.
    pub fn afm_fonts(&self, directory: AfmDirectory) -> Result<Vec<&'static AfmFont>, TextError> {
        Ok(AfmFont::find_all_in(
            &self.query,
            &self.generics,
            directory,
        )?)
    }

    /// The metrics of [`afm_font`](Self::afm_font).
    ///
    /// # Errors
    ///
    /// When the font's metrics are not bundled.
    pub fn afm_metrics(&self, directory: AfmDirectory) -> Result<&'static AfmMetrics, TextError> {
        Ok(self.afm_font(directory)?.metrics()?)
    }

    /// The width, height and descent of `text` in the request's AFM font
    /// of `directory`, in pixels at `dpi`: `get_str_bbox_and_descent`
    /// scaled to the font size (matplotlib's backends work at 72 dpi).
    ///
    /// # Errors
    ///
    /// When the font's metrics are not bundled, or `text` has a character
    /// the font lacks and the font has no question mark (`ZapfDingbats`).
    pub fn afm_line_metrics(
        &self,
        text: &str,
        directory: AfmDirectory,
        dpi: f64,
    ) -> Result<LineMetrics, TextError> {
        let bbox = self.afm_metrics(directory)?.str_bbox_and_descent(text)?;
        let (scale, resolution) = (self.query.size / UNITS_PER_EM, dpi / 72.0);
        Ok(LineMetrics {
            width: bbox.width * scale * resolution,
            height: bbox.height * scale * resolution,
            descent: bbox.descent * scale * resolution,
        })
    }

    /// The minimum line ascent and descent and the line gap of text in the
    /// request's AFM font of `directory`, in pixels at `dpi`: the AFM
    /// ascender and descender, with a gap of a fifth of their sum
    /// (matplotlib's line spacing of 1.2).
    ///
    /// # Errors
    ///
    /// When the font's metrics are not bundled or lack an ascender or
    /// descender (Symbol and `ZapfDingbats`).
    pub fn afm_height_metrics(
        &self,
        directory: AfmDirectory,
        dpi: f64,
    ) -> Result<FontHeightMetrics, TextError> {
        let metrics = self.afm_metrics(directory)?;
        let (scale, resolution) = (self.query.size / UNITS_PER_EM, dpi / 72.0);
        let ascent = metrics.ascender()? * scale;
        let descent = -metrics.descender()? * scale;
        Ok(FontHeightMetrics {
            ascent: ascent * resolution,
            descent: descent * resolution,
            line_gap: (ascent + descent) * 0.2 * resolution,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs_font::{FontQuery, FontStyle};

    fn request(families: &[&str], size: f64) -> FontRequest {
        FontRequest::new(FontQuery {
            size,
            ..FontQuery::families(families.iter().copied())
        })
    }

    #[test]
    fn fonts_come_from_the_directory() {
        let core = AfmDirectory::PdfCore;
        let dejavu = request(&["DejaVu Sans"], 10.0);
        assert_eq!(dejavu.afm_font(core).map(|f| f.file), Ok("Helvetica.afm"));
        let italic = FontRequest::new(FontQuery::families(["serif"]).with_style(FontStyle::Italic));
        assert_eq!(
            italic.afm_font(core).map(|f| f.file),
            Ok("Times-Italic.afm")
        );
        let files: Vec<&str> = request(&["Times", "Foo", "Courier"], 10.0)
            .afm_fonts(core)
            .expect("matching families")
            .iter()
            .map(|f| f.file)
            .collect();
        assert_eq!(files, ["Times-Roman.afm", "Courier.afm"]);
        let serif = request(&["serif"], 12.0);
        assert_eq!(
            serif.afm_font(AfmDirectory::PostScript).map(|f| f.file),
            Ok("pncr8a.afm")
        );
    }

    #[test]
    fn postscript_extents_match_matplotlib() {
        // matplotlib with ps.useafm: RendererPS
        // .get_text_width_height_descent('AVATAR', FontProperties(family=
        // 'serif', size=12), False) and _get_font_height_metrics.
        let serif = request(&["serif"], 12.0);
        let line = serif
            .afm_line_metrics("AVATAR", AfmDirectory::PostScript, 72.0)
            .expect("bundled");
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(
            close(line.width, 49.104) && close(line.height, 9.024) && close(line.descent, 0.18)
        );
        let heights = serif
            .afm_height_metrics(AfmDirectory::PostScript, 72.0)
            .expect("bundled");
        assert!(close(heights.ascent, 8.844) && close(heights.descent, 2.46));
        assert!(close(heights.line_gap, 2.2608));
    }

    #[test]
    fn extents_match_matplotlib() {
        // matplotlib with pdf.use14corefonts: RendererPdf
        // .get_text_width_height_descent('AVATAR', FontProperties(size=12),
        // False) and _get_font_height_metrics.
        let helvetica = request(&["DejaVu Sans"], 12.0);
        let line = helvetica
            .afm_line_metrics("AVATAR", AfmDirectory::PdfCore, 72.0)
            .expect("bundled");
        assert_eq!(
            (line.width, line.height, line.descent),
            (43.332, 8.616, 0.0)
        );
        let heights = helvetica
            .afm_height_metrics(AfmDirectory::PdfCore, 72.0)
            .expect("bundled");
        assert_eq!(
            (heights.ascent, heights.descent, heights.line_gap),
            (8.616, 2.484, 2.22)
        );
        let doubled = helvetica
            .afm_line_metrics("AVATAR", AfmDirectory::PdfCore, 144.0)
            .expect("bundled");
        assert_eq!(doubled.width, 2.0 * line.width);
        let symbol = request(&["Symbol"], 12.0);
        assert!(
            symbol
                .afm_height_metrics(AfmDirectory::PdfCore, 72.0)
                .is_err()
        );
    }
}
