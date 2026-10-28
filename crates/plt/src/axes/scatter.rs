//! Scatter plots and adding collections.

use std::rc::Rc;

use plt_core::Point;

use super::children::DataUpdate;
use super::data_coords::data_offsets;
use super::{Axes, AxesState, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionArtist, CollectionKind, ColorMapping, EdgeColors};
use crate::colors::{Alpha, Color, ColorizerOptions, impl_colorize};
use crate::markers::{FillStyle, MarkerStyle};
use crate::numeric::{Number, to_f64s};
use crate::rc::RcParams;
use crate::transforms::Transform;
use crate::units::AxisValues;

/// How scatter markers are coloured (matplotlib's `c` / `color`).
#[derive(Debug, Default)]
pub enum ScatterColors {
    /// The next colour of the property cycle.
    #[default]
    Cycle,
    /// One colour for every marker.
    Single(Color),
    /// One colour per marker, or one for all of them; other lengths are
    /// rejected, as matplotlib's `c` and `color` sequences are.
    PerPoint(Vec<Color>),
    /// Values mapped through a colormap and norm.
    Mapped(Box<MappedColors>),
}

impl From<MappedColors> for ScatterColors {
    fn from(mapped: MappedColors) -> Self {
        Self::Mapped(Box::new(mapped))
    }
}

/// One value per marker (or arrow), mapped to colours (scatter's numeric
/// `c` with `cmap`, `norm`, `vmin` and `vmax`, a norm excluding limits).
///
/// ```
/// use plt::axes::MappedColors;
/// use plt::colors::{Colorize, colormaps};
/// let colors = MappedColors::new(vec![1.0, 2.0, 3.0])
///     .cmap(colormaps::get("plasma")?)
///     .limits(Some(0.0), None);
/// assert_eq!(colors.colorizer.vmin, Some(0.0));
/// # Ok::<(), plt::colors::ColormapError>(())
/// ```
#[derive(Clone, Debug)]
pub struct MappedColors {
    /// The values; non-finite ones are masked.
    pub values: Vec<f64>,
    /// The colormap, norm and limits.
    pub colorizer: ColorizerOptions,
}

impl_colorize!(MappedColors);

impl MappedColors {
    /// `values` mapped through `image.cmap` and a linear norm spanning
    /// the finite values.
    pub fn new(values: impl IntoIterator<Item: Number>) -> Self {
        Self {
            values: to_f64s(values),
            colorizer: ColorizerOptions::default(),
        }
    }

    /// The colormap and norm (matplotlib's `set_cmap`, `set_norm` and
    /// `_scale_norm`) plus the values with non-finite ones masked.
    pub(crate) fn into_mapping(self, rc: &RcParams) -> Result<ColorMapping, PlotError> {
        let mappable = self.colorizer.into_mappable(rc)?;
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
    /// Marker shape, fill style and transform; default `scatter.marker`
    /// with the fill style of `markers.fillstyle`.
    pub marker: Option<MarkerStyle>,
    /// Edge colours; default `scatter.edgecolors`.
    pub edgecolors: Option<EdgeColors>,
    /// Edge widths in points.
    pub linewidths: Option<Vec<f64>>,
    /// Opacity: one for every marker or one per marker (matplotlib's
    /// `alpha`, a number or an array); [`Alpha::Keep`] keeps the colours'
    /// own.
    pub alpha: Alpha,
    /// Draw points whose mapped value is not finite in the colormap's
    /// "bad" colour instead of omitting them.
    pub plot_nonfinite: bool,
    /// Coordinates of the points (matplotlib's `transform=`); data by
    /// default. Points in other coordinates do not count for autoscaling.
    pub offset_transform: Option<Transform>,
}

impl Axes {
    /// A scatter plot of `y` against `x` with default styling
    /// (matplotlib's `ax.scatter(x, y)`).
    pub fn scatter(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
    ) -> Result<Collection, PlotError> {
        self.scatter_with(x, y, ScatterOptions::default())
    }

    /// A scatter plot with explicit options.
    pub fn scatter_with(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        options: ScatterOptions,
    ) -> Result<Collection, PlotError> {
        let (x, y) = (x.into(), y.into());
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let x = self.xaxis().convert_units(x);
        let y = self.yaxis().convert_units(y);
        let (x, y) = (x.as_slice(), y.as_slice());
        let rc = crate::rc::params();
        // matplotlib 1.x defaults in classic mode: 20 pt² blue markers
        // with the default patch edges.
        let classic = rc.internal_classic_mode;
        let style = match options.marker {
            Some(style) => style,
            None => MarkerStyle::new(rc.scatter_marker.clone(), rc.markers_fillstyle)?,
        };
        let shape = style.path().transformed(&style.transform());
        let sizes = options.sizes.unwrap_or_else(|| {
            vec![if classic {
                20.0
            } else {
                rc.lines_markersize.powi(2)
            }]
        });
        if sizes.len() != 1 {
            matching("s", sizes.len(), x.len())?;
        }

        let mut collection =
            CollectionArtist::new(CollectionKind::Path, vec![shape], Rc::clone(&rc));
        let mut edges = options.edgecolors.unwrap_or_else(|| {
            if classic {
                EdgeColors::Default
            } else {
                scatter_edges(&rc.scatter_edgecolors)
            }
        });
        let (faces, mapping) = self.scatter_faces(options.colors, x.len(), &rc)?;
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
        collection.offsets_masked = true;
        collection.sizes = Some(sizes);
        collection.facecolors = faces;
        collection.edgecolors = edges;
        collection.linewidths = linewidths;
        apply_alpha(&mut collection, options.alpha, x.len())?;
        let base = collection.base_mut();
        base.transform = Some(Transform::Display);
        if classic && !x.is_empty() {
            // Classic mode leaves room for the markers' size.
            self.raw().with_mut::<AxesState, _>(|axes| {
                for axis in [&mut axes.xaxis, &mut axes.yaxis] {
                    if axis.autoscale.margin < 0.05 {
                        axis.autoscale.margin = 0.05;
                        axis.autoscale.stale = true;
                    }
                }
            });
        }
        // Marker shapes live in screen space: only the offsets bound the
        // data, and only when they are in data coordinates.
        let transform = options
            .offset_transform
            .unwrap_or_else(|| self.trans_data());
        let update = DataUpdate::data(
            data_offsets(&transform, self.id(), &collection.offsets).unwrap_or_default(),
        );
        collection.offset_transform = Some(transform);
        let handle = Collection::from_raw(self.adopt_collection(collection, &update));
        if let Some(mapping) = mapping {
            handle.set_mapping(mapping);
        }
        Ok(handle)
    }
}

impl Axes {
    /// The face colours or the colour mapping of `colors` for `n` points
    /// (matplotlib's `_parse_scatter_color_args`); unset colours are blue
    /// in classic mode.
    fn scatter_faces(
        &self,
        colors: ScatterColors,
        n: usize,
        rc: &RcParams,
    ) -> Result<(Option<Vec<Color>>, Option<ColorMapping>), PlotError> {
        Ok(match colors {
            ScatterColors::Cycle if rc.internal_classic_mode => {
                (Some(vec![Color::from("b")]), None)
            }
            ScatterColors::Cycle => (Some(vec![self.next_patch_color()]), None),
            ScatterColors::Single(color) => (Some(vec![color]), None),
            ScatterColors::PerPoint(colors) => {
                if colors.len() > 1 {
                    matching("c", colors.len(), n)?;
                }
                (Some(colors), None)
            }
            ScatterColors::Mapped(mapped) => {
                matching("c", mapped.values.len(), n)?;
                (None, Some(mapped.into_mapping(rc)?))
            }
        })
    }
}

/// Fails unless argument `argument` holds one value per point.
fn matching(argument: &'static str, len: usize, points: usize) -> Result<(), PlotError> {
    if len == points {
        Ok(())
    } else {
        Err(PlotError::Broadcast {
            argument,
            len,
            points,
        })
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

/// Gives the markers one opacity, or one each when there is one per
/// point (matplotlib's `alpha` number or array).
fn apply_alpha(
    collection: &mut CollectionArtist,
    alpha: Alpha,
    points: usize,
) -> Result<(), PlotError> {
    match alpha {
        Alpha::Keep => {}
        Alpha::Uniform(alpha) => collection.base_mut().alpha = Some(alpha),
        Alpha::Each(alphas) if alphas.len() == points => collection.alphas = Some(alphas),
        Alpha::Each(alphas) => {
            return Err(PlotError::Broadcast {
                argument: "alpha",
                len: alphas.len(),
                points,
            });
        }
    }
    Ok(())
}

/// `scatter.edgecolors`: `"face"` or a colour.
fn scatter_edges(setting: &str) -> EdgeColors {
    if setting == "face" {
        EdgeColors::Face
    } else {
        EdgeColors::Colors(vec![Color::from(setting)])
    }
}

#[cfg(test)]
mod tests {
    use plt_core::Affine2D;

    use super::*;
    use crate::figure::Figure;

    #[test]
    fn per_point_colours_match_the_points_or_are_one() {
        let ax = Figure::new().subplot();
        let colored = |n: usize| ScatterOptions {
            colors: ScatterColors::PerPoint(vec![Color::from("r"); n]),
            ..ScatterOptions::default()
        };
        let x = [0.0, 1.0, 2.0, 3.0];
        assert!(ax.scatter_with(x, x, colored(1)).is_ok());
        assert!(ax.scatter_with(x, x, colored(4)).is_ok());
        for n in [3, 5] {
            assert!(matches!(
                ax.scatter_with(x, x, colored(n)),
                Err(PlotError::Broadcast { argument: "c", .. })
            ));
        }
    }

    #[test]
    fn offsets_in_data_coordinates_count_for_autoscaling() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let at = |transform: Transform| ScatterOptions {
            offset_transform: Some(transform),
            ..ScatterOptions::default()
        };
        ax.scatter_with(vec![0.0], vec![0.0], at(ax.trans_data()))
            .unwrap();
        let shifted = Transform::Affine(Affine2D::translation(5.0, 6.0)).then(ax.trans_data());
        ax.scatter_with(vec![0.0], vec![0.0], at(shifted)).unwrap();
        ax.scatter_with(vec![9.0], vec![9.0], at(ax.trans_axes()))
            .unwrap();
        let limits = ax.data_lim().bbox;
        assert_eq!((limits.x0(), limits.y0()), (0.0, 0.0));
        assert_eq!((limits.x1(), limits.y1()), (5.0, 6.0));
    }
}
