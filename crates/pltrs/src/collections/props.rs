//! Property sets for new collections (matplotlib's collection `**kwargs`)
//! and the look of existing ones.

use pltrs_render::{CapStyle, JoinStyle};

use super::{CollectionArtist, CollectionKind, EdgeColors, HatchColors};
use crate::artist::{Artist, BaseProps};
use crate::colors::Color;
use crate::lines::LineStyle;

/// The look of a collection's items as set, before defaults and colour
/// mapping are applied ([`Collection::style`](super::Collection::style);
/// matplotlib's `get_linewidth`, `get_linestyle`, `get_hatch`, ...; the
/// colours drawn come from [`Collection::colors`](super::Collection::colors)).
/// Lists are cycled over the items.
#[derive(Clone, Debug, PartialEq)]
pub struct CollectionStyle {
    /// Face colours (`None` = the kind's default).
    pub facecolors: Option<Vec<Color>>,
    /// Edge colours.
    pub edgecolors: EdgeColors,
    /// Edge widths in points (`None` = the kind's default).
    pub linewidths: Option<Vec<f64>>,
    /// Edge line styles.
    pub linestyles: Vec<LineStyle>,
    /// One opacity per item, replacing the artist's alpha.
    pub alphas: Option<Vec<f64>>,
    /// Anti-aliasing (`None` = the rc default).
    pub antialiased: Option<Vec<bool>>,
    /// Hatch pattern of every item.
    pub hatch: Option<String>,
    /// Hatch colours.
    pub hatchcolors: HatchColors,
    /// Width of the hatch lines in points.
    pub hatch_linewidth: f64,
    /// Cap style of the edges (`None` = the renderer's default).
    pub capstyle: Option<CapStyle>,
    /// Join style of the edges (`None` = the renderer's default).
    pub joinstyle: Option<JoinStyle>,
    /// Colours in the gaps of dashed edges (`None` = empty gaps).
    pub gapcolors: Option<Vec<Color>>,
}

impl CollectionStyle {
    /// The style of `collection`.
    pub(super) fn of(collection: &CollectionArtist) -> Self {
        Self {
            facecolors: collection.facecolors.clone(),
            edgecolors: collection.edgecolors.clone(),
            linewidths: collection.linewidths.clone(),
            linestyles: collection.linestyles.clone(),
            alphas: collection.alphas.clone(),
            antialiased: collection.antialiased.clone(),
            hatch: collection.hatch.clone(),
            hatchcolors: collection.hatchcolors.clone(),
            hatch_linewidth: collection.hatch_linewidth,
            capstyle: collection.capstyle,
            joinstyle: collection.joinstyle,
            gapcolors: collection.gapcolors.clone(),
        }
    }
}

/// Properties of a new collection; unset ones keep their defaults. Lists
/// are cycled over the items.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollectionProps {
    /// Properties common to all artists.
    pub base: BaseProps,
    /// Face and edge colour (only the edge colour of line collections),
    /// overridden by `facecolors` and `edgecolors`.
    pub color: Option<Color>,
    /// Face colours.
    pub facecolors: Option<Vec<Color>>,
    /// Edge colours.
    pub edgecolors: Option<EdgeColors>,
    /// Edge widths in points.
    pub linewidths: Option<Vec<f64>>,
    /// Edge line styles.
    pub linestyles: Option<Vec<LineStyle>>,
    /// Hatch pattern of every item.
    pub hatch: Option<String>,
    /// Hatch colours (default: `hatch.color`).
    pub hatchcolors: Option<HatchColors>,
    /// Anti-aliasing per item.
    pub antialiased: Option<Vec<bool>>,
    /// One opacity per item, replacing the artist's alpha (matplotlib's
    /// array `alpha`).
    pub alphas: Option<Vec<f64>>,
    /// Cap style of the edges.
    pub capstyle: Option<CapStyle>,
    /// Join style of the edges.
    pub joinstyle: Option<JoinStyle>,
}

impl CollectionProps {
    /// Face and edge colour (only the edge colour of line collections).
    #[must_use]
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// One face colour for every item.
    #[must_use]
    pub fn facecolor(mut self, color: impl Into<Color>) -> Self {
        self.facecolors = Some(vec![color.into()]);
        self
    }

    /// One edge colour for every item.
    #[must_use]
    pub fn edgecolor(mut self, color: impl Into<Color>) -> Self {
        self.edgecolors = Some(EdgeColors::Colors(vec![color.into()]));
        self
    }

    /// One edge width for every item.
    #[must_use]
    pub fn linewidth(mut self, width: f64) -> Self {
        self.linewidths = Some(vec![width]);
        self
    }

    /// One edge style for every item.
    #[must_use]
    pub fn linestyle(mut self, style: LineStyle) -> Self {
        self.linestyles = Some(vec![style]);
        self
    }

    /// Hatch pattern of every item.
    #[must_use]
    pub fn hatch(mut self, pattern: impl Into<String>) -> Self {
        self.hatch = Some(pattern.into());
        self
    }

    /// Cap style of the edges (matplotlib's `capstyle`).
    #[must_use]
    pub fn capstyle(mut self, capstyle: CapStyle) -> Self {
        self.capstyle = Some(capstyle);
        self
    }

    /// Join style of the edges (matplotlib's `joinstyle`).
    #[must_use]
    pub fn joinstyle(mut self, joinstyle: JoinStyle) -> Self {
        self.joinstyle = Some(joinstyle);
        self
    }

    base_prop_builders!();

    /// Whether a face colour is given (`color` or `facecolors`).
    pub fn sets_face(&self) -> bool {
        self.color.is_some() || self.facecolors.is_some()
    }

    /// Writes the set properties into a collection.
    pub(crate) fn apply(self, collection: &mut CollectionArtist) {
        self.base.apply(collection.base_mut());
        if let Some(color) = self.color {
            // matplotlib's `LineCollection.set_color` only colours the lines.
            if collection.kind != CollectionKind::Line {
                collection.facecolors = Some(vec![color.clone()]);
            }
            collection.edgecolors = EdgeColors::Colors(vec![color]);
        }
        if self.facecolors.is_some() {
            collection.facecolors = self.facecolors;
        }
        if let Some(edges) = self.edgecolors {
            collection.edgecolors = edges;
        }
        if self.linewidths.is_some() {
            collection.linewidths = self.linewidths;
        }
        if let Some(styles) = self.linestyles {
            collection.linestyles = styles;
        }
        if self.hatch.is_some() {
            collection.hatch = self.hatch;
        }
        if let Some(colors) = self.hatchcolors {
            collection.hatchcolors = colors;
        }
        if self.antialiased.is_some() {
            collection.antialiased = self.antialiased;
        }
        if self.alphas.is_some() {
            collection.alphas = self.alphas;
        }
        if self.capstyle.is_some() {
            collection.capstyle = self.capstyle;
        }
        if self.joinstyle.is_some() {
            collection.joinstyle = self.joinstyle;
        }
    }
}
