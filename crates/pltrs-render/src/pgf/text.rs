//! Text as `\pgftext` commands for LaTeX to typeset (matplotlib's
//! `draw_text` and `_escape_and_apply_props`).

#![expect(
    clippy::float_cmp,
    reason = "matplotlib compares alpha and angle exactly"
)]

use std::fmt::Write as _;

use pltrs_core::Point;
use pltrs_font::FontStyle;
use pltrs_text::{FontRequest, HAlign, RotationMode, TextRun, VAlign};

use super::{PgfRenderer, num};
use crate::gc::GraphicsContext;
use crate::renderer::fill_text;

/// The weight keywords matplotlib's PGF backend sets in bold (weights
/// given as numbers never are).
const BOLD: [&str; 7] = [
    "semibold",
    "demibold",
    "demi",
    "bold",
    "heavy",
    "extra bold",
    "black",
];

impl PgfRenderer {
    /// Writes one line of text, anchored as matplotlib anchors it: at the
    /// text's own position and alignment for single-line text that is not
    /// rotated (or rotated about its anchor), else at the line's baseline.
    /// Runs not laid out from a string are drawn as glyph outlines.
    pub(super) fn pgf_text(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        origin: Point,
        angle: f64,
    ) {
        let Some(source) = &run.source else {
            fill_text(self, gc, run, origin, angle);
            return;
        };
        let text = escape_and_apply_props(source.text(), &self.font_commands(source.font()));
        self.line(r"\begin{pgfscope}");
        self.clip(gc);
        let alpha = gc.alpha.unwrap_or(1.0);
        if alpha != 1.0 {
            self.line(&format!(r"\pgfsetfillopacity{{{}}}", num(alpha)));
            self.line(&format!(r"\pgfsetstrokeopacity{{{}}}", num(alpha)));
        }
        self.define_color("textcolor", gc.foreground);
        self.line(r"\pgfsetstrokecolor{textcolor}");
        self.line(r"\pgfsetfillcolor{textcolor}");
        let anchor = run.anchor.filter(|anchor| {
            (angle == 0.0 || anchor.rotation_mode == RotationMode::Anchor)
                && anchor.valign != VAlign::CenterBaseline
        });
        let mut args = match anchor {
            Some(anchor) => vec![
                format!("x={}in", num(self.inches(anchor.point.x))),
                format!("y={}in", num(self.inches(anchor.point.y))),
                match anchor.halign {
                    HAlign::Left => "left",
                    HAlign::Right => "right",
                    HAlign::Center => "",
                }
                .to_owned(),
                match anchor.valign {
                    VAlign::Top => "top",
                    VAlign::Bottom => "bottom",
                    VAlign::Baseline => "base",
                    VAlign::Center | VAlign::CenterBaseline => "",
                }
                .to_owned(),
            ],
            None => vec![format!(
                "x={}in, y={}in, left, base",
                num(self.inches(origin.x)),
                num(self.inches(origin.y))
            )],
        };
        if angle != 0.0 {
            args.push(format!("rotate={}", num(angle)));
        }
        self.line(&format!(
            r"\pgftext[{}]{{\color{{textcolor}}{text}}}",
            args.join(",")
        ));
        self.line(r"\end{pgfscope}");
    }

    /// The commands selecting the family, size, style and weight of
    /// `request`.
    fn font_commands(&self, request: &FontRequest) -> String {
        let font = &request.query;
        let mut commands = String::new();
        let family = font.families.first().map_or("sans-serif", String::as_str);
        match family {
            "serif" => commands.push_str(r"\rmfamily"),
            "sans" | "sans-serif" => commands.push_str(r"\sffamily"),
            "monospace" => commands.push_str(r"\ttfamily"),
            _ if !self.options.rcfonts => commands.push_str(r"\fontfamily{\familydefault}"),
            name if self.options.font_names.iter().any(|known| known == name) => {
                let _ = write!(
                    commands,
                    r"\ifdefined\pdftexversion\else\setmainfont{{{name}}}\rmfamily\fi"
                );
            }
            // matplotlib ignores unknown fonts with a warning.
            _ => {}
        }
        let _ = write!(
            commands,
            r"\fontsize{{{}}}{{{}}}",
            num(font.size),
            num(font.size * 1.2)
        );
        commands.push_str(match font.style {
            FontStyle::Normal => "",
            FontStyle::Italic => r"\itshape",
            FontStyle::Oblique => r"\slshape",
        });
        if request
            .weight_keyword
            .is_some_and(|keyword| BOLD.contains(&keyword))
        {
            commands.push_str(r"\bfseries");
        }
        commands.push_str(r"\selectfont");
        commands
    }
}

/// `text` in braces after the font `commands`, with `^` and `%` made
/// harmless outside math and the minus sign spelled for TeX.
fn escape_and_apply_props(text: &str, commands: &str) -> String {
    format!(
        "{{{commands}{}{}{}}}",
        r"\catcode`\^=\active\def^{\ifmmode\sp\else\^{}\fi}",
        r"\catcode`\%=\active\def%{\%}",
        text.replace('\u{2212}', r"\ensuremath{-}")
    )
}

#[cfg(test)]
mod tests {
    use pltrs_font::{FontQuery, Weight};

    use super::*;
    use crate::pgf::PgfOptions;

    fn renderer(options: PgfOptions) -> PgfRenderer {
        PgfRenderer::new(1.0, 1.0, 100.0, options).unwrap()
    }

    #[test]
    fn generic_families_and_known_fonts() {
        let options = PgfOptions {
            font_names: vec!["DejaVu Serif".to_owned()],
            ..PgfOptions::default()
        };
        let pgf = renderer(options);
        let query = |family: &str| {
            FontRequest::new(FontQuery {
                families: vec![family.to_owned()],
                ..FontQuery::default()
            })
        };
        assert_eq!(
            pgf.font_commands(&query("sans-serif")),
            r"\sffamily\fontsize{10.000000}{12.000000}\selectfont"
        );
        assert!(
            pgf.font_commands(&query("DejaVu Serif")).starts_with(
                r"\ifdefined\pdftexversion\else\setmainfont{DejaVu Serif}\rmfamily\fi"
            )
        );
        assert!(
            pgf.font_commands(&query("Unknown"))
                .starts_with(r"\fontsize")
        );
        let mut bold_italic = query("monospace");
        bold_italic.query.style = FontStyle::Italic;
        bold_italic.query.weight = Weight(700);
        let commands = pgf.font_commands(&bold_italic);
        assert!(
            commands.ends_with(r"\itshape\selectfont"),
            "a number: {commands}"
        );
        bold_italic.weight_keyword = Some("bold");
        let commands = pgf.font_commands(&bold_italic);
        assert!(
            commands.ends_with(r"\itshape\bfseries\selectfont"),
            "{commands}"
        );
        let latex_fonts = renderer(PgfOptions {
            rcfonts: false,
            ..PgfOptions::default()
        });
        assert!(
            latex_fonts
                .font_commands(&query("DejaVu Serif"))
                .starts_with(r"\fontfamily{\familydefault}")
        );
    }

    #[test]
    fn minus_signs_and_catcodes() {
        let text = escape_and_apply_props("\u{2212}1", "");
        assert!(text.ends_with(r"\ensuremath{-}1}"));
        assert!(text.starts_with(r"{\catcode`\^=\active"));
    }
}
