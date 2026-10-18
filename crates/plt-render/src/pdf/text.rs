//! Text (`draw_text`, `draw_mathtext`): plain text as `TJ` arrays with the
//! kerning between glyphs, mathtext glyph by glyph, both in one-byte codes
//! of 256-glyph Type 3 font subsets or two-byte codes of Type 42 fonts.

use plt_core::Point;
use plt_text::TextRun;

use super::PdfRenderer;
use super::object::Obj;
use crate::gc::GraphicsContext;
use crate::vector::{FontType, VectorError, glyph_chars, in_points, prev_kerns};

/// One element of a `TJ` array being collected.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shown {
    Kern(f64),
    Code(u32),
}

/// `_encode_glyphs`: codes as one byte each in Type 3 fonts, as two bytes
/// (big-endian) in Type 42 fonts.
fn encode_codes(codes: impl Iterator<Item = u32>, fonttype: FontType) -> Vec<u8> {
    match fonttype {
        FontType::Type3 => codes.map(|code| code as u8).collect(),
        FontType::Type42 => codes.flat_map(|code| (code as u16).to_be_bytes()).collect(),
    }
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
                Shown::Code(_) => items.push(Obj::Str(encode_codes(
                    group.iter().filter_map(|s| match s {
                        Shown::Code(c) => Some(*c),
                        Shown::Kern(_) => None,
                    }),
                    self.options.fonttype,
                ))),
            }
        }
        self.output(&[Obj::Array(items), Obj::op("TJ")]);
        chunk.clear();
    }

    /// `draw_text` for plain text laid out in points.
    fn plain_text(&mut self, run: &TextRun, fontsize: f64, origin: Point, angle: f64) {
        self.output(&placement(origin, angle)[..1]);
        self.output(&placement(origin, angle)[1..]);
        self.output(&[Obj::op("BT")]);
        let kerns = prev_kerns(run, &mut self.fonts);
        let mut chunk = Vec::new();
        let mut previous_font = None;
        let mut previous_start = 0.0;
        for (glyph, kern) in run.glyphs.iter().zip(kerns) {
            let chars = glyph_chars(run, glyph);
            let (subset, code) = self.tracker.track(glyph.font, &chars, glyph.glyph);
            if previous_font != Some((glyph.font, subset)) {
                self.show_chunk(&mut chunk, fontsize);
                let name = self.font_names.name(glyph.font, subset);
                self.output(&[Obj::name(name), Obj::Real(fontsize), Obj::op("Tf")]);
                self.output(&[
                    Obj::Real(glyph.x - previous_start),
                    Obj::Int(0),
                    Obj::op("Td"),
                ]);
                previous_font = Some((glyph.font, subset));
                previous_start = glyph.x;
            }
            if glyph.y != 0.0 {
                self.show_chunk(&mut chunk, fontsize);
                self.output(&[Obj::Real(glyph.y), Obj::op("Ts")]);
            }
            if kern != 0.0 {
                chunk.push(Shown::Kern(kern));
            }
            chunk.push(Shown::Code(code));
            if glyph.y != 0.0 {
                self.show_chunk(&mut chunk, fontsize);
                self.output(&[Obj::Int(0), Obj::op("Ts")]);
            }
        }
        self.show_chunk(&mut chunk, fontsize);
        self.output(&[Obj::op("ET")]);
        self.output(&[Obj::op("Q")]);
    }

    /// `draw_mathtext` for math typeset in points: every glyph positioned
    /// and shown on its own, then the rules.
    fn math_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        self.output(&placement(origin, angle)[..1]);
        self.output(&placement(origin, angle)[1..]);
        self.check_gc(gc, Some(gc.stroke_color()));
        self.output(&[Obj::op("BT")]);
        let mut previous_font = None;
        let (mut old_x, mut old_y) = (0.0, 0.0);
        for glyph in &run.glyphs {
            let chars = glyph_chars(run, glyph);
            let (subset, code) = self.tracker.track(glyph.font, &chars, glyph.glyph);
            self.output(&[
                Obj::Real(glyph.x - old_x),
                Obj::Real(glyph.y - old_y),
                Obj::op("Td"),
            ]);
            (old_x, old_y) = (glyph.x, glyph.y);
            let font = Some((glyph.font, subset, glyph.size.to_bits()));
            if previous_font != font {
                let name = self.font_names.name(glyph.font, subset);
                self.output(&[Obj::name(name), Obj::Real(glyph.size), Obj::op("Tf")]);
                previous_font = font;
            }
            let encoded = encode_codes(std::iter::once(code), self.options.fonttype);
            self.output(&[Obj::Str(encoded), Obj::op("Tj")]);
        }
        self.output(&[Obj::op("ET")]);
        for rect in &run.rects {
            let (x, y, w, h) = rect.bounds();
            self.output(&[
                Obj::op("q"),
                Obj::Real(x),
                Obj::Real(y),
                Obj::Real(w),
                Obj::Real(h),
                Obj::op("re"),
                Obj::op("f"),
                Obj::op("Q"),
            ]);
        }
        self.output(&[Obj::op("Q")]);
    }

    /// `draw_text`: the text colour fills glyphs; runs without a source
    /// are filled as paths.
    pub(super) fn pdf_text(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        origin: Point,
        angle: f64,
    ) {
        let Some(source) = &run.source else {
            crate::renderer::fill_text(self, gc, run, origin, angle);
            return;
        };
        self.check_gc(gc, Some(gc.stroke_color()));
        let Some(points) = self
            .problems
            .record(in_points(run, source, &self.fonts).map_err(VectorError::from))
        else {
            return;
        };
        if source.is_math() {
            self.math_text(gc, &points, origin, angle);
        } else {
            self.plain_text(&points, source.size(), origin, angle);
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
