//! PostScript a distiller wrote (`ps.usedistiller`, and every usetex
//! document, which Ghostscript distills) as drawing items.
//!
//! What such a document holds is the distiller's own PostScript: what is
//! compared is what plt-rs decides and what it drew. That is the header
//! comments (the page, the bounding box and, from Ghostscript, the
//! command line it ran with), the prolog and trailer `pstoeps` adds to
//! EPS output, then the drawing:
//!
//! * Ghostscript's `ps2write` device writes the PDF objects of the page
//!   into the PostScript, for a procedure set of its own to run; they are
//!   read as a PDF document's (`pdf::embedded_items`), so every painted
//!   element carries its graphics state and every glyph its font.
//! * `pdftops` (the `xpdf` distiller) writes PDF operators in PostScript
//!   syntax: the setup (font programs) and every page become one item per
//!   operator with its operands, embedded Type 1 programs left out.

use super::lexer::{Lexer, Tok};
use crate::figures::vector::diff::{Item, Token};
use crate::figures::vector::pdf;

/// The program that wrote a distilled document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Distiller {
    /// Ghostscript's `ps2write` device.
    Ghostscript,
    /// poppler's `pdftops`.
    Pdftops,
}

/// The header comments that name the program, the date or the file.
const UNSTABLE: [&str; 4] = ["%%Creator", "%%CreationDate", "%%Title", "%%For"];

/// Which distiller wrote `text`, if one did: its `%%Creator` comment says.
pub(super) fn distiller(text: &str) -> Option<Distiller> {
    let header = &text[..text.find("%%EndComments").unwrap_or(text.len())];
    let creator = header
        .lines()
        .find_map(|line| line.strip_prefix("%%Creator: "))?;
    if creator.starts_with("GPL Ghostscript") && creator.contains("(ps2write)") {
        Some(Distiller::Ghostscript)
    } else if creator.starts_with("poppler pdftops") || creator.starts_with("xpdf") {
        Some(Distiller::Pdftops)
    } else {
        None
    }
}

/// The words of `text`, comments included, numbers as numbers.
fn words(text: &str) -> Vec<Token> {
    text.split_whitespace()
        .map(|word| word.parse().map_or_else(|_| Token::word(word), Token::Num))
        .collect()
}

/// The header comments but those naming the program, date or file.
fn header_item(header: &str) -> Item {
    let mut item = Item::new("header");
    for line in header.lines().filter(|line| line.starts_with("%%")) {
        if !UNSTABLE.iter().any(|unstable| line.starts_with(unstable)) {
            item.tokens.extend(words(line));
        }
    }
    item
}

/// The prolog `pstoeps` puts after the header of EPS output (up to its
/// `%%Page 1 1` and the rotation that may follow), as an item.
fn eps_prolog_item(rest: &str) -> Option<Item> {
    let prolog = rest
        .strip_prefix("%%BeginProlog\n")
        .filter(|p| p.starts_with("save\n"))?;
    let end = prolog.find("%%Page 1 1\n")? + "%%Page 1 1\n".len();
    let after = &prolog[end..];
    let rotation = after
        .find("90 rotate\n")
        .filter(|&at| after[..at].lines().count() == 1)
        .map_or("", |at| &after[..at + "90 rotate\n".len()]);
    let mut item = Item::new("eps-prolog");
    item.tokens.extend(words(&prolog[..end]));
    item.tokens.extend(words(rotation));
    Some(item)
}

/// A token as compared: strings by their text.
fn token(tok: Tok) -> Token {
    match tok {
        Tok::Num(n) => Token::Num(n),
        Tok::Name(n) => Token::word(n),
        Tok::Lit(n) => Token::word(format!("/{n}")),
        Tok::Str(s) => Token::word(format!("({})", String::from_utf8_lossy(&s))),
        Tok::ProcStart => Token::word("{"),
        Tok::ProcEnd => Token::word("}"),
    }
}

/// Whether a name is an operand (array and dictionary marks, constants)
/// rather than an operator.
fn is_operand(name: &str) -> bool {
    matches!(name, "[" | "]" | "<<" | ">>" | "true" | "false" | "null")
}

/// `text` without the encrypted part of embedded Type 1 programs
/// (`currentfile eexec` to `cleartomark`).
fn without_eexec(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("currentfile eexec") {
        out.push_str(&rest[..start]);
        rest = rest[start..]
            .find("cleartomark")
            .map_or("", |end| &rest[start + end..]);
    }
    out.push_str(rest);
    out
}

/// One item per operator of PostScript text, with its operands; the
/// items of procedures follow their operator words inside the item.
fn operator_items(text: &str, items: &mut Vec<Item>) {
    let mut operands = Vec::new();
    let mut depth = 0usize;
    for tok in Lexer::new(without_eexec(text).as_bytes()) {
        match &tok {
            Tok::ProcStart => depth += 1,
            Tok::ProcEnd => depth = depth.saturating_sub(1),
            Tok::Name(name) if depth == 0 && !is_operand(name) => {
                items.push(Item {
                    kind: name.clone(),
                    tokens: std::mem::take(&mut operands),
                });
                continue;
            }
            _ => {}
        }
        operands.push(token(tok));
    }
}

/// The text between the line `start` and the line `end` after it.
fn section<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = text.find(start)? + start.len();
    let to = text[from..].find(end)? + from;
    Some(&text[from..to])
}

/// The drawing items of a document `distiller` wrote.
pub(super) fn items(text: &str, distiller: Distiller) -> Result<Vec<Item>, String> {
    let (header, rest) = text
        .split_once("%%EndComments\n")
        .ok_or("no %%EndComments")?;
    let mut items = vec![header_item(header)];
    items.extend(eps_prolog_item(rest));
    match distiller {
        Distiller::Ghostscript => items.extend(pdf::embedded_items(text.as_bytes())?),
        Distiller::Pdftops => {
            let setup = section(text, "\n%%BeginSetup\n", "\n%%EndSetup\n").unwrap_or("");
            operator_items(setup, &mut items);
            for page in text.split("\n%%Page: ").skip(1) {
                // The page's label and ordinal end the comment's line.
                let page = page.split_once('\n').map_or("", |(_, body)| body);
                let page = &page[..page.find("\n%%PageTrailer").unwrap_or(page.len())];
                items.push(Item::new("page"));
                operator_items(page, &mut items);
            }
        }
    }
    let trailer = text.rfind("\n%%Trailer\n").map_or("", |at| &text[at..]);
    items.push(Item {
        kind: "trailer".to_owned(),
        tokens: words(trailer),
    });
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PDFTOPS: &str = "%!PS-Adobe-3.0 EPSF-3.0\n%%Creator: poppler pdftops version: 25.03.0\n\
        %%BoundingBox: 0 0 461 346\n%%EndComments\n%%BeginProlog\nsave\ncountdictstack\nmark\n\
        newpath\n/showpage {} def\n/setpagedevice {pop} def\n%%EndProlog\n%%Page 1 1\n\
        %%BeginProlog\n%%EndProlog\n%%BeginSetup\nxpdf begin\n/F1 /Helvetica 1 1 [ /a ] pdfMakeFont\n\
        /T1 findfont dup currentfile eexec\nA0B1C2\ncleartomark\n%%EndSetup\n%%Page: 1 1\n\
        [] 0 d\n[0.1 0 0 0.1 0 0] cm\n/DeviceGray {} cs\n(Hi) [1 0] Tj\n%%PageTrailer\n\
        %%Trailer\nend\ncleartomark\nrestore\nshowpage\n%%EOF\n";

    #[test]
    fn distillers_are_named_by_the_creator() {
        assert_eq!(distiller(PDFTOPS), Some(Distiller::Pdftops));
        assert_eq!(
            distiller("%!PS\n%%Creator: GPL Ghostscript 10051 (ps2write)\n%%EndComments\n"),
            Some(Distiller::Ghostscript)
        );
        assert_eq!(
            distiller("%!PS\n%%Creator: Matplotlib\n%%EndComments\n"),
            None
        );
        assert_eq!(distiller("%!PS\n%%EndComments\n%%Creator: pdftops"), None);
    }

    #[test]
    fn pdftops_documents_become_operators() {
        let items: Vec<String> = items(PDFTOPS, Distiller::Pdftops)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(items[0], "header %%BoundingBox: 0 0 461 346");
        assert_eq!(
            items[1],
            "eps-prolog save countdictstack mark newpath /showpage {} def \
             /setpagedevice {pop} def %%EndProlog %%Page 1 1"
        );
        assert_eq!(items[2..4], ["xpdf", "begin"]);
        assert_eq!(items[4], "pdfMakeFont /F1 /Helvetica 1 1 [ /a ]");
        assert_eq!(items[5..9], ["findfont /T1", "dup", "cleartomark", "page"]);
        assert_eq!(items[9], "d [ ] 0");
        assert_eq!(items[11], "cs /DeviceGray { }");
        assert_eq!(items[12], "Tj (Hi) [ 1 0 ]");
        assert_eq!(
            items.last().map(String::as_str),
            Some("trailer %%Trailer end cleartomark restore showpage %%EOF")
        );
    }

    #[test]
    fn rotations_belong_to_the_eps_prolog() {
        let rest =
            "%%BeginProlog\nsave\n%%EndProlog\n%%Page 1 1\n144.00 0.00 translate\n90 rotate\nq\n";
        let item = eps_prolog_item(rest).unwrap().to_string();
        assert_eq!(
            item,
            "eps-prolog save %%EndProlog %%Page 1 1 144 0 translate 90 rotate"
        );
        assert!(eps_prolog_item("%%BeginProlog\n10 dict\n").is_none());
        assert_eq!(
            without_eexec("a currentfile eexec\nFF\ncleartomark b"),
            "a cleartomark b"
        );
    }
}
