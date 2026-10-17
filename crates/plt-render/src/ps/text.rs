//! Text (`draw_text`, `draw_mathtext`): glyphs shown by name from the
//! Type 3 font of their subset.

#![allow(clippy::float_cmp, reason = "matplotlib compares colours exactly")]

use std::fmt::Write as _;

use plt_core::Point;
use plt_text::TextRun;

use super::PsRenderer;
use crate::gc::GraphicsContext;
use crate::vector::{RunKind, general, glyph_chars, kind, repr};

impl PsRenderer {
    /// The subset font name and glyph name of a glyph, recording its use.
    fn glyph_names(&mut self, glyph: &plt_text::PlacedGlyph) -> (String, String) {
        let chars = glyph_chars(glyph, &mut self.fonts);
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

    /// `draw_text` for plain text: one block per run of glyphs from the
    /// same font subset.
    fn plain_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        let to_points = 72.0 / run.dpi;
        let stream: Vec<(String, f64, f64, String)> = run
            .glyphs
            .iter()
            .map(|glyph| {
                let (font, name) = self.glyph_names(glyph);
                (font, glyph.x * to_points, glyph.y * to_points, name)
            })
            .collect();
        let rgb = gc.stroke_color();
        self.set_color([rgb.r, rgb.g, rgb.b], true);
        let size = run.glyphs.first().map_or(0.0, |g| g.size);
        for group in stream.chunk_by(|a, b| a.0 == b.0) {
            let _ = writeln!(self.body, "/{} {size:.3} selectfont", group[0].0);
            let shown: Vec<String> = group
                .iter()
                .map(|(_, x, y, name)| {
                    format!("{} {} m /{name} glyphshow", general(*x), general(*y))
                })
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
    }

    /// `draw_mathtext`: glyphs moved to one by one, then the rules.
    fn math_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        let to_points = 72.0 / run.dpi;
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
            let (font, name) = self.glyph_names(glyph);
            let key = (font.clone(), glyph.size.to_bits());
            if last_font.as_ref() != Some(&key) {
                let _ = writeln!(self.body, "/{font} {} selectfont", repr(glyph.size));
                last_font = Some(key);
            }
            let _ = writeln!(
                self.body,
                "{} {} moveto\n/{name} glyphshow",
                general(glyph.x * to_points),
                general(glyph.y * to_points)
            );
        }
        for rect in &run.rects {
            let (x, y, w, h) = rect.bounds();
            let values = [x, y, w, h].map(|v| repr(v * to_points));
            let _ = writeln!(self.body, "{} rectfill", values.join(" "));
        }
        self.body.push_str("grestore\n");
    }

    /// `draw_text`: fully transparent text is not drawn.
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
        match kind(run, &mut self.fonts) {
            RunKind::Plain => self.plain_text(gc, run, origin, angle),
            RunKind::Math => self.math_text(gc, run, origin, angle),
        }
    }
}
