//! Hexagonal binning of scattered points (matplotlib's `hexbin`).

use std::fmt;
use std::rc::Rc;

use pltrs_core::Point;

use super::children::DataUpdate;
use super::hexbin_lattice::{accumulate, lattice, scaled};
use super::hexbin_marginals::Marginals;
use super::{Axes, AxesState, AxisName};
use crate::artist::Artist;
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, ColorMapping, EdgeColors,
};
use crate::colors::{Colorize, ColorizerOptions, LogNorm, impl_colorize};
use crate::image::{Extent, ImageError};
use crate::numeric::{Number, to_f64s};
use crate::scale::LogScale;
use crate::ticker::LogBase;
use crate::transforms::Transform;

/// The most hexagons (of both interleaved lattices) a hexbin grid holds,
/// about a grid size of 4000: matplotlib allocates whatever grid is asked
/// for, until memory runs out.
pub const MAX_HEXAGONS: usize = 1 << 24;

/// The number of hexagons along x (and y) of a hexbin grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HexGridSize {
    /// Hexagons along x; along y as many as keep them regular
    /// (`int(nx / sqrt(3))`).
    Count(usize),
    /// Explicit counts along x and y.
    Shape(usize, usize),
}

impl Default for HexGridSize {
    fn default() -> Self {
        Self::Count(100)
    }
}

/// How hexagon values are discretised before colour mapping (matplotlib's
/// `bins`).
#[derive(Clone, Debug, PartialEq)]
pub enum HexBins {
    /// A logarithmic colour scale (`bins='log'`).
    Log,
    /// `n` equal-width bins between the smallest and largest value.
    Count(usize),
    /// Explicit bin edges.
    Edges(Vec<f64>),
}

/// A scale of the binning grid along one axis (matplotlib's `xscale` /
/// `yscale`): log bins hexagons in decades and sets the axis to log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HexScale {
    /// Linear bins.
    #[default]
    Linear,
    /// Logarithmic bins (all data must be positive).
    Log,
}

/// A reduction of the values of the points in one hexagon
/// ([`HexReduce::Custom`]).
pub type HexReduceFn = dyn Fn(&[f64]) -> f64;

/// How the values of the points in a hexagon combine (matplotlib's
/// `reduce_C_function`).
#[derive(Clone, Default)]
pub enum HexReduce {
    /// numpy's `mean`.
    #[default]
    Mean,
    /// numpy's `sum`.
    Sum,
    /// numpy's `median`.
    Median,
    /// The largest value.
    Max,
    /// The smallest value.
    Min,
    /// Any reduction ([`HexReduce::custom`]).
    Custom(Rc<HexReduceFn>),
}

impl fmt::Debug for HexReduce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mean => "Mean",
            Self::Sum => "Sum",
            Self::Median => "Median",
            Self::Max => "Max",
            Self::Min => "Min",
            Self::Custom(_) => "Custom(..)",
        })
    }
}

impl HexReduce {
    /// The reduction `reduce` (a function of the values in one hexagon).
    ///
    /// ```
    /// use pltrs::axes::HexReduce;
    /// let range = HexReduce::custom(|values| {
    ///     let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    ///     let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    ///     max - min
    /// });
    /// assert!(matches!(range, HexReduce::Custom(_)));
    /// ```
    pub fn custom(reduce: impl Fn(&[f64]) -> f64 + 'static) -> Self {
        Self::Custom(Rc::new(reduce))
    }

    pub(super) fn apply(&self, values: &[f64]) -> f64 {
        match self {
            Self::Mean => crate::stats::mean(values),
            Self::Sum => crate::stats::pairwise_sum(values),
            Self::Median => crate::stats::median(values),
            Self::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            Self::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
            Self::Custom(f) => f(values),
        }
    }
}

/// Options of [`Axes::hexbin`].
#[derive(Debug, Default)]
pub struct HexbinOptions {
    /// One value per point, reduced per hexagon (`None` counts points).
    pub values: Option<Vec<f64>>,
    /// How the values in a hexagon combine.
    pub reduce: HexReduce,
    /// Hexagons along x (and y).
    pub gridsize: HexGridSize,
    /// Discretisation of the hexagon values.
    pub bins: Option<HexBins>,
    /// Binning scale along x.
    pub xscale: HexScale,
    /// Binning scale along y.
    pub yscale: HexScale,
    /// The binned region (`None` = the data range).
    pub extent: Option<Extent>,
    /// Only show hexagons with at least this many points.
    pub mincnt: Option<usize>,
    /// Colormap, norm and limits.
    pub colorizer: ColorizerOptions,
    /// Edge colours (default: the face colours) and widths, opacity,
    /// z-order.
    pub props: CollectionProps,
    /// Also draw bars along the bottom and the left edge of the axes
    /// (matplotlib's `marginals`): the values of the points in each of
    /// `nx` bins of x and `2 ny` bins of y, combined by `reduce` (counts
    /// when there are no values) and coloured through the hexagons'
    /// colormap — and through their norm when one is given or `bins` is
    /// [`HexBins::Log`], else through their own. The bars keep the colours
    /// they are created with.
    pub marginals: bool,
}

impl_colorize!(HexbinOptions);

impl Axes {
    /// Bins points into hexagons coloured by their count, or by the
    /// reduced values of their points (matplotlib's `hexbin`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::{HexGridSize, HexbinOptions};
    /// let ax = Figure::new().subplot();
    /// let options = HexbinOptions { gridsize: HexGridSize::Count(5), ..Default::default() };
    /// ax.hexbin(&[0.0, 1.0, 1.0, 2.0], &[0.0, 1.0, 1.0, 0.5], options)?;
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When x, y and the values differ in length, the grid size is zero or
    /// makes more than [`MAX_HEXAGONS`] hexagons, the extent is inverted,
    /// log binning meets non-positive data, or the colormap options are
    /// invalid.
    pub fn hexbin(
        &self,
        x: impl IntoIterator<Item: Number>,
        y: impl IntoIterator<Item: Number>,
        options: HexbinOptions,
    ) -> Result<Collection, ImageError> {
        let Points {
            x,
            y,
            values: weights,
        } = kept_points(&to_f64s(x), &to_f64s(y), options.values.as_deref())?;
        let (nx, ny) = match options.gridsize {
            HexGridSize::Count(nx) => (nx, (nx as f64 / 3.0_f64.sqrt()) as usize),
            HexGridSize::Shape(nx, ny) => (nx, ny),
        };
        if nx == 0 || ny == 0 {
            return Err(ImageError::GridSize);
        }
        let hexagons = (nx.checked_add(1).zip(ny.checked_add(1)))
            .and_then(|(nx1, ny1)| nx1.checked_mul(ny1))
            .and_then(|outer| outer.checked_add(nx.checked_mul(ny)?));
        if hexagons.is_none_or(|count| count > MAX_HEXAGONS) {
            return Err(ImageError::TooManyHexagons(nx, ny));
        }
        let tx = scaled(&x, options.xscale)?;
        let ty = scaled(&y, options.yscale)?;
        let lattice = lattice(&tx, &ty, options.extent, nx, ny)?;
        let accum = accumulate(
            &lattice,
            &tx,
            &ty,
            weights.as_deref(),
            &options.reduce,
            options.mincnt,
        );
        let (offsets, kept): (Vec<Point>, Vec<f64>) = (0..lattice.len())
            .filter(|&k| !accum[k].is_nan())
            .map(|k| (lattice.centre(k), accum[k]))
            .unzip();

        let (colorizer, accum) = binned(options.colorizer, options.bins, kept);
        let marginal_norm = colorizer.norm.clone();
        let mut mappable = colorizer.into_mappable(&crate::rc::params())?;
        let values: Vec<Option<f64>> = accum.iter().map(|&v| Some(v)).collect();
        mappable.norm_mut().autoscale_none(&values);

        let polygon = lattice.hexagon(options.xscale, options.yscale);
        let mut props = options.props.clone();
        props.edgecolors.get_or_insert(EdgeColors::Face);
        props
            .linewidths
            .get_or_insert_with(|| vec![crate::rc::params().patch_linewidth]);
        let mut collection =
            CollectionArtist::new(CollectionKind::Poly, vec![polygon], crate::rc::params());
        props.apply(&mut collection);
        collection.offsets = offsets;
        collection.offset_transform = Some(Transform::AffineDelta(Box::new(self.trans_data())));
        collection.base_mut().transform = Some(self.trans_data());

        for (name, scale) in [(AxisName::X, options.xscale), (AxisName::Y, options.yscale)] {
            if scale == HexScale::Log {
                self.set_scale(name, Box::new(LogScale::from_base(LogBase::TEN)));
            }
        }
        let corners = lattice.corners(options.xscale, options.yscale);
        // Added with `autolim=False` after the data limits: `relim` leaves
        // the cells out.
        let handle =
            Collection::from_raw(self.adopt(collection, &DataUpdate::data(corners.to_vec())));
        self.raw.with_mut::<AxesState, _>(|axes| axes.tight = true);
        if options.marginals {
            let marginals = Marginals {
                x: &x,
                y: &y,
                weights: weights.as_deref(),
                corners,
                gridsize: (nx, ny),
                scales: (options.xscale, options.yscale),
                reduce: options.reduce,
            };
            self.add_hexbin_marginals(&marginals, &mappable, marginal_norm, &options.props);
        }
        handle.set_mapping(ColorMapping { values, mappable });
        Ok(handle)
    }
}

/// The binned points and their optional values.
struct Points {
    x: Vec<f64>,
    y: Vec<f64>,
    values: Option<Vec<f64>>,
}

/// The points (and values) with every coordinate finite (matplotlib's
/// `delete_masked_points`).
fn kept_points(x: &[f64], y: &[f64], values: Option<&[f64]>) -> Result<Points, ImageError> {
    if x.len() != y.len() {
        return Err(ImageError::ValueCount {
            values: y.len(),
            points: x.len(),
        });
    }
    if let Some(values) = values
        && values.len() != x.len()
    {
        return Err(ImageError::ValueCount {
            values: values.len(),
            points: x.len(),
        });
    }
    let keep: Vec<usize> = (0..x.len())
        .filter(|&i| {
            x[i].is_finite() && y[i].is_finite() && values.is_none_or(|v| v[i].is_finite())
        })
        .collect();
    let pick = |a: &[f64]| keep.iter().map(|&i| a[i]).collect::<Vec<f64>>();
    Ok(Points {
        x: pick(x),
        y: pick(y),
        values: values.map(pick),
    })
}

/// The colour options and hexagon values after `bins`: a log norm (taking
/// over `vmin` and `vmax`) unless a norm is given, or the values replaced
/// by their bin indices.
fn binned(
    mut colorizer: ColorizerOptions,
    bins: Option<HexBins>,
    accum: Vec<f64>,
) -> (ColorizerOptions, Vec<f64>) {
    match bins {
        Some(HexBins::Log) if colorizer.norm.is_none() => {
            let mut norm = LogNorm::default();
            if let Some(vmin) = colorizer.vmin.take() {
                norm = norm.with_vmin(vmin);
            }
            if let Some(vmax) = colorizer.vmax.take() {
                norm = norm.with_vmax(vmax);
            }
            (colorizer.norm(norm), accum)
        }
        Some(HexBins::Count(n)) => {
            let edges = equal_bins(&accum, n);
            (colorizer, discretise(&accum, &edges))
        }
        Some(HexBins::Edges(edges)) => (colorizer, discretise(&accum, &edges)),
        _ => (colorizer, accum),
    }
}

/// `n` equal-width bin edges from the smallest to the largest value,
/// excluding the last (`minimum + (maximum - minimum) * arange(n - 1) /
/// (n - 1)`).
fn equal_bins(values: &[f64], n: usize) -> Vec<f64> {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let edges = n.saturating_sub(1);
    (0..edges)
        .map(|i| min + (max - min) * i as f64 / edges as f64)
        .collect()
}

/// Each value replaced by its bin index (`np.sort(bins).searchsorted`).
fn discretise(values: &[f64], edges: &[f64]) -> Vec<f64> {
    let mut sorted = edges.to_vec();
    sorted.sort_by(f64::total_cmp);
    values
        .iter()
        .map(|&v| sorted.partition_point(|&edge| edge < v) as f64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    #[test]
    fn bins_discretise_values() {
        assert_eq!(equal_bins(&[0.0, 10.0], 3), vec![0.0, 5.0]);
        assert_eq!(
            discretise(&[0.0, 4.0, 5.0, 9.0], &[5.0, 0.0]),
            vec![0.0, 1.0, 1.0, 2.0]
        );
    }

    #[test]
    fn invalid_input_is_rejected() {
        let ax = Figure::new().subplot();
        let zero = HexbinOptions {
            gridsize: HexGridSize::Shape(0, 3),
            ..HexbinOptions::default()
        };
        assert_eq!(
            ax.hexbin(&[1.0], &[1.0], zero).err(),
            Some(ImageError::GridSize)
        );
        // Found by fuzzing: a grid size of 2^20 asked for 30 TB.
        let huge = HexbinOptions {
            gridsize: HexGridSize::Count(1 << 20),
            ..HexbinOptions::default()
        };
        assert_eq!(
            ax.hexbin(&[1.0], &[1.0], huge).err(),
            Some(ImageError::TooManyHexagons(1 << 20, 605_395))
        );
        let overflowing = HexbinOptions {
            gridsize: HexGridSize::Shape(usize::MAX, 2),
            ..HexbinOptions::default()
        };
        assert!(ax.hexbin(&[1.0], &[1.0], overflowing).is_err());
        assert!(matches!(
            ax.hexbin(&[1.0, 2.0], &[1.0], HexbinOptions::default()),
            Err(ImageError::ValueCount { .. })
        ));
        let inverted = HexbinOptions {
            extent: Some(Extent::new(1.0, 0.0, 0.0, 1.0)),
            ..HexbinOptions::default()
        };
        assert_eq!(
            ax.hexbin(&[0.5], &[0.5], inverted).err(),
            Some(ImageError::InvertedExtent)
        );
        let log = HexbinOptions {
            xscale: HexScale::Log,
            ..HexbinOptions::default()
        };
        assert!(ax.hexbin(&[0.0], &[1.0], log).is_err());
    }

    #[test]
    fn hexbin_fits_the_view_to_the_lattice() {
        let ax = Figure::new().subplot();
        let options = HexbinOptions {
            gridsize: HexGridSize::Count(4),
            extent: Some(Extent::new(0.0, 4.0, 0.0, 2.0)),
            ..HexbinOptions::default()
        };
        let mesh = ax.hexbin(&[1.0, f64::NAN], &[1.0, 1.0], options).unwrap();
        assert_eq!(mesh.offsets().len(), 5 * 3 + 4 * 2);
        let (x0, x1) = ax.xlim();
        assert!(x0 < 0.0 && x1 > 4.0, "margins around the tight data limits");
        assert_eq!(mesh.sticky_edges().0, Vec::<f64>::new());
    }
}
