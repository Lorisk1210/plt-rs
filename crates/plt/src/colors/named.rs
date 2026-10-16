//! Lookup of matplotlib's named colours.

use std::collections::HashMap;
use std::sync::OnceLock;

use plt_core::Rgba;

use super::named_data::{BASE, CSS4, TABLEAU, XKCD};

/// matplotlib's `_colors_full_map`: every named colour, built in reverse
/// priority order so later tables win (base > Tableau > CSS4 > xkcd), with
/// the grey/gray spelling variants matplotlib adds.
fn full_map() -> &'static HashMap<String, Rgba> {
    static MAP: OnceLock<HashMap<String, Rgba>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = HashMap::new();
        let mut insert_hex = |table: &[(&str, &str)], from: &str, to: &str| {
            for &(name, hex) in table {
                let rgba = parse_hex6(hex);
                if from.is_empty() {
                    map.insert(name.to_owned(), rgba);
                } else if name.contains(from) {
                    map.insert(name.replace(from, to), rgba);
                }
            }
        };
        insert_hex(XKCD, "", "");
        insert_hex(XKCD, "grey", "gray");
        insert_hex(CSS4, "", "");
        insert_hex(TABLEAU, "", "");
        insert_hex(TABLEAU, "gray", "grey");
        for &(name, [r, g, b]) in BASE {
            map.insert(name.to_owned(), Rgba::opaque(r, g, b));
        }
        map
    })
}

/// Looks up a named colour exactly as matplotlib does: the name as given,
/// then (for names longer than one character) its lowercase form.
pub(super) fn lookup(name: &str) -> Option<Rgba> {
    let map = full_map();
    map.get(name)
        .or_else(|| {
            (name.chars().count() != 1)
                .then(|| map.get(&name.to_lowercase()))
                .flatten()
        })
        .copied()
}

/// Every named colour (name, colour), for listing and table probes.
pub fn named_colors() -> impl Iterator<Item = (&'static str, Rgba)> {
    full_map().iter().map(|(name, rgba)| (name.as_str(), *rgba))
}

/// One of matplotlib's named-colour tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorTable {
    /// Single-letter colours (`b`, `g`, `r`, `c`, `m`, `y`, `k`, `w`).
    Base,
    /// The Tableau palette (`tab:blue`, ...).
    Tableau,
    /// CSS4 colour names.
    Css4,
    /// xkcd colour names (`xkcd:sky blue`, ...).
    Xkcd,
}

impl ColorTable {
    /// The table's entries in matplotlib's order as `(name, colour)`.
    pub fn entries(self) -> Vec<(&'static str, Rgba)> {
        let hex = |table: &[(&'static str, &str)]| {
            table
                .iter()
                .map(|&(name, hex)| (name, parse_hex6(hex)))
                .collect()
        };
        match self {
            Self::Base => BASE
                .iter()
                .map(|&(name, [r, g, b])| (name, Rgba::opaque(r, g, b)))
                .collect(),
            Self::Tableau => hex(TABLEAU),
            Self::Css4 => hex(CSS4),
            Self::Xkcd => hex(XKCD),
        }
    }

    /// The hex strings as matplotlib stores them (base colours are tuples
    /// and have none).
    pub fn hex_entries(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Base => &[],
            Self::Tableau => TABLEAU,
            Self::Css4 => CSS4,
            Self::Xkcd => XKCD,
        }
    }
}

/// Parses a generated `#rrggbb` table entry.
fn parse_hex6(hex: &str) -> Rgba {
    let channel = |i: usize| {
        let byte = u8::from_str_radix(&hex[i..i + 2], 16).expect("generated tables hold valid hex");
        f64::from(byte) / 255.0
    };
    Rgba::opaque(channel(1), channel(3), channel(5))
}
