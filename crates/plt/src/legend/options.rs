//! The options of a legend (matplotlib's `legend` keyword arguments).

use super::handle::LegendHandle;
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};
use crate::rc::LabelColor;
use crate::transforms::Transform;

/// Where a legend goes (matplotlib's `loc`), validated when the legend is
/// made.
#[derive(Clone, Debug, PartialEq)]
pub enum LegendLoc {
    /// A location name: `"best"` (axes legends only), `"upper right"`,
    /// `"upper left"`, `"lower left"`, `"lower right"`, `"right"`,
    /// `"center left"`, `"center right"`, `"lower center"`,
    /// `"upper center"` or `"center"`.
    Name(String),
    /// matplotlib's numeric location code, `0` (`"best"`) to `10`.
    Code(i64),
    /// The lower-left corner of the legend, in fractions of the box given
    /// by `bbox_to_anchor` (by default the parent axes or figure).
    Point(f64, f64),
}

impl From<&str> for LegendLoc {
    fn from(name: &str) -> Self {
        Self::Name(name.to_owned())
    }
}

impl From<String> for LegendLoc {
    fn from(name: String) -> Self {
        Self::Name(name)
    }
}

impl From<i64> for LegendLoc {
    fn from(code: i64) -> Self {
        Self::Code(code)
    }
}

impl From<i32> for LegendLoc {
    fn from(code: i32) -> Self {
        Self::Code(i64::from(code))
    }
}

impl From<(f64, f64)> for LegendLoc {
    fn from((x, y): (f64, f64)) -> Self {
        Self::Point(x, y)
    }
}

/// The box a legend is placed in (matplotlib's `bbox_to_anchor`), in the
/// coordinates of `bbox_transform`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BboxToAnchor {
    /// A point: the corner of the legend named by `loc` goes there.
    Point(f64, f64),
    /// A box `(x, y, width, height)` the legend is placed in.
    Bounds(f64, f64, f64, f64),
}

impl From<(f64, f64)> for BboxToAnchor {
    fn from((x, y): (f64, f64)) -> Self {
        Self::Point(x, y)
    }
}

impl From<(f64, f64, f64, f64)> for BboxToAnchor {
    fn from((x, y, width, height): (f64, f64, f64, f64)) -> Self {
        Self::Bounds(x, y, width, height)
    }
}

/// How the columns of a legend use the available width (matplotlib's
/// `mode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LegendMode {
    /// As wide as the entries need (`mode=None`).
    #[default]
    Fit,
    /// Stretched to the width of `bbox_to_anchor` (`mode="expand"`).
    Expand,
}

/// How the title and the block of entries line up (matplotlib's
/// `alignment`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LegendAlignment {
    /// Centred.
    #[default]
    Center,
    /// Flush left.
    Left,
    /// Flush right.
    Right,
}

/// The colour of the legend labels (matplotlib's `labelcolor`).
#[derive(Clone, Debug, PartialEq)]
pub enum LegendLabelColor {
    /// A colour, or a rule taking it from each entry's artist (as
    /// `legend.labelcolor` accepts).
    Single(LabelColor),
    /// Colours cycled over the labels.
    Cycle(Vec<Color>),
}

impl From<LabelColor> for LegendLabelColor {
    fn from(color: LabelColor) -> Self {
        Self::Single(color)
    }
}

impl From<&str> for LegendLabelColor {
    /// `"linecolor"`, `"markerfacecolor"` (`"mfc"`), `"markeredgecolor"`
    /// (`"mec"`) select a rule; anything else is a colour.
    fn from(spec: &str) -> Self {
        let rule = match spec {
            "linecolor" => LabelColor::LineColor,
            "markerfacecolor" | "mfc" => LabelColor::MarkerFaceColor,
            "markeredgecolor" | "mec" => LabelColor::MarkerEdgeColor,
            color => LabelColor::Color(Color::from(color)),
        };
        Self::Single(rule)
    }
}

impl From<Color> for LegendLabelColor {
    fn from(color: Color) -> Self {
        Self::Single(LabelColor::Color(color))
    }
}

impl From<Vec<Color>> for LegendLabelColor {
    fn from(colors: Vec<Color>) -> Self {
        Self::Cycle(colors)
    }
}

/// Options of [`Axes::legend`](crate::axes::Axes::legend) and
/// [`Figure::legend`](crate::figure::Figure::legend) (matplotlib's keyword
/// arguments); unset ones come from the `legend.*` rcParams.
///
/// ```
/// use plt::legend::LegendOptions;
/// let options = LegendOptions::default().loc("upper left").ncols(2).title("Runs");
/// assert_eq!(options.ncols, 2);
/// ```
#[derive(Clone, Debug)]
pub struct LegendOptions {
    /// The artists to list (matplotlib's `handles`); by default every
    /// labelled artist and container of the axes.
    pub handles: Option<Vec<LegendHandle>>,
    /// The labels (matplotlib's `labels`); by default the handles' own.
    pub labels: Option<Vec<String>>,
    /// Location (`legend.loc` for axes, `"upper right"` for figures).
    pub loc: Option<LegendLoc>,
    /// The box `loc` refers to; by default the parent axes or figure.
    pub bbox_to_anchor: Option<BboxToAnchor>,
    /// Coordinates of `bbox_to_anchor`; by default fractions of the parent.
    pub bbox_transform: Option<Transform>,
    /// Number of columns.
    pub ncols: usize,
    /// Font of the labels; replaces `fontsize`.
    pub prop: Option<FontProperties>,
    /// Size of the labels (`legend.fontsize`).
    pub fontsize: Option<FontSize>,
    /// Colour of the labels (`legend.labelcolor`, then `text.color`).
    pub labelcolor: Option<LegendLabelColor>,
    /// Markers on a line entry (`legend.numpoints`).
    pub numpoints: Option<usize>,
    /// Markers on a scatter entry (`legend.scatterpoints`).
    pub scatterpoints: Option<usize>,
    /// Heights of the scatter markers as fractions of the entry height,
    /// cycled (default `[0.375, 0.5, 0.3125]`).
    pub scatteryoffsets: Option<Vec<f64>>,
    /// Size of the markers relative to the plotted ones
    /// (`legend.markerscale`).
    pub markerscale: Option<f64>,
    /// Draw the handle left of the label (`true`) or right of it.
    pub markerfirst: bool,
    /// List the entries in reverse order.
    pub reverse: bool,
    /// Draw the frame (`legend.frameon`).
    pub frameon: Option<bool>,
    /// Round the frame's corners (`legend.fancybox`).
    pub fancybox: Option<bool>,
    /// Draw a shadow under the frame (`legend.shadow`).
    pub shadow: Option<bool>,
    /// Opacity of the frame (`legend.framealpha`; 1 with a shadow).
    pub framealpha: Option<f64>,
    /// Frame fill (`legend.facecolor`; `"inherit"` uses `axes.facecolor`).
    pub facecolor: Option<Color>,
    /// Frame edge (`legend.edgecolor`; `"inherit"` uses `axes.edgecolor`).
    pub edgecolor: Option<Color>,
    /// Frame edge width in points (`legend.linewidth`, then
    /// `patch.linewidth`).
    pub linewidth: Option<f64>,
    /// Column widths.
    pub mode: LegendMode,
    /// Title above the entries.
    pub title: Option<String>,
    /// Size of the title (`legend.title_fontsize`, then `font.size`).
    pub title_fontsize: Option<FontSize>,
    /// Font of the title; cannot be combined with `title_fontsize`.
    pub title_fontproperties: Option<FontProperties>,
    /// How the title and the entries line up.
    pub alignment: LegendAlignment,
    /// Space inside the frame, in font sizes (`legend.borderpad`).
    pub borderpad: Option<f64>,
    /// Space between entries, in font sizes (`legend.labelspacing`).
    pub labelspacing: Option<f64>,
    /// Length of the handles, in font sizes (`legend.handlelength`).
    pub handlelength: Option<f64>,
    /// Height of the handles, in font sizes (`legend.handleheight`).
    pub handleheight: Option<f64>,
    /// Space between handle and label, in font sizes
    /// (`legend.handletextpad`).
    pub handletextpad: Option<f64>,
    /// Space between the frame and the anchor box, in font sizes
    /// (`legend.borderaxespad`).
    pub borderaxespad: Option<f64>,
    /// Space between columns, in font sizes (`legend.columnspacing`).
    pub columnspacing: Option<f64>,
}

impl Default for LegendOptions {
    fn default() -> Self {
        Self {
            handles: None,
            labels: None,
            loc: None,
            bbox_to_anchor: None,
            bbox_transform: None,
            ncols: 1,
            prop: None,
            fontsize: None,
            labelcolor: None,
            numpoints: None,
            scatterpoints: None,
            scatteryoffsets: None,
            markerscale: None,
            markerfirst: true,
            reverse: false,
            frameon: None,
            fancybox: None,
            shadow: None,
            framealpha: None,
            facecolor: None,
            edgecolor: None,
            linewidth: None,
            mode: LegendMode::Fit,
            title: None,
            title_fontsize: None,
            title_fontproperties: None,
            alignment: LegendAlignment::Center,
            borderpad: None,
            labelspacing: None,
            handlelength: None,
            handleheight: None,
            handletextpad: None,
            borderaxespad: None,
            columnspacing: None,
        }
    }
}

/// Chainable setters for the optional fields of [`LegendOptions`].
macro_rules! optional_setters {
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

impl LegendOptions {
    optional_setters! {
        /// Sets [`LegendOptions::loc`].
        loc: LegendLoc;
        /// Sets [`LegendOptions::bbox_to_anchor`].
        bbox_to_anchor: BboxToAnchor;
        /// Sets [`LegendOptions::bbox_transform`].
        bbox_transform: Transform;
        /// Sets [`LegendOptions::prop`].
        prop: FontProperties;
        /// Sets [`LegendOptions::fontsize`].
        fontsize: FontSize;
        /// Sets [`LegendOptions::labelcolor`].
        labelcolor: LegendLabelColor;
        /// Sets [`LegendOptions::numpoints`].
        numpoints: usize;
        /// Sets [`LegendOptions::scatterpoints`].
        scatterpoints: usize;
        /// Sets [`LegendOptions::scatteryoffsets`].
        scatteryoffsets: Vec<f64>;
        /// Sets [`LegendOptions::markerscale`].
        markerscale: f64;
        /// Sets [`LegendOptions::frameon`].
        frameon: bool;
        /// Sets [`LegendOptions::fancybox`].
        fancybox: bool;
        /// Sets [`LegendOptions::shadow`].
        shadow: bool;
        /// Sets [`LegendOptions::framealpha`].
        framealpha: f64;
        /// Sets [`LegendOptions::facecolor`].
        facecolor: Color;
        /// Sets [`LegendOptions::edgecolor`].
        edgecolor: Color;
        /// Sets [`LegendOptions::linewidth`].
        linewidth: f64;
        /// Sets [`LegendOptions::title`].
        title: String;
        /// Sets [`LegendOptions::title_fontsize`].
        title_fontsize: FontSize;
        /// Sets [`LegendOptions::title_fontproperties`].
        title_fontproperties: FontProperties;
        /// Sets [`LegendOptions::borderpad`].
        borderpad: f64;
        /// Sets [`LegendOptions::labelspacing`].
        labelspacing: f64;
        /// Sets [`LegendOptions::handlelength`].
        handlelength: f64;
        /// Sets [`LegendOptions::handleheight`].
        handleheight: f64;
        /// Sets [`LegendOptions::handletextpad`].
        handletextpad: f64;
        /// Sets [`LegendOptions::borderaxespad`].
        borderaxespad: f64;
        /// Sets [`LegendOptions::columnspacing`].
        columnspacing: f64;
    }

    /// Lists exactly these artists (matplotlib's `handles`).
    #[must_use]
    pub fn handles(mut self, handles: impl IntoIterator<Item = impl Into<LegendHandle>>) -> Self {
        self.handles = Some(handles.into_iter().map(Into::into).collect());
        self
    }

    /// Uses these labels (matplotlib's `labels`).
    #[must_use]
    pub fn labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.labels = Some(labels.into_iter().map(Into::into).collect());
        self
    }

    /// Sets [`LegendOptions::ncols`].
    #[must_use]
    pub fn ncols(mut self, ncols: usize) -> Self {
        self.ncols = ncols;
        self
    }

    /// Sets [`LegendOptions::markerfirst`].
    #[must_use]
    pub fn markerfirst(mut self, markerfirst: bool) -> Self {
        self.markerfirst = markerfirst;
        self
    }

    /// Sets [`LegendOptions::reverse`].
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Sets [`LegendOptions::mode`].
    #[must_use]
    pub fn mode(mut self, mode: LegendMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets [`LegendOptions::alignment`].
    #[must_use]
    pub fn alignment(mut self, alignment: LegendAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_colour_rules_and_colours() {
        assert_eq!(
            LegendLabelColor::from("mfc"),
            LegendLabelColor::Single(LabelColor::MarkerFaceColor)
        );
        assert_eq!(
            LegendLabelColor::from("red"),
            LegendLabelColor::Single(LabelColor::Color(Color::from("red")))
        );
    }

    #[test]
    fn defaults_follow_matplotlib() {
        let options = LegendOptions::default();
        assert!(options.markerfirst);
        assert_eq!(options.ncols, 1);
        assert_eq!(options.mode, LegendMode::Fit);
        assert_eq!(
            LegendOptions::default().loc(3).loc,
            Some(LegendLoc::Code(3))
        );
    }
}
