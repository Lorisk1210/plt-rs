//! Font maps (`pdftex.map`, `psfonts.map`; matplotlib's
//! `dviread.PsfontsMap`) and PostScript encoding files (`_parse_enc`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use plt_font::Type1Effects;

use super::error::{TexError, TexFile};
use super::files::FileFinder;

/// The PostScript font a font map gives for a TeX font (matplotlib's
/// `PsFont`).
#[derive(Clone, Debug, PartialEq)]
pub struct PsFont {
    /// The TeX font name.
    pub texname: Vec<u8>,
    /// The PostScript name of the font.
    pub psname: Vec<u8>,
    /// The `SlantFont` and `ExtendFont` effects.
    pub effects: Type1Effects,
    /// The encoding file, if the map names one.
    pub encoding: Option<PathBuf>,
    /// The font file, if the map names one.
    pub filename: Option<PathBuf>,
}

/// A font map. Its lines are only parsed when a font they define is asked
/// for, as matplotlib does: TeX distributions ship maps of thousands of
/// lines.
#[derive(Debug)]
pub struct PsfontsMap {
    name: String,
    lines: Mutex<Lines>,
}

/// The lines of a map: those not parsed yet, by their first word, and the
/// fonts parsed so far. One lock covers both, so that a font being parsed
/// is never missing from both for another thread.
#[derive(Debug, Default)]
struct Lines {
    unparsed: HashMap<Vec<u8>, Vec<Vec<u8>>>,
    parsed: HashMap<Vec<u8>, PsFont>,
}

/// A word of a map line: a double-quoted special or an unquoted word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Word<'a> {
    Quoted(&'a [u8]),
    Unquoted(&'a [u8]),
}

/// Python's `bytes.isspace` for one byte (`\S` in a bytes pattern).
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c')
}

/// `re.finditer(br'"([^"]*)(?:"|$)|(\S+)', line)`.
fn words(line: &[u8]) -> Vec<Word<'_>> {
    let mut words = Vec::new();
    let mut i = 0;
    while i < line.len() {
        if line[i] == b'"' {
            let start = i + 1;
            let end = line[start..]
                .iter()
                .position(|&b| b == b'"')
                .map_or(line.len(), |offset| start + offset);
            // Without a closing quote, `$` matches at the very end.
            words.push(Word::Quoted(&line[start..end]));
            i = (end + 1).min(line.len());
        } else if is_space(line[i]) {
            i += 1;
        } else {
            // `\S+` runs over quotes too.
            let start = i;
            while i < line.len() && !is_space(line[i]) {
                i += 1;
            }
            words.push(Word::Unquoted(&line[start..i]));
        }
    }
    words
}

/// Python's `float(bytes)`: surrounding whitespace allowed, underscores
/// only between digits.
pub(crate) fn python_float(text: &[u8]) -> Option<f64> {
    let text = std::str::from_utf8(text)
        .ok()?
        .trim_matches(|c: char| c.is_ascii_whitespace());
    let bytes = text.as_bytes();
    let digit = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_digit);
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'_' && !(i > 0 && digit(i - 1) && digit(i + 1)) {
            return None;
        }
    }
    let cleaned = text.replace('_', "");
    let lower = cleaned.to_ascii_lowercase();
    let unsigned = lower.trim_start_matches(['+', '-']);
    if unsigned.len() + 1 < lower.len() {
        return None;
    }
    if matches!(unsigned, "inf" | "infinity" | "nan")
        || unsigned.starts_with(|c: char| c.is_ascii_digit() || c == '.')
    {
        cleaned.parse().ok()
    } else {
        None
    }
}

impl PsfontsMap {
    /// A map read from `data` (the contents of the file called `name`).
    pub fn parse(data: &[u8], name: &str) -> Self {
        let mut unparsed: HashMap<Vec<u8>, Vec<Vec<u8>>> = HashMap::new();
        for line in data.split_inclusive(|&b| b == b'\n') {
            let key = line.split(|&b| b == b' ').next().unwrap_or_default();
            unparsed
                .entry(key.to_vec())
                .or_default()
                .push(line.to_vec());
        }
        Self {
            name: name.to_owned(),
            lines: Mutex::new(Lines {
                unparsed,
                parsed: HashMap::new(),
            }),
        }
    }

    /// The PostScript font for TeX font `texname`, its files found with
    /// `finder` (matplotlib's `map[texname]`).
    ///
    /// # Errors
    ///
    /// When the map has no usable line for the font, a line is malformed,
    /// or the encoding or font file a line names cannot be found.
    pub fn get(&self, texname: &[u8], finder: &dyn FileFinder) -> Result<PsFont, TexError> {
        let mut lines = self.lines.lock().unwrap_or_else(PoisonError::into_inner);
        for line in lines.unparsed.remove(texname).into_iter().flatten() {
            if let Some((tfmname, font)) = parse_line(&line, finder)? {
                if let Some(tfmname) = tfmname {
                    lines.parsed.insert(tfmname, font);
                }
                break;
            }
        }
        lines
            .parsed
            .get(texname)
            .cloned()
            .ok_or_else(|| TexError::UnmappedFont {
                map: self.name.clone(),
                texname: String::from_utf8_lossy(texname).into_owned(),
            })
    }
}

/// A parsed map line: the TeX name it defines a font for, if it has one,
/// and the font.
type MapLine = (Option<Vec<u8>>, PsFont);

/// `_parse_and_cache_line`: the font a line defines (with the TeX name
/// it defines it for, if it has one), `None` for comments and lines a
/// PDF backend cannot use.
fn parse_line(line: &[u8], finder: &dyn FileFinder) -> Result<Option<MapLine>, TexError> {
    if line.is_empty() || line.starts_with(b" ") || b"%*;#".contains(&line[0]) {
        return Ok(None);
    }
    let malformed = |message: &str| TexError::malformed(TexFile::FontMap, message);
    let (mut tfmname, mut basename, mut special) = (None, None, None);
    let (mut encodingfile, mut fontfile) = (None, None);
    let mut is_subsetted = false;
    let mut words = words(line).into_iter();
    while let Some(word) = words.next() {
        match word {
            Word::Unquoted(word) => {
                if let Some(file) = word.strip_prefix(b"<<") {
                    fontfile = Some(file);
                } else if let Some(file) = word.strip_prefix(b"<[") {
                    encodingfile = Some(file);
                } else if let Some(file) = word.strip_prefix(b"<") {
                    let file = if file.is_empty() {
                        match words.next() {
                            Some(Word::Unquoted(next) | Word::Quoted(next)) if !next.is_empty() => {
                                next
                            }
                            _ => return Err(malformed("missing file name after '<'")),
                        }
                    } else {
                        file
                    };
                    if file.ends_with(b".enc") {
                        encodingfile = Some(file);
                    } else {
                        fontfile = Some(file);
                        is_subsetted = true;
                    }
                } else if tfmname.is_none() {
                    tfmname = Some(word);
                } else if basename.is_none() {
                    basename = Some(word);
                }
            }
            Word::Quoted(quoted) if !quoted.is_empty() => special = Some(quoted),
            Word::Quoted(_) => {}
        }
    }
    let mut effects = Type1Effects::default();
    if let Some(special) = special {
        let mut words = special
            .split(|&b| is_space(b))
            .filter(|w| !w.is_empty())
            .rev();
        while let Some(word) = words.next() {
            let slot = match word {
                b"SlantFont" => &mut effects.slant,
                b"ExtendFont" => &mut effects.extend,
                _ => continue,
            };
            let value = words
                .next()
                .and_then(python_float)
                .ok_or_else(|| malformed("invalid effect value"))?;
            *slot = Some(value);
        }
    }
    let (mut is_t1, mut is_truetype) = (false, false);
    if let Some(file) = fontfile {
        if file.ends_with(b".ttf") || file.ends_with(b".ttc") {
            is_truetype = true;
        } else if !file.ends_with(b".otf") {
            is_t1 = true;
        }
    } else if basename.is_some() {
        is_t1 = true;
    }
    let has_effects = effects.slant.is_some() || effects.extend.is_some();
    if (is_truetype && is_subsetted && encodingfile.is_none())
        || (!is_t1 && has_effects)
        || effects.slant.unwrap_or(0.0).abs() > 1.0
        || effects.extend.unwrap_or(0.0).abs() > 2.0
    {
        return Ok(None);
    }
    let find = |file: &[u8]| finder.find(&String::from_utf8_lossy(file));
    let encoding = encodingfile.map(find).transpose()?;
    let filename = fontfile.map(find).transpose()?;
    let texname = tfmname.unwrap_or_default().to_vec();
    Ok(Some((
        tfmname.map(<[u8]>::to_vec),
        PsFont {
            psname: basename.map_or_else(|| texname.clone(), <[u8]>::to_vec),
            texname,
            effects,
            encoding,
            filename,
        },
    )))
}

/// `_parse_enc`: the glyph names of a PostScript encoding file, by
/// character code.
///
/// # Errors
///
/// When the file is not ASCII or holds no array of names.
pub fn parse_enc(data: &[u8]) -> Result<Vec<String>, TexError> {
    let malformed = |message: &str| TexError::malformed(TexFile::Encoding, message);
    let text = std::str::from_utf8(data)
        .ok()
        .filter(|text| text.is_ascii())
        .ok_or_else(|| malformed("not ASCII"))?;
    // `re.sub("%.*", "", text)`: comments end before the line break.
    let no_comments: String = text
        .split_inclusive('\n')
        .map(|line| match line.find('%') {
            Some(start) if line.ends_with('\n') => format!("{}\n", &line[..start]),
            Some(start) => line[..start].to_owned(),
            None => line.to_owned(),
        })
        .collect();
    let array = no_comments
        .find('[')
        .zip(no_comments.rfind(']'))
        .filter(|(start, end)| start < end)
        .map(|(start, end)| &no_comments[start + 1..end])
        .ok_or_else(|| malformed("no encoding array"))?;
    let names: Vec<&str> = array.split_ascii_whitespace().collect();
    if names.iter().all(|name| name.starts_with('/')) {
        Ok(names.iter().map(|name| name[1..].to_owned()).collect())
    } else {
        Err(malformed("Failed to parse as Postscript encoding"))
    }
}

#[cfg(test)]
mod tests;
