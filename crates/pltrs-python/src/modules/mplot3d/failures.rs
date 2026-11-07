//! The problems a 3-D axes meets drawing its artists, recorded in advance
//! ([`Axes3D::set_draw_failure`]): matplotlib accepts data its 3-D artists
//! cannot project, and 2-D collections and patches without 3-D
//! projections, and raises only when its draw projects or draws them. The
//! package computes the exception matplotlib's draw would raise and
//! records it, as the name of its class and its message.

use pltrs::artist::{DrawErrorKind, RawHandle};
use pltrs::mplot3d::{Axes3D, DrawFailure};

use crate::binding::prelude::*;

/// The kind of draw problem an exception of the class `name` stands for
/// (the inverse of [`draw_exception`](super::super::figure::draw_exception)).
fn kind_of(name: &str) -> DrawErrorKind {
    match name {
        "ValueError" => DrawErrorKind::InvalidValue,
        "TypeError" => DrawErrorKind::InvalidType,
        "OverflowError" => DrawErrorKind::Overflow,
        "NotImplementedError" => DrawErrorKind::Unsupported,
        "IndexError" => DrawErrorKind::OutOfRange,
        "AttributeError" => DrawErrorKind::MissingAttribute,
        "MemoryError" => DrawErrorKind::OutOfMemory,
        "FileNotFoundError" => DrawErrorKind::FileNotFound,
        _ => DrawErrorKind::Other,
    }
}

/// Records that drawing `artist` raises the exception of the class named
/// first with the message second; `None` forgets a problem recorded.
fn set_draw_failure(
    axes: Axes3D,
    artist: RawHandle,
    failure: Option<(String, String)>,
) -> PyResult<()> {
    let failure = failure.map(|(class, message)| DrawFailure {
        kind: kind_of(&class),
        message,
    });
    axes.set_draw_failure(&artist, failure)?;
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_draw_failure", set_draw_failure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::figure::draw_exception;

    #[test]
    fn exception_classes_name_their_kinds_both_ways() {
        for name in [
            "ValueError",
            "TypeError",
            "OverflowError",
            "NotImplementedError",
            "IndexError",
            "AttributeError",
            "MemoryError",
            "FileNotFoundError",
        ] {
            let kind = draw_exception(kind_of(name)).expect("a class of its own");
            assert_eq!(format!("{kind:?}"), name);
        }
        assert_eq!(kind_of("KeyError"), DrawErrorKind::Other);
    }
}
