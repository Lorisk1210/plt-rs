//! Scatter plots and adding collections.

use std::rc::Rc;

use plt_core::Point;

use super::children::DataUpdate;
use super::{Axes, PlotError};
use crate::artist::Artist;
use crate::collections::{Collection, CollectionArtist, CollectionKind, ColorMapping, EdgeColors};
use crate::colors::{Color, Colormap, Norm, ScalarMappable, colormaps};
use crate::markers::{FillStyle, Marker, MarkerStyle};
use crate::rc::RcParams;
use crate::transforms::Transform;

/// How scatter markers are coloured (matplotlib's `c` / `color`).
#[derive(Debug, Default)]
pub enum ScatterColors {
    /// The next colour of the property cycle.
    #[default]
    Cycle,
    /// One colour for every marker.
    Single(Color),
    /// One colour per marker (cycled).
    PerPoint(Vec<Color>),
    /// Values mapped through a colormap and norm.
    Mapped(Box<MappedColors>),
}

impl From<MappedColors> for ScatterColors {
    fn from(mapped: MappedColors) -> Self {
        Self::Mapped(Box::new(mapped))
    }
}

/// One value per marker, mapped to colours through a colormap and a norm
/// (scatter's numeric `c` with `cmap`, `norm`, `vmin` and `vmax`).
///
/// ```
/// use plt::axes::MappedColors;
/// use plt::colors::colormaps;
/// let colors = MappedColors::new(vec![1.0, 2.0, 3.0])
///     .with_cmap(colormaps::get("plasma")?)
///     .with_limits(Some(0.0), None);
/// # Ok::<(), plt::colors::ColormapError>(())
/// ```
#[derive(Debug)]
pub struct MappedColors {
    values: Vec<f64>,
    cmap: Option<Colormap>,
    norm: Option<Box<dyn Norm>>,
    vmin: Option<f64>,
    vmax: Option<f64>,
}

impl MappedColors {
    /// `values` mapped through `image.cmap` and a linear norm spanning
    /// the finite values.
    pub fn new(values: impl Into<Vec<f64>>) -> Self {
        Self {
            values: values.into(),
            cmap: None,
            norm: None,
            vmin: None,
            vmax: None,
        }
    }

    /// Uses `cmap` instead of `image.cmap`.
    #[must_use]
    pub fn with_cmap(self, cmap: Colormap) -> Self {
        Self {
            cmap: Some(cmap),
            ..self
        }
    }

    /// Uses `norm` instead of a linear norm; excludes [`Self::with_limits`].
    #[must_use]
    pub fn with_norm(self, norm: impl Norm + 'static) -> Self {
        Self {
            norm: Some(Box::new(norm)),
            ..self
        }
    }

    /// Fixes the values mapped to the colormap's ends (`None` = the data
    /// minimum or maximum).
    #[must_use]
    pub fn with_limits(self, vmin: Option<f64>, vmax: Option<f64>) -> Self {
        Self { vmin, vmax, ..self }
    }

    /// The colormap and norm (matplotlib's `set_cmap`, `set_norm` and
    /// `_scale_norm`) plus the values with non-finite ones masked.
    fn into_mapping(self, rc: &RcParams) -> Result<ColorMapping, PlotError> {
        let cmap = match self.cmap {
            Some(cmap) => cmap,
            None => colormaps::get(&rc.image_cmap)?,
        };
        let mappable = match self.norm {
            Some(_) if self.vmin.is_some() || self.vmax.is_some() => {
                return Err(PlotError::NormWithLimits);
            }
            Some(norm) => ScalarMappable::with_norm(cmap, norm),
            None => {
                let mut mappable = ScalarMappable::new(cmap);
                mappable.norm_mut().set_vmin(self.vmin);
                mappable.norm_mut().set_vmax(self.vmax);
                mappable
            }
        };
        let values = self
            .values
            .into_iter()
            .map(|v| v.is_finite().then_some(v))
            .collect();
        Ok(ColorMapping { values, mappable })
    }
}

/// Options of [`Axes::scatter_with`] (matplotlib's `scatter` keywords).
#[derive(Debug, Default)]
pub struct ScatterOptions {
    /// Marker areas in points² (one, or one per point); default
    /// `lines.markersize²`.
    pub sizes: Option<Vec<f64>>,
    /// Marker colours.
    pub colors: ScatterColors,
    /// Marker shape; default `scatter.marker`.
    pub marker: Option<Marker>,
    /// Edge colours; default `scatter.edgecolors`.
    pub edgecolors: Option<EdgeColors>,
    /// Edge widths in points.
    pub linewidths: Option<Vec<f64>>,
    /// Opacity.
    pub alpha: Option<f64>,
    /// Draw points whose mapped value is not finite in the colormap's
    /// "bad" colour instead of omitting them.
    pub plot_nonfinite: bool,
}

impl Axes {
    /// A scatter plot of `y` against `x` with default styling
    /// (matplotlib's `ax.scatter(x, y)`).
    pub fn scatter(
        &self,
        x: impl AsRef<[f64]>,
        y: impl AsRef<[f64]>,
    ) -> Result<Collection, PlotError> {
        self.scatter_with(x, y, ScatterOptions::default())
    }

    /// A scatter plot with explicit options.
    pub fn scatter_with(
        &self,
        x: impl AsRef<[f64]>,
        y: impl AsRef<[f64]>,
        options: ScatterOptions,
    ) -> Result<Collection, PlotError> {
        let (x, y) = (x.as_ref(), y.as_ref());
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let rc = crate::rc::params();
        let marker = options.marker.unwrap_or_else(|| rc.scatter_marker.clone());
        let style = MarkerStyle::new(marker, FillStyle::Full)?;
        let shape = style.path().transformed(&style.transform());
        let sizes = options
            .sizes
            .unwrap_or_else(|| vec![rc.lines_markersize.powi(2)]);
        if sizes.len() != 1 && sizes.len() != x.len() {
            return Err(PlotError::Broadcast {
                argument: "s",
                len: sizes.len(),
                points: x.len(),
            });
        }

        let mut collection =
            CollectionArtist::new(CollectionKind::Path, vec![shape], Rc::clone(&rc));
        let mut edges = options
            .edgecolors
            .unwrap_or_else(|| scatter_edges(&rc.scatter_edgecolors));
        let (faces, mapping) = match options.colors {
            ScatterColors::Cycle => (Some(vec![self.next_patch_color()]), None),
            ScatterColors::Single(color) => (Some(vec![color]), None),
            ScatterColors::PerPoint(colors) => (Some(colors), None),
            ScatterColors::Mapped(mapped) => {
                if mapped.values.len() != x.len() {
                    return Err(PlotError::Broadcast {
                        argument: "c",
                        len: mapped.values.len(),
                        points: x.len(),
                    });
                }
                (None, Some(mapped.into_mapping(&rc)?))
            }
        };
        let mut faces = faces;
        let mut linewidths = options.linewidths;
        if !style.is_filled() {
            // Unfilled markers are drawn with their face colour as edges.
            edges = if style.fillstyle() == FillStyle::None {
                let edge_colors = faces.take().unwrap_or_default();
                faces = Some(vec![Color::none()]);
                EdgeColors::Colors(edge_colors)
            } else {
                EdgeColors::Face
            };
            linewidths.get_or_insert_with(|| vec![rc.lines_linewidth]);
        }
        collection.offsets = masked_offsets(x, y, &sizes, mapping.as_ref(), options.plot_nonfinite);
        collection.sizes = Some(sizes);
        collection.facecolors = faces;
        collection.edgecolors = edges;
        collection.linewidths = linewidths;
        let base = collection.base_mut();
        base.alpha = options.alpha;
        base.transform = Some(Transform::Display);
        collection.offset_transform = Some(self.trans_data());
        // Marker shapes live in screen space: only the offsets bound the data.
        let update = DataUpdate::data(collection.offsets.clone());
        let handle = Collection::from_raw(self.adopt(collection, &update));
        if let Some(mapping) = mapping {
            handle.set_mapping(mapping);
        }
        Ok(handle)
    }
}

/// The points as offsets, with points that have a non-finite coordinate,
/// size or (unless `plot_nonfinite`) mapped value moved to NaN so they are
/// neither drawn nor counted in the data limits (matplotlib's
/// `cbook._combine_masks`).
fn masked_offsets(
    x: &[f64],
    y: &[f64],
    sizes: &[f64],
    mapping: Option<&ColorMapping>,
    plot_nonfinite: bool,
) -> Vec<Point> {
    let per_point_sizes = (sizes.len() == x.len()).then_some(sizes);
    let mapped_values = mapping.filter(|_| !plot_nonfinite).map(|m| &m.values);
    x.iter()
        .zip(y)
        .enumerate()
        .map(|(i, (&x, &y))| {
            let bad_size = per_point_sizes.is_some_and(|s| !s[i].is_finite());
            let bad_value = mapped_values.is_some_and(|v| v[i].is_none());
            if bad_size || bad_value {
                Point::new(f64::NAN, f64::NAN)
            } else {
                Point::new(x, y)
            }
        })
        .collect()
}

/// `scatter.edgecolors`: `"face"` or a colour.
fn scatter_edges(setting: &str) -> EdgeColors {
    if setting == "face" {
        EdgeColors::Face
    } else {
        EdgeColors::Colors(vec![Color::from(setting)])
    }
}
