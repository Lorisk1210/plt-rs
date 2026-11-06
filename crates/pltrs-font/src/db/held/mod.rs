//! Font files held for drawing (see [`crate::region`]): read when one of
//! their faces is first used, with every table but those drawing reads
//! glyph by glyph or not at all (`glyf`, bitmaps, variations) and, of a
//! CFF table, all but its `CharStrings` data. The rest is read from the
//! file, a range at a time, when it is used, and kept while the file is.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};

use super::arena::Arena;
use super::sparse::{DRAWN, Face, Font, Table};
use crate::cff::{CffParts, Index, charstrings_offset};
use crate::error::{FontError, Result};
use crate::region::{LazyFile, Region};
use crate::types::Tag;

/// The tag of CFF tables.
const CFF: [u8; 4] = *b"CFF ";

/// The first bytes of a CFF table read to find its `CharStrings`: its
/// header, Name INDEX and Top DICT INDEX take far fewer.
const CFF_HEAD: usize = 64 << 10;

/// The most bytes the count, offset size and offsets of an INDEX take
/// (65 535 entries with four-byte offsets).
const INDEX_HEAD: usize = 3 + 4 * (u16::MAX as usize + 1);

/// A CFF table held but for its `CharStrings` data.
#[derive(Debug)]
struct SplitCff {
    /// Where the table is in the file.
    table: Range<usize>,
    /// Where its `CharStrings` data are in the table.
    charstrings: Range<usize>,
    /// The bytes before them.
    head: Vec<u8>,
    /// The bytes after them.
    tail: Vec<u8>,
}

/// A font file held for drawing.
#[derive(Debug)]
pub(super) struct HeldFile {
    path: PathBuf,
    /// The faces' directories and the tables held, packed.
    held: Vec<u8>,
    /// Each table not held: its face, its tag, and where it is in the file.
    elsewhere: Vec<(u32, [u8; 4], Range<usize>)>,
    /// The CFF tables held but for their `CharStrings` data.
    cffs: Vec<SplitCff>,
    /// The file, for what is not held.
    file: Mutex<File>,
    /// The file's length.
    length: usize,
    /// The ranges read since the file was opened.
    pieces: Arena<Box<[u8]>>,
    /// Which of `pieces` holds each range read.
    known: Mutex<HashMap<(usize, usize), usize>>,
    /// The whole file, once asked for.
    whole: OnceLock<std::result::Result<Vec<u8>, io::ErrorKind>>,
}

impl HeldFile {
    /// Opens the font file at `path` for drawing. A file that is no sfnt
    /// font, or whose tables leave it, is read whole.
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        let mut font = Font::open(path)?;
        let length = usize::try_from(font.length())
            .map_err(|_| io::Error::from(io::ErrorKind::FileTooLarge))?;
        let (held, elsewhere, cffs) = match font.faces()? {
            Some((header, faces)) => {
                let cffs = split_cffs(&mut font, &faces)?;
                let kept = |table: &Table| is_held(table, &cffs);
                let held = font.pack(header, &faces, kept)?;
                let elsewhere = faces
                    .iter()
                    .zip(0..)
                    .flat_map(|(face, index)| {
                        face.tables
                            .iter()
                            .filter(|table| !kept(table))
                            .filter_map(move |table| Some((index, table.tag(), table.range()?)))
                    })
                    .collect();
                (held, elsewhere, cffs)
            }
            None => (font.whole()?, Vec::new(), Vec::new()),
        };
        Ok(Self {
            path: path.to_path_buf(),
            held,
            elsewhere,
            cffs,
            file: Mutex::new(font.into_file()),
            length,
            pieces: Arena::new(),
            known: Mutex::new(HashMap::new()),
            whole: OnceLock::new(),
        })
    }
}

/// Whether `table` is held: it is no table drawing reads glyph by glyph
/// or not at all, and no CFF table of `split`.
fn is_held(table: &Table, split: &[SplitCff]) -> bool {
    let Some(range) = table.range() else {
        return true;
    };
    !DRAWN.contains(&table.tag()) && !split.iter().any(|cff| cff.table == range)
}

/// The CFF tables of `faces`, each once, held but for their `CharStrings`
/// data; those whose `CharStrings` cannot be found are left out (and held
/// whole).
fn split_cffs(font: &mut Font, faces: &[Face]) -> io::Result<Vec<SplitCff>> {
    let mut split: Vec<SplitCff> = Vec::new();
    for table in faces.iter().flat_map(|face| &face.tables) {
        let Some(range) = table.range().filter(|_| table.tag() == CFF) else {
            continue;
        };
        if split.iter().all(|cff| cff.table != range)
            && let Some(cff) = split_cff(font, range)?
        {
            split.push(cff);
        }
    }
    Ok(split)
}

/// The CFF table at `table` of the file, held but for its `CharStrings`
/// data; `None` when they cannot be found.
fn split_cff(font: &mut Font, table: Range<usize>) -> io::Result<Option<SplitCff>> {
    let mut read = |start: usize, len: usize| font.bytes(start as u64, len as u64);
    let Some(head) = read(table.start, table.len().min(CFF_HEAD))? else {
        return Ok(None);
    };
    let Some(at) = charstrings_offset(&head)
        .ok()
        .filter(|&at| at < table.len())
    else {
        return Ok(None);
    };
    let Some(index) = read(table.start + at, (table.len() - at).min(INDEX_HEAD))? else {
        return Ok(None);
    };
    let Ok(data) = Index::data_range(&index) else {
        return Ok(None);
    };
    let charstrings = at + data.start..at + data.end;
    if charstrings.end > table.len() {
        return Ok(None);
    }
    let head = read(table.start, charstrings.start)?;
    let tail = read(table.start + charstrings.end, table.len() - charstrings.end)?;
    Ok(head.zip(tail).map(|(head, tail)| SplitCff {
        table,
        charstrings,
        head,
        tail,
    }))
}

impl LazyFile for HeldFile {
    fn held(&self) -> &[u8] {
        &self.held
    }

    fn table(&self, face: u32, tag: Tag) -> Option<Range<usize>> {
        self.elsewhere
            .iter()
            .find(|(index, elsewhere, _)| *index == face && Tag(*elsewhere) == tag)
            .map(|(_, _, range)| range.clone())
    }

    fn cff(&self, face: u32) -> Option<CffParts<'_>> {
        let table = self.table(face, Tag(CFF))?;
        let split = self.cffs.iter().find(|cff| cff.table == table)?;
        let charstrings =
            table.start + split.charstrings.start..table.start + split.charstrings.end;
        Some(CffParts {
            head: &split.head,
            tail: &split.tail,
            charstrings: Region::lazy(self, charstrings),
        })
    }

    fn read(&self, range: Range<usize>) -> Option<&[u8]> {
        if range.start > range.end || range.end > self.length {
            return None;
        }
        if range.is_empty() {
            return Some(&[]);
        }
        let mut known = self.known.lock().unwrap_or_else(PoisonError::into_inner);
        let key = (range.start, range.end);
        let index = if let Some(&index) = known.get(&key) {
            index
        } else {
            let mut bytes = vec![0; range.len()];
            let mut file = self.file.lock().unwrap_or_else(PoisonError::into_inner);
            file.seek(SeekFrom::Start(range.start as u64)).ok()?;
            file.read_exact(&mut bytes).ok()?;
            let index = self.pieces.push(bytes.into_boxed_slice());
            known.insert(key, index);
            index
        };
        drop(known);
        self.pieces.get(index).map(|piece| &**piece)
    }

    fn whole(&self) -> Result<&[u8]> {
        self.whole
            .get_or_init(|| fs::read(&self.path).map_err(|error| error.kind()))
            .as_deref()
            .map_err(|&kind| FontError::Io {
                path: self.path.clone(),
                kind,
            })
    }
}

#[cfg(test)]
mod tests;
