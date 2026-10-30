//! Writing archives.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use super::{Archive, ArchiveError, External, KindOf, ProgramKinds};

/// Writes values to an archive (see [`Archive::encode`]).
pub struct Encoder<'a> {
    bytes: Vec<u8>,
    /// The index of each shared value written, by address and type.
    shared: HashMap<(usize, TypeId), u64>,
    /// The shared values written, kept alive so that their addresses are
    /// not reused by another value while the archive is written.
    alive: Vec<Box<dyn Any>>,
    externals: Vec<External>,
    program: &'a dyn ProgramKinds,
}

impl std::fmt::Debug for Encoder<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Encoder")
            .field("len", &self.bytes.len())
            .field("shared", &self.shared.len())
            .field("externals", &self.externals.len())
            .finish_non_exhaustive()
    }
}

impl<'a> Encoder<'a> {
    /// An empty encoder; `program` archives the program's own kinds.
    pub(super) fn new(program: &'a dyn ProgramKinds) -> Self {
        Self {
            bytes: Vec::new(),
            shared: HashMap::new(),
            alive: Vec::new(),
            externals: Vec::new(),
            program,
        }
    }

    /// The bytes written and the externals they refer to.
    pub(super) fn finish(self) -> (Vec<u8>, Vec<External>) {
        (self.bytes, self.externals)
    }

    /// Writes `bytes` as they are.
    pub fn write_raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    /// Writes one byte.
    pub fn write_u8(&mut self, byte: u8) {
        self.bytes.push(byte);
    }

    /// Writes an unsigned integer as a LEB128 varint.
    pub fn write_varint(&mut self, mut value: u64) {
        loop {
            let low = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                self.bytes.push(low);
                return;
            }
            self.bytes.push(low | 0x80);
        }
    }

    /// Writes a signed integer, zigzag-encoded as a varint.
    pub fn write_signed(&mut self, value: i64) {
        self.write_varint(((value << 1) ^ (value >> 63)) as u64);
    }

    /// Writes a length or an index.
    pub fn write_len(&mut self, len: usize) {
        self.write_varint(len as u64);
    }

    /// Writes a float exactly (its bits).
    pub fn write_f64(&mut self, value: f64) {
        self.bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }

    /// Writes a string, its length first.
    pub fn write_str(&mut self, text: &str) {
        self.write_len(text.len());
        self.write_raw(text.as_bytes());
    }

    /// Writes the variant `name` of an enum whose variants are `names`,
    /// as its index.
    pub fn write_variant(&mut self, names: &[&str], name: &str) {
        let index = names
            .iter()
            .position(|candidate| *candidate == name)
            .expect("the variant is one of the enum's variants");
        self.write_len(index);
    }

    /// Writes the shared value `rc`: in full, with `contents`, the first
    /// time, and as a reference to that the next times, so that it reads
    /// back shared (see [`Decoder::shared`](super::Decoder::shared)).
    pub fn shared<T: ?Sized + 'static>(
        &mut self,
        rc: &Rc<T>,
        contents: impl FnOnce(&mut Self) -> Result<(), ArchiveError>,
    ) -> Result<(), ArchiveError> {
        self.shared_pointer(Rc::as_ptr(rc).cast::<()>().addr(), Rc::clone(rc), contents)
    }

    /// Writes the shared value `arc` as [`Encoder::shared`] writes an
    /// `Rc` (see [`Decoder::shared_arc`](super::Decoder::shared_arc)).
    pub fn shared_arc<T: ?Sized + Send + Sync + 'static>(
        &mut self,
        arc: &Arc<T>,
        contents: impl FnOnce(&mut Self) -> Result<(), ArchiveError>,
    ) -> Result<(), ArchiveError> {
        self.shared_pointer(
            Arc::as_ptr(arc).cast::<()>().addr(),
            Arc::clone(arc),
            contents,
        )
    }

    /// Writes the value the pointer `pointer` (an `Rc` or `Arc`) at
    /// `address` shares.
    fn shared_pointer<P: 'static>(
        &mut self,
        address: usize,
        pointer: P,
        contents: impl FnOnce(&mut Self) -> Result<(), ArchiveError>,
    ) -> Result<(), ArchiveError> {
        let key = (address, TypeId::of::<P>());
        if let Some(&index) = self.shared.get(&key) {
            self.write_varint(index + 1);
            return Ok(());
        }
        self.shared.insert(key, self.alive.len() as u64);
        self.alive.push(Box::new(pointer));
        self.write_varint(0);
        contents(self)
    }

    /// Writes the external `value` as its index in the list of externals
    /// (each external is listed once).
    pub fn write_external(&mut self, value: &External) {
        let known = self.externals.iter().position(|known| known.same(value));
        let index = known.unwrap_or_else(|| {
            self.externals.push(value.clone());
            self.externals.len() - 1
        });
        self.write_len(index);
    }

    /// Writes `value`, of a type of the program's behind a `dyn` trait of
    /// kind `of`, through the program's [`ProgramKinds`].
    pub fn write_program_kind(&mut self, of: KindOf, value: &dyn Any) -> Result<(), ArchiveError> {
        let program = self.program;
        program.encode_kind(of, value, self)
    }

    /// Writes `value` (shorthand for [`Archive::encode`]).
    pub fn write<T: Archive>(&mut self, value: &T) -> Result<(), ArchiveError> {
        value.encode(self)
    }
}
