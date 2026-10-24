//! Text as SVG text (`_draw_text_as_text`, `svg.fonttype = "none"`):
//! plain text as one `<text>` element styled with the font's CSS
//! properties, mathtext as one `<tspan>` per glyph with the properties of
//! the glyph's face, plus its rules as rectangles.

#![expect(
    clippy::float_cmp,
    reason = "matplotlib's SVG writer leaves out exactly-default values"
)]

use plt_core::Point;
use plt_font::{FontStyle, FontVariant, GenericFamily, Stretch, Weight};
use plt_text::{FontRequest, HAlign, RotationMode, TextRun, TextSource};

use super::SvgRenderer;
use super::style::{Style, TransformOp, css, transform_attr};
use crate::gc::GraphicsContext;
use crate::vector::{repr_str, short};

/// The family names matplotlib treats as generic (`font_family_aliases`).
const ALIASES: [&str; 7] = [
    "serif",
    "sans-serif",
    "sans serif",
    "cursive",
    "fantasy",
    "monospace",
    "sans",
];

/// A mathtext glyph as a `<tspan>`: its position and the character shown.
type Tspan = (f64, f64, char);

/// matplotlib's name of a stretch (the number when it has none).
fn stretch_name(stretch: Stretch) -> String {
    let name = match stretch.0 {
        100 => "ultra-condensed",
        200 => "extra-condensed",
        300 => "condensed",
        400 => "semi-condensed",
        500 => "normal",
        600 => "semi-expanded",
        700 => "expanded",
        800 => "extra-expanded",
        900 => "ultra-expanded",
        other => return other.to_string(),
    };
    name.to_owned()
}

/// The slant, variant, weight and stretch properties that differ from the
/// defaults.
fn face_style(style: FontStyle, variant: FontVariant, weight: Weight) -> Style {
    let mut css = Style::new();
    if style != FontStyle::Normal {
        css.push(("font-style", style.name().to_owned()));
    }
    if variant != FontVariant::Normal {
        css.push(("font-variant", variant.name().to_owned()));
    }
    if weight != Weight::NORMAL {
        css.push(("font-weight", weight.0.to_string()));
    }
    css
}

/// `_get_all_quoted_names`: the families of `font`, generic ones preceded
/// by the families they stand for, names quoted unless generic, without
/// repetitions.
fn font_families(font: &FontRequest) -> String {
    let normalize = |name: &str| {
        if name == "sans" || name == "sans serif" {
            "sans-serif".to_owned()
        } else {
            name.to_owned()
        }
    };
    let mut names: Vec<String> = Vec::new();
    for family in &font.query.families {
        let family = normalize(family);
        let mut expanded = Vec::new();
        if ALIASES.contains(&family.as_str())
            && let Ok(generic) = family.parse::<GenericFamily>()
        {
            expanded.extend(font.generics.get(generic).iter().map(|n| normalize(n)));
        }
        expanded.push(family);
        for name in expanded {
            let quoted = if ALIASES.contains(&name.as_str()) {
                name
            } else {
                repr_str(&name)
            };
            if !names.contains(&quoted) {
                names.push(quoted);
            }
        }
    }
    names.join(", ")
}

impl SvgRenderer {
    /// The fill and opacity of text drawn with `gc`.
    fn color_style(gc: &GraphicsContext) -> Style {
        let color = gc.stroke_color();
        let mut style = Style::new();
        if color.to_hex(false) != "#000000" {
            style.push(("fill", color.to_hex(false)));
        }
        let alpha = gc.alpha.unwrap_or(color.a);
        if alpha != 1.0 {
            style.push(("opacity", short(alpha)));
        }
        style
    }

    /// Plain text as one `<text>` element, anchored like the text artist
    /// when it is not rotated or rotates about its anchor.
    fn plain_text_element(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        text: &str,
        font: &FontRequest,
        origin: Point,
        angle: f64,
    ) {
        let query = &font.query;
        let mut style = face_style(query.style, query.variant, query.weight);
        style.push(("font-size", format!("{}px", short(query.size))));
        style.push(("font-family", font_families(font)));
        if query.stretch != Stretch::NORMAL {
            style.push(("font-stretch", stretch_name(query.stretch)));
        }
        let (x, y) = (origin.x, self.height - origin.y);
        let anchor = run
            .anchor
            .filter(|anchor| angle == 0.0 || anchor.rotation_mode == RotationMode::Anchor);
        let attributes = if let Some(anchor) = anchor {
            let (mut ax, mut ay) = (anchor.point.x, self.height - anchor.point.y);
            let (sin, cos) = angle.to_radians().sin_cos();
            let offset = sin * (x - ax) + cos * (y - ay);
            ax += offset * sin;
            ay += offset * cos;
            let text_anchor = match anchor.halign {
                HAlign::Left => "start",
                HAlign::Center => "middle",
                HAlign::Right => "end",
            };
            style.push(("text-anchor", text_anchor.to_owned()));
            style.extend(Self::color_style(gc));
            vec![
                ("x", short(ax)),
                ("y", short(ay)),
                ("style", css(&style)),
                (
                    "transform",
                    transform_attr(&[TransformOp::RotateAbout(-angle, ax, ay)]),
                ),
            ]
        } else {
            style.extend(Self::color_style(gc));
            vec![
                ("style", css(&style)),
                (
                    "transform",
                    transform_attr(&[TransformOp::Translate(x, y), TransformOp::Rotate(-angle)]),
                ),
            ]
        };
        self.writer.element("text", text, &attributes);
    }

    /// Mathtext as glyph `<tspan>`s grouped by face, and its rules.
    fn math_text_element(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        text: &str,
        origin: Point,
        angle: f64,
    ) {
        self.writer.comment(text);
        let color = Self::color_style(gc);
        self.writer.start(
            "g",
            &[
                ("style", css(&color)),
                (
                    "transform",
                    transform_attr(&[
                        TransformOp::Translate(origin.x, self.height - origin.y),
                        TransformOp::Rotate(-angle),
                    ]),
                ),
            ],
        );
        self.writer.start("text", &[]);
        let entries = self.fonts.engine().database().entries();
        let mut spans: Vec<(String, Vec<Tspan>)> = Vec::new();
        for glyph in &run.glyphs {
            let entry = entries.iter().find(|e| e.font == glyph.font);
            let mut style = entry.map_or_else(Style::new, |e| {
                face_style(e.style, e.variant, Weight(e.weight))
            });
            style.push(("font-size", format!("{}px", short(glyph.size))));
            let name = entry.map_or_else(String::new, |e| e.name.clone());
            style.push(("font-family", repr_str(&name)));
            if let Some(entry) = entry.filter(|e| e.stretch != Stretch::NORMAL) {
                style.push(("font-stretch", stretch_name(entry.stretch)));
            }
            style.extend(color.iter().cloned());
            let shown = run
                .glyph_text(glyph)
                .and_then(|text| text.chars().next())
                .map_or(char::REPLACEMENT_CHARACTER, |c| {
                    if c == ' ' { '\u{a0}' } else { c }
                });
            let style = css(&style);
            let span = (glyph.x, -glyph.y, shown);
            match spans.iter_mut().find(|(known, _)| *known == style) {
                Some((_, glyphs)) => glyphs.push(span),
                None => spans.push((style, vec![span])),
            }
        }
        for (style, mut glyphs) in spans {
            glyphs.sort_by(|a, b| {
                a.0.total_cmp(&b.0)
                    .then(a.1.total_cmp(&b.1))
                    .then(a.2.cmp(&b.2))
            });
            for (x, y, shown) in glyphs {
                self.writer.element(
                    "tspan",
                    &shown.to_string(),
                    &[("x", short(x)), ("y", short(y)), ("style", style.clone())],
                );
            }
        }
        self.writer.end(true);
        for rect in &run.rects {
            let (x, y, w, h) = rect.bounds();
            self.writer.element(
                "rect",
                "",
                &[
                    ("x", short(x)),
                    ("y", short(-y - 1.0)),
                    ("width", short(w)),
                    ("height", short(h)),
                ],
            );
        }
        self.writer.end(true);
    }

    /// `_draw_text_as_text` for a run laid out in points.
    pub(super) fn text_elements(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        source: &TextSource,
        origin: Point,
        angle: f64,
    ) {
        match source {
            // `<text>` elements carry no features or language, as in
            // matplotlib.
            TextSource::Plain { text, font, .. } => {
                self.plain_text_element(gc, run, text, font, origin, angle);
            }
            TextSource::Math { text, .. } => self.math_text_element(gc, run, text, origin, angle),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_font::{FontQuery, GenericFamilies};

    #[test]
    fn families_expand_generic_names() {
        let generics = GenericFamilies {
            sans_serif: vec!["DejaVu Sans".to_owned(), "sans-serif".to_owned()],
            ..GenericFamilies::default()
        };
        let font = FontRequest {
            generics,
            ..FontRequest::new(FontQuery::families(["sans", "Comic Neue"]))
        };
        assert_eq!(
            font_families(&font),
            "'DejaVu Sans', sans-serif, 'Comic Neue'"
        );
    }

    #[test]
    fn styles_name_what_differs() {
        assert_eq!(
            face_style(FontStyle::Normal, FontVariant::Normal, Weight::NORMAL),
            Style::new()
        );
        assert_eq!(
            css(&face_style(
                FontStyle::Oblique,
                FontVariant::SmallCaps,
                Weight::BOLD
            )),
            "font-style: oblique; font-variant: small-caps; font-weight: 700"
        );
        assert_eq!(stretch_name(Stretch::CONDENSED), "condensed");
        assert_eq!(stretch_name(Stretch(550)), "550");
    }
}
