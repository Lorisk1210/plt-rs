//! The definitions of a keymap's sections, as written: key codes and
//! names, key types, symbol interpretations, keys ([`super::keys`]) and
//! modifier maps.
//! Modifier names stay names here; [`super::Keymap`] resolves them.

use std::collections::HashMap;

use super::keys::{KeyDef, ModMapKey, key};
use super::lexer::{ModNames, Token, assignment, boolean, group, mod_names, split, statements};
use crate::error::Error;
use crate::keysym;

/// A key type: which modifiers it looks at and the level they select.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TypeDef {
    /// The type's name (`"ALPHABETIC"`).
    pub(crate) name: String,
    /// `modifiers=`.
    pub(crate) modifiers: ModNames,
    /// `map[mods]= level` entries (levels from 0).
    pub(crate) map: Vec<(ModNames, usize)>,
    /// `preserve[mods]= mods` entries.
    pub(crate) preserve: Vec<(ModNames, ModNames)>,
}

/// How an interpretation's modifiers must match a key's modifier map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Predicate {
    /// `NoneOf`.
    NoneOf,
    /// `AnyOfOrNone`.
    AnyOfOrNone,
    /// `AnyOf`.
    AnyOf,
    /// `AllOf`.
    AllOf,
    /// `Exactly`.
    Exactly,
}

/// A symbol interpretation (`interpret Num_Lock+AnyOf(all) { ... }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Interpret {
    /// The keysym it applies to, `None` for `Any`.
    pub(crate) keysym: Option<u32>,
    /// How the modifiers match.
    pub(crate) predicate: Predicate,
    /// The modifiers matched.
    pub(crate) mods: ModNames,
    /// `virtualModifier=`.
    pub(crate) virtual_mod: Option<String>,
    /// `useModMapMods= level1`: only binds to a key's first level.
    pub(crate) level_one_only: bool,
    /// `repeat=`.
    pub(crate) repeat: bool,
}

/// Everything the sections define.
#[derive(Debug, Default)]
pub(crate) struct Definitions {
    /// Key names and their codes.
    pub(crate) keycodes: HashMap<String, u32>,
    /// Alias names and the names they stand for.
    pub(crate) aliases: HashMap<String, String>,
    /// Declared virtual modifiers, with an explicit mapping if written.
    pub(crate) virtual_mods: Vec<(String, Option<ModNames>)>,
    /// Key types.
    pub(crate) types: Vec<TypeDef>,
    /// Symbol interpretations, in order of priority.
    pub(crate) interprets: Vec<Interpret>,
    /// Keys.
    pub(crate) keys: Vec<KeyDef>,
    /// `modifier_map` statements: the real modifier and its keys.
    pub(crate) modifier_maps: Vec<(String, Vec<ModMapKey>)>,
}

/// Interpretation defaults (`interpret.repeat= False;`).
#[derive(Debug, Clone, Copy)]
struct InterpretDefaults {
    level_one_only: bool,
    repeat: bool,
}

/// The most key codes, keys and modifier map entries a keymap may define
/// (compositor keymaps have about 500 key codes and 400 keys); more means
/// the input is not a keymap. The limits keep resolving it fast.
const MAX_KEYS: usize = 4096;
/// The most keysyms of all keys together (compositor keymaps have about
/// 2000).
const MAX_KEYSYMS: usize = 32_768;
/// The most symbol interpretations (compositor keymaps have about 120).
const MAX_INTERPRETS: usize = 1024;
/// The most key types, and level map entries per type.
const MAX_TYPES: usize = 256;
/// The most virtual modifiers declared (XKB has 16).
const MAX_VIRTUAL_MODS: usize = 64;
/// The most levels a key group may have (XKB's limit).
const MAX_LEVELS: usize = 64;

impl Definitions {
    /// Reads a keymap's sections: those of its `xkb_keymap` block, or the
    /// sections themselves.
    pub(crate) fn parse(tokens: &[Token<'_>]) -> Result<Self, Error> {
        let keymap = statements(tokens).into_iter().find_map(|statement| {
            (statement.first().and_then(Token::word) == Some("xkb_keymap"))
                .then(|| group(statement, '{'))
                .flatten()
        });
        let tokens = keymap.map_or(tokens, |(body, _)| body);
        let mut definitions = Self::default();
        let mut found = false;
        for statement in statements(tokens) {
            let Some((kind, (body, _))) = statement
                .first()
                .and_then(Token::word)
                .zip(group(statement, '{'))
            else {
                continue;
            };
            match kind {
                "xkb_keycodes" => definitions.keycodes_section(body),
                "xkb_types" => definitions.types_section(body),
                "xkb_compatibility" | "xkb_compatibility_map" | "xkb_compat" => {
                    definitions.compat_section(body);
                }
                "xkb_symbols" => {
                    definitions.symbols_section(body);
                    found = true;
                }
                _ => {}
            }
        }
        if !found {
            return Err(Error::Malformed("keymap without symbols"));
        }
        if definitions.is_too_large() {
            return Err(Error::Malformed("keymap too large"));
        }
        Ok(definitions)
    }

    /// Whether the definitions exceed what keymaps hold.
    fn is_too_large(&self) -> bool {
        let groups = || self.keys.iter().flat_map(|key| &key.groups);
        let keysyms: usize = groups().map(|group| group.keysyms.len()).sum();
        let mod_map_keys: usize = self.modifier_maps.iter().map(|(_, keys)| keys.len()).sum();
        self.keycodes.len() > MAX_KEYS
            || self.keys.len() > MAX_KEYS
            || mod_map_keys > MAX_KEYS
            || keysyms > MAX_KEYSYMS
            || groups().any(|group| group.keysyms.len() > MAX_LEVELS)
            || self.interprets.len() > MAX_INTERPRETS
            || self.types.len() > MAX_TYPES
            || self
                .types
                .iter()
                .any(|key_type| key_type.map.len().max(key_type.preserve.len()) > MAX_TYPES)
    }

    /// The key types of the `xkb_types` sections in `tokens`.
    pub(crate) fn parse_types(tokens: &[Token<'_>]) -> Vec<TypeDef> {
        let mut definitions = Self::default();
        for statement in statements(tokens) {
            if let (Some("xkb_types"), Some((body, _))) = (
                statement.first().and_then(Token::word),
                group(statement, '{'),
            ) {
                definitions.types_section(body);
            }
        }
        definitions.types
    }

    fn keycodes_section(&mut self, body: &[Token<'_>]) {
        for statement in statements(body) {
            match statement {
                [Token::KeyName(name), eq, Token::Word(code)] if eq.is('=') => {
                    if let Some(code) = number(code) {
                        self.keycodes.insert((*name).to_owned(), code);
                    }
                }
                [
                    Token::Word("alias"),
                    Token::KeyName(alias),
                    eq,
                    Token::KeyName(name),
                ] if eq.is('=') => {
                    self.aliases.insert((*alias).to_owned(), (*name).to_owned());
                }
                _ => {}
            }
        }
    }

    fn types_section(&mut self, body: &[Token<'_>]) {
        for statement in statements(body) {
            match statement {
                [Token::Word("virtual_modifiers"), rest @ ..] => self.declare_virtual_mods(rest),
                [Token::Word("type"), Token::Str(name), ..] => {
                    if let Some((inner, _)) = group(statement, '{') {
                        self.types.push(key_type(name, inner));
                    }
                }
                _ => {}
            }
        }
    }

    fn declare_virtual_mods(&mut self, tokens: &[Token<'_>]) {
        for declaration in split(tokens, ',') {
            let Some(name) = declaration.first().and_then(Token::word) else {
                continue;
            };
            let mapping = match declaration {
                [_, eq, rest @ ..] if eq.is('=') => Some(mod_names(rest)),
                _ => None,
            };
            let room = self.virtual_mods.len() < MAX_VIRTUAL_MODS;
            match self
                .virtual_mods
                .iter_mut()
                .find(|(known, _)| known == name)
            {
                Some((_, known)) => {
                    if mapping.is_some() {
                        *known = mapping;
                    }
                }
                None if room => self.virtual_mods.push((name.to_owned(), mapping)),
                None => {}
            }
        }
    }

    fn compat_section(&mut self, body: &[Token<'_>]) {
        let mut defaults = InterpretDefaults {
            level_one_only: false,
            repeat: false,
        };
        for statement in statements(body) {
            match statement {
                [Token::Word("virtual_modifiers"), rest @ ..] => self.declare_virtual_mods(rest),
                [
                    Token::Word("interpret"),
                    dot,
                    Token::Word(field),
                    eq,
                    value @ ..,
                ] if dot.is('.') && eq.is('=') => match *field {
                    "useModMapMods" => defaults.level_one_only = is_level_one(value),
                    "repeat" => defaults.repeat = boolean(value).unwrap_or(false),
                    _ => {}
                },
                [Token::Word("interpret"), rest @ ..] => {
                    if let Some(interpret) = interpret(rest, defaults) {
                        self.interprets.push(interpret);
                    }
                }
                _ => {}
            }
        }
    }

    fn symbols_section(&mut self, body: &[Token<'_>]) {
        for statement in statements(body) {
            match statement {
                [Token::Word("key"), Token::KeyName(name), ..] => {
                    if let Some((inner, _)) = group(statement, '{') {
                        self.keys.push(key(name, inner));
                    }
                }
                [Token::Word("modifier_map"), Token::Word(modifier), ..] => {
                    if let Some((inner, _)) = group(statement, '{') {
                        let keys = split(inner, ',')
                            .into_iter()
                            .filter_map(|entry| match entry {
                                [Token::KeyName(name)] => Some(ModMapKey::Name((*name).to_owned())),
                                [Token::Word(name)] => {
                                    keysym::from_name(name).map(ModMapKey::Keysym)
                                }
                                _ => None,
                            })
                            .collect();
                        self.modifier_maps.push(((*modifier).to_owned(), keys));
                    }
                }
                [Token::Word("virtual_modifiers"), rest @ ..] => self.declare_virtual_mods(rest),
                _ => {}
            }
        }
    }
}

/// A decimal or hexadecimal number.
fn number(word: &str) -> Option<u32> {
    match word.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => word.parse().ok(),
    }
}

/// A level (`2` or `Level2`), from 0.
fn level(tokens: &[Token<'_>]) -> Option<usize> {
    let word = tokens.first()?.word()?;
    let digits = word
        .strip_prefix("Level")
        .or_else(|| word.strip_prefix("level"))
        .unwrap_or(word);
    digits.parse::<usize>().ok()?.checked_sub(1)
}

/// Whether a `useModMapMods=` value means the first level only.
fn is_level_one(tokens: &[Token<'_>]) -> bool {
    tokens
        .first()
        .and_then(Token::word)
        .is_some_and(|word| word.eq_ignore_ascii_case("level1"))
}

fn key_type(name: &str, body: &[Token<'_>]) -> TypeDef {
    let mut key_type = TypeDef {
        name: name.to_owned(),
        ..TypeDef::default()
    };
    for statement in statements(body) {
        match assignment(statement) {
            Some(("modifiers", None, value)) => key_type.modifiers = mod_names(value),
            Some(("map", Some(mods), value)) => {
                if let Some(level) = level(value) {
                    key_type.map.push((mod_names(mods), level));
                }
            }
            Some(("preserve", Some(mods), value)) => {
                key_type.preserve.push((mod_names(mods), mod_names(value)));
            }
            _ => {}
        }
    }
    key_type
}

/// An interpretation from what follows `interpret`.
fn interpret(tokens: &[Token<'_>], defaults: InterpretDefaults) -> Option<Interpret> {
    let (body, _) = group(tokens, '{')?;
    let header = &tokens[..tokens.iter().position(|t| t.is('{'))?];
    let symbol = header.first()?.word()?;
    let keysym = if symbol == "Any" {
        None
    } else {
        Some(keysym::from_name(symbol)?)
    };
    let (predicate, mods) = match header.get(1..) {
        Some([plus, Token::Word(word), ..]) if plus.is('+') => {
            let predicate = match *word {
                "NoneOf" => Some(Predicate::NoneOf),
                "AnyOfOrNone" => Some(Predicate::AnyOfOrNone),
                "AnyOf" => Some(Predicate::AnyOf),
                "AllOf" => Some(Predicate::AllOf),
                "Exactly" => Some(Predicate::Exactly),
                _ => None,
            };
            match predicate {
                Some(predicate) => (predicate, mod_names(group(header, '(')?.0)),
                // `Sym+Mods`: modifiers without a predicate match exactly.
                None => (Predicate::Exactly, mod_names(&header[2..])),
            }
        }
        _ => (Predicate::AnyOfOrNone, vec![String::from("all")]),
    };
    let mut interpret = Interpret {
        keysym,
        predicate,
        mods,
        virtual_mod: None,
        level_one_only: defaults.level_one_only,
        repeat: defaults.repeat,
    };
    for statement in statements(body) {
        match assignment(statement) {
            Some(("virtualModifier" | "virtualmodifier", None, value)) => {
                interpret.virtual_mod = value.first().and_then(Token::word).map(str::to_owned);
            }
            Some(("useModMapMods" | "usemodmapmods", None, value)) => {
                interpret.level_one_only = is_level_one(value);
            }
            Some(("repeat", None, value)) => interpret.repeat = boolean(value).unwrap_or(false),
            _ => {}
        }
    }
    Some(interpret)
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::super::lexer::tokenize;
    use super::*;

    fn parse(text: &str) -> Definitions {
        Definitions::parse(&tokenize(text).unwrap()).unwrap()
    }

    #[test]
    fn keycodes_and_aliases() {
        let defs = parse(
            "xkb_keymap { xkb_keycodes \"x\" { minimum = 8; <AC01> = 38; <I708> = 0x2c4; \
             alias <LatA> = <AC01>; indicator 1 = \"Caps Lock\"; }; xkb_symbols { }; };",
        );
        assert_eq!(defs.keycodes["AC01"], 38);
        assert_eq!(defs.keycodes["I708"], 708);
        assert_eq!(defs.aliases["LatA"], "AC01");
    }

    #[test]
    fn types_and_virtual_modifiers() {
        let defs = parse(
            "xkb_types { virtual_modifiers NumLock,Alt=Mod1; type \"KEYPAD\" { \
             modifiers= Shift+NumLock; map[None]= Level1; map[Shift]= 2; \
             preserve[Shift]= Shift; level_name[1]= \"Base\"; }; }; \
             xkb_compat { virtual_modifiers NumLock=Mod2; }; xkb_symbols { };",
        );
        assert_eq!(
            defs.virtual_mods,
            [
                (String::from("NumLock"), Some(vec![String::from("Mod2")])),
                (String::from("Alt"), Some(vec![String::from("Mod1")])),
            ]
        );
        let keypad = &defs.types[0];
        assert_eq!(keypad.name, "KEYPAD");
        assert_eq!(keypad.modifiers, ["Shift", "NumLock"]);
        assert_eq!(
            keypad.map,
            [
                (vec![String::from("None")], 0),
                (vec![String::from("Shift")], 1)
            ]
        );
        assert_eq!(keypad.preserve.len(), 1);
    }

    #[test]
    fn interpretations_with_defaults() {
        let defs = parse(
            "xkb_compatibility { interpret.useModMapMods= AnyLevel; interpret.repeat= False; \
             interpret Num_Lock+AnyOf(all) { virtualModifier= NumLock; action= LockMods(modifiers=NumLock); }; \
             interpret ISO_Level3_Shift+AnyOf(all) { virtualModifier= LevelThree; useModMapMods=level1; }; \
             interpret KP_1+AnyOfOrNone(all) { repeat= True; }; interpret Any+Exactly(Lock) { }; \
             interpret Caps_Lock { }; interpret no_such_keysym+AnyOf(all) { }; }; xkb_symbols { };",
        );
        let [num_lock, level3, kp1, any, caps] = &defs.interprets[..] else {
            panic!("{:?}", defs.interprets);
        };
        assert_eq!(num_lock.keysym, Some(0xff7f));
        assert_eq!(num_lock.predicate, Predicate::AnyOf);
        assert_eq!(num_lock.mods, ["all"]);
        assert_eq!(num_lock.virtual_mod.as_deref(), Some("NumLock"));
        assert!(!num_lock.level_one_only && !num_lock.repeat);
        assert!(level3.level_one_only);
        assert!(kp1.repeat);
        assert_eq!((any.keysym, any.predicate), (None, Predicate::Exactly));
        assert_eq!(caps.predicate, Predicate::AnyOfOrNone);
    }

    #[test]
    fn keymaps_need_symbols_and_reasonable_sizes() {
        let tokens = tokenize("xkb_keymap { xkb_keycodes { }; };").unwrap();
        assert!(Definitions::parse(&tokens).is_err());
        assert!(Definitions::parse(&[]).is_err());
        let nested = format!(
            "{}xkb_symbols {{ }};{}",
            "xkb_keymap { ".repeat(10_000),
            "};".repeat(10_000)
        );
        assert!(
            Definitions::parse(&tokenize(&nested).unwrap()).is_err(),
            "no recursion"
        );
        let keys = (0..=MAX_KEYS).fold(String::new(), |mut keys, i| {
            write!(keys, "key <K{i}> {{ [ a ] }};").unwrap();
            keys
        });
        let many = format!("xkb_symbols {{ {keys} }};");
        assert!(Definitions::parse(&tokenize(&many).unwrap()).is_err());
        let wide = format!(
            "xkb_symbols {{ key <A> {{ [ {} ] }}; }};",
            vec!["a"; MAX_LEVELS + 1].join(",")
        );
        assert!(Definitions::parse(&tokenize(&wide).unwrap()).is_err());
    }
}
