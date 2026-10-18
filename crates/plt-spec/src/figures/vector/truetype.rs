//! Embedded TrueType font programs (PDF `FontFile2`, PostScript Type 42
//! `sfnts`) as drawing tokens: the outline of each glyph shown, so that
//! subsets written differently compare equal when they draw the same.

use plt_font::{Face, GlyphId, OutlineRecorder, PathCommand};

use super::diff::Token;

/// The outlines of `glyphs` in the font program `data`, each introduced
/// by its glyph id; a program that cannot be read gives one word saying
/// so.
pub(super) fn glyph_tokens(
    data: &[u8],
    glyphs: impl IntoIterator<Item = u16>,
    out: &mut Vec<Token>,
) {
    let face = match Face::parse(data, 0) {
        Ok(face) => face,
        Err(err) => {
            out.push(Token::word(format!("bad-font:{err}")));
            return;
        }
    };
    for glyph in glyphs {
        out.push(Token::word(format!("gid{glyph}")));
        let mut recorder = OutlineRecorder::default();
        if face.outline(GlyphId(glyph), &mut recorder).is_err() {
            out.push(Token::word("no-outline"));
            continue;
        }
        for command in recorder.commands {
            let (word, values): (&str, Vec<f32>) = match command {
                PathCommand::MoveTo(x, y) => ("M", vec![x, y]),
                PathCommand::LineTo(x, y) => ("L", vec![x, y]),
                PathCommand::QuadTo(a, b, x, y) => ("Q", vec![a, b, x, y]),
                PathCommand::CurveTo(a, b, c, d, x, y) => ("C", vec![a, b, c, d, x, y]),
                PathCommand::Close => ("Z", Vec::new()),
            };
            out.push(Token::word(word));
            out.extend(values.into_iter().map(|v| Token::Num(f64::from(v))));
        }
    }
}
