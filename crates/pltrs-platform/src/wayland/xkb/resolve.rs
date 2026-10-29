//! Resolving a keymap's names: real and virtual modifiers, the keys'
//! modifier maps, symbol interpretations, and automatic key types.

use std::collections::HashMap;

use super::keys::{KeyDef, ModMapKey};
use super::lexer::ModNames;
use super::sections::{Definitions, Interpret, Predicate, TypeDef};
use crate::keysym::{self, NO_SYMBOL};

/// The real modifiers, by bit.
const REAL_MODS: [&str; 8] = [
    "shift", "lock", "control", "mod1", "mod2", "mod3", "mod4", "mod5",
];
/// All real modifiers.
pub(super) const ALL_REAL: u32 = 0xff;
/// The most virtual modifiers (bits 8 to 31).
const MAX_VIRTUAL: usize = 24;

/// Resolves modifier names to real modifier masks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Mods {
    /// Virtual modifier names, in declaration order (bit 8 onwards).
    names: Vec<String>,
    /// The real modifiers each virtual one maps to.
    mapping: Vec<u32>,
}

impl Mods {
    /// The virtual modifiers declared by `definitions`, not yet mapped.
    pub(super) fn declared(definitions: &Definitions) -> Self {
        let names: Vec<String> = definitions
            .virtual_mods
            .iter()
            .take(MAX_VIRTUAL)
            .map(|(name, _)| name.clone())
            .collect();
        Self {
            mapping: vec![0; names.len()],
            names,
        }
    }

    /// The real bits and virtual modifier indices named by `names`.
    fn split(&self, names: &ModNames) -> (u32, Vec<usize>) {
        let mut real = 0;
        let mut virtual_mods = Vec::new();
        for name in names {
            let lower = name.to_ascii_lowercase();
            if let Some(bit) = REAL_MODS.iter().position(|&known| known == lower) {
                real |= 1 << bit;
            } else if lower == "all" {
                real |= ALL_REAL;
                virtual_mods.extend(0..self.names.len());
            } else if let Some(index) = self.names.iter().position(|known| known == name) {
                virtual_mods.push(index);
            }
        }
        (real, virtual_mods)
    }

    /// The real modifier mask of `names`.
    pub(super) fn mask(&self, names: &ModNames) -> u32 {
        let (real, virtual_mods) = self.split(names);
        virtual_mods
            .into_iter()
            .fold(real, |mask, index| mask | self.mapping[index])
    }

    /// Whether `names` can match: they are all known and resolve to some
    /// modifier, or say `none`. An expression with unknown names or of
    /// unmapped virtual modifiers only never matches.
    pub(super) fn is_active(&self, names: &ModNames) -> bool {
        let known = names.iter().all(|name| {
            let lower = name.to_ascii_lowercase();
            REAL_MODS.contains(&lower.as_str())
                || lower == "all"
                || lower == "none"
                || self.names.contains(name)
        });
        let none = names.iter().all(|name| name.eq_ignore_ascii_case("none"));
        known && (none || self.mask(names) != 0)
    }

    /// The real modifiers of the virtual modifier `name` (0 if unknown).
    pub(super) fn virtual_mask(&self, name: &str) -> u32 {
        self.names
            .iter()
            .position(|known| known == name)
            .map_or(0, |index| self.mapping[index])
    }

    /// A state mask with its virtual modifier bits (if a compositor sends
    /// any) replaced by the real modifiers they map to.
    pub(super) fn to_real(&self, state: u32) -> u32 {
        (0..self.mapping.len())
            .filter(|index| state & (1 << (index + 8)) != 0)
            .fold(state & ALL_REAL, |mask, index| mask | self.mapping[index])
    }
}

/// What resolution yields per key, by keycode.
#[derive(Debug)]
pub(super) struct ResolvedKey<'d> {
    /// The key's definition.
    pub(super) def: &'d KeyDef,
    /// Whether it repeats.
    pub(super) repeats: bool,
}

/// Resolves the modifier mapping of `definitions` into `mods` and returns
/// the keys by keycode.
pub(super) fn resolve<'d>(
    definitions: &'d Definitions,
    mods: &mut Mods,
) -> HashMap<u32, ResolvedKey<'d>> {
    let code_of = |name: &str| {
        let name = definitions.aliases.get(name).map_or(name, String::as_str);
        definitions.keycodes.get(name).copied()
    };
    let keys: HashMap<u32, &KeyDef> = definitions
        .keys
        .iter()
        .filter_map(|key| Some((code_of(&key.name)?, key)))
        .collect();
    let mut modmaps: HashMap<u32, u32> = HashMap::new();
    for (modifier, entries) in &definitions.modifier_maps {
        let (bit, _) = mods.split(&vec![modifier.clone()]);
        for entry in entries {
            let code = match entry {
                ModMapKey::Name(name) => code_of(name),
                ModMapKey::Keysym(keysym) => first_key_with(&keys, *keysym),
            };
            if let Some(code) = code {
                *modmaps.entry(code).or_default() |= bit;
            }
        }
    }
    let interprets = Interprets::new(&definitions.interprets, mods);
    let mut resolved = HashMap::new();
    let mut key_vmods = Vec::new();
    for (&code, &def) in &keys {
        let modmap = modmaps.get(&code).copied().unwrap_or(0);
        let base = interprets.find(def, 0, 0, modmap);
        let vmods = match &def.virtual_mods {
            Some(names) => mods.split(names).1,
            None => interprets.virtual_mods(def, modmap),
        };
        key_vmods.push((vmods, modmap));
        let repeats = def
            .repeat
            .unwrap_or(base.is_none_or(|interpret| interpret.repeat));
        resolved.insert(code, ResolvedKey { def, repeats });
    }
    let mut mapping = vec![0; mods.names.len()];
    for (vmods, modmap) in key_vmods {
        for index in vmods {
            mapping[index] |= modmap;
        }
    }
    for (index, (_, explicit)) in definitions
        .virtual_mods
        .iter()
        .take(MAX_VIRTUAL)
        .enumerate()
    {
        if let Some(names) = explicit {
            mapping[index] = mods.split(names).0;
        }
    }
    mods.mapping = mapping;
    resolved
}

/// The lowest keycode producing `keysym` at its first level, else at any.
fn first_key_with(keys: &HashMap<u32, &KeyDef>, keysym: u32) -> Option<u32> {
    let first_level = keys
        .iter()
        .filter(|(_, key)| key.groups.first().and_then(|g| g.keysyms.first()) == Some(&keysym))
        .map(|(&code, _)| code)
        .min();
    first_level.or_else(|| {
        keys.iter()
            .filter(|(_, key)| key.groups.iter().any(|g| g.keysyms.contains(&keysym)))
            .map(|(&code, _)| code)
            .min()
    })
}

/// The symbol interpretations, matched against keys.
struct Interprets<'d, 'm> {
    /// Each interpretation with the real modifiers it matches.
    list: Vec<(&'d Interpret, u32)>,
    mods: &'m Mods,
}

impl<'d, 'm> Interprets<'d, 'm> {
    fn new(list: &'d [Interpret], mods: &'m Mods) -> Self {
        let list = list
            .iter()
            .map(|interpret| (interpret, mods.split(&interpret.mods).0))
            .collect();
        Self { list, mods }
    }

    /// The interpretation of level `level` of group `group` of `key`
    /// (with modifier map `modmap`): the first matching one naming the
    /// level's keysym, else the first matching `Any`.
    fn find(&self, key: &KeyDef, group: usize, level: usize, modmap: u32) -> Option<&'d Interpret> {
        let keysym = *key.groups.get(group)?.keysyms.get(level)?;
        if keysym == NO_SYMBOL {
            return None;
        }
        let matches = |&&(interpret, wanted): &&(&Interpret, u32)| {
            let mods = if level == 0 || !interpret.level_one_only {
                modmap
            } else {
                0
            };
            match interpret.predicate {
                Predicate::NoneOf => mods & wanted == 0,
                Predicate::AnyOfOrNone => mods == 0 || mods & wanted != 0,
                Predicate::AnyOf => mods & wanted != 0,
                Predicate::AllOf => mods & wanted == wanted,
                Predicate::Exactly => mods == wanted,
            }
        };
        let list = self.list.iter();
        list.clone()
            .filter(|(interpret, _)| interpret.keysym == Some(keysym))
            .find(matches)
            .or_else(|| {
                list.filter(|(interpret, _)| interpret.keysym.is_none())
                    .find(matches)
            })
            .map(|&(interpret, _)| interpret)
    }

    /// The virtual modifiers the interpretations of a key's levels bind.
    fn virtual_mods(&self, key: &KeyDef, modmap: u32) -> Vec<usize> {
        let mut found = Vec::new();
        for (group, def) in key.groups.iter().enumerate() {
            for level in 0..def.keysyms.len() {
                let Some(interpret) = self.find(key, group, level, modmap) else {
                    continue;
                };
                let applies = (group == 0 && level == 0) || !interpret.level_one_only;
                if let Some(name) = interpret.virtual_mod.as_ref().filter(|_| applies) {
                    found.extend(self.mods.names.iter().position(|known| known == name));
                }
            }
        }
        found
    }
}

/// The type XKB assigns to a group of `keysyms` without an explicit one.
pub(super) fn automatic_type(keysyms: &[u32]) -> &'static str {
    let is_lower = |keysym: u32| keysym::to_upper(keysym) != keysym;
    let is_upper = |keysym: u32| keysym::to_lower(keysym) != keysym;
    let level = |index: usize| keysyms.get(index).copied().unwrap_or(NO_SYMBOL);
    let keypad = keysym::is_keypad(level(0)) || keysym::is_keypad(level(1));
    match keysyms.len() {
        2 if is_lower(level(0)) && is_upper(level(1)) => "ALPHABETIC",
        2 if keypad => "KEYPAD",
        2 => "TWO_LEVEL",
        3 | 4 if is_lower(level(0)) && is_upper(level(1)) => {
            if is_lower(level(2)) && is_upper(level(3)) {
                "FOUR_LEVEL_ALPHABETIC"
            } else {
                "FOUR_LEVEL_SEMIALPHABETIC"
            }
        }
        3 | 4 if keypad => "FOUR_LEVEL_KEYPAD",
        3 | 4 => "FOUR_LEVEL",
        // One level, and widths XKB has no automatic type for.
        _ => "ONE_LEVEL",
    }
}

/// The standard key types, for keymaps that rely on ones they do not
/// define.
pub(super) const STANDARD_TYPES: &str = r#"xkb_types {
    virtual_modifiers NumLock,LevelThree;
    type "ONE_LEVEL" { modifiers= none; };
    type "TWO_LEVEL" { modifiers= Shift; map[Shift]= 2; };
    type "ALPHABETIC" { modifiers= Shift+Lock; map[Shift]= 2; map[Lock]= 2; };
    type "KEYPAD" { modifiers= Shift+NumLock; map[Shift]= 2; map[NumLock]= 2; };
    type "FOUR_LEVEL" { modifiers= Shift+LevelThree; map[Shift]= 2;
        map[LevelThree]= 3; map[Shift+LevelThree]= 4; };
    type "FOUR_LEVEL_ALPHABETIC" { modifiers= Shift+Lock+LevelThree; map[Shift]= 2;
        map[Lock]= 2; map[LevelThree]= 3; map[Shift+LevelThree]= 4;
        map[Lock+LevelThree]= 4; map[Shift+Lock+LevelThree]= 3; };
    type "FOUR_LEVEL_SEMIALPHABETIC" { modifiers= Shift+Lock+LevelThree; map[Shift]= 2;
        map[Lock]= 2; map[LevelThree]= 3; map[Shift+LevelThree]= 4;
        map[Lock+LevelThree]= 3; preserve[Lock+LevelThree]= Lock;
        map[Shift+Lock+LevelThree]= 4; preserve[Shift+Lock+LevelThree]= Lock; };
    type "FOUR_LEVEL_KEYPAD" { modifiers= Shift+NumLock+LevelThree; map[Shift]= 2;
        map[NumLock]= 2; map[LevelThree]= 3; map[Shift+LevelThree]= 4;
        map[NumLock+LevelThree]= 4; map[Shift+NumLock+LevelThree]= 3; };
};"#;

/// The type named `name`: the keymap's own, else a standard one.
pub(super) fn find_type<'t>(
    types: &'t [TypeDef],
    standard: &'t [TypeDef],
    name: &str,
) -> Option<&'t TypeDef> {
    types
        .iter()
        .chain(standard)
        .find(|key_type| key_type.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_types_follow_xkb() {
        assert_eq!(automatic_type(&[0xff1b]), "ONE_LEVEL");
        assert_eq!(automatic_type(&[0x61, 0x41]), "ALPHABETIC");
        assert_eq!(automatic_type(&[0x31, 0x21]), "TWO_LEVEL");
        assert_eq!(automatic_type(&[0xff9c, 0xffb1]), "KEYPAD");
        assert_eq!(
            automatic_type(&[0x71, 0x51, 0x40, 0x7d9]),
            "FOUR_LEVEL_SEMIALPHABETIC"
        );
        assert_eq!(
            automatic_type(&[0x61, 0x41, 0xe6, 0xc6]),
            "FOUR_LEVEL_ALPHABETIC"
        );
        assert_eq!(automatic_type(&[0x31, 0x21, 0xb9]), "FOUR_LEVEL");
        assert_eq!(automatic_type(&[0xffb1, 0xffb1, 0, 0]), "FOUR_LEVEL_KEYPAD");
        assert_eq!(automatic_type(&[1, 2, 3, 4, 5]), "ONE_LEVEL");
        assert_eq!(automatic_type(&[]), "ONE_LEVEL");
    }

    #[test]
    fn modifier_names_resolve() {
        let mut mods = Mods {
            names: vec![String::from("NumLock"), String::from("LevelThree")],
            mapping: vec![1 << 4, 0],
        };
        let names = |text: &str| text.split('+').map(String::from).collect::<ModNames>();
        assert_eq!(mods.mask(&names("Shift+Control+NumLock")), 0b1_0101);
        assert_eq!(mods.mask(&names("None")), 0);
        assert_eq!(mods.mask(&names("all")), ALL_REAL);
        assert!(!mods.is_active(&names("LevelThree")), "unmapped");
        assert!(mods.is_active(&names("None")));
        assert!(!mods.is_active(&names("Bogus")), "unknown");
        assert!(mods.is_active(&names("Shift+LevelThree")));
        mods.mapping[1] = 1 << 7;
        assert_eq!(mods.virtual_mask("LevelThree"), 1 << 7);
        assert_eq!(mods.virtual_mask("Hyper"), 0);
        assert_eq!(mods.to_real(1 | 1 << 9), 1 | 1 << 7);
    }
}
