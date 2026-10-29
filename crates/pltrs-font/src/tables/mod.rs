//! Parsers for individual sfnt tables.
//!
//! Each parser borrows the table bytes and decodes lazily where tables are
//! large (`glyf`, `cmap`, `kern`); small header tables are decoded into plain
//! value structs that are re-exported from the crate root.

pub(crate) mod cmap;
pub(crate) mod directory;
pub(crate) mod glyf;
pub(crate) mod head;
pub(crate) mod hmtx;
pub(crate) mod kern;
pub(crate) mod name;
pub(crate) mod os2;
pub(crate) mod pclt;
pub(crate) mod post;
