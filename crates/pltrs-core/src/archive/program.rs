//! What the program using pltrs archives itself: its own kinds of trait
//! objects, its functions, and values the archive cannot hold.

use std::any::{Any, TypeId};
use std::fmt;
use std::rc::Rc;

use super::{Archive, ArchiveError, Decoder, Encoder};

/// A value of the program's that an archive refers to by index instead of
/// holding it (a Python object, for pltrs's Python package): the program
/// archives the list of externals beside the bytes, in its own way, and
/// hands it back to read them.
#[derive(Clone)]
pub struct External(Rc<dyn Any>);

impl External {
    /// Wraps `value`.
    pub fn new(value: impl Any) -> Self {
        Self(Rc::new(value))
    }

    /// The wrapped value, if it is a `T`.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    /// Whether `self` and `other` wrap the same value.
    pub(super) fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for External {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("External(..)")
    }
}

/// A trait whose objects an archive holds: its `dyn` type and its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindOf {
    /// The `TypeId` of `dyn Trait`.
    pub id: TypeId,
    /// The trait's name, for messages.
    pub name: &'static str,
}

impl KindOf {
    /// The kind of the trait objects of type `T` (`dyn Trait`), named
    /// `name`.
    pub fn of<T: ?Sized + 'static>(name: &'static str) -> Self {
        Self {
            id: TypeId::of::<T>(),
            name,
        }
    }
}

/// How the program using pltrs archives what pltrs cannot: the types it
/// implements pltrs's traits with, and the functions it gives pltrs. The
/// default methods archive nothing ([`NoProgram`]).
pub trait ProgramKinds {
    /// Writes `value`, a value of one of the program's types behind a
    /// `dyn` trait of kind `of`.
    fn encode_kind(
        &self,
        of: KindOf,
        _value: &dyn Any,
        _encoder: &mut Encoder<'_>,
    ) -> Result<(), ArchiveError> {
        Err(ArchiveError::unsupported(format!(
            "a {} of a type of the program's",
            of.name
        )))
    }

    /// Reads back a value written by [`encode_kind`](Self::encode_kind),
    /// as the `Box<dyn Trait>` of kind `of`, boxed again as `dyn Any`.
    fn decode_kind(
        &self,
        of: KindOf,
        _decoder: &mut Decoder<'_>,
    ) -> Result<Box<dyn Any>, ArchiveError> {
        Err(ArchiveError::invalid(format!(
            "a {} of a type of a program's, which this program cannot read",
            of.name
        )))
    }

    /// The function of type `function` (the `TypeId` of an `Rc<dyn Fn
    /// ...>`) made from `origin`, the external a [`ProgramFn`] was
    /// archived as, boxed as `dyn Any`.
    fn function(
        &self,
        _function: TypeId,
        _origin: &External,
    ) -> Result<Box<dyn Any>, ArchiveError> {
        Err(ArchiveError::invalid(
            "a function of a program's, which this program cannot rebuild",
        ))
    }
}

/// The program that archives nothing of its own (a pure Rust program).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoProgram;

impl ProgramKinds for NoProgram {}

/// A function the program gave pltrs (a closure, shared by clones),
/// archived as its origin: the external the program made it from (a
/// Python callable), from which [`ProgramKinds::function`] makes it again.
/// A function without an origin cannot be archived.
pub struct ProgramFn<F: ?Sized> {
    function: Rc<F>,
    origin: Option<External>,
}

impl<F: ?Sized> ProgramFn<F> {
    /// The function `function`, which cannot be archived.
    pub fn new(function: Rc<F>) -> Self {
        Self {
            function,
            origin: None,
        }
    }

    /// The function `function`, archived as `origin`.
    pub fn with_origin(function: Rc<F>, origin: External) -> Self {
        Self {
            function,
            origin: Some(origin),
        }
    }

    /// The function.
    pub fn get(&self) -> &F {
        &self.function
    }

    /// The external the function is archived as, if it has one.
    pub fn origin(&self) -> Option<&External> {
        self.origin.as_ref()
    }
}

impl<F: ?Sized> Clone for ProgramFn<F> {
    fn clone(&self) -> Self {
        Self {
            function: Rc::clone(&self.function),
            origin: self.origin.clone(),
        }
    }
}

impl<F: ?Sized> fmt::Debug for ProgramFn<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramFn")
            .field("archivable", &self.origin.is_some())
            .finish_non_exhaustive()
    }
}

impl<F: ?Sized + 'static> Archive for ProgramFn<F> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        let origin = self
            .origin
            .as_ref()
            .ok_or_else(|| ArchiveError::unsupported("a function given without an origin"))?;
        encoder.write_external(origin);
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let origin = decoder.read_external()?;
        let function = decoder.program_function::<Rc<F>>(&origin)?;
        Ok(Self::with_origin(function, origin))
    }
}
