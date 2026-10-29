//! Structures shared by `GSUB` and `GPOS`: script/feature/lookup lists
//! ([`table`]), coverage tables and class definitions ([`coverage`]).

mod coverage;
mod table;
#[cfg(test)]
pub(crate) mod tests;

pub(crate) use coverage::{ClassDef, Coverage};
pub(crate) use table::{LangSys, LayoutTable, Lookup};

use crate::reader::FromBytes;
use crate::types::Tag;

/// Lookup flag bits.
pub(crate) mod lookup_flag {
    pub(crate) const RIGHT_TO_LEFT: u16 = 0x0001;
    pub(crate) const IGNORE_BASE_GLYPHS: u16 = 0x0002;
    pub(crate) const IGNORE_LIGATURES: u16 = 0x0004;
    pub(crate) const IGNORE_MARKS: u16 = 0x0008;
    pub(crate) const IGNORE_FLAGS: u16 = IGNORE_BASE_GLYPHS | IGNORE_LIGATURES | IGNORE_MARKS;
    pub(crate) const USE_MARK_FILTERING_SET: u16 = 0x0010;
    pub(crate) const MARK_ATTACHMENT_TYPE: u16 = 0xFF00;
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TagRecord {
    pub(crate) tag: Tag,
    pub(crate) offset: u16,
}

impl FromBytes for TagRecord {
    const SIZE: usize = 6;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            tag: Tag::from_bytes(&bytes[0..4]),
            offset: u16::from_bytes(&bytes[4..6]),
        }
    }
}
