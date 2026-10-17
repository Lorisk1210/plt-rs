//! Property sets for new patches (matplotlib's patch `**kwargs`).

use plt_render::{CapStyle, JoinStyle};

use super::artist::PatchArtist;
use super::{PatchShape, PatchStyle};
use crate::artist::{Artist, BaseProps};
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::rc::ColorOrEdge;

/// Properties of a new patch; unset ones keep their defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PatchProps {
    /// Properties common to all artists.
    pub base: BaseProps,
    /// Face and edge colour, overridden by `facecolor` and `edgecolor`.
    pub color: Option<Color>,
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
        /// Face and edge colour.
        color: Color;
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
            color: overrides.color.or(self.color),
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

    /// Whether the property named `name` (matplotlib's spelling) is set.
    pub fn is_set(&self, name: &str) -> bool {
        match name {
            "color" => self.color.is_some(),
            "facecolor" => self.facecolor.is_some(),
            "edgecolor" => self.edgecolor.is_some(),
            "linewidth" => self.linewidth.is_some(),
            "linestyle" => self.linestyle.is_some(),
            "fill" => self.fill.is_some(),
            "hatch" => self.hatch.is_some(),
            "hatchcolor" => self.hatchcolor.is_some(),
            "antialiased" => self.antialiased.is_some(),
            "capstyle" => self.capstyle.is_some(),
            "joinstyle" => self.joinstyle.is_some(),
            "alpha" => self.base.alpha.is_some(),
            "zorder" => self.base.zorder.is_some(),
            "label" => self.base.label.is_some(),
            _ => false,
        }
    }

    /// Writes the set properties into a patch.
    pub(crate) fn apply<S: PatchShape>(self, patch: &mut PatchArtist<S>) {
        let base = self.apply_style(&mut patch.style);
        base.apply(patch.base_mut());
    }

    /// Writes the set edge, face and hatch properties into `style` and
    /// returns the common artist properties for the caller to apply.
    pub(crate) fn apply_style(self, style: &mut PatchStyle) -> BaseProps {
        if let Some(color) = self.color {
            style.facecolor = Some(color.clone());
            style.edgecolor = Some(color);
        }
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
        self.base
    }
}
