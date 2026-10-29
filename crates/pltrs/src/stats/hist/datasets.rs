//! The datasets a histogram takes: one, or several side by side.

use crate::stats::BinSample;

/// The data of a histogram: one dataset or several, each a list of
/// samples of type `T` (matplotlib's `x`, an array or a sequence of
/// arrays). One dataset is a slice, a `Vec` or an array of samples;
/// several are a slice, a `Vec` or an array of datasets, each a `Vec`, an
/// array or a reference to a slice, a `Vec` or an array.
///
/// ```
/// use pltrs::stats::{Bins, HistOptions, hist};
/// let low = vec![0.0, 1.0, 1.5];
/// let high = vec![2.5, 3.0];
/// let options = || HistOptions { bins: Some(Bins::Count(3)), ..HistOptions::default() };
/// assert_eq!(hist(&low, options())?.tops.len(), 1, "one dataset");
/// assert_eq!(hist(&[&low, &high], options())?.tops.len(), 2, "two side by side");
/// assert_eq!(hist(&[3_u8, 1, 2], options())?.tops[0], [1.0, 1.0, 1.0]);
/// # Ok::<(), pltrs::stats::StatsError>(())
/// ```
pub trait Datasets<T: BinSample> {
    /// The samples of each dataset.
    fn datasets(&self) -> Vec<&[T]>;
}

impl<T: BinSample> Datasets<T> for [T] {
    fn datasets(&self) -> Vec<&[T]> {
        vec![self]
    }
}

impl<T: BinSample> Datasets<T> for Vec<T> {
    fn datasets(&self) -> Vec<&[T]> {
        vec![self]
    }
}

impl<T: BinSample, const N: usize> Datasets<T> for [T; N] {
    fn datasets(&self) -> Vec<&[T]> {
        vec![self]
    }
}

/// Implements [`Datasets`] for slices, `Vec`s and arrays of each kind of
/// dataset, given after the generic parameters it needs besides `T`:
/// several datasets, side by side.
macro_rules! several {
    ($([$($generics:tt)*] $dataset:ty),+ $(,)?) => {$(
        impl<$($generics)* T: BinSample> Datasets<T> for [$dataset] {
            fn datasets(&self) -> Vec<&[T]> {
                self.iter().map(|dataset| dataset.as_ref()).collect()
            }
        }

        impl<$($generics)* T: BinSample> Datasets<T> for Vec<$dataset> {
            fn datasets(&self) -> Vec<&[T]> {
                self.as_slice().datasets()
            }
        }

        impl<$($generics)* T: BinSample, const N: usize> Datasets<T> for [$dataset; N] {
            fn datasets(&self) -> Vec<&[T]> {
                self.as_slice().datasets()
            }
        }
    )+};
}

several!(
    [] Vec<T>,
    ['a,] &'a [T],
    ['a,] &'a Vec<T>,
    [const M: usize,] [T; M],
    ['a, const M: usize,] &'a [T; M],
);
