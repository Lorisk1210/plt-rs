//! Lookup of matplotlib's named colours, and the names a program gives
//! colours or takes away (matplotlib's `_colors_full_map` changed).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::OnceLock;

use pltrs_core::Rgba;

use super::named_data::{BASE, CSS4, TABLEAU, XKCD};

/// Looks a colour name up in the program's own mapping of names.
type NameLookup = Rc<dyn Fn(&str) -> Option<Rgba>>;

pltrs_core::thread_state! {
    /// The names the program changed, over the tables: their colour, or
    /// `None` for a name taken away.
    static PROGRAM_NAMES: RefCell<BTreeMap<String, Option<Rgba>>> =
        const { RefCell::new(BTreeMap::new()) };
    /// Where names neither the tables nor [`set_color_name`] know are
    /// looked up: a mapping of names the program put in place of
    /// matplotlib's (ultraplot's `ColorDatabase`).
    static NAME_LOOKUP: RefCell<Option<NameLookup>> =
        const { RefCell::new(None) };
    /// Whether the program's mapping of names stands in place of
    /// matplotlib's, so that every name is looked up there first
    /// ([`set_program_mapping`]).
    static PROGRAM_MAPPING: Cell<bool> = const { Cell::new(false) };
}

/// A named colour as matplotlib's tables store it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NamedColor {
    /// A base colour's RGB triple (`"g"` is `(0, 0.5, 0)`).
    Rgb([f64; 3]),
    /// A `#rrggbb` code, in the table's letter case.
    Hex(&'static str),
}

impl NamedColor {
    /// The opaque colour.
    pub fn rgba(self) -> Rgba {
        match self {
            Self::Rgb([r, g, b]) => Rgba::opaque(r, g, b),
            Self::Hex(hex) => parse_hex6(hex),
        }
    }
}

/// matplotlib's `_colors_full_map`: every named colour, built in reverse
/// priority order so later tables win (base > Tableau > CSS4 > xkcd), with
/// the grey/gray spelling variants matplotlib adds.
fn full_map() -> &'static HashMap<String, NamedColor> {
    static MAP: OnceLock<HashMap<String, NamedColor>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = HashMap::new();
        let mut insert_hex = |table: &[(&'static str, &'static str)], from: &str, to: &str| {
            for &(name, hex) in table {
                if from.is_empty() {
                    map.insert(name.to_owned(), NamedColor::Hex(hex));
                } else if name.contains(from) {
                    map.insert(name.replace(from, to), NamedColor::Hex(hex));
                }
            }
        };
        insert_hex(XKCD, "", "");
        insert_hex(XKCD, "grey", "gray");
        insert_hex(CSS4, "", "");
        insert_hex(TABLEAU, "", "");
        insert_hex(TABLEAU, "gray", "grey");
        for &(name, rgb) in BASE {
            map.insert(name.to_owned(), NamedColor::Rgb(rgb));
        }
        map
    })
}

/// Looks up a named colour exactly as matplotlib does: in the program's
/// mapping of names first while one stands in place of matplotlib's
/// ([`set_program_mapping`]); then the name as given, then (for names
/// longer than one character) its lowercase form, each as the program
/// named it ([`set_color_name`]), else in the tables; else in the
/// program's mapping.
pub(super) fn lookup(name: &str) -> Option<Rgba> {
    // Called with the slot free: the lookup runs the program's code.
    let program = || {
        NAME_LOOKUP
            .with_borrow(Clone::clone)
            .and_then(|lookup| lookup(name))
    };
    let first = PROGRAM_MAPPING.get();
    if first && let Some(color) = program() {
        return Some(color);
    }
    let find = |key: &str| {
        PROGRAM_NAMES
            .with_borrow(|names| names.get(key).copied())
            .unwrap_or_else(|| full_map().get(key).map(|color| color.rgba()))
    };
    find(name)
        .or_else(|| {
            (name.chars().count() != 1)
                .then(|| find(&name.to_lowercase()))
                .flatten()
        })
        .or_else(|| if first { None } else { program() })
}

/// Has names the tables and [`set_color_name`] do not know looked up
/// with `lookup` (matplotlib reads a name from whatever mapping the
/// program put in place of `_colors_full_map`).
pub fn set_color_name_lookup(lookup: impl Fn(&str) -> Option<Rgba> + 'static) {
    NAME_LOOKUP.with_borrow_mut(|slot| *slot = Some(Rc::new(lookup)));
}

/// Has every name looked up first with the lookup
/// [`set_color_name_lookup`] gave, on this thread, while `program` is
/// true: the program put a mapping of names of its own in place of
/// matplotlib's (ultraplot's `ColorDatabase`, which renames `"plum"` and
/// others), which matplotlib reads every name from.
pub fn set_program_mapping(program: bool) {
    PROGRAM_MAPPING.set(program);
}

/// Names `color` `name` on this thread, over the tables, or takes the
/// name away with `None` (matplotlib's `_colors_full_map[name] = color`
/// and `del _colors_full_map[name]`; seaborn's `set_color_codes` gives
/// `"b"`, `"g"` ... the colours of a palette).
///
/// ```
/// use pltrs::colors::{Color, set_color_name};
/// use pltrs_core::Rgba;
/// let muted = Rgba::opaque(0.28, 0.47, 0.81);
/// set_color_name("b", Some(muted));
/// assert_eq!("b".parse::<Color>()?.resolve()?, muted);
/// set_color_name("b", None);
/// assert!("b".parse::<Color>().is_err());
/// pltrs::colors::reset_color_names();
/// assert_eq!("b".parse::<Color>()?.resolve()?, Rgba::opaque(0.0, 0.0, 1.0));
/// # Ok::<(), pltrs::colors::ColorError>(())
/// ```
pub fn set_color_name(name: &str, color: Option<Rgba>) {
    PROGRAM_NAMES.with_borrow_mut(|names| names.insert(name.to_owned(), color));
}

/// Forgets the names the program changed ([`set_color_name`]): the
/// tables name every colour again.
pub fn reset_color_names() {
    PROGRAM_NAMES.with_borrow_mut(BTreeMap::clear);
}

/// Every named colour (name, colour), for listing and table probes.
pub fn named_colors() -> impl Iterator<Item = (&'static str, Rgba)> {
    named_color_table().map(|(name, color)| (name, color.rgba()))
}

/// Every named colour with its value as matplotlib stores it
/// (`get_named_colors_mapping`).
pub fn named_color_table() -> impl Iterator<Item = (&'static str, NamedColor)> {
    full_map()
        .iter()
        .map(|(name, color)| (name.as_str(), *color))
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
