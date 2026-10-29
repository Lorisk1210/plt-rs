//! usetex text in PostScript with `text.usetex` on (matplotlib's
//! `draw_tex` and the first half of `_print_figure_tex`): every text is a
//! tag shown where it goes, which LaTeX's `psfrag` package replaces by the
//! typeset text, and the figure is an EPS file without fonts that LaTeX
//! includes. Running LaTeX, `dvips` and Ghostscript on it is left to the
//! caller.

use std::fmt::Write as _;

use plt_core::{Bbox, Point};
use plt_text::tex::{TexError, TexText};

use super::dsc::bbox_header;
use super::{PS_DEFS, PageOrientation, PsRenderer, draw};
use crate::gc::GraphicsContext;
use crate::vector::{VectorError, fixed, trimmed};

/// A figure whose usetex texts are psfrag tags, ready for LaTeX
/// (`_print_figure_tex` up to `_convert_psfrags`).
#[derive(Clone, Debug, PartialEq)]
pub struct PsfragFigure {
    /// The figure as an EPS file without fonts, every usetex text a tag
    /// (the `tmp.ps` LaTeX includes).
    pub figure: String,
    /// The `\psfrag` command replacing each tag by its text.
    pub psfrags: Vec<String>,
    /// The figure's width in inches.
    pub width: f64,
    /// The figure's height in inches.
    pub height: f64,
    /// The figure's bounding box in points, from the origin, turned on its
    /// side in landscape (the `bbox` given to `gs_distill`).
    pub bbox: Bbox,
}

/// The LaTeX font family command of the generic family `family`
/// (`font.family[0]`), as `draw_tex` picks it.
fn family_command(family: Option<&str>) -> &'static str {
    match family {
        Some("sans-serif") => r"\sffamily",
        Some("monospace") => r"\ttfamily",
        _ => r"\rmfamily",
    }
}

impl PsRenderer {
    /// `draw_tex` with `text.usetex`: shows the tag `psmarker<n>` at the
    /// text's bottom left (the baseline moved down by the descent TeX
    /// measures) and records the `\psfrag` command that puts the text,
    /// at its size and colour and in the family of `font.family`, in its
    /// place, rotated by `angle`.
    pub(super) fn psfrag_mark(
        &mut self,
        gc: &GraphicsContext,
        tex: &TexText,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        let (_, _, descent) = tex
            .manager
            .text_width_height_descent(&tex.tex, tex.size(), 1.0)?;
        let tag = format!("psmarker{}", self.psfrags.len());
        let rgb = gc.stroke_color();
        let color = [rgb.r, rgb.g, rgb.b].map(|v| trimmed(v, 3)).join(",");
        let family = family_command(tex.manager.fonts().family.first().map(String::as_str));
        let size = tex.size();
        self.psfrags.push(format!(
            r"\psfrag{{{tag}}}[bl][bl][1][{}]{{\fontsize{{{}}}{{{}}}\color[rgb]{{{color}}} {{{family} {}}}}}",
            fixed(angle, 6),
            fixed(size, 6),
            fixed(size * 1.25, 6),
            tex.tex
        ));
        let up = (angle + 90.0).to_radians();
        let position = draw::nums(&[origin.x - descent * up.cos(), origin.y - descent * up.sin()]);
        let _ = write!(
            self.body,
            "gsave\n{position} moveto\n({tag})\nshow\ngrestore\n"
        );
        Ok(())
    }

    /// The figure LaTeX includes (`tmp.ps`): EPS at the origin with the
    /// procedures but no fonts, the drawing with its tags, and the psfrag
    /// commands.
    ///
    /// # Errors
    ///
    /// When something drawn could not be written.
    pub(super) fn psfrag_figure(self) -> Result<PsfragFigure, VectorError> {
        let (width, height) = (self.width, self.height);
        let page = Bbox::from_extents(0.0, 0.0, width * 72.0, height * 72.0);
        let mut figure = format!(
            "%!PS-Adobe-3.0 EPSF-3.0\n%%LanguageLevel: 3\n{}",
            self.dsc_comments()
        );
        let _ = write!(
            figure,
            "{}\n%%EndComments\n%%BeginProlog\n/mpldict {} dict def\nmpldict begin\n{}\nend\n\
             %%EndProlog\nmpldict begin\n0 0 translate\n0 0 {} rectclip\n{}\nend\nshowpage\n",
            bbox_header(&page),
            PS_DEFS.len(),
            PS_DEFS.concat(),
            draw::nums(&[width * 72.0, height * 72.0]),
            self.body
        );
        let bbox = match self.options.orientation {
            PageOrientation::Portrait => page,
            PageOrientation::Landscape => Bbox::from_extents(0.0, 0.0, page.y1(), page.x1()),
        };
        self.problems.into_result(PsfragFigure {
            figure,
            psfrags: self.psfrags,
            width,
            height,
            bbox,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_family_picks_the_command() {
        assert_eq!(family_command(Some("sans-serif")), r"\sffamily");
        assert_eq!(family_command(Some("monospace")), r"\ttfamily");
        assert_eq!(family_command(Some("Sans-Serif")), r"\rmfamily");
        assert_eq!(family_command(None), r"\rmfamily");
    }
}
