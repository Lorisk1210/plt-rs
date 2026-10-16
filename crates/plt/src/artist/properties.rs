//! Generation of property accessors on artist handles and of the property
//! sets passed when creating artists (matplotlib's `**kwargs`).

use super::ArtistBase;

/// Common artist properties to set on creation (`None` = default).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BaseProps {
    /// Opacity override in `[0, 1]`.
    pub alpha: Option<f64>,
    /// Drawing order.
    pub zorder: Option<f64>,
    /// Legend label.
    pub label: Option<String>,
    /// Whether the artist is drawn.
    pub visible: Option<bool>,
    /// Whether the artist is clipped to its axes.
    pub clip_on: Option<bool>,
    /// Rasterize even in vector output.
    pub rasterized: Option<bool>,
    /// Group id for vector output.
    pub gid: Option<String>,
    /// Hyperlink for vector output.
    pub url: Option<String>,
}

impl BaseProps {
    /// Writes the set properties into `base`.
    pub(crate) fn apply(self, base: &mut ArtistBase) {
        if let Some(alpha) = self.alpha {
            base.alpha = Some(alpha);
        }
        if let Some(zorder) = self.zorder {
            base.zorder = zorder;
        }
        if let Some(label) = self.label {
            base.label = label;
        }
        if let Some(visible) = self.visible {
            base.visible = visible;
        }
        if let Some(clip_on) = self.clip_on {
            base.clip_on = clip_on;
        }
        if let Some(rasterized) = self.rasterized {
            base.rasterized = rasterized;
        }
        if self.gid.is_some() {
            base.gid = self.gid;
        }
        if self.url.is_some() {
            base.url = self.url;
        }
    }
}

/// Implements matplotlib-style `property()` / `set_property(value)` pairs
/// on a handle type for fields of its artist state.
///
/// Each entry is `field, setter: Type;` and may end with `=> hook` to call
/// `state.hook()` after the field changed (e.g. to rebuild a cached path).
/// Setters accept `impl Into<Type>` and return `&Self` for chaining.
///
/// With `$handle => $state, $props { ... }` the macro also generates the
/// property set `$props`: one `Option` per entry plus the [`BaseProps`],
/// with chainable builder methods named after the properties, so that new
/// artists can be configured before they consume the property cycle.
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
    ($handle:ty => $state:ty, $props:ident {
        $( $(#[$doc:meta])* $field:ident, $setter:ident : $ty:ty $(=> $hook:ident)? ; )*
    }) => {
        artist_properties!($handle => $state {
            $( $(#[$doc])* $field, $setter: $ty $(=> $hook)?; )*
        });

        #[doc = concat!(
            "Properties of a new [`", stringify!($handle), "`] (matplotlib's keyword ",
            "arguments); unset ones keep their defaults."
        )]
        #[derive(Clone, Debug, Default)]
        pub struct $props {
            /// Properties common to all artists.
            pub base: $crate::artist::BaseProps,
            $(
                $(#[$doc])*
                pub $field: Option<$ty>,
            )*
        }

        impl $props {
            $(
                $(#[$doc])*
                #[must_use]
                pub fn $field(mut self, value: impl Into<$ty>) -> Self {
                    self.$field = Some(value.into());
                    self
                }
            )*

            /// Opacity override in `[0, 1]`.
            #[must_use]
            pub fn alpha(mut self, alpha: f64) -> Self {
                self.base.alpha = Some(alpha);
                self
            }

            /// Drawing order.
            #[must_use]
            pub fn zorder(mut self, zorder: f64) -> Self {
                self.base.zorder = Some(zorder);
                self
            }

            /// Legend label.
            #[must_use]
            pub fn label(mut self, label: impl Into<String>) -> Self {
                self.base.label = Some(label.into());
                self
            }

            /// Whether the artist is drawn.
            #[must_use]
            pub fn visible(mut self, visible: bool) -> Self {
                self.base.visible = Some(visible);
                self
            }

            /// Whether the artist is clipped to its axes.
            #[must_use]
            pub fn clip_on(mut self, clip_on: bool) -> Self {
                self.base.clip_on = Some(clip_on);
                self
            }

            /// Whether the property named `name` (matplotlib's spelling) is set.
            pub fn is_set(&self, name: &str) -> bool {
                match name {
                    $( stringify!($field) => self.$field.is_some(), )*
                    "alpha" => self.base.alpha.is_some(),
                    "zorder" => self.base.zorder.is_some(),
                    "label" => self.base.label.is_some(),
                    _ => false,
                }
            }

            /// Writes the set properties into a new artist's state.
            pub(crate) fn apply(self, state: &mut $state) {
                $crate::artist::BaseProps::apply(self.base, $crate::artist::Artist::base_mut(state));
                $(
                    if let Some(value) = self.$field {
                        state.$field = value;
                        $( state.$hook(); )?
                    }
                )*
            }
        }
    };
}
