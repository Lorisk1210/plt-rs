//! The macros implementing [`Archive`](super::Archive) for plain types and
//! trait objects.

/// Implements [`Archive`](crate::archive::Archive) for a struct by
/// archiving its fields in the order listed, which must name all of them
/// (the compiler checks that).
///
/// ```text
/// archive_struct!(Name { field, other });        // named fields
/// archive_struct!(Name { field } skip { cache }); // `cache` reads back as its Default
/// archive_struct!(Name { field } check Name::valid); // read back only if `valid` accepts it
/// archive_struct!(Name(first, second));          // a tuple struct (names bind the fields)
/// archive_struct!(Name);                         // a unit struct
/// archive_struct!(Name<T: Bound> { field });     // generic, with T: Archive + Bound
/// ```
///
/// A `check` names a function `fn(&Name) -> Result<(), ArchiveError>`
/// telling whether a value read back keeps the invariants its
/// constructors keep (a grid cell inside its grid): a corrupted archive
/// may hold any value of every field.
///
/// Invoke it where the fields are visible, next to the struct.
#[macro_export]
macro_rules! archive_struct {
    ($name:ident $(< $($generic:ident $(: $bound:path)?),+ >)? { $($field:ident),* $(,)? }
        $(skip { $($skipped:ident),* $(,)? })? $(check $check:path)?) => {
        impl $(< $($generic: $crate::archive::Archive $(+ $bound)?),+ >)? $crate::archive::Archive
            for $name $(< $($generic),+ >)?
        {
            fn encode(
                &self,
                encoder: &mut $crate::archive::Encoder<'_>,
            ) -> ::std::result::Result<(), $crate::archive::ArchiveError> {
                let Self { $($field,)* $($($skipped: _,)*)? } = self;
                $($crate::archive::Archive::encode($field, encoder)?;)*
                ::std::result::Result::Ok(())
            }

            fn decode(
                decoder: &mut $crate::archive::Decoder<'_>,
            ) -> ::std::result::Result<Self, $crate::archive::ArchiveError> {
                $(let $field = $crate::archive::Archive::decode(decoder)?;)*
                let value = Self {
                    $($field,)*
                    $($($skipped: ::std::default::Default::default(),)*)?
                };
                $($check(&value)?;)?
                ::std::result::Result::Ok(value)
            }
        }
    };
    ($name:ident $(< $($generic:ident $(: $bound:path)?),+ >)? ( $($field:ident),* $(,)? )) => {
        impl $(< $($generic: $crate::archive::Archive $(+ $bound)?),+ >)? $crate::archive::Archive
            for $name $(< $($generic),+ >)?
        {
            fn encode(
                &self,
                encoder: &mut $crate::archive::Encoder<'_>,
            ) -> ::std::result::Result<(), $crate::archive::ArchiveError> {
                let Self($($field),*) = self;
                $($crate::archive::Archive::encode($field, encoder)?;)*
                ::std::result::Result::Ok(())
            }

            fn decode(
                decoder: &mut $crate::archive::Decoder<'_>,
            ) -> ::std::result::Result<Self, $crate::archive::ArchiveError> {
                $(let $field = $crate::archive::Archive::decode(decoder)?;)*
                ::std::result::Result::Ok(Self($($field),*))
            }
        }
    };
    ($name:ident) => {
        impl $crate::archive::Archive for $name {
            fn encode(
                &self,
                _encoder: &mut $crate::archive::Encoder<'_>,
            ) -> ::std::result::Result<(), $crate::archive::ArchiveError> {
                ::std::result::Result::Ok(())
            }

            fn decode(
                _decoder: &mut $crate::archive::Decoder<'_>,
            ) -> ::std::result::Result<Self, $crate::archive::ArchiveError> {
                ::std::result::Result::Ok(Self)
            }
        }
    };
}

/// Implements [`Archive`](crate::archive::Archive) for an enum: the
/// variant's index, then its fields. The variants must all be listed (the
/// compiler checks that); tuple variants name their fields to bind them.
///
/// ```text
/// archive_enum!(Name { Unit, Tuple(first, second), Struct { field } });
/// archive_enum!(Name<T: Bound> { ... });         // generic, with T: Archive + Bound
/// ```
#[macro_export]
macro_rules! archive_enum {
    ($name:ident $(< $($generic:ident $(: $bound:path)?),+ >)? {
        $($variant:ident $(( $($tuple:ident),* $(,)? ))? $({ $($field:ident),* $(,)? })?),*
        $(,)?
    }) => {
        impl $(< $($generic: $crate::archive::Archive $(+ $bound)?),+ >)? $crate::archive::Archive
            for $name $(< $($generic),+ >)?
        {
            fn encode(
                &self,
                encoder: &mut $crate::archive::Encoder<'_>,
            ) -> ::std::result::Result<(), $crate::archive::ArchiveError> {
                const NAMES: &[&str] = &[$(stringify!($variant)),*];
                match self {
                    $(Self::$variant $(( $($tuple),* ))? $({ $($field),* })? => {
                        encoder.write_variant(NAMES, stringify!($variant));
                        $($($crate::archive::Archive::encode($tuple, encoder)?;)*)?
                        $($($crate::archive::Archive::encode($field, encoder)?;)*)?
                    })*
                }
                ::std::result::Result::Ok(())
            }

            fn decode(
                decoder: &mut $crate::archive::Decoder<'_>,
            ) -> ::std::result::Result<Self, $crate::archive::ArchiveError> {
                const NAMES: &[&str] = &[$(stringify!($variant)),*];
                let variant = decoder.read_variant(NAMES)?;
                $(if variant == stringify!($variant) {
                    $($(let $tuple = $crate::archive::Archive::decode(decoder)?;)*)?
                    $($(let $field = $crate::archive::Archive::decode(decoder)?;)*)?
                    return ::std::result::Result::Ok(
                        Self::$variant $(( $($tuple),* ))? $({ $($field),* })?
                    );
                })*
                ::std::result::Result::Err($crate::archive::ArchiveError::invalid(
                    concat!("unknown variant of ", stringify!($name)),
                ))
            }
        }
    };
}

/// Implements [`Archive`](crate::archive::Archive) for `Box<dyn Trait>`,
/// and [`ArchiveRc`](crate::archive::ArchiveRc) for `dyn Trait`, so that
/// `Rc<dyn Trait>` is archived too (shared, see
/// [`Encoder::shared`](crate::archive::Encoder::shared)): the tag of the
/// concrete type, then the value. `Trait` must have `Any` as a supertrait,
/// and every type listed must implement `Archive`; a type of the
/// program's is archived by its
/// [`ProgramKinds`](crate::archive::ProgramKinds).
///
/// ```text
/// archive_kinds!(dyn Locator {
///     "null" => NullLocator,
///     "auto" => AutoLocator,
/// });
/// ```
#[macro_export]
macro_rules! archive_kinds {
    (dyn $trait:ident { $($tag:literal => $kind:ty),* $(,)? }) => {
        impl $crate::archive::Archive for ::std::boxed::Box<dyn $trait> {
            fn encode(
                &self,
                encoder: &mut $crate::archive::Encoder<'_>,
            ) -> ::std::result::Result<(), $crate::archive::ArchiveError> {
                $crate::archive::ArchiveRc::encode_rc(&**self, encoder)
            }

            fn decode(
                decoder: &mut $crate::archive::Decoder<'_>,
            ) -> ::std::result::Result<Self, $crate::archive::ArchiveError> {
                let tag = decoder.read_str()?;
                $(if tag == $tag {
                    let value: $kind = decoder.nested($crate::archive::Archive::decode)?;
                    return ::std::result::Result::Ok(::std::boxed::Box::new(value));
                })*
                if tag.is_empty() {
                    return decoder.read_program_kind::<Self>(
                        $crate::archive::KindOf::of::<dyn $trait>(stringify!($trait)),
                    );
                }
                ::std::result::Result::Err($crate::archive::ArchiveError::invalid(format!(
                    "unknown kind {tag:?} of {}",
                    stringify!($trait),
                )))
            }
        }

        impl $crate::archive::ArchiveRc for dyn $trait {
            fn encode_rc(
                &self,
                encoder: &mut $crate::archive::Encoder<'_>,
            ) -> ::std::result::Result<(), $crate::archive::ArchiveError> {
                let any: &dyn ::std::any::Any = self;
                $(if let ::std::option::Option::Some(value) = any.downcast_ref::<$kind>() {
                    encoder.write_str($tag);
                    return $crate::archive::Archive::encode(value, encoder);
                })*
                encoder.write_str("");
                encoder.write_program_kind(
                    $crate::archive::KindOf::of::<dyn $trait>(stringify!($trait)),
                    any,
                )
            }

            fn decode_rc(
                decoder: &mut $crate::archive::Decoder<'_>,
            ) -> ::std::result::Result<::std::rc::Rc<Self>, $crate::archive::ArchiveError> {
                <::std::boxed::Box<dyn $trait> as $crate::archive::Archive>::decode(decoder)
                    .map(::std::rc::Rc::from)
            }
        }
    };
}
