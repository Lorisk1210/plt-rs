//! The primitive number types plotting takes, and how each becomes the
//! `f64` plt-rs computes in.

/// A number plotting takes: a primitive integer or float of any width,
/// or a reference to one. Data arguments that are plain numbers take any
/// collection or iterator of them (`impl IntoIterator<Item: Number>`), so
/// counts in a `Vec<u32>` or values computed by an iterator go in as they
/// are.
///
/// Numbers convert as numpy converts them: exactly, except 64-bit and
/// pointer-sized integers beyond 2^53, which round to the nearest `f64`.
///
/// ```
/// use plt::prelude::*;
/// let ax = Figure::new().subplot();
/// let counts: Vec<u32> = vec![12, 7, 9];
/// let wedges = ax.pie(&counts, Default::default())?;
/// assert_eq!(wedges.wedges.len(), 3);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait Number: Copy {
    /// The number as an `f64`.
    fn to_f64(self) -> f64;
}

impl<N: Number> Number for &N {
    fn to_f64(self) -> f64 {
        (*self).to_f64()
    }
}

impl Number for f64 {
    fn to_f64(self) -> f64 {
        self
    }
}

/// Implements [`Number`] for types that convert to `f64` exactly.
macro_rules! exact_numbers {
    ($($number:ty),+) => {$(
        impl Number for $number {
            fn to_f64(self) -> f64 {
                f64::from(self)
            }
        }
    )+};
}

/// Implements [`Number`] for types whose large values round to the
/// nearest `f64`.
macro_rules! rounded_numbers {
    ($($number:ty),+) => {$(
        impl Number for $number {
            fn to_f64(self) -> f64 {
                self as f64
            }
        }
    )+};
}

exact_numbers!(f32, i8, i16, i32, u8, u16, u32);
rounded_numbers!(i64, u64, isize, usize);

/// The numbers `values` holds, as `f64`s: how methods taking
/// `impl IntoIterator<Item: Number>` read them.
pub(crate) fn to_f64s(values: impl IntoIterator<Item: Number>) -> Vec<f64> {
    values.into_iter().map(Number::to_f64).collect()
}

/// Calls `$callback!` with what `$args` gives, followed by every
/// primitive [`Number`] type, so that the conversions from numbers list
/// the types once.
macro_rules! with_numbers {
    ($callback:ident!($($args:tt)*)) => {
        $callback!($($args)* f64, f32, i8, i16, i32, i64, u8, u16, u32, u64, usize, isize);
    };
}

pub(crate) use with_numbers;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_convert_as_numpy_converts_them() {
        assert_eq!(0.5_f32.to_f64(), 0.5);
        assert_eq!((-3_i8).to_f64(), -3.0);
        assert_eq!(u32::MAX.to_f64(), 4_294_967_295.0);
        assert_eq!(
            (1_u64 << 53 | 1).to_f64(),
            9_007_199_254_740_992.0,
            "to the nearest"
        );
        assert_eq!((&7_usize).to_f64(), 7.0);
        assert_eq!(to_f64s(vec![1_i32, 2]), [1.0, 2.0]);
        assert_eq!(to_f64s([0.25, 0.5]), [0.25, 0.5]);
    }
}
