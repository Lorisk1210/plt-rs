//! The keyword arguments of `Figure.colorbar` (matplotlib's
//! `_make_axes_kw_doc` and `_colormap_kw_doc`).

use std::rc::Rc;

use pltrs_core::Anchor;

use crate::axes::Axes;
use crate::colors::Extend;
use crate::orientation::Orientation;
use crate::spines::SpineSide;
use crate::ticker::{Formatter, LabelFormat, Locator};

/// Where a colorbar is drawn (matplotlib's `ax` and `cax` arguments).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ColorbarAxes {
    /// Steal space from the axes the mappable is drawn in.
    #[default]
    FromMappable,
    /// Steal space from one axes (`ax=ax`): through its grid cell when it
    /// has one, as matplotlib's `make_axes_gridspec` does.
    Parent(Axes),
    /// Steal space from the bounding box of several axes (`ax=[...]`,
    /// matplotlib's `make_axes`).
    Parents(Vec<Axes>),
    /// Draw into an existing axes (`cax=...`).
    Cax(Axes),
}

/// How much room the extension triangles (or rectangles) take, as a
/// fraction of the interior length (matplotlib's `extendfrac`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ExtendFrac {
    /// 5% for each extension.
    #[default]
    Default,
    /// As long as the adjacent interior colour block.
    Auto,
    /// The same fraction for both extensions.
    Both(f64),
    /// The lower and upper fractions.
    Each(f64, f64),
}

pltrs_core::archive_enum!(ExtendFrac {
    Default,
    Auto,
    Both(v0),
    Each(v0, v1),
});

/// How discrete colorbars (boundary norms, contours) space their colours
/// (matplotlib's `spacing`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Spacing {
    /// Every colour gets the same length.
    #[default]
    Uniform,
    /// Lengths proportional to the data intervals.
    Proportional,
}

pltrs_core::archive_enum!(Spacing {
    Uniform,
    Proportional,
});

/// The major ticks of a colorbar (matplotlib's `ticks`).
#[derive(Clone, Debug)]
pub enum ColorbarTicks {
    /// Ticks at these values.
    Values(Vec<f64>),
    /// Ticks placed by a locator.
    Locator(Rc<dyn Locator>),
}

/// The tick label format of a colorbar (matplotlib's `format`).
#[derive(Clone, Debug)]
pub enum ColorbarFormat {
    /// A `%`-style template such as `"%.2f"`, or a `str.format` one with
    /// the fields `x` and `pos` such as `"{x:.2e}"` when it is not a valid
    /// `%` template; or a function of the value.
    Label(LabelFormat),
    /// A formatter.
    Formatter(Rc<dyn Formatter>),
}

impl From<LabelFormat> for ColorbarFormat {
    fn from(format: LabelFormat) -> Self {
        Self::Label(format)
    }
}

/// What happens to the anchor of the axes a colorbar steals space from
/// (matplotlib's `panchor`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ParentAnchor {
    /// Towards the colorbar: `(1, 0.5)` for a colorbar on the right, ...
    #[default]
    Default,
    /// Left unchanged (`panchor=False`).
    Unchanged,
    /// Set to this anchor.
    At(Anchor),
}

/// Options of [`Figure::colorbar`](crate::figure::Figure::colorbar);
/// the defaults are matplotlib's.
#[derive(Clone, Debug)]
pub struct ColorbarOptions {
    /// Where the colorbar is drawn.
    pub axes: ColorbarAxes,
    /// Place a colorbar stealing space from a single grid-placed axes in
    /// that axes' grid cell (matplotlib's `use_gridspec`).
    pub use_gridspec: bool,
    /// The side of the parent axes the colorbar goes on; also sets the
    /// orientation and the tick side.
    pub location: Option<SpineSide>,
    /// The orientation (vertical unless `location` says otherwise).
    pub orientation: Option<Orientation>,
    /// The side the ticks and label go on (by default `location`, else
    /// right for vertical and bottom for horizontal colorbars).
    pub ticklocation: Option<SpineSide>,
    /// Fraction of the parent axes used for the colorbar.
    pub fraction: f64,
    /// Factor applied to the length of the colorbar.
    pub shrink: f64,
    /// Ratio of the long to the short side.
    pub aspect: f64,
    /// Gap between the colorbar and the parent axes, as a fraction of the
    /// parent (`None` = 0.05 on the right and top, 0.10 on the left, 0.15
    /// at the bottom, 0.05 under constrained layout).
    pub pad: Option<f64>,
    /// Where the colorbar sits in its slot (`None` = towards the parent).
    pub anchor: Option<Anchor>,
    /// What happens to the parent's anchor.
    pub panchor: ParentAnchor,
    /// Pointed (or rectangular) ends for out-of-range values (`None` =
    /// the colormap's or the norm's choice).
    pub extend: Option<Extend>,
    /// The length of the extensions.
    pub extendfrac: ExtendFrac,
    /// Rectangular instead of triangular extensions.
    pub extendrect: bool,
    /// The major ticks (`None` = chosen from the norm).
    pub ticks: Option<ColorbarTicks>,
    /// The tick label format (`None` = the axis default).
    pub format: Option<ColorbarFormat>,
    /// Draw lines between the colours.
    pub drawedges: bool,
    /// The label of the long axis.
    pub label: String,
    /// Colour spacing of discrete colorbars.
    pub spacing: Spacing,
    /// The edges of the colour blocks (`None` = from the norm).
    pub boundaries: Option<Vec<f64>>,
    /// The value coloured in each block (one fewer than `boundaries`).
    pub values: Option<Vec<f64>>,
    /// The opacity of a colorbar of a bare norm and colormap
    /// ([`Mappable::Scalar`](super::Mappable::Scalar)); a colorbar of a
    /// drawn artist takes the artist's opacity instead, as in matplotlib.
    pub alpha: Option<f64>,
}

impl ColorbarOptions {
    /// The default options for a colorbar stealing space from `ax`
    /// (matplotlib's `fig.colorbar(mappable, ax=ax)`).
    ///
    /// ```
    /// use pltrs::axes::ImshowOptions;
    /// use pltrs::colorbar::ColorbarOptions;
    /// use pltrs::numeric::Grid;
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let image = ax.imshow(Grid::from_fn(2, 2, |i, j| (i + j) as f64), ImshowOptions::default())?;
    /// fig.colorbar(&image, ColorbarOptions::beside(&ax))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn beside(ax: &Axes) -> Self {
        Self {
            axes: ColorbarAxes::Parent(ax.clone()),
            ..Self::default()
        }
    }
}

impl Default for ColorbarOptions {
    fn default() -> Self {
        Self {
            axes: ColorbarAxes::default(),
            use_gridspec: true,
            location: None,
            orientation: None,
            ticklocation: None,
            fraction: 0.15,
            shrink: 1.0,
            aspect: 20.0,
            pad: None,
            anchor: None,
            panchor: ParentAnchor::default(),
            extend: None,
            extendfrac: ExtendFrac::default(),
            extendrect: false,
            ticks: None,
            format: None,
            drawedges: false,
            label: String::new(),
            spacing: Spacing::default(),
            boundaries: None,
            values: None,
            alpha: None,
        }
    }
}
