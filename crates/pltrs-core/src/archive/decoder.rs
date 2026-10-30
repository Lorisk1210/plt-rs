//! Reading archives.

use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

use super::{Archive, ArchiveError, External, KindOf, ProgramKinds};

/// How deeply values may nest in an archive (boxes, shared values and
/// sequences inside one another) before reading refuses it.
pub const MAX_DEPTH: usize = 256;

/// Reads values from an archive (see [`Archive::decode`]).
pub struct Decoder<'a> {
    bytes: &'a [u8],
    position: usize,
    /// The shared values read so far, by index; `None` while one is being
    /// read.
    shared: Vec<Option<Box<dyn Any>>>,
    externals: &'a [External],
    program: &'a dyn ProgramKinds,
    depth: usize,
}

impl std::fmt::Debug for Decoder<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Decoder")
            .field("position", &self.position)
            .field("len", &self.bytes.len())
            .field("depth", &self.depth)
            .finish_non_exhaustive()
    }
}

impl<'a> Decoder<'a> {
    /// A decoder of `bytes`, whose externals are `externals`; `program`
    /// reads back the program's own kinds.
    pub(super) fn new(
        bytes: &'a [u8],
        externals: &'a [External],
        program: &'a dyn ProgramKinds,
    ) -> Self {
        Self {
            bytes,
            position: 0,
            shared: Vec::new(),
            externals,
            program,
            depth: 0,
        }
    }

    /// Checks that every byte was read.
    pub(super) fn finish(&self) -> Result<(), ArchiveError> {
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err(ArchiveError::invalid(format!(
                "{} bytes left over",
                self.bytes.len() - self.position
            )))
        }
    }

    /// How many bytes are left to read.
    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    /// Reads the next `len` bytes as they are.
    pub fn read_raw(&mut self, len: usize) -> Result<&'a [u8], ArchiveError> {
        let end = self
            .position
            .checked_add(len)
            .filter(|&end| end <= self.bytes.len())
            .ok_or(ArchiveError::UnexpectedEnd)?;
        let bytes = &self.bytes[self.position..end];
        self.position = end;
        Ok(bytes)
    }

    /// Reads one byte.
    pub fn read_u8(&mut self) -> Result<u8, ArchiveError> {
        Ok(self.read_raw(1)?[0])
    }

    /// Reads a LEB128 varint.
    pub fn read_varint(&mut self) -> Result<u64, ArchiveError> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = self.read_u8()?;
            let bits = u64::from(byte & 0x7f);
            if shift == 63 && bits > 1 {
                break;
            }
            value |= bits << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ArchiveError::invalid("varint longer than 64 bits"))
    }

    /// Reads a zigzag-encoded signed integer.
    pub fn read_signed(&mut self) -> Result<i64, ArchiveError> {
        let value = self.read_varint()?;
        Ok((value >> 1) as i64 ^ -((value & 1) as i64))
    }

    /// Reads a length or an index.
    pub fn read_len(&mut self) -> Result<usize, ArchiveError> {
        usize::try_from(self.read_varint()?).map_err(|_| ArchiveError::invalid("length overflow"))
    }

    /// Reads the length of a sequence, each of whose elements takes at
    /// least one byte, refusing lengths the remaining bytes cannot hold.
    pub fn read_sequence_len(&mut self) -> Result<usize, ArchiveError> {
        let len = self.read_len()?;
        if len > self.remaining() {
            return Err(ArchiveError::UnexpectedEnd);
        }
        Ok(len)
    }

    /// Reads a float written by [`Encoder::write_f64`](super::Encoder::write_f64).
    pub fn read_f64(&mut self) -> Result<f64, ArchiveError> {
        let bytes = self.read_raw(8)?;
        let mut bits = [0; 8];
        bits.copy_from_slice(bytes);
        Ok(f64::from_bits(u64::from_le_bytes(bits)))
    }

    /// Reads a string.
    pub fn read_str(&mut self) -> Result<String, ArchiveError> {
        let len = self.read_len()?;
        let bytes = self.read_raw(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ArchiveError::invalid("string not UTF-8"))
    }

    /// Reads the variant of an enum whose variants are `names`, returning
    /// its name.
    pub fn read_variant(&mut self, names: &[&'static str]) -> Result<&'static str, ArchiveError> {
        let index = self.read_len()?;
        names
            .get(index)
            .copied()
            .ok_or_else(|| ArchiveError::invalid(format!("variant {index} out of range")))
    }

    /// Runs `read` one nesting level deeper, refusing to go deeper than
    /// [`MAX_DEPTH`].
    pub fn nested<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T, ArchiveError>,
    ) -> Result<T, ArchiveError> {
        if self.depth >= MAX_DEPTH {
            return Err(ArchiveError::TooDeep);
        }
        self.depth += 1;
        let result = read(self);
        self.depth -= 1;
        result
    }

    /// Reads a shared value written by
    /// [`Encoder::shared`](super::Encoder::shared): `contents` reads it
    /// the first time, later references give the same `Rc`.
    pub fn shared<T: ?Sized + 'static>(
        &mut self,
        contents: impl FnOnce(&mut Self) -> Result<Rc<T>, ArchiveError>,
    ) -> Result<Rc<T>, ArchiveError> {
        self.shared_pointer(contents)
    }

    /// Reads a shared value written by
    /// [`Encoder::shared_arc`](super::Encoder::shared_arc), as
    /// [`Decoder::shared`] reads an `Rc`.
    pub fn shared_arc<T: ?Sized + Send + Sync + 'static>(
        &mut self,
        contents: impl FnOnce(&mut Self) -> Result<Arc<T>, ArchiveError>,
    ) -> Result<Arc<T>, ArchiveError> {
        self.shared_pointer(contents)
    }

    /// Reads the value a pointer `P` (an `Rc` or `Arc`) shares.
    fn shared_pointer<P: Clone + 'static>(
        &mut self,
        contents: impl FnOnce(&mut Self) -> Result<P, ArchiveError>,
    ) -> Result<P, ArchiveError> {
        let tag = self.read_varint()?;
        if tag == 0 {
            let index = self.shared.len();
            self.shared.push(None);
            let pointer = self.nested(contents)?;
            self.shared[index] = Some(Box::new(pointer.clone()));
            return Ok(pointer);
        }
        let index = usize::try_from(tag - 1)
            .map_err(|_| ArchiveError::invalid("shared value index overflow"))?;
        match self.shared.get(index) {
            Some(Some(known)) => known
                .downcast_ref::<P>()
                .cloned()
                .ok_or_else(|| ArchiveError::invalid("shared value of another type")),
            Some(None) => Err(ArchiveError::invalid("shared value refers to itself")),
            None => Err(ArchiveError::invalid(format!(
                "reference to unknown shared value {index}"
            ))),
        }
    }

    /// Reads an external written by
    /// [`Encoder::write_external`](super::Encoder::write_external).
    pub fn read_external(&mut self) -> Result<External, ArchiveError> {
        let index = self.read_len()?;
        self.externals
            .get(index)
            .cloned()
            .ok_or_else(|| ArchiveError::invalid(format!("external {index} out of range")))
    }

    /// Reads a value of the program's behind a `dyn` trait of kind `of`,
    /// as `T` (the `Box<dyn Trait>`), through the program's
    /// [`ProgramKinds`].
    pub fn read_program_kind<T: Any>(&mut self, of: KindOf) -> Result<T, ArchiveError> {
        let program = self.program;
        let value = program.decode_kind(of, self)?;
        value.downcast::<T>().map(|value| *value).map_err(|_| {
            ArchiveError::invalid(format!("the program read back another kind of {}", of.name))
        })
    }

    /// The function the program rebuilds from `origin` (see
    /// [`ProgramFn`](super::ProgramFn)), as `T` (an `Rc<dyn Fn ...>`).
    pub fn program_function<T: Any>(&self, origin: &External) -> Result<T, ArchiveError> {
        let value = self.program.function(std::any::TypeId::of::<T>(), origin)?;
        value
            .downcast::<T>()
            .map(|value| *value)
            .map_err(|_| ArchiveError::invalid("the program rebuilt another kind of function"))
    }

    /// Reads a value (shorthand for [`Archive::decode`]).
    pub fn read<T: Archive>(&mut self) -> Result<T, ArchiveError> {
        T::decode(self)
    }
}
