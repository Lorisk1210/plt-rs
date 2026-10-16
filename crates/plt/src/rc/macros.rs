//! The `rc_params!` macro that turns the parameter table into [`RcParams`].
//!
//! [`RcParams`]: super::RcParams

/// Declares every rcParam as a typed field plus string-keyed access.
///
/// Each entry reads `field: Type = "default" => "key" [where check];` where
/// the default is written in rc-file syntax and `check` is an optional
/// extra validator `Fn(&Type) -> Result<(), String>`.
macro_rules! rc_params {
    ($(
        $(#[$doc:meta])*
        $field:ident : $ty:ty = $default:literal => $key:literal $(where $check:expr)? ;
    )*) => {
        /// Every runtime configuration parameter (matplotlib's `rcParams`),
        /// one typed field per key.
        #[derive(Clone, Debug, PartialEq)]
        pub struct RcParams {
            $( $(#[$doc])* pub $field: $ty, )*
        }

        impl RcParams {
            /// Every parameter name, in matplotlib's order.
            pub const KEYS: &'static [&'static str] = &[$($key),*];

            /// matplotlib's defaults (`matplotlib.rcParamsDefault`).
            pub fn matplotlib() -> Self {
                Self {
                    $( $field: super::value::parse_checked::<$ty>(
                        $key,
                        $default,
                        |_value: &$ty| -> Result<(), String> { $( ($check)(_value)?; )? Ok(()) },
                    ).expect("generated rc defaults are valid"), )*
                }
            }

            /// Sets the parameter `key` from its rc-file text, validating it
            /// exactly as matplotlib does.
            pub fn set(&mut self, key: &str, value: &str) -> Result<(), super::RcError> {
                match key {
                    $( $key => {
                        self.$field = super::value::parse_checked::<$ty>(
                            $key,
                            value,
                            |_value: &$ty| -> Result<(), String> { $( ($check)(_value)?; )? Ok(()) },
                        )?;
                        Ok(())
                    } )*
                    _ => Err(super::RcError::UnknownKey(key.to_owned())),
                }
            }

            /// The rc-file text of the parameter `key`, if it exists.
            pub fn get(&self, key: &str) -> Option<String> {
                match key {
                    $( $key => Some(super::value::RcValue::format_rc(&self.$field)), )*
                    _ => None,
                }
            }
        }
    };
}
