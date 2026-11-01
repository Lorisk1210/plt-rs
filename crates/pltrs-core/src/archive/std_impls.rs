//! [`Archive`] for the standard library's types.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use super::{Archive, ArchiveError, ArchiveRc, Decoder, Encoder};

impl Archive for bool {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_u8(u8::from(*self));
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        match decoder.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            byte => Err(ArchiveError::invalid(format!("{byte} is no boolean"))),
        }
    }
}

/// [`Archive`] for unsigned integers, as varints.
macro_rules! archive_unsigned {
    ($($ty:ty),*) => {$(
        impl Archive for $ty {
            fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
                let value = u64::try_from(*self).map_err(|_| {
                    ArchiveError::unsupported(concat!("a ", stringify!($ty), " beyond 64 bits"))
                })?;
                encoder.write_varint(value);
                Ok(())
            }

            fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
                <$ty>::try_from(decoder.read_varint()?).map_err(|_| {
                    ArchiveError::invalid(concat!("integer out of range for ", stringify!($ty)))
                })
            }
        }
    )*};
}

archive_unsigned!(u8, u16, u32, u64, usize);

/// [`Archive`] for signed integers, as zigzag varints.
macro_rules! archive_signed {
    ($($ty:ty),*) => {$(
        impl Archive for $ty {
            fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
                let value = i64::try_from(*self).map_err(|_| {
                    ArchiveError::unsupported(concat!("an ", stringify!($ty), " beyond 64 bits"))
                })?;
                encoder.write_signed(value);
                Ok(())
            }

            fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
                <$ty>::try_from(decoder.read_signed()?).map_err(|_| {
                    ArchiveError::invalid(concat!("integer out of range for ", stringify!($ty)))
                })
            }
        }
    )*};
}

archive_signed!(i8, i16, i32, i64, isize);

impl Archive for f64 {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_f64(*self);
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decoder.read_f64()
    }
}

impl Archive for f32 {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_raw(&self.to_bits().to_le_bytes());
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let mut bits = [0; 4];
        bits.copy_from_slice(decoder.read_raw(4)?);
        Ok(f32::from_bits(u32::from_le_bytes(bits)))
    }
}

impl Archive for char {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_varint(u64::from(*self));
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        u32::try_from(decoder.read_varint()?)
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| ArchiveError::invalid("not a character"))
    }
}

impl Archive for String {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_str(self);
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decoder.read_str()
    }
}

/// Strings read back as `&'static str` are interned: each distinct string
/// is allocated once, for the life of the process.
impl Archive for &'static str {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_str(self);
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        Ok(crate::interned(&decoder.read_str()?))
    }
}

impl Archive for Cow<'static, str> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_str(self);
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decoder.read_str().map(Cow::Owned)
    }
}

impl Archive for PathBuf {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        let text = self
            .to_str()
            .ok_or_else(|| ArchiveError::unsupported("a path that is not UTF-8"))?;
        encoder.write_str(text);
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decoder.read_str().map(PathBuf::from)
    }
}

impl Archive for () {
    fn encode(&self, _encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        Ok(())
    }

    fn decode(_decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        Ok(())
    }
}

impl<T: ?Sized> Archive for PhantomData<T> {
    fn encode(&self, _encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        Ok(())
    }

    fn decode(_decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        Ok(PhantomData)
    }
}

impl<T: Archive> Archive for Option<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        match self {
            None => {
                encoder.write_u8(0);
                Ok(())
            }
            Some(value) => {
                encoder.write_u8(1);
                value.encode(encoder)
            }
        }
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        match decoder.read_u8()? {
            0 => Ok(None),
            1 => T::decode(decoder).map(Some),
            byte => Err(ArchiveError::invalid(format!("{byte} is no option tag"))),
        }
    }
}

impl<T: Archive, E: Archive> Archive for Result<T, E> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        match self {
            Ok(value) => {
                encoder.write_u8(0);
                value.encode(encoder)
            }
            Err(error) => {
                encoder.write_u8(1);
                error.encode(encoder)
            }
        }
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        match decoder.read_u8()? {
            0 => T::decode(decoder).map(Ok),
            1 => E::decode(decoder).map(Err),
            byte => Err(ArchiveError::invalid(format!("{byte} is no result tag"))),
        }
    }
}

impl<T: Archive> Archive for Box<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        (**self).encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decoder.nested(T::decode).map(Box::new)
    }
}

impl<T: Archive + 'static> ArchiveRc for T {
    fn encode_rc(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        self.encode(encoder)
    }

    fn decode_rc(decoder: &mut Decoder<'_>) -> Result<Rc<Self>, ArchiveError> {
        T::decode(decoder).map(Rc::new)
    }
}

impl ArchiveRc for str {
    fn encode_rc(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_str(self);
        Ok(())
    }

    fn decode_rc(decoder: &mut Decoder<'_>) -> Result<Rc<Self>, ArchiveError> {
        decoder.read_str().map(Rc::from)
    }
}

impl<T: Archive + 'static> ArchiveRc for [T] {
    fn encode_rc(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encode_items(self.iter(), self.len(), encoder)
    }

    fn decode_rc(decoder: &mut Decoder<'_>) -> Result<Rc<Self>, ArchiveError> {
        Vec::<T>::decode(decoder).map(Rc::from)
    }
}

impl<T: ?Sized + ArchiveRc> Archive for Rc<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.shared(self, |encoder| (**self).encode_rc(encoder))
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decoder.shared(T::decode_rc)
    }
}

impl<T: Archive + Send + Sync + 'static> Archive for Arc<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.shared_arc(self, |encoder| (**self).encode(encoder))
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decoder.shared_arc(|decoder| T::decode(decoder).map(Arc::new))
    }
}

impl<T: Archive> Archive for RefCell<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        self.borrow().encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        T::decode(decoder).map(RefCell::new)
    }
}

impl<T: Archive + Copy> Archive for Cell<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        self.get().encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        T::decode(decoder).map(Cell::new)
    }
}

/// Writes `len` items, the length first.
fn encode_items<'v, T: Archive + 'v>(
    items: impl Iterator<Item = &'v T>,
    len: usize,
    encoder: &mut Encoder<'_>,
) -> Result<(), ArchiveError> {
    encoder.write_len(len);
    items.into_iter().try_for_each(|item| item.encode(encoder))
}

/// Reads items written by [`encode_items`] into a collection.
fn decode_items<T: Archive, C: FromIterator<T>>(
    decoder: &mut Decoder<'_>,
) -> Result<C, ArchiveError> {
    let len = decoder.read_sequence_len()?;
    decoder.nested(|decoder| (0..len).map(|_| T::decode(decoder)).collect())
}

impl<T: Archive> Archive for Vec<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encode_items(self.iter(), self.len(), encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decode_items(decoder)
    }
}

impl<T: Archive> Archive for VecDeque<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encode_items(self.iter(), self.len(), encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decode_items(decoder)
    }
}

impl<T: Archive + Ord> Archive for BTreeSet<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encode_items(self.iter(), self.len(), encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decode_items(decoder)
    }
}

impl<T: Archive + Eq + Hash, S: BuildHasher + Default> Archive for HashSet<T, S> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encode_items(self.iter(), self.len(), encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decode_items(decoder)
    }
}

impl<K: Archive + Ord, V: Archive> Archive for BTreeMap<K, V> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_len(self.len());
        self.iter().try_for_each(|(key, value)| {
            key.encode(encoder)?;
            value.encode(encoder)
        })
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decode_items::<(K, V), _>(decoder)
    }
}

impl<K: Archive + Eq + Hash, V: Archive, S: BuildHasher + Default> Archive for HashMap<K, V, S> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        encoder.write_len(self.len());
        self.iter().try_for_each(|(key, value)| {
            key.encode(encoder)?;
            value.encode(encoder)
        })
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        decode_items::<(K, V), _>(decoder)
    }
}

impl<T: Archive, const N: usize> Archive for [T; N] {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        self.iter().try_for_each(|item| item.encode(encoder))
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let items: Vec<T> = (0..N)
            .map(|_| T::decode(decoder))
            .collect::<Result<_, _>>()?;
        items
            .try_into()
            .map_err(|_| ArchiveError::invalid("array of the wrong length"))
    }
}

impl<T: Archive> Archive for Range<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        self.start.encode(encoder)?;
        self.end.encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        Ok(T::decode(decoder)?..T::decode(decoder)?)
    }
}

/// [`Archive`] for tuples, element by element.
macro_rules! archive_tuple {
    ($($name:ident),+) => {
        impl<$($name: Archive),+> Archive for ($($name,)+) {
            #[allow(non_snake_case, reason = "the elements are named after their types")]
            fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
                let ($($name,)+) = self;
                $($name.encode(encoder)?;)+
                Ok(())
            }

            fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
                Ok(($($name::decode(decoder)?,)+))
            }
        }
    };
}

archive_tuple!(A);
archive_tuple!(A, B);
archive_tuple!(A, B, C);
archive_tuple!(A, B, C, D);
archive_tuple!(A, B, C, D, E);
archive_tuple!(A, B, C, D, E, F);
archive_tuple!(A, B, C, D, E, F, G);
archive_tuple!(A, B, C, D, E, F, G, H);
