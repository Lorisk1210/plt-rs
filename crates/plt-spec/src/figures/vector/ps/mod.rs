//! PostScript documents as drawing items: the header's bounding box, the
//! page description run by a small interpreter ([`interp`]) — one item
//! per fill, stroke, glyph, image and shading with its graphics state —
//! and the Type 3 fonts of the prolog.

mod interp;
mod lexer;

use self::interp::Interpreter;
use self::lexer::{Lexer, Tok};
use super::diff::{Item, Token};

/// Tokens of a PostScript fragment.
fn tokens(text: &str) -> Vec<Token> {
    Lexer::new(text.as_bytes())
        .map(|token| match token {
            Tok::Num(n) => Token::Num(n),
            Tok::Name(n) => Token::word(n),
            Tok::Lit(n) => Token::word(format!("/{n}")),
            Tok::Str(s) => Token::word(format!("<{} bytes>", s.len())),
            Tok::ProcStart => Token::word("{"),
            Tok::ProcEnd => Token::word("}"),
        })
        .collect()
}

/// The text between `start` and the following `end` in `text`.
fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = text.find(start)? + start.len();
    let to = text[from..].find(end)? + from;
    Some(&text[from..to])
}

/// One Type 3 font program: name, matrix, box, encoding and every glyph
/// procedure (sorted by glyph name).
fn font_item(program: &str) -> Item {
    let mut item = Item::new("font");
    for (start, end) in [
        ("/FontName ", " def"),
        ("/FontMatrix ", " def"),
        ("/FontBBox ", " def"),
        ("/Encoding ", " def"),
    ] {
        if let Some(value) = between(program, start, end) {
            item.tokens.extend(tokens(value));
        }
    }
    let glyphs = between(program, "/.notdef 0 def\n", "end readonly def").unwrap_or("");
    let mut entries: Vec<&str> = glyphs
        .split("} _d")
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .collect();
    entries.sort_unstable();
    for entry in entries {
        item.tokens.extend(tokens(entry));
    }
    item
}

/// The drawing items of a PostScript document.
pub(super) fn items(document: &[u8]) -> Result<Vec<Item>, String> {
    let text = std::str::from_utf8(document).map_err(|e| e.to_string())?;
    let (prolog, body) = text.split_once("%%EndProlog\n").ok_or("no %%EndProlog")?;
    let mut header = Item::new("header");
    for line in prolog.lines() {
        for key in [
            "%%BoundingBox:",
            "%%HiResBoundingBox:",
            "%%Orientation:",
            "%%DocumentPaperSizes:",
        ] {
            if let Some(value) = line.strip_prefix(key) {
                header.tokens.push(Token::word(key));
                header.tokens.extend(tokens(value));
            }
        }
    }
    let mut interpreter = Interpreter::new();
    interpreter.run(body.as_bytes());
    let mut items = vec![header];
    items.extend(interpreter.items);
    let mut fonts: Vec<Item> = prolog
        .split("%!PS-Adobe-3.0 Resource-Font")
        .skip(1)
        .map(font_item)
        .collect();
    fonts.sort_by_key(ToString::to_string);
    items.extend(fonts);
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonts_and_headers() {
        let program = "\n10 dict begin\n/FontName /F-0 def\n/FontMatrix [0.5 0 0 0.5 0 0] def\n\
                       /Encoding [/b /a] def\n/CharStrings 3 dict dup begin\n/.notdef 0 def\n\
                       /b{1 0 0 0 1 1 sc\n0 0 m\nce} _d\n/a{2 0 0 0 1 1 sc\nce} _d\nend readonly def\n";
        let item = font_item(program);
        let text = item.to_string();
        assert!(
            text.starts_with("font /F-0 [ 0.5 0 0 0.5 0 0 ] [ /b /a ]"),
            "{text}"
        );
        assert!(
            text.ends_with("/a { 2 0 0 0 1 1 sc ce /b { 1 0 0 0 1 1 sc 0 0 m ce"),
            "{text}"
        );
        let doc = b"%!PS-Adobe-3.0\n%%BoundingBox: 0 0 10 10\n%%EndComments\n%%BeginProlog\n%%EndProlog\n\
                    mpldict begin\n0 0 m 1 1 l stroke\nend\nshowpage\n";
        let items = items(doc).unwrap();
        assert_eq!(items[0].to_string(), "header %%BoundingBox: 0 0 10 10");
        assert_eq!(items[1].kind, "stroke");
    }
}
