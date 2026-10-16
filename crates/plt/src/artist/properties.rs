//! Generation of property accessors on artist handles.

/// Implements matplotlib-style `property()` / `set_property(value)` pairs
/// on a handle type for fields of its artist state.
///
/// Each entry is `field, setter: Type;` and may end with `=> hook` to call
/// `state.hook()` after the field changed (e.g. to rebuild a cached path).
/// Setters accept `impl Into<Type>` and return `&Self` for chaining.
macro_rules! artist_properties {
    ($handle:ty => $state:ty {
        $( $(#[$doc:meta])* $field:ident, $setter:ident : $ty:ty $(=> $hook:ident)? ; )*
    }) => {
        impl $handle {
            $(
                $(#[$doc])*
                pub fn $field(&self) -> $ty {
                    self.raw.with::<$state, _>(|state| state.$field.clone())
                }

                #[doc = concat!("Sets [`", stringify!($field), "`](Self::", stringify!($field), ").")]
                pub fn $setter(&self, value: impl Into<$ty>) -> &Self {
                    let value = value.into();
                    self.raw.with_mut::<$state, _>(|state| {
                        state.$field = value;
                        $( state.$hook(); )?
                    });
                    self
                }
            )*
        }
    };
}
