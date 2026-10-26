//! The forms a list of extents comes in, so that every list type takes
//! them alike: one value, a `Vec`, a slice, an array, and references to a
//! `Vec` and to an array. ([`AxisValues`](super::AxisValues) take any
//! collection or iterator instead; see [`AxisItem`](super::AxisItem).)

/// Implements `From` for each form of a list of `$element`s (one element,
/// `&[_]`, `&Vec<_>`, `[_; N]` and `&[_; N]`) for the list type `$list`,
/// through its `From<Vec<$element>>`. A generic element is written
/// `impl<T: Bound> List: T`; `numbers List` does it for every primitive
/// number type, through `From<Vec<f64>>`.
macro_rules! from_value_lists {
    (numbers $list:ty) => {
        $crate::numeric::with_numbers!(from_value_lists!(@numbers $list;));
    };
    (@numbers $list:ty; f64, $($number:ty),+) => {
        from_value_lists!(@forms [] $list, f64);
        $(
            impl From<Vec<$number>> for $list {
                fn from(values: Vec<$number>) -> Self {
                    values
                        .into_iter()
                        .map($crate::numeric::Number::to_f64)
                        .collect::<Vec<f64>>()
                        .into()
                }
            }

            from_value_lists!(@forms [] $list, $number);
        )+
    };
    (@forms [$($generics:tt)*] $list:ty, $element:ty) => {
        impl<$($generics)*> From<$element> for $list {
            /// The one value.
            fn from(value: $element) -> Self {
                vec![value].into()
            }
        }

        impl<$($generics)*> From<&[$element]> for $list {
            fn from(values: &[$element]) -> Self {
                values.to_vec().into()
            }
        }

        impl<$($generics)*> From<&Vec<$element>> for $list {
            fn from(values: &Vec<$element>) -> Self {
                values.as_slice().into()
            }
        }

        impl<$($generics)* const N: usize> From<[$element; N]> for $list {
            fn from(values: [$element; N]) -> Self {
                Vec::from(values).into()
            }
        }

        impl<$($generics)* const N: usize> From<&[$element; N]> for $list {
            fn from(values: &[$element; N]) -> Self {
                values.as_slice().into()
            }
        }
    };
    (impl<$param:ident: $bound:path> $list:ty: $element:ty) => {
        from_value_lists!(@forms [$param: $bound,] $list, $element);
    };
    ($list:ty: $($element:ty),+) => {
        $(from_value_lists!(@forms [] $list, $element);)+
    };
}

pub(crate) use from_value_lists;
