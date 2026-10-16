//! Unicode scripts, character properties and libraqm's script itemization.

use std::fmt;

use crate::shape::unicode_data::{COMMON, INHERITED, MARK_RANGES, SCRIPT_RUNS, SCRIPTS};
use crate::types::Tag;

/// A Unicode script (ISO 15924), e.g. Latin, Greek, Common.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Script(u8);

impl Script {
    /// The Common script (`Zyyy`): punctuation, digits, symbols.
    pub const COMMON: Self = Self(COMMON);
    /// The Inherited script (`Zinh`): combining marks.
    pub const INHERITED: Self = Self(INHERITED);

    /// Script of `c`.
    pub fn of(c: char) -> Self {
        let code = u32::from(c);
        let index = SCRIPT_RUNS.partition_point(|&(start, _)| start <= code);
        Self(
            SCRIPT_RUNS
                .get(index.wrapping_sub(1))
                .map_or(COMMON, |&(_, script)| script),
        )
    }

    /// Looks a script up by its four-letter ISO 15924 code (`"Latn"`).
    pub fn from_iso_code(code: &str) -> Option<Self> {
        let index = SCRIPTS
            .iter()
            .position(|(iso, _)| iso.as_slice() == code.as_bytes())?;
        u8::try_from(index).ok().map(Self)
    }

    /// The ISO 15924 code (`"Latn"`).
    pub fn iso_code(self) -> &'static str {
        SCRIPTS
            .get(usize::from(self.0))
            .and_then(|(iso, _)| std::str::from_utf8(*iso).ok())
            .unwrap_or("Zzzz")
    }

    /// The OpenType script tag, `None` for Common/Inherited/Unknown.
    pub fn opentype_tag(self) -> Option<Tag> {
        SCRIPTS
            .get(usize::from(self.0))
            .and_then(|(_, tag)| tag.map(Tag::new))
    }
}

impl fmt::Debug for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Script({})", self.iso_code())
    }
}

/// Whether `c` is a combining mark, and if so whether it is non-spacing
/// (general category Mn) rather than Mc/Me.
pub(crate) fn mark_kind(c: char) -> Option<bool> {
    let code = u32::from(c);
    let index = MARK_RANGES.partition_point(|&(start, _, _)| start <= code);
    let &(_, end, nonspacing) = MARK_RANGES.get(index.checked_sub(1)?)?;
    (code <= end).then_some(nonspacing)
}

/// HarfBuzz's `is_default_ignorable`.
pub(crate) fn is_default_ignorable(c: char) -> bool {
    matches!(
        u32::from(c),
        0x00AD
            | 0x034F
            | 0x061C
            | 0x17B4..=0x17B5
            | 0x180B..=0x180E
            | 0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x206F
            | 0xFE00..=0xFE0F
            | 0xFEFF
            | 0xFFF0..=0xFFF8
            | 0x1D173..=0x1D17A
            | 0xE0000..=0xE0FFF
    )
}

/// Paired punctuation recognised by libraqm, openers at even indices.
const PAIRED: [u32; 34] = [
    0x0028, 0x0029, 0x003C, 0x003E, 0x005B, 0x005D, 0x007B, 0x007D, 0x00AB, 0x00BB, 0x2018, 0x2019,
    0x201C, 0x201D, 0x2039, 0x203A, 0x3008, 0x3009, 0x300A, 0x300B, 0x300C, 0x300D, 0x300E, 0x300F,
    0x3010, 0x3011, 0x3014, 0x3015, 0x3016, 0x3017, 0x3018, 0x3019, 0x301A, 0x301B,
];

/// libraqm's `_raqm_resolve_scripts`: Common and Inherited characters take
/// the script of the preceding character (paired brackets that of their
/// opener), and leading ones that of the following character.
pub(crate) fn resolve_scripts(text: &[char]) -> Vec<Script> {
    let mut scripts: Vec<Script> = text.iter().map(|&c| Script::of(c)).collect();
    let mut last_script_index: Option<usize> = None;
    let mut last_set_index: Option<usize> = None;
    let mut last_script = Script::COMMON;
    let mut stack: Vec<(Script, usize)> = Vec::new();
    for i in 0..text.len() {
        let script = scripts[i];
        if script == Script::COMMON && last_script_index.is_some() {
            let pair = PAIRED.iter().position(|&p| p == u32::from(text[i]));
            match pair {
                Some(pair) if pair % 2 == 0 => {
                    scripts[i] = last_script;
                    stack.push((last_script, pair));
                }
                Some(pair) => {
                    while stack.last().is_some_and(|&(_, open)| open != pair & !1) {
                        stack.pop();
                    }
                    if let Some(&(opener, _)) = stack.last() {
                        scripts[i] = opener;
                        last_script = opener;
                    } else {
                        scripts[i] = last_script;
                    }
                }
                None => scripts[i] = last_script,
            }
            last_set_index = Some(i);
        } else if script == Script::INHERITED && last_script_index.is_some() {
            scripts[i] = last_script;
            last_set_index = Some(i);
        } else {
            let fill_from = last_set_index.map_or(0, |index| index + 1);
            for slot in &mut scripts[fill_from..i] {
                *slot = script;
            }
            last_script = script;
            last_script_index = Some(i);
            last_set_index = Some(i);
        }
    }
    for i in (0..text.len().saturating_sub(1)).rev() {
        if scripts[i] == Script::COMMON || scripts[i] == Script::INHERITED {
            scripts[i] = scripts[i + 1];
        }
    }
    scripts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn latin() -> Script {
        Script::from_iso_code("Latn").unwrap()
    }

    fn greek() -> Script {
        Script::from_iso_code("Grek").unwrap()
    }

    #[test]
    fn script_lookup() {
        assert_eq!(Script::of('A'), latin());
        assert_eq!(Script::of('α'), greek());
        assert_eq!(Script::of('1'), Script::COMMON);
        assert_eq!(Script::of('\u{301}'), Script::INHERITED);
        assert_eq!(Script::of('\u{0}'), Script::COMMON);
        assert_eq!(latin().opentype_tag(), Some(Tag::new(b"latn")));
        assert_eq!(Script::COMMON.opentype_tag(), None);
        assert_eq!(Script::of('ア').opentype_tag(), Some(Tag::new(b"kana")));
        assert_eq!(latin().iso_code(), "Latn");
        assert_eq!(format!("{:?}", greek()), "Script(Grek)");
        assert!(Script::from_iso_code("Nope").is_none());
    }

    #[test]
    fn character_properties() {
        assert_eq!(mark_kind('\u{301}'), Some(true));
        assert_eq!(mark_kind('\u{903}'), Some(false));
        assert_eq!(mark_kind('a'), None);
        assert!(is_default_ignorable('\u{200D}'));
        assert!(is_default_ignorable('\u{AD}'));
        assert!(!is_default_ignorable(' '));
    }

    #[test]
    fn raqm_resolution() {
        let text: Vec<char> = "1 Aα. (β) x\u{301}".chars().collect();
        let scripts = resolve_scripts(&text);
        let l = latin();
        let g = greek();
        assert_eq!(scripts, vec![l, l, l, g, g, g, g, g, g, g, l, l]);
        let all_common: Vec<char> = "1+2".chars().collect();
        assert_eq!(resolve_scripts(&all_common), vec![Script::COMMON; 3]);
        assert_eq!(resolve_scripts(&[]), Vec::new());
    }
}
