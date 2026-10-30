//! Generation of property accessors on artist handles and of the property
//! sets passed when creating artists (matplotlib's `**kwargs`).

use super::{ArtistBase, ClipPathSpec, Picker};
use crate::patheffects::PathEffect;
use crate::transforms::Transform;

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
    /// Whether the artist is left out of normal draws, to be drawn by
    /// blitting (matplotlib's `animated`).
    pub animated: Option<bool>,
    /// Rasterize even in vector output.
    pub rasterized: Option<bool>,
    /// Group id for vector output.
    pub gid: Option<String>,
    /// Hyperlink for vector output.
    pub url: Option<String>,
    /// The coordinates of the artist's data (matplotlib's `transform`;
    /// `None` keeps the default of the method adding it, usually the
    /// axes' data coordinates).
    pub transform: Option<Transform>,
    /// A clip path set before the artist is added (matplotlib's
    /// `clip_path`): the axes then do not clip it to their box.
    pub clip_path: Option<ClipPathSpec>,
    /// Effects the artist is drawn through (matplotlib's `path_effects`).
    pub path_effects: Option<Vec<PathEffect>>,
    /// How mouse presses pick the artist (matplotlib's `picker`).
    pub picker: Option<Picker>,
}

impl BaseProps {
    /// `self` with every property set in `overrides` replaced.
    #[must_use]
    pub fn overlay(self, overrides: Self) -> Self {
        Self {
            alpha: overrides.alpha.or(self.alpha),
            zorder: overrides.zorder.or(self.zorder),
            label: overrides.label.or(self.label),
            visible: overrides.visible.or(self.visible),
            clip_on: overrides.clip_on.or(self.clip_on),
            animated: overrides.animated.or(self.animated),
            rasterized: overrides.rasterized.or(self.rasterized),
            gid: overrides.gid.or(self.gid),
            url: overrides.url.or(self.url),
            transform: overrides.transform.or(self.transform),
            clip_path: overrides.clip_path.or(self.clip_path),
            path_effects: overrides.path_effects.or(self.path_effects),
            picker: overrides.picker.or(self.picker),
        }
    }

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
        if let Some(animated) = self.animated {
            base.animated = animated;
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
        if self.transform.is_some() {
            base.transform = self.transform;
        }
        if self.clip_path.is_some() {
            base.clip_path = self.clip_path;
        }
        if let Some(effects) = self.path_effects {
            base.path_effects = effects;
        }
        if self.picker.is_some() {
            base.picker = self.picker;
        }
    }
}

/// Chainable builder methods for the [`BaseProps`] in the `base` field of
/// a property set.
macro_rules! base_prop_builders {
    () => {
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

        /// Whether the artist is rasterized in vector output (matplotlib's
        /// `rasterized`).
        #[must_use]
        pub fn rasterized(mut self, rasterized: bool) -> Self {
            self.base.rasterized = Some(rasterized);
            self
        }

        /// The group id of the artist in vector output (matplotlib's
        /// `gid`).
        #[must_use]
        pub fn gid(mut self, gid: impl Into<String>) -> Self {
            self.base.gid = Some(gid.into());
            self
        }

        /// The hyperlink of the artist in vector output (matplotlib's
        /// `url`).
        #[must_use]
        pub fn url(mut self, url: impl Into<String>) -> Self {
            self.base.url = Some(url.into());
            self
        }

        /// Whether the artist is left out of normal draws, to be drawn by
        /// blitting (matplotlib's `animated`).
        #[must_use]
        pub fn animated(mut self, animated: bool) -> Self {
            self.base.animated = Some(animated);
            self
        }

        /// The coordinates of the artist's data (matplotlib's `transform`).
        #[must_use]
        pub fn transform(mut self, transform: $crate::transforms::Transform) -> Self {
            self.base.transform = Some(transform);
            self
        }

        /// Clips the artist to `clip` instead of its axes' box (matplotlib's
        /// `clip_path`, set before the artist is added).
        #[must_use]
        pub fn clip_path(mut self, clip: $crate::artist::ClipPathSpec) -> Self {
            self.base.clip_path = Some(clip);
            self
        }

        /// Effects the artist is drawn through (matplotlib's
        /// `path_effects`).
        #[must_use]
        pub fn path_effects(mut self, effects: Vec<$crate::patheffects::PathEffect>) -> Self {
            self.base.path_effects = Some(effects);
            self
        }

        /// How mouse presses pick the artist (matplotlib's `picker`).
        #[must_use]
        pub fn picker(mut self, picker: $crate::artist::Picker) -> Self {
            self.base.picker = Some(picker);
            self
        }
    };
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

        impl $handle {
            /// Sets every property `props` sets (matplotlib's `Artist.set`).
            pub fn update_props(&self, props: $props) -> &Self {
                self.raw.with_mut::<$state, _>(|state| props.apply(state));
                self
            }
        }

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

            base_prop_builders!();

            /// `self` with every property set in `overrides` replaced.
            #[must_use]
            pub fn overlay(self, overrides: Self) -> Self {
                Self {
                    base: self.base.overlay(overrides.base),
                    $( $field: overrides.$field.or(self.$field), )*
                }
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

            /// Writes the set properties into an artist's state.
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
