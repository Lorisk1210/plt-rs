//! PGF pictures reduced to drawing items: one per command line (comments
//! left out), with the numbers matplotlib writes as `%f` (six decimals)
//! compared tolerantly and everything else, the typeset text included,
//! exactly. Image files are named after the document, so only their
//! number is kept.

use super::diff::{Item, Token};

/// The number written as `%f` at the start of `text`, and its length.
fn fixed_number(text: &str) -> Option<(f64, usize)> {
    let bytes = text.as_bytes();
    let sign = usize::from(bytes.first() == Some(&b'-'));
    let integer = bytes[sign..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    let point = sign + integer;
    if integer == 0 || bytes.get(point) != Some(&b'.') {
        return None;
    }
    let decimals = bytes[point + 1..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if decimals != 6 {
        return None;
    }
    let len = point + 1 + decimals;
    text[..len].parse().ok().map(|value| (value, len))
}

/// The item of one command line.
fn command(line: &str) -> Item {
    let name_len = line
        .char_indices()
        .skip(1)
        .find(|&(_, c)| !c.is_ascii_alphabetic() && c != '@')
        .map_or(line.len(), |(i, _)| i);
    let mut item = Item::new(&line[..name_len]);
    let mut word = String::new();
    let mut rest = &line[name_len..];
    while let Some(c) = rest.chars().next() {
        let at_number = word
            .chars()
            .last()
            .is_none_or(|last| !last.is_ascii_digit() && last != '.');
        if let Some((value, len)) = at_number.then(|| fixed_number(rest)).flatten() {
            if !word.is_empty() {
                item.tokens.push(Token::word(std::mem::take(&mut word)));
            }
            item.tokens.push(Token::Num(value));
            rest = &rest[len..];
        } else {
            word.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    if !word.is_empty() {
        item.tokens.push(Token::word(image_number(&word)));
    }
    item
}

/// `word` with image file names `<stem>-img<N>.png` shortened to
/// `img<N>.png`.
fn image_number(word: &str) -> String {
    match (word.rfind("-img"), word.rfind('{')) {
        (Some(img), Some(open)) if open < img && word.ends_with(".png}}") => {
            format!("{}{}", &word[..=open], &word[img + 1..])
        }
        _ => word.to_owned(),
    }
}

/// Reduces a PGF picture to drawing items.
pub(super) fn items(document: &[u8]) -> Result<Vec<Item>, String> {
    let text = std::str::from_utf8(document).map_err(|e| format!("not UTF-8: {e}"))?;
    Ok(text
        .lines()
        .filter(|line| !line.starts_with('%') && !line.is_empty())
        .map(|line| command(line.strip_suffix('%').unwrap_or(line)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_written_as_percent_f_are_tolerant() {
        let item = command(r"\pgfpathmoveto{\pgfqpoint{1.250000in}{-0.500000in}}");
        assert_eq!(item.kind, r"\pgfpathmoveto");
        assert_eq!(
            item.tokens,
            [
                Token::word(r"{\pgfqpoint{"),
                Token::Num(1.25),
                Token::word("in}{"),
                Token::Num(-0.5),
                Token::word("in}}"),
            ]
        );
        // Typeset text keeps its digits.
        let text = command(r"\pgftext[x=1.000000in,y=2.000000in,,base]{0.25}");
        assert_eq!(text.tokens.last(), Some(&Token::word("in,,base]{0.25}")));
    }

    #[test]
    fn comments_go_and_image_names_shorten() {
        let document = "%% header\n\\begin{pgfscope}%\n\\pgftext[left,bottom]{\\includegraphics[interpolate=true,width=1.000000in,height=1.000000in]{case-img3.png}}%\n";
        let items = items(document.as_bytes()).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind, r"\begin");
        assert_eq!(items[1].tokens.last(), Some(&Token::word("in]{img3.png}}")));
    }
}
