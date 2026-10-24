//! Hexagonal binning of scattered points (matplotlib's `hexbin`).

use plt_core::{Path, Point};

use super::children::DataUpdate;
use super::hexbin_marginals::Marginals;
use super::{Axes, AxesState, AxisName};
use crate::artist::Artist;
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, ColorMapping, EdgeColors,
};
use crate::colors::{Colorize, ColorizerOptions, LogNorm, impl_colorize};
use crate::image::{Extent, ImageError};
use crate::scale::LogScale;
use crate::transforms::{Transform, nonsingular};

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

/// How the values of the points in a hexagon combine (matplotlib's
/// `reduce_C_function`).
#[derive(Clone, Copy, Debug, Default)]
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
    /// Any reduction.
    Custom(fn(&[f64]) -> f64),
}

impl HexReduce {
    pub(super) fn apply(self, values: &[f64]) -> f64 {
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

/// The hexagon lattice: two interleaved rectangular grids.
struct Lattice {
    nx: usize,
    ny: usize,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    sx: f64,
    sy: f64,
}

impl Lattice {
    /// The hexagon index of a point (counting both grids, `None` outside),
    /// as matplotlib computes it.
    fn index(&self, tx: f64, ty: f64) -> Option<usize> {
        let (nx1, ny1) = (self.nx + 1, self.ny + 1);
        let ix = (tx - self.xmin) / self.sx;
        let iy = (ty - self.ymin) / self.sy;
        let (ix1, iy1) = (ix.round_ties_even(), iy.round_ties_even());
        let (ix2, iy2) = (ix.floor(), iy.floor());
        let d1 = (ix - ix1).powi(2) + 3.0 * (iy - iy1).powi(2);
        let d2 = (ix - ix2 - 0.5).powi(2) + 3.0 * (iy - iy2 - 0.5).powi(2);
        let cell = |i: f64, j: f64, ni: usize, nj: usize| {
            let inside = (0.0..ni as f64).contains(&i) && (0.0..nj as f64).contains(&j);
            inside.then(|| i as usize * nj + j as usize)
        };
        if d1 < d2 {
            cell(ix1, iy1, nx1, ny1)
        } else {
            cell(ix2, iy2, self.nx, self.ny).map(|k| nx1 * ny1 + k)
        }
    }

    /// The centre of hexagon `k`.
    fn centre(&self, k: usize) -> Point {
        let (nx1, ny1) = (self.nx + 1, self.ny + 1);
        let (x, y) = if k < nx1 * ny1 {
            ((k / ny1) as f64, (k % ny1) as f64)
        } else {
            let k = k - nx1 * ny1;
            ((k / self.ny) as f64 + 0.5, (k % self.ny) as f64 + 0.5)
        };
        Point::new(x * self.sx + self.xmin, y * self.sy + self.ymin)
    }

    fn len(&self) -> usize {
        (self.nx + 1) * (self.ny + 1) + self.nx * self.ny
    }

    /// The outline of a hexagon around the origin, in data units.
    fn hexagon(&self, xscale: HexScale, yscale: HexScale) -> Path {
        let corners = [
            (0.5, -0.5),
            (0.5, 0.5),
            (0.0, 1.0),
            (-0.5, 0.5),
            (-0.5, -0.5),
            (0.0, -1.0),
        ];
        Path::polygon(corners.map(|(vx, vy)| {
            Point::new(
                unscale(self.sx * vx, xscale),
                unscale(self.sy / 3.0 * vy, yscale),
            )
        }))
    }

    /// The lower-left and upper-right corners of the binned region, in
    /// data units.
    fn corners(&self, xscale: HexScale, yscale: HexScale) -> [Point; 2] {
        [
            Point::new(unscale(self.xmin, xscale), unscale(self.ymin, yscale)),
            Point::new(unscale(self.xmax, xscale), unscale(self.ymax, yscale)),
        ]
    }
}

impl Axes {
    /// Bins points into hexagons coloured by their count, or by the
    /// reduced values of their points (matplotlib's `hexbin`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::{HexGridSize, HexbinOptions};
    /// let ax = Figure::new().subplot();
    /// let options = HexbinOptions { gridsize: HexGridSize::Count(5), ..Default::default() };
    /// ax.hexbin(&[0.0, 1.0, 1.0, 2.0], &[0.0, 1.0, 1.0, 0.5], options)?;
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When x, y and the values differ in length, the grid size is zero,
    /// the extent is inverted, log binning meets non-positive data, or the
    /// colormap options are invalid.
    pub fn hexbin(
        &self,
        x: &[f64],
        y: &[f64],
        options: HexbinOptions,
    ) -> Result<Collection, ImageError> {
        let Points {
            x,
            y,
            values: weights,
        } = kept_points(x, y, options.values.as_deref())?;
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
            return Err(ImageError::GridTooLarge(nx, ny));
        }
        let tx = scaled(&x, options.xscale)?;
        let ty = scaled(&y, options.yscale)?;
        let lattice = lattice(&tx, &ty, options.extent, nx, ny)?;
        let accum = accumulate(
            &lattice,
            &tx,
            &ty,
            weights.as_deref(),
            options.reduce,
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
                let log = LogScale::new(10.0).expect("10 is a valid logarithm base");
                self.set_scale(name, Box::new(log));
            }
        }
        let corners = lattice.corners(options.xscale, options.yscale);
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

/// The data in binning coordinates (decades for log binning).
fn scaled(values: &[f64], scale: HexScale) -> Result<Vec<f64>, ImageError> {
    match scale {
        HexScale::Linear => Ok(values.to_vec()),
        HexScale::Log if values.iter().any(|&v| v <= 0.0) => Err(ImageError::InvalidCoordinates),
        HexScale::Log => Ok(values.iter().map(|v| v.log10()).collect()),
    }
}

/// A binning coordinate back in data units.
fn unscale(value: f64, scale: HexScale) -> f64 {
    match scale {
        HexScale::Linear => value,
        HexScale::Log => 10.0_f64.powf(value),
    }
}

/// The lattice over the extent, or over the (non-singular) data range
/// widened by a tiny padding along x.
fn lattice(
    tx: &[f64],
    ty: &[f64],
    extent: Option<Extent>,
    nx: usize,
    ny: usize,
) -> Result<Lattice, ImageError> {
    let range = |v: &[f64]| {
        if v.is_empty() {
            (0.0, 1.0)
        } else {
            let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            nonsingular(lo, hi, 0.1, 1e-15, true)
        }
    };
    let ((mut xmin, mut xmax), (ymin, ymax)) = match extent {
        Some(e) if e.left > e.right || e.bottom > e.top => {
            return Err(ImageError::InvertedExtent);
        }
        Some(e) => ((e.left, e.right), (e.bottom, e.top)),
        None => (range(tx), range(ty)),
    };
    let padding = 1.0e-9 * (xmax - xmin);
    xmin -= padding;
    xmax += padding;
    Ok(Lattice {
        nx,
        ny,
        xmin,
        xmax,
        ymin,
        ymax,
        sx: (xmax - xmin) / nx as f64,
        sy: (ymax - ymin) / ny as f64,
    })
}

/// Per-hexagon counts (`mincnt` turns sparse ones into NaN) or reduced
/// values (NaN with fewer than `mincnt`, default 1, points).
fn accumulate(
    lattice: &Lattice,
    tx: &[f64],
    ty: &[f64],
    values: Option<&[f64]>,
    reduce: HexReduce,
    mincnt: Option<usize>,
) -> Vec<f64> {
    let mut members: Vec<Vec<f64>> = vec![Vec::new(); lattice.len()];
    for (i, (&x, &y)) in tx.iter().zip(ty).enumerate() {
        if let Some(k) = lattice.index(x, y) {
            members[k].push(values.map_or(1.0, |v| v[i]));
        }
    }
    if values.is_some() {
        let min = mincnt.unwrap_or(1);
        return members
            .iter()
            .map(|m| {
                if m.len() >= min {
                    reduce.apply(m)
                } else {
                    f64::NAN
                }
            })
            .collect();
    }
    members
        .iter()
        .map(|m| {
            let count = m.len() as f64;
            if mincnt.is_some_and(|min| count < min as f64) {
                f64::NAN
            } else {
                count
            }
        })
        .collect()
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

    fn unit_lattice() -> Lattice {
        Lattice {
            nx: 2,
            ny: 1,
            xmin: 0.0,
            xmax: 2.0,
            ymin: 0.0,
            ymax: 1.0,
            sx: 1.0,
            sy: 1.0,
        }
    }

    #[test]
    fn points_fall_into_the_nearest_hexagon() {
        let lattice = unit_lattice();
        assert_eq!(lattice.len(), 3 * 2 + 2);
        assert_eq!(lattice.index(0.0, 0.0), Some(0));
        assert_eq!(
            lattice.index(0.5, 0.5),
            Some(6),
            "centre of the offset grid"
        );
        assert_eq!(lattice.index(1.1, 0.9), Some(3));
        assert_eq!(lattice.index(-5.0, 0.0), None);
        assert_eq!(lattice.centre(7), Point::new(1.5, 0.5));
    }

    #[test]
    fn counts_values_and_mincnt() {
        let lattice = unit_lattice();
        let x = [0.0, 0.1, 0.5];
        let y = [0.0, 0.0, 0.5];
        let counts = accumulate(&lattice, &x, &y, None, HexReduce::Mean, None);
        assert_eq!(counts[0], 2.0);
        assert_eq!(counts[1], 0.0, "empty hexagons are kept without mincnt");
        let sparse = accumulate(&lattice, &x, &y, None, HexReduce::Mean, Some(2));
        assert!(sparse[6].is_nan());
        let means = accumulate(
            &lattice,
            &x,
            &y,
            Some(&[1.0, 3.0, 5.0]),
            HexReduce::Mean,
            None,
        );
        assert_eq!((means[0], means[6]), (2.0, 5.0));
        assert!(means[1].is_nan(), "hexagons without points have no value");
    }

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
            Some(ImageError::GridTooLarge(1 << 20, 605_395))
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
