//! Text in Type 3 fonts (`draw_text`, `draw_mathtext`): plain text as
//! `TJ` arrays with the kerning between glyphs, mathtext glyph by glyph,
//! both in one-byte codes of 256-glyph font subsets.

use plt_core::Point;
use plt_text::TextRun;

use super::PdfRenderer;
use super::object::Obj;
use crate::gc::GraphicsContext;
use crate::vector::{RunKind, glyph_chars, kind, prev_kerns};

/// One element of a `TJ` array being collected.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shown {
    Kern(f64),
    Code(u8),
}

/// `q cos sin -sin cos x y cm`: the rotation and position of a text.
fn placement(origin: Point, angle: f64) -> [Obj; 8] {
    let (sin, cos) = angle.to_radians().sin_cos();
    [
        Obj::op("q"),
        Obj::Real(cos),
        Obj::Real(sin),
        Obj::Real(-sin),
        Obj::Real(cos),
        Obj::Real(origin.x),
        Obj::Real(origin.y),
        Obj::op("cm"),
    ]
}

impl PdfRenderer {
    /// `output_singlebyte_chunk`: one `TJ` with strings of consecutive
    /// codes and the kerns between them in thousandths of the font size.
    fn show_chunk(&mut self, chunk: &mut Vec<Shown>, fontsize: f64) {
        if chunk.is_empty() {
            return;
        }
        let mut items = Vec::new();
        for group in chunk.chunk_by(|a, b| {
            matches!(
                (a, b),
                (Shown::Kern(_), Shown::Kern(_)) | (Shown::Code(_), Shown::Code(_))
            )
        }) {
            match group[0] {
                Shown::Kern(kern) => items.push(Obj::Real(-1000.0 * kern / fontsize)),
                Shown::Code(_) => items.push(Obj::Str(
                    group
                        .iter()
                        .filter_map(|s| match s {
                            Shown::Code(c) => Some(*c),
                            Shown::Kern(_) => None,
                        })
                        .collect(),
                )),
            }
        }
        self.output(&[Obj::Array(items), Obj::op("TJ")]);
        chunk.clear();
    }

    /// `draw_text` for plain text.
    fn plain_text(&mut self, run: &TextRun, origin: Point, angle: f64) {
        let to_points = 72.0 / run.dpi;
        let fontsize = run.glyphs.first().map_or(0.0, |g| g.size);
        self.output(&placement(origin, angle)[..1]);
        self.output(&placement(origin, angle)[1..]);
        self.output(&[Obj::op("BT")]);
        let kerns = prev_kerns(run, &mut self.fonts);
        let mut chunk = Vec::new();
        let mut previous_font = None;
        let mut previous_start = 0.0;
        for (glyph, kern) in run.glyphs.iter().zip(kerns) {
            let chars = glyph_chars(glyph, &mut self.fonts);
            let (subset, code) = self.tracker.track(glyph.font, &chars, glyph.glyph);
            let x = glyph.x * to_points;
            if previous_font != Some((glyph.font, subset)) {
                self.show_chunk(&mut chunk, fontsize);
                let name = self.font_names.name(glyph.font, subset);
                self.output(&[Obj::name(name), Obj::Real(fontsize), Obj::op("Tf")]);
                self.output(&[Obj::Real(x - previous_start), Obj::Int(0), Obj::op("Td")]);
                previous_font = Some((glyph.font, subset));
                previous_start = x;
            }
            let rise = glyph.y * to_points;
            if rise != 0.0 {
                self.show_chunk(&mut chunk, fontsize);
                self.output(&[Obj::Real(rise), Obj::op("Ts")]);
            }
            if kern != 0.0 {
                chunk.push(Shown::Kern(kern * to_points));
            }
            chunk.push(Shown::Code(code as u8));
            if rise != 0.0 {
                self.show_chunk(&mut chunk, fontsize);
                self.output(&[Obj::Int(0), Obj::op("Ts")]);
            }
        }
        self.show_chunk(&mut chunk, fontsize);
        self.output(&[Obj::op("ET")]);
        self.output(&[Obj::op("Q")]);
    }

    /// `draw_mathtext`: every glyph positioned and shown on its own, then
    /// the rules.
    fn math_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        let to_points = 72.0 / run.dpi;
        self.output(&placement(origin, angle)[..1]);
        self.output(&placement(origin, angle)[1..]);
        self.check_gc(gc, Some(gc.stroke_color()));
        self.output(&[Obj::op("BT")]);
        let mut previous_font = None;
        let (mut old_x, mut old_y) = (0.0, 0.0);
        for glyph in &run.glyphs {
            let chars = glyph_chars(glyph, &mut self.fonts);
            let (subset, code) = self.tracker.track(glyph.font, &chars, glyph.glyph);
            let (x, y) = (glyph.x * to_points, glyph.y * to_points);
            self.output(&[Obj::Real(x - old_x), Obj::Real(y - old_y), Obj::op("Td")]);
            (old_x, old_y) = (x, y);
            let font = Some((glyph.font, subset, glyph.size.to_bits()));
            if previous_font != font {
                let name = self.font_names.name(glyph.font, subset);
                self.output(&[Obj::name(name), Obj::Real(glyph.size), Obj::op("Tf")]);
                previous_font = font;
            }
            self.output(&[Obj::Str(vec![code as u8]), Obj::op("Tj")]);
        }
        self.output(&[Obj::op("ET")]);
        for rect in &run.rects {
            let (x, y, w, h) = rect.bounds();
            self.output(&[
                Obj::op("q"),
                Obj::Real(x * to_points),
                Obj::Real(y * to_points),
                Obj::Real(w * to_points),
                Obj::Real(h * to_points),
                Obj::op("re"),
                Obj::op("f"),
                Obj::op("Q"),
            ]);
        }
        self.output(&[Obj::op("Q")]);
    }

    /// `draw_text`: the text colour fills glyphs.
    pub(super) fn pdf_text(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        origin: Point,
        angle: f64,
    ) {
        self.check_gc(gc, Some(gc.stroke_color()));
        match kind(run, &mut self.fonts) {
            RunKind::Plain => self.plain_text(run, origin, angle),
            RunKind::Math => self.math_text(gc, run, origin, angle),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placements_rotate() {
        let ops = placement(Point::new(1.0, 2.0), 90.0);
        let text: Vec<String> = ops
            .iter()
            .map(|o| String::from_utf8(o.repr().unwrap()).unwrap())
            .collect();
        assert_eq!(text, ["q", "0", "1", "-1", "0", "1", "2", "cm"]);
    }
}
