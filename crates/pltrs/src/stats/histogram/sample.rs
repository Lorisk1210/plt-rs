//! Element types a histogram accepts, with numpy's dtype-dependent rules.

/// A sample value numpy can bin.
///
/// numpy treats integer (and boolean) arrays differently from float ones
/// in one respect: automatic bin-width estimators never produce bins
/// narrower than one for integer data. Implementors report which kind they
/// are through [`BinSample::INTEGER`].
pub trait BinSample: Copy {
    /// `true` for integer and boolean types.
    const INTEGER: bool;

    /// The sample as a float, as numpy converts it for binning.
    fn to_f64(self) -> f64;
}

macro_rules! bin_samples {
    (@impl $integer:literal, $t:ty, |$v:ident| $convert:expr) => {
        impl BinSample for $t {
            const INTEGER: bool = $integer;

            fn to_f64(self) -> f64 {
                let $v = self;
                $convert
            }
        }
    };
    (exact $integer:literal: $($t:ty),*) => {
        $(bin_samples!(@impl $integer, $t, |v| f64::from(v));)*
    };
    // 64-bit integers round to the nearest double, as numpy converts them.
    (rounded $integer:literal: $($t:ty),*) => {
        $(bin_samples!(@impl $integer, $t, |v| v as f64);)*
    };
}

bin_samples!(exact false: f64, f32);
bin_samples!(exact true: i8, i16, i32, u8, u16, u32);
bin_samples!(rounded true: i64, isize, u64, usize);

impl BinSample for bool {
    const INTEGER: bool = true;

    /// numpy bins booleans as the `uint8` values 0 and 1.
    fn to_f64(self) -> f64 {
        f64::from(u8::from(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_values() {
        const { assert!(!f64::INTEGER && i32::INTEGER && bool::INTEGER) };
        assert_eq!(true.to_f64(), 1.0);
        assert_eq!((-3_i64).to_f64(), -3.0);
        assert_eq!(0.5_f32.to_f64(), 0.5);
    }
}
