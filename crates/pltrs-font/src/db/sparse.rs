//! Font files read for the faces they hold: every table the face parser
//! reads, and none of the outlines, bitmaps, variations and Apple
//! shaping tables, which only drawing reads and which make most of a
//! large font (190 of the 192 MB of macOS's Apple Color Emoji are `sbix`
//! bitmaps). The tables read are packed into a font of their own, whose
//! directories point at them, the others left empty; a face that draws
//! reads its file whole. Cataloguing the system's fonts so takes the
//! memory of their descriptions, not that of their files.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::Path;

/// The tables only drawing reads, and those it reads glyph by glyph or
/// not at all: outlines (`glyf` is only located), bitmaps, variations and
/// Apple's outlines.
pub(super) const DRAWN: [[u8; 4]; 10] = [
    *b"glyf", *b"sbix", *b"CBDT", *b"EBDT", *b"bdat", *b"SVG ", *b"hvgl", *b"gvar", *b"hdmx",
    *b"CFF2",
];

/// The tables neither the face parser nor the catalogue reads: those, and
/// Apple's shaping.
const UNREAD: [[u8; 4]; 11] = [
    DRAWN[0], DRAWN[1], DRAWN[2], DRAWN[3], DRAWN[4], DRAWN[5], DRAWN[6], DRAWN[7], DRAWN[8],
    DRAWN[9], *b"morx",
];

/// The tag of TrueType/OpenType collections.
const COLLECTION: [u8; 4] = *b"ttcf";

/// The tags of single sfnt faces: TrueType, Apple's TrueType, CFF.
const FACES: [[u8; 4]; 3] = [[0, 1, 0, 0], *b"true", *b"OTTO"];

/// The size of a collection's header before its face offsets.
const COLLECTION_HEADER: usize = 12;

/// The size of a face's offset table, before its table records.
const OFFSET_TABLE: usize = 12;

/// The size of a table record.
const TABLE_RECORD: usize = 16;

/// The font of the file at `path` for its faces' descriptions: its faces
/// with their tables but the [`UNREAD`] ones, which are empty. Files that
/// are no sfnt fonts, or whose tables leave the file, are read whole.
pub(super) fn read(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = Font::open(path)?;
    match file.faces()? {
        Some((header, faces)) => file.pack(header, &faces, Table::is_read),
        None => file.whole(),
    }
}

/// A table of a face: its tag and checksum, and where its bytes are in
/// the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Table {
    tag_and_checksum: [u8; 8],
    offset: u64,
    length: u32,
}

impl Table {
    /// Whether the face parser reads the table.
    fn is_read(&self) -> bool {
        !UNREAD.iter().any(|tag| self.tag() == *tag)
    }

    /// The table's tag.
    pub(super) fn tag(&self) -> [u8; 4] {
        let [tag @ .., _, _, _, _] = self.tag_and_checksum;
        tag
    }

    /// Where the table's bytes are in the file, if their offsets fit in
    /// memory's.
    pub(super) fn range(&self) -> Option<Range<usize>> {
        let start = usize::try_from(self.offset).ok()?;
        Some(start..start.checked_add(usize::try_from(self.length).ok()?)?)
    }
}

/// A face: its offset table and its tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Face {
    offset_table: [u8; OFFSET_TABLE],
    /// The tables, in directory order.
    pub(super) tables: Vec<Table>,
}

/// A font file being read.
#[derive(Debug)]
pub(super) struct Font {
    file: File,
    length: u64,
}

impl Font {
    /// The font file at `path`.
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let length = file.metadata()?.len();
        Ok(Self { file, length })
    }

    /// The whole file.
    pub(super) fn whole(&mut self) -> io::Result<Vec<u8>> {
        let mut whole = Vec::new();
        self.file.seek(SeekFrom::Start(0))?;
        self.file.read_to_end(&mut whole)?;
        Ok(whole)
    }

    /// The file's length.
    pub(super) const fn length(&self) -> u64 {
        self.length
    }

    /// The file handle, to read from later.
    pub(super) fn into_file(self) -> File {
        self.file
    }

    /// The `length` bytes at `offset`, if the file holds them.
    pub(super) fn bytes(&mut self, offset: u64, length: u64) -> io::Result<Option<Vec<u8>>> {
        if offset.saturating_add(length) > self.length {
            return Ok(None);
        }
        let mut bytes = vec![0; usize::try_from(length).map_err(|_| too_large())?];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut bytes)?;
        Ok(Some(bytes))
    }

    /// The first bytes of the file and its faces, if it is an sfnt font
    /// whose faces and tables lie in it.
    pub(super) fn faces(&mut self) -> io::Result<Option<([u8; 8], Vec<Face>)>> {
        let Some(header) = self.bytes(0, COLLECTION_HEADER as u64)? else {
            return Ok(None);
        };
        let tag = [header[0], header[1], header[2], header[3]];
        let offsets = if tag == COLLECTION {
            let count = u64::from(be_u32(&header[8..]));
            let Some(offsets) = self.bytes(COLLECTION_HEADER as u64, count * 4)? else {
                return Ok(None);
            };
            offsets
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&offset| u64::from(u32::from_be_bytes(offset)))
                .collect()
        } else if FACES.contains(&tag) {
            vec![0]
        } else {
            return Ok(None);
        };
        let mut faces = Vec::with_capacity(offsets.len());
        for offset in offsets {
            match self.face(offset)? {
                Some(face) => faces.push(face),
                None => return Ok(None),
            }
        }
        let mut first = [0; 8];
        first.copy_from_slice(&header[..8]);
        Ok(Some((first, faces)))
    }

    /// The face whose offset table is at `offset`, if it and its tables
    /// lie in the file.
    fn face(&mut self, offset: u64) -> io::Result<Option<Face>> {
        let Some(head) = self.bytes(offset, OFFSET_TABLE as u64)? else {
            return Ok(None);
        };
        let count = u64::from(u16::from_be_bytes([head[4], head[5]]));
        let at = offset + OFFSET_TABLE as u64;
        let Some(records) = self.bytes(at, count * TABLE_RECORD as u64)? else {
            return Ok(None);
        };
        let mut tables = Vec::new();
        for &record in records.as_chunks::<TABLE_RECORD>().0 {
            let [tag_and_checksum @ .., o0, o1, o2, o3, l0, l1, l2, l3] = record;
            let table = Table {
                tag_and_checksum,
                offset: u64::from(u32::from_be_bytes([o0, o1, o2, o3])),
                length: u32::from_be_bytes([l0, l1, l2, l3]),
            };
            if table.offset + u64::from(table.length) > self.length {
                return Ok(None);
            }
            tables.push(table);
        }
        let mut offset_table = [0; OFFSET_TABLE];
        offset_table.copy_from_slice(&head);
        Ok(Some(Face {
            offset_table,
            tables,
        }))
    }

    /// The font of `faces`: for several, the collection header (whose
    /// first bytes are `header`); their directories; then the bytes of
    /// the tables `kept`, each once, at four-byte boundaries. The others
    /// are empty.
    pub(super) fn pack(
        &mut self,
        header: [u8; 8],
        faces: &[Face],
        kept: impl Fn(&Table) -> bool,
    ) -> io::Result<Vec<u8>> {
        let collection = header[..4] == COLLECTION;
        let mut at = if collection {
            COLLECTION_HEADER + 4 * faces.len()
        } else {
            0
        };
        let mut directories = Vec::with_capacity(faces.len());
        for face in faces {
            directories.push(at);
            at += OFFSET_TABLE + TABLE_RECORD * face.tables.len();
        }
        // Where each table read goes, in the order the faces list them.
        let mut placed: Vec<(Table, usize)> = Vec::new();
        for table in faces.iter().flat_map(|face| &face.tables) {
            let same = |(other, _): &(Table, usize)| {
                (other.offset, other.length) == (table.offset, table.length)
            };
            if kept(table) && !placed.iter().any(same) {
                placed.push((*table, at));
                at = (at + table.length as usize).next_multiple_of(4);
            }
        }
        let mut bytes = Vec::with_capacity(at);
        if collection {
            bytes.extend_from_slice(&header);
            bytes.extend(offset(faces.len())?.to_be_bytes());
            for &directory in &directories {
                bytes.extend(offset(directory)?.to_be_bytes());
            }
        }
        for face in faces {
            bytes.extend_from_slice(&face.offset_table);
            for table in &face.tables {
                let (at, length) = placed
                    .iter()
                    .find(|(read, _)| {
                        kept(table) && (read.offset, read.length) == (table.offset, table.length)
                    })
                    .map_or((0, 0), |&(read, at)| (at, read.length));
                bytes.extend_from_slice(&table.tag_and_checksum);
                bytes.extend(offset(at)?.to_be_bytes());
                bytes.extend(length.to_be_bytes());
            }
        }
        for (table, at) in placed {
            bytes.resize(at, 0);
            bytes.resize(at + table.length as usize, 0);
            self.file.seek(SeekFrom::Start(table.offset))?;
            self.file.read_exact(&mut bytes[at..])?;
        }
        Ok(bytes)
    }
}

/// `at` as a 32-bit offset or count.
fn offset(at: usize) -> io::Result<u32> {
    u32::try_from(at).map_err(|_| too_large())
}

/// The error of a font too large to read.
fn too_large() -> io::Error {
    io::Error::from(io::ErrorKind::FileTooLarge)
}

/// The big-endian 32-bit number `bytes` start with.
fn be_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::db::{bundled_font_files, face_style};
    use crate::face::Face as ParsedFace;
    use crate::tables::directory::face_count;

    /// A file of `bytes` in the temporary directory, removed when dropped.
    struct TempFont(PathBuf);

    impl TempFont {
        fn new(name: &str, bytes: &[u8]) -> Self {
            let path = std::env::temp_dir()
                .join(format!("pltrs-font-sparse-{}-{name}", std::process::id()));
            fs::write(&path, bytes).unwrap();
            Self(path)
        }
    }

    impl Drop for TempFont {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    /// Checks that `read` gives the faces of `bytes`, described alike,
    /// with every table but the unread ones, empty.
    fn assert_reads(name: &str, bytes: &[u8]) -> Vec<u8> {
        let font = TempFont::new(name, bytes);
        let read = read(&font.0).unwrap();
        assert_eq!(face_count(&read).unwrap(), face_count(bytes).unwrap());
        for index in 0..face_count(bytes).unwrap() {
            let (whole, packed) = (
                ParsedFace::parse(bytes, index).unwrap(),
                ParsedFace::parse(&read, index).unwrap(),
            );
            assert_eq!(face_style(&whole), face_style(&packed), "{name}");
            assert!(whole.table_tags().eq(packed.table_tags()), "{name}");
            for tag in whole.table_tags() {
                let expected = if UNREAD.contains(&tag.0) {
                    Some(&[][..])
                } else {
                    whole.table(tag)
                };
                assert_eq!(packed.table(tag), expected, "{name}: {tag:?}");
            }
        }
        read
    }

    #[test]
    fn fonts_read_but_their_outlines_and_bitmaps() {
        for (name, bytes) in bundled_font_files().filter(|(_, bytes)| face_count(bytes).is_ok()) {
            let read = assert_reads(name, bytes);
            assert!(read.len() < bytes.len(), "{name}: no outlines");
        }
    }

    /// A collection of `faces`, their tables moved behind its header.
    fn collection(faces: &[&[u8]]) -> Vec<u8> {
        let header = COLLECTION_HEADER + 4 * faces.len();
        let mut bytes = [&COLLECTION[..], &[0, 1, 0, 0]].concat();
        bytes.extend((faces.len() as u32).to_be_bytes());
        let mut at = header;
        let mut bases = Vec::new();
        for face in faces {
            bases.push(at);
            bytes.extend((at as u32).to_be_bytes());
            at += face.len();
        }
        for (face, base) in faces.iter().zip(bases) {
            let mut moved = face.to_vec();
            let tables = usize::from(u16::from_be_bytes([face[4], face[5]]));
            for table in 0..tables {
                let field = OFFSET_TABLE + table * TABLE_RECORD + 8;
                let offset = u32::from_be_bytes(face[field..field + 4].try_into().unwrap());
                moved[field..field + 4].copy_from_slice(&(offset + base as u32).to_be_bytes());
            }
            bytes.extend(moved);
        }
        bytes
    }

    #[test]
    fn collections_read_every_face() {
        let faces = ["DejaVuSans.ttf", "DejaVuSerif-Bold.ttf"].map(|name| {
            bundled_font_files()
                .find(|(file, _)| *file == name)
                .unwrap()
                .1
        });
        let bytes = collection(&faces);
        assert_eq!(face_count(&bytes).unwrap(), 2);
        assert_reads("collection.ttc", &bytes);
    }

    #[test]
    fn shared_tables_are_read_once() {
        let face = bundled_font_files()
            .find(|(file, _)| *file == "DejaVuSans.ttf")
            .unwrap()
            .1;
        // Two faces of one directory: a header of 20 bytes, then the face.
        let mut bytes = [&COLLECTION[..], &[0, 1, 0, 0], &[0, 0, 0, 2]].concat();
        bytes.extend([20_u32, 20].iter().flat_map(|offset| offset.to_be_bytes()));
        let tables = usize::from(u16::from_be_bytes([face[4], face[5]]));
        let mut moved = face.to_vec();
        for table in 0..tables {
            let field = OFFSET_TABLE + table * TABLE_RECORD + 8;
            let offset = be_u32(&face[field..]) + 20;
            moved[field..field + 4].copy_from_slice(&offset.to_be_bytes());
        }
        bytes.extend(moved);
        let shared = assert_reads("shared.ttc", &bytes);
        let single = assert_reads("single.ttf", face);
        let directory = OFFSET_TABLE + TABLE_RECORD * tables;
        assert_eq!(shared.len(), single.len() + 20 + directory);
    }

    #[test]
    fn other_files_and_broken_fonts_read_whole() {
        let text = b"not a font at all, but long enough to have a header".repeat(3);
        let broken_table = [
            &b"OTTO"[..],
            &[0, 1][..],
            &[0; 6][..],
            &b"name"[..],
            &[0; 4][..],
            &[0, 0, 0, 28][..],
            &[255; 4][..],
        ]
        .concat();
        for (name, bytes) in [
            ("text.ttf", text),
            ("short.ttf", b"true".to_vec()),
            (
                "faces.ttc",
                [&COLLECTION[..], &[0, 1, 0, 0][..], &[255; 4][..]].concat(),
            ),
            ("table.otf", broken_table),
        ] {
            let file = TempFont::new(name, &bytes);
            assert_eq!(read(&file.0).unwrap(), bytes, "{name}");
        }
        assert!(read(Path::new("/nonexistent/font.ttf")).is_err());
    }
}
