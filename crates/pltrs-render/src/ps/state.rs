//! The graphics state of a page as matplotlib's `RendererPS` keeps it:
//! colour, line width, join, cap, dashes and font are written only when
//! they change (`set_color`, `set_linewidth`, `set_linejoin`,
//! `set_linecap`, `set_linedash`, `set_font`), hatches are defined once
//! (`create_hatch`), and what was written so far can be taken out of the
//! page (`take_body`), as a renderer writing to a program's file does.

use std::fmt::Write as _;

use pltrs_core::Affine2D;

use super::PsRenderer;
use super::draw::{cap_code, color_cmd, join_code, nums};
use crate::gc::{CapStyle, GraphicsContext, JoinStyle};
use crate::vector::general;

impl PsRenderer {
    /// The PostScript written to the page since the document began or the
    /// last call, which leaves the page empty (matplotlib's `RendererPS`
    /// writes it to its `pswriter` as it goes).
    pub fn take_body(&mut self) -> String {
        std::mem::take(&mut self.body)
    }

    /// Sets the colour (`setgray` for greys) when it changed, keeping it
    /// as the page's when `store` (matplotlib's `set_color`).
    pub fn set_color(&mut self, rgb: [f64; 3], store: bool) {
        if self.line.color != Some(rgb) {
            self.body.push_str(&color_cmd(rgb));
            self.body.push('\n');
            if store {
                self.line.color = Some(rgb);
            }
        }
    }

    /// Sets the line width in points when it changed (matplotlib's
    /// `set_linewidth`).
    pub fn set_linewidth(&mut self, width: f64, store: bool) {
        if self.line.width != Some(width) {
            let _ = writeln!(self.body, "{} setlinewidth", nums(&[width]));
            if store {
                self.line.width = Some(width);
            }
        }
    }

    /// Sets the line join when it changed (matplotlib's `set_linejoin`).
    pub fn set_linejoin(&mut self, join: JoinStyle, store: bool) {
        if self.line.join != Some(join) {
            let _ = writeln!(self.body, "{} setlinejoin", join_code(join));
            if store {
                self.line.join = Some(join);
            }
        }
    }

    /// Sets the line cap when it changed (matplotlib's `set_linecap`).
    pub fn set_linecap(&mut self, cap: CapStyle, store: bool) {
        if self.line.cap != Some(cap) {
            let _ = writeln!(self.body, "{} setlinecap", cap_code(cap));
            if store {
                self.line.cap = Some(cap);
            }
        }
    }

    /// Sets the dashes, `offset` into `pattern` (none, or an empty one,
    /// for solid lines), when they changed (matplotlib's `set_linedash`).
    pub fn set_linedash(&mut self, offset: f64, pattern: Option<&[f64]>, store: bool) {
        let dash = (
            offset,
            pattern
                .filter(|pattern| !pattern.is_empty())
                .map(<[f64]>::to_vec),
        );
        if self.line.dash.as_ref() == Some(&dash) {
            return;
        }
        if let Some(pattern) = &dash.1 {
            let _ = writeln!(self.body, "[{}] {} setdash", nums(pattern), nums(&[dash.0]));
        } else {
            self.body.push_str("[] 0 setdash\n");
        }
        if store {
            self.line.dash = Some(dash);
        }
    }

    /// Selects the font `name` at `size` points when it changed
    /// (matplotlib's `set_font`).
    pub fn set_font(&mut self, name: &str, size: f64, store: bool) {
        let font = (name.to_owned(), size);
        if self.line.font.as_ref() != Some(&font) {
            let _ = writeln!(self.body, "/{name} {size:.3} selectfont");
            if store {
                self.line.font = Some(font);
            }
        }
    }

    /// The name of the pattern of `hatch`, defined on first use with lines
    /// `linewidth` points wide (matplotlib's `create_hatch`; hatches are
    /// told apart by their pattern only).
    pub fn create_hatch(&mut self, pattern: &str, linewidth: f64) -> String {
        if let Some(index) = self.hatches.iter().position(|h| h == pattern) {
            return format!("H{index}");
        }
        let name = format!("H{}", self.hatches.len());
        let tile = pltrs_core::hatch::pattern_path(pattern, pltrs_core::hatch::DEFAULT_DENSITY)
            .unwrap_or_default();
        let path = self.convert(&tile, Affine2D::scaling(72.0, 72.0), false, None);
        let _ = writeln!(
            self.body,
            "  << /PatternType 1\n     /PaintType 2\n     /TilingType 2\n     \
             /BBox[0 0 72 72]\n     /XStep 72\n     /YStep 72\n\n     /PaintProc {{\n        \
             pop\n        {} setlinewidth\n{path}\n        gsave\n        fill\n        \
             grestore\n        stroke\n     }} bind\n   >>\n   matrix\n   0 {} translate\n   \
             makepattern\n   /{name} exch def",
            general(linewidth),
            general(self.height * 72.0)
        );
        self.hatches.push(pattern.to_owned());
        name
    }

    /// The line properties `_draw_ps` sets before stroking.
    pub(super) fn set_line(&mut self, gc: &GraphicsContext) {
        self.set_linewidth(gc.linewidth, true);
        self.set_linejoin(gc.join, true);
        self.set_linecap(gc.cap, true);
        self.set_linedash(gc.dashes.offset, gc.dashes.pattern.as_deref(), true);
    }
}

#[cfg(test)]
mod tests {
    use crate::ps::{PsOptions, PsRenderer};
    use crate::{CapStyle, JoinStyle};

    #[test]
    fn states_are_written_once_until_they_change() {
        let mut ps = PsRenderer::new(2.0, 2.0, 72.0, PsOptions::default()).unwrap();
        ps.set_color([1.0, 0.0, 0.0], true);
        ps.set_color([1.0, 0.0, 0.0], true);
        ps.set_linewidth(2.0, true);
        ps.set_linejoin(JoinStyle::Round, true);
        ps.set_linecap(CapStyle::Projecting, false);
        ps.set_linecap(CapStyle::Projecting, false);
        ps.set_linedash(0.0, Some(&[3.0, 1.5]), true);
        ps.set_font("DejaVuSans", 12.0, true);
        assert_eq!(
            ps.take_body(),
            "1 0 0 setrgbcolor\n2 setlinewidth\n1 setlinejoin\n2 setlinecap\n\
             2 setlinecap\n[3 1.5] 0 setdash\n/DejaVuSans 12.000 selectfont\n"
        );
        ps.set_linedash(0.0, None, true);
        assert_eq!(ps.take_body(), "[] 0 setdash\n");
    }

    #[test]
    fn hatches_are_defined_once() {
        let mut ps = PsRenderer::new(2.0, 2.0, 72.0, PsOptions::default()).unwrap();
        assert_eq!(ps.create_hatch("/", 1.0), "H0");
        let defined = ps.take_body();
        assert!(defined.contains("/H0 exch def"), "{defined}");
        assert_eq!(ps.create_hatch("/", 1.0), "H0");
        assert_eq!(ps.create_hatch("x", 1.0), "H1");
        assert!(ps.take_body().contains("/H1 exch def"));
    }
}
