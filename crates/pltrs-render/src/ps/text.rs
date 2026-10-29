//! Text (`draw_text`, `draw_mathtext`): glyphs shown by name from the
//! Type 3 font of their subset.

use std::fmt::Write as _;

use pltrs_core::Point;
use pltrs_text::tex::{TexError, TexText};
use pltrs_text::{PlacedGlyph, Shaping, TextRun, TextSource};

use super::PsRenderer;
use crate::gc::GraphicsContext;
use crate::vector::{POINTS_DPI, VectorError, general, glyph_chars, in_points, repr};

impl PsRenderer {
    /// The subset font name and glyph name of a glyph of `run`, recording
    /// its use.
    fn glyph_names(&mut self, run: &TextRun, glyph: &PlacedGlyph) -> (String, String) {
        let chars = glyph_chars(run, glyph);
        let (subset, _) = self.tracker.track(glyph.font, &chars, glyph.glyph);
        let names = self.fonts.get(glyph.font).map(|font| {
            let ps_name: String = font
                .postscript_name()
                .chars()
                .map(|c| if c.is_ascii() { c } else { '?' })
                .collect();
            (format!("{ps_name}-{subset}"), font.glyph_name(glyph.glyph))
        });
        match self.problems.record(names.map_err(Into::into)) {
            Some(names) => names,
            None => (format!("UNAVAILABLE-{subset}"), ".notdef".to_owned()),
        }
    }

    /// `draw_text` for plain text laid out in points: one block per run of
    /// glyphs from the same font subset.
    fn plain_text(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        size: f64,
        origin: Point,
        angle: f64,
    ) {
        let stream: Vec<(String, f64, f64, String)> = run
            .glyphs
            .iter()
            .map(|glyph| {
                let (font, name) = self.glyph_names(run, glyph);
                (font, glyph.x, glyph.y, name)
            })
            .collect();
        let rgb = gc.stroke_color();
        self.set_color([rgb.r, rgb.g, rgb.b], true);
        for group in stream.chunk_by(|a, b| a.0 == b.0) {
            let glyphs: Vec<(f64, f64, &str)> = group
                .iter()
                .map(|(_, x, y, name)| (*x, *y, name.as_str()))
                .collect();
            self.show_glyphs(gc, &group[0].0, size, origin, angle, &glyphs);
        }
    }

    /// Shows glyphs of the font `font` at `size` by name, each at its
    /// `(x, y)` relative to `origin`, the whole rotated by `angle`: one
    /// `selectfont` and one clipped `gsave` block of `glyphshow`s.
    pub(super) fn show_glyphs(
        &mut self,
        gc: &GraphicsContext,
        font: &str,
        size: f64,
        origin: Point,
        angle: f64,
        glyphs: &[(f64, f64, &str)],
    ) {
        let _ = writeln!(self.body, "/{font} {size:.3} selectfont");
        let shown: Vec<String> = glyphs
            .iter()
            .map(|(x, y, name)| format!("{} {} m /{name} glyphshow", general(*x), general(*y)))
            .collect();
        let clip = self.clip_cmd(gc);
        let _ = writeln!(
            self.body,
            "gsave\n{clip}\n{} {} translate\n{} rotate\n{}\ngrestore",
            general(origin.x),
            general(origin.y),
            general(angle),
            shown.join("\n")
        );
    }

    /// `draw_mathtext` for math typeset in points: glyphs moved to one by
    /// one, then the rules.
    fn math_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        let rgb = gc.stroke_color();
        self.set_color([rgb.r, rgb.g, rgb.b], true);
        let _ = writeln!(
            self.body,
            "gsave\n{} {} translate\n{} rotate",
            general(origin.x),
            general(origin.y),
            general(angle)
        );
        let mut last_font = None;
        for glyph in &run.glyphs {
            let (font, name) = self.glyph_names(run, glyph);
            let key = (font.clone(), glyph.size.to_bits());
            if last_font.as_ref() != Some(&key) {
                let _ = writeln!(self.body, "/{font} {} selectfont", repr(glyph.size));
                last_font = Some(key);
            }
            let _ = writeln!(
                self.body,
                "{} {} moveto\n/{name} glyphshow",
                general(glyph.x),
                general(glyph.y)
            );
        }
        for rect in &run.rects {
            let (x, y, w, h) = rect.bounds();
            let values = [x, y, w, h].map(repr);
            let _ = writeln!(self.body, "{} rectfill", values.join(" "));
        }
        self.body.push_str("grestore\n");
    }

    /// `draw_tex`: usetex text is a tag for `psfrag` to replace when
    /// `text.usetex` is on (`psfrag_mark`); with it off, the text is drawn
    /// as plain text, as matplotlib does.
    pub(super) fn ps_tex(
        &mut self,
        gc: &GraphicsContext,
        tex: &TexText,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        if gc.stroke_color().a == 0.0 {
            return Ok(());
        }
        if self.options.usetex {
            return self.psfrag_mark(gc, tex, origin, angle);
        }
        let plain = TextSource::Plain {
            text: tex.tex.clone(),
            font: tex.font.clone(),
            shaping: Shaping::default(),
        };
        let run = plain
            .layout(self.fonts.engine(), tex.size(), POINTS_DPI)
            .map_err(VectorError::from);
        if let Some(run) = self.problems.record(run) {
            self.ps_text(gc, &run, origin, angle);
        }
        Ok(())
    }

    /// `draw_text`: fully transparent text is not drawn; text that is not
    /// mathtext is shown in an AFM font with `ps.useafm`; runs without a
    /// source are filled as paths.
    pub(super) fn ps_text(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        origin: Point,
        angle: f64,
    ) {
        if gc.stroke_color().a == 0.0 {
            return;
        }
        let Some(source) = &run.source else {
            crate::renderer::fill_text(self, gc, run, origin, angle);
            return;
        };
        if let TextSource::Tex(tex) = source {
            let result = self.ps_tex(gc, tex, origin, angle);
            self.problems.record(result.map_err(VectorError::from));
            return;
        }
        // AFM text ignores the features and language, as matplotlib does.
        if let (TextSource::Plain { text, font, .. }, Some(directory)) =
            (source, self.options.afm_directory())
        {
            self.afm_text(gc, text, font, directory, origin, angle);
            return;
        }
        let Some(points) = self
            .problems
            .record(in_points(run, source, &self.fonts).map_err(VectorError::from))
        else {
            return;
        };
        if source.is_math() {
            self.math_text(gc, &points, origin, angle);
        } else {
            self.plain_text(gc, &points, source.size(), origin, angle);
        }
    }
}
