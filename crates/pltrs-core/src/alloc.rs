//! Allocations a program's data can make enormous (a canvas, an image, an
//! encoded file), made so that running out of memory is an error rather
//! than the end of the process: where matplotlib raises `MemoryError`,
//! pltrs returns [`OutOfMemory`] (which pltrs (Python) raises as
//! `MemoryError`).
//!
//! ```
//! use pltrs_core::alloc::{OutOfMemory, try_with_capacity};
//! let buffer: Vec<u8> = try_with_capacity(16)?;
//! assert!(buffer.capacity() >= 16);
//! assert!(try_with_capacity::<u64>(usize::MAX).is_err());
//! # Ok::<(), OutOfMemory>(())
//! ```

use std::error::Error;
use std::fmt;

/// An allocation the system refused, or one larger than pltrs makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutOfMemory {
    /// The bytes asked for (`usize::MAX` when they do not fit a `usize`).
    pub bytes: usize,
}

impl OutOfMemory {
    /// The failure to allocate `count` values of `T`.
    pub fn of<T>(count: usize) -> Self {
        Self {
            bytes: count.saturating_mul(size_of::<T>()),
        }
    }
}

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot allocate {} bytes", self.bytes)
    }
}

impl Error for OutOfMemory {}

/// An empty vector with room for `capacity` values.
///
/// # Errors
///
/// When the memory cannot be allocated.
pub fn try_with_capacity<T>(capacity: usize) -> Result<Vec<T>, OutOfMemory> {
    let mut values = Vec::new();
    try_reserve(&mut values, capacity)?;
    Ok(values)
}

/// Makes room for `additional` more values in `values`.
///
/// # Errors
///
/// When the memory cannot be allocated.
pub fn try_reserve<T>(values: &mut Vec<T>, additional: usize) -> Result<(), OutOfMemory> {
    values
        .try_reserve(additional)
        .map_err(|_| OutOfMemory::of::<T>(additional))
}

/// A copy of `values`.
///
/// # Errors
///
/// When the memory cannot be allocated.
pub fn try_copy<T: Clone>(values: &[T]) -> Result<Vec<T>, OutOfMemory> {
    let mut copy = try_with_capacity(values.len())?;
    copy.extend_from_slice(values);
    Ok(copy)
}

/// Whether `err`, or an error it was caused by, is an [`OutOfMemory`].
///
/// ```
/// use pltrs_core::alloc::{OutOfMemory, caused_by_out_of_memory};
/// assert!(caused_by_out_of_memory(&OutOfMemory { bytes: 8 }));
/// assert!(!caused_by_out_of_memory(&std::fmt::Error));
/// ```
pub fn caused_by_out_of_memory(err: &(dyn Error + 'static)) -> bool {
    let mut cause = Some(err);
    while let Some(err) = cause {
        if err.is::<OutOfMemory>() {
            return true;
        }
        cause = err.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Wrapping(OutOfMemory);

    impl fmt::Display for Wrapping {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "wrapped: {}", self.0)
        }
    }

    impl Error for Wrapping {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn refused_allocations_are_errors() {
        let refused = try_with_capacity::<u32>(usize::MAX / 2);
        assert_eq!(refused, Err(OutOfMemory::of::<u32>(usize::MAX / 2)));
        assert_eq!(
            OutOfMemory::of::<u32>(usize::MAX).bytes,
            usize::MAX,
            "saturates"
        );
        let mut values = vec![1_u8];
        assert!(try_reserve(&mut values, usize::MAX).is_err());
        assert_eq!(try_copy(&[1, 2, 3]), Ok(vec![1, 2, 3]));
    }

    #[test]
    fn causes_are_followed() {
        let err = Wrapping(OutOfMemory { bytes: 1 });
        assert!(caused_by_out_of_memory(&err));
        assert_eq!(err.to_string(), "wrapped: cannot allocate 1 bytes");
    }
}
