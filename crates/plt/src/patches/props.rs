//! Property sets for new patches (matplotlib's patch `**kwargs`).

use plt_render::{CapStyle, JoinStyle};

use super::PatchShape;
use super::artist::PatchArtist;
use crate::artist::{Artist, BaseProps};
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::rc::ColorOrEdge;

/// Properties of a new patch; unset ones keep their defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PatchProps {
    /// Properties common to all artists.
    pub base: BaseProps,
    /// Face colour.
    pub facecolor: Option<Color>,
    /// Edge colour.
    pub edgecolor: Option<Color>,
    /// Edge width in points.
    pub linewidth: Option<f64>,
    /// Edge line style.
    pub linestyle: Option<LineStyle>,
    /// Whether the face is filled.
    pub fill: Option<bool>,
    /// Hatch pattern such as `"//"` or `"x"`.
    pub hatch: Option<String>,
    /// Hatch colour.
    pub hatchcolor: Option<Color>,
    /// Anti-aliasing.
    pub antialiased: Option<bool>,
    /// Edge end caps.
    pub capstyle: Option<CapStyle>,
    /// Edge corner joins.
    pub joinstyle: Option<JoinStyle>,
}

/// Chainable setters for the optional fields of [`PatchProps`].
macro_rules! patch_prop_builders {
    ($( $(#[$doc:meta])* $field:ident: $ty:ty; )*) => {
        $(
            $(#[$doc])*
            #[must_use]
            pub fn $field(mut self, value: impl Into<$ty>) -> Self {
                self.$field = Some(value.into());
                self
            }
        )*
    };
}

impl PatchProps {
    patch_prop_builders! {
        /// Face colour.
        facecolor: Color;
        /// Edge colour.
        edgecolor: Color;
        /// Edge width in points.
        linewidth: f64;
        /// Edge line style.
        linestyle: LineStyle;
        /// Whether the face is filled.
        fill: bool;
        /// Hatch pattern.
        hatch: String;
        /// Hatch colour.
        hatchcolor: Color;
        /// Anti-aliasing.
        antialiased: bool;
        /// Edge end caps.
        capstyle: CapStyle;
        /// Edge corner joins.
        joinstyle: JoinStyle;
    }

    base_prop_builders!();

    /// `self` with every property set in `overrides` replaced.
    #[must_use]
    pub fn overlay(self, overrides: Self) -> Self {
        Self {
            base: self.base.overlay(overrides.base),
            facecolor: overrides.facecolor.or(self.facecolor),
            edgecolor: overrides.edgecolor.or(self.edgecolor),
            linewidth: overrides.linewidth.or(self.linewidth),
            linestyle: overrides.linestyle.or(self.linestyle),
            fill: overrides.fill.or(self.fill),
            hatch: overrides.hatch.or(self.hatch),
            hatchcolor: overrides.hatchcolor.or(self.hatchcolor),
            antialiased: overrides.antialiased.or(self.antialiased),
            capstyle: overrides.capstyle.or(self.capstyle),
            joinstyle: overrides.joinstyle.or(self.joinstyle),
        }
    }

    /// Writes the set properties into a new patch.
    pub(crate) fn apply<S: PatchShape>(self, patch: &mut PatchArtist<S>) {
        self.base.apply(patch.base_mut());
        let style = &mut patch.style;
        if self.facecolor.is_some() {
            style.facecolor = self.facecolor;
        }
        if self.edgecolor.is_some() {
            style.edgecolor = self.edgecolor;
        }
        if self.linewidth.is_some() {
            style.linewidth = self.linewidth;
        }
        if let Some(linestyle) = self.linestyle {
            style.linestyle = linestyle;
        }
        if let Some(fill) = self.fill {
            style.fill = fill;
        }
        if self.hatch.is_some() {
            style.hatch = self.hatch;
        }
        if let Some(color) = self.hatchcolor {
            style.hatchcolor = ColorOrEdge::Color(color);
        }
        if self.antialiased.is_some() {
            style.antialiased = self.antialiased;
        }
        if let Some(cap) = self.capstyle {
            style.capstyle = cap;
        }
        if let Some(join) = self.joinstyle {
            style.joinstyle = join;
        }
    }
}
