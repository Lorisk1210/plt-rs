//! XKB keymaps in the text form Wayland compositors send
//! (`wl_keyboard.keymap`), parsed far enough to turn key codes and
//! modifier states into keysyms.
//!
//! Supported: key names and aliases, key types with their level maps and
//! preserved modifiers (automatic types for keys without one), several
//! groups (layouts), virtual modifiers mapped explicitly or through
//! symbol interpretations and modifier maps, Caps Lock's capitalization,
//! and per-key repeat. Actions are not interpreted: the compositor
//! tracks the modifier state and reports it with `wl_keyboard.modifiers`.

mod keys;
mod lexer;
mod resolve;
mod sections;

use std::collections::HashMap;

use resolve::{Mods, STANDARD_TYPES, automatic_type, find_type};
use sections::{Definitions, TypeDef};

use crate::error::Error;
use crate::keyboard::Modifiers;
use crate::keysym::{self, NO_SYMBOL};

/// The `Shift` bit.
const SHIFT: u32 = 1;
/// The `Lock` bit.
const LOCK: u32 = 1 << 1;
/// The `Control` bit.
const CONTROL: u32 = 1 << 2;
/// `Mod1`, Alt's usual modifier.
const MOD1: u32 = 1 << 3;
/// `Mod4`, Super's usual modifier.
const MOD4: u32 = 1 << 6;

/// A level of a key type: the modifiers selecting it, and those it
/// leaves for others to see.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    mods: u32,
    level: usize,
    preserve: u32,
}

/// A key type, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyType {
    mask: u32,
    entries: Vec<Entry>,
}

/// One group of a key: its type and keysyms by level.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Group {
    key_type: usize,
    keysyms: Vec<u32>,
}

/// A key: its groups, and whether it repeats.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Key {
    groups: Vec<Group>,
    repeats: bool,
}

/// A keyboard mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Keymap {
    keys: HashMap<u32, Key>,
    types: Vec<KeyType>,
    alt: u32,
    super_key: u32,
    mods: Mods,
}

impl Keymap {
    /// Parses a keymap in XKB text form.
    pub(crate) fn parse(text: &str) -> Result<Self, Error> {
        let tokens = lexer::tokenize(text)?;
        let definitions = Definitions::parse(&tokens)?;
        let standard_tokens = lexer::tokenize(STANDARD_TYPES)?;
        let standard = Definitions::parse_types(&standard_tokens);
        let mut mods = Mods::declared(&definitions);
        let resolved = resolve::resolve(&definitions, &mut mods);
        let mut types: Vec<KeyType> = Vec::new();
        let mut type_indices: HashMap<String, usize> = HashMap::new();
        let mut keys = HashMap::new();
        for (code, key) in resolved {
            let mut groups = Vec::new();
            for group in &key.def.groups {
                let name = group
                    .type_name
                    .as_deref()
                    .unwrap_or_else(|| automatic_type(&group.keysyms));
                let key_type = if let Some(&index) = type_indices.get(name) {
                    index
                } else {
                    let def = find_type(&definitions.types, &standard, name);
                    types.push(def.map_or_else(KeyType::one_level, |def| KeyType::new(def, &mods)));
                    type_indices.insert(name.to_owned(), types.len() - 1);
                    types.len() - 1
                };
                groups.push(Group {
                    key_type,
                    keysyms: group.keysyms.clone(),
                });
            }
            if !groups.is_empty() {
                let repeats = key.repeats;
                keys.insert(code, Key { groups, repeats });
            }
        }
        let alt = Some(mods.virtual_mask("Alt"))
            .filter(|&mask| mask != 0)
            .unwrap_or(MOD1);
        let super_key = Some(mods.virtual_mask("Super"))
            .filter(|&mask| mask != 0)
            .unwrap_or(MOD4);
        Ok(Self {
            keys,
            types,
            alt,
            super_key,
            mods,
        })
    }

    /// The keysym key `keycode` produces with the effective modifiers
    /// `state` in layout `group`.
    pub(crate) fn keysym(&self, keycode: u32, state: u32, group: u32) -> u32 {
        let Some(key) = self.keys.get(&keycode) else {
            return NO_SYMBOL;
        };
        let state = self.mods.to_real(state);
        let group = &key.groups[group as usize % key.groups.len()];
        let key_type = &self.types[group.key_type];
        let active = state & key_type.mask;
        let entry = key_type.entries.iter().find(|entry| entry.mods == active);
        let (level, preserve) = entry.map_or((0, 0), |entry| (entry.level, entry.preserve));
        let keysym = group.keysyms.get(level).copied().unwrap_or(NO_SYMBOL);
        let consumed = key_type.mask & !preserve;
        if state & LOCK != 0 && consumed & LOCK == 0 {
            keysym::to_upper(keysym)
        } else {
            keysym
        }
    }

    /// Whether key `keycode` repeats while held.
    pub(crate) fn repeats(&self, keycode: u32) -> bool {
        self.keys.get(&keycode).is_some_and(|key| key.repeats)
    }

    /// The modifiers held in the effective state `state`.
    pub(crate) fn modifiers(&self, state: u32) -> Modifiers {
        let state = self.mods.to_real(state);
        Modifiers::NONE
            .with(Modifiers::SHIFT, state & SHIFT != 0)
            .with(Modifiers::CONTROL, state & CONTROL != 0)
            .with(Modifiers::ALT, state & self.alt != 0)
            .with(Modifiers::SUPER, state & self.super_key != 0)
    }
}

impl KeyType {
    /// A type resolved with the keymap's modifiers; entries of unmapped
    /// virtual modifiers are dropped, as they never match.
    fn new(def: &TypeDef, mods: &Mods) -> Self {
        let entries = def
            .map
            .iter()
            .filter(|(names, _)| mods.is_active(names))
            .map(|(names, level)| {
                let preserve = def
                    .preserve
                    .iter()
                    .find(|(preserved, _)| mods.mask(preserved) == mods.mask(names))
                    .map_or(0, |(_, kept)| mods.mask(kept));
                Entry {
                    mods: mods.mask(names),
                    level: *level,
                    preserve,
                }
            })
            .collect();
        Self {
            mask: mods.mask(&def.modifiers),
            entries,
        }
    }

    /// A type with a single level.
    fn one_level() -> Self {
        Self {
            mask: 0,
            entries: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests;
