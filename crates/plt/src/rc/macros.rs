//! The `rc_params!` macro that turns the parameter table into [`RcParams`].
//!
//! [`RcParams`]: super::RcParams

/// The rc-file parser of a parameter: its own, or the type's.
macro_rules! rc_parser {
    ($ty:ty) => {
        <$ty as super::value::RcValue>::parse_rc
    };
    ($ty:ty, $parse:expr) => {
        $parse
    };
}

/// Declares every rcParam as a typed field plus string-keyed access.
///
/// Each entry reads `field: Type = "default" => "key" [using(parse)]
/// [where check];` where the default is written in rc-file syntax, `parse`
/// an optional `Fn(&str) -> Result<Type, String>` replacing the type's own
/// rc-file parsing (for validators with a syntax of their own) and `check`
/// an optional extra validator `Fn(&Type) -> Result<(), String>`.
macro_rules! rc_params {
    ($(
        $(#[$doc:meta])*
        $field:ident : $ty:ty = $default:literal => $key:literal
            $(using($parse:expr))? $(where $check:expr)? ;
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
                        rc_parser!($ty $(, $parse)?),
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
                            rc_parser!($ty $(, $parse)?),
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
