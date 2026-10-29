//! The keys of the symbols section: their groups, types, keysyms and
//! flags (`key <AE01> { type= "FOUR_LEVEL", symbols[Group1]= [ 1, exclam ] };`).

use super::lexer::{ModNames, Token, assignment, boolean, group, mod_names, split};
use crate::keysym::{self, NO_SYMBOL};

/// One group of a key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GroupDef {
    /// The explicit type, if any.
    pub(crate) type_name: Option<String>,
    /// The keysym of each level.
    pub(crate) keysyms: Vec<u32>,
}

/// A key of the symbols section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct KeyDef {
    /// The key's name (`"AC01"`).
    pub(crate) name: String,
    /// Its groups.
    pub(crate) groups: Vec<GroupDef>,
    /// `virtualMods=`, if given.
    pub(crate) virtual_mods: Option<ModNames>,
    /// `repeat=`, if given.
    pub(crate) repeat: Option<bool>,
}

/// A key a `modifier_map` names: by name, or by a keysym it produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModMapKey {
    /// `<LALT>`.
    Name(String),
    /// `Alt_L`.
    Keysym(u32),
}

/// A key from its name and the tokens inside its braces.
pub(crate) fn key(name: &str, body: &[Token<'_>]) -> KeyDef {
    let mut key = KeyDef {
        name: name.to_owned(),
        ..KeyDef::default()
    };
    let mut bare_groups = 0;
    let mut shared_type = None;
    for item in split(body, ',') {
        if item.first().is_some_and(|t| t.is('[')) {
            group_mut(&mut key, bare_groups).keysyms = keysym_list(item);
            bare_groups += 1;
            continue;
        }
        match assignment(item) {
            Some(("symbols", Some(index), value)) => {
                if let Some(index) = group_index(index) {
                    group_mut(&mut key, index).keysyms = keysym_list(value);
                }
            }
            Some(("type", index, [Token::Str(type_name)])) => match index.map(group_index) {
                None => shared_type = Some(type_name.clone()),
                Some(Some(index)) => group_mut(&mut key, index).type_name = Some(type_name.clone()),
                Some(None) => {}
            },
            Some(("virtualMods" | "virtualmods" | "vmods", None, value)) => {
                key.virtual_mods = Some(mod_names(value));
            }
            Some(("repeat", None, value)) => key.repeat = boolean(value),
            _ => {}
        }
    }
    if let Some(type_name) = shared_type {
        for group in &mut key.groups {
            group.type_name.get_or_insert_with(|| type_name.clone());
        }
    }
    key
}

/// The most groups a key may have (XKB's limit).
const MAX_GROUPS: usize = 4;

/// Group `index` of `key`, adding empty groups up to it (indices beyond
/// [`MAX_GROUPS`] share the last).
fn group_mut(key: &mut KeyDef, index: usize) -> &mut GroupDef {
    let index = index.min(MAX_GROUPS - 1);
    if key.groups.len() <= index {
        key.groups.resize_with(index + 1, GroupDef::default);
    }
    &mut key.groups[index]
}

/// A group index (`Group2` or `2`), from 0.
fn group_index(tokens: &[Token<'_>]) -> Option<usize> {
    let word = tokens.first()?.word()?;
    let digits = word
        .strip_prefix("Group")
        .or_else(|| word.strip_prefix("group"))
        .unwrap_or(word);
    digits.parse::<usize>().ok()?.checked_sub(1)
}

/// The keysyms of `[ a, A, { b, c } ]`: unknown names and levels with
/// several keysyms give no symbol.
fn keysym_list(tokens: &[Token<'_>]) -> Vec<u32> {
    let Some((inside, _)) = group(tokens, '[') else {
        return Vec::new();
    };
    split(inside, ',')
        .into_iter()
        .map(|level| match level {
            [Token::Word(name)] => keysym::from_name(name).unwrap_or(NO_SYMBOL),
            _ => NO_SYMBOL,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::lexer::tokenize;
    use super::super::sections::Definitions;
    use super::*;

    fn definitions(text: &str) -> Definitions {
        Definitions::parse(&tokenize(text).unwrap()).unwrap()
    }

    #[test]
    fn keys_in_every_form() {
        let defs = definitions(
            "xkb_symbols { key <ESC> { [ Escape ] }; \
             key <AE01> { type[Group1]= \"FOUR_LEVEL\", symbols[Group1]= [ 1, exclam, onesuperior, exclamdown ], \
             symbols[Group2]= [ 1, exclam ] }; \
             key <RALT> { type= \"TWO_LEVEL\", repeat= No, virtualMods= LevelThree, \
             symbols[Group1]= [ ISO_Level3_Shift, Multi_key ] }; \
             key <TWO> { [ a, A ], [ { b, c }, bogus ] }; \
             modifier_map Mod1 { <LALT>, Meta_L }; };",
        );
        let [escape, one, ralt, two] = &defs.keys[..] else {
            panic!("{:?}", defs.keys);
        };
        assert_eq!(
            escape.groups,
            [GroupDef {
                type_name: None,
                keysyms: vec![0xff1b]
            }]
        );
        assert_eq!(one.groups.len(), 2);
        assert_eq!(one.groups[0].type_name.as_deref(), Some("FOUR_LEVEL"));
        assert_eq!(one.groups[0].keysyms, [0x31, 0x21, 0xb9, 0xa1]);
        assert_eq!(one.groups[1].type_name, None);
        assert_eq!(ralt.groups[0].type_name.as_deref(), Some("TWO_LEVEL"));
        assert_eq!(ralt.repeat, Some(false));
        assert_eq!(
            ralt.virtual_mods.as_deref(),
            Some(&[String::from("LevelThree")][..])
        );
        assert_eq!(two.groups[1].keysyms, [NO_SYMBOL, NO_SYMBOL]);
        assert_eq!(
            defs.modifier_maps,
            [(
                String::from("Mod1"),
                vec![
                    ModMapKey::Name(String::from("LALT")),
                    ModMapKey::Keysym(0xffe7)
                ]
            )]
        );
    }
}
