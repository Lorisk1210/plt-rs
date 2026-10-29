//! The marginal bars of `hexbin` (matplotlib's `marginals=True`): along the
//! bottom and the left edge of the axes, the values of the points in each
//! bin of x and of y, reduced like the hexagons and coloured through the
//! same colormap.

use std::borrow::Cow;

use pltrs_core::{Path, Point};

use super::children::DataUpdate;
use super::hexbin::{HexReduce, HexScale};
use super::{Axes, AxisName};
use crate::artist::Artist;
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, ColorMapping, EdgeColors,
};
use crate::colors::{NormSpec, ScalarMappable};
use crate::numeric::{geomspace, linspace};

/// How far the bars reach into the axes, as a fraction of it.
const DEPTH: f64 = 0.05;

/// What the marginals of one `hexbin` call are made of.
pub(super) struct Marginals<'a> {
    /// The x of every point.
    pub(super) x: &'a [f64],
    /// The y of every point.
    pub(super) y: &'a [f64],
    /// One value per point (`None` counts the points).
    pub(super) weights: Option<&'a [f64]>,
    /// The lower-left and upper-right corners of the binned region, in data
    /// units.
    pub(super) corners: [Point; 2],
    /// The hexagons along x and y: the marginals have `nx` and `2 ny` bins.
    pub(super) gridsize: (usize, usize),
    /// The binning scales along x and y.
    pub(super) scales: (HexScale, HexScale),
    /// How the values in a bin combine.
    pub(super) reduce: HexReduce,
}

impl Marginals<'_> {
    /// The bins along x and along y.
    fn bins(&self) -> [Marginal<'_>; 2] {
        let [lo, hi] = self.corners;
        let (nx, ny) = self.gridsize;
        [
            Marginal {
                axis: AxisName::X,
                coords: self.x,
                range: (lo.x, hi.x),
                scale: self.scales.0,
                nbins: nx,
            },
            Marginal {
                axis: AxisName::Y,
                coords: self.y,
                range: (lo.y, hi.y),
                scale: self.scales.1,
                nbins: 2 * ny,
            },
        ]
    }
}

/// The bins of one marginal.
struct Marginal<'a> {
    /// The axis the bars run along.
    axis: AxisName,
    /// Every point's coordinate along that axis.
    coords: &'a [f64],
    /// The binned range, in data units.
    range: (f64, f64),
    /// Whether the bins are spaced evenly in decades.
    scale: HexScale,
    /// The number of bins.
    nbins: usize,
}

impl Marginal<'_> {
    /// The bin edges (`np.linspace` or `np.geomspace` over the range).
    fn edges(&self) -> Vec<f64> {
        let (lo, hi) = self.range;
        match self.scale {
            HexScale::Linear => linspace(lo, hi, self.nbins + 1),
            HexScale::Log => geomspace(lo, hi, self.nbins + 1),
        }
    }

    /// Each bin's reduced weights, `None` for bins without points (or with
    /// a NaN result). A point belongs to the bin whose lower edge it
    /// exceeds and whose upper edge it does not (matplotlib's
    /// `np.searchsorted(edges, z) - 1`).
    fn reduced(&self, edges: &[f64], weights: &[f64], reduce: HexReduce) -> Vec<Option<f64>> {
        let mut members = vec![Vec::new(); self.nbins];
        for (&z, &weight) in self.coords.iter().zip(weights) {
            let bin = edges.partition_point(|&edge| edge < z).checked_sub(1);
            if let Some(bin) = bin.filter(|&bin| bin < self.nbins) {
                members[bin].push(weight);
            }
        }
        members
            .iter()
            .map(|m| {
                (!m.is_empty())
                    .then(|| reduce.apply(m))
                    .filter(|v| !v.is_nan())
            })
            .collect()
    }

    /// The bars of the bins with a value, and those values.
    fn bars(&self, weights: &[f64], reduce: HexReduce) -> (Vec<Path>, Vec<Option<f64>>) {
        let edges = self.edges();
        self.reduced(&edges, weights, reduce)
            .into_iter()
            .zip(edges.windows(2))
            .filter_map(|(value, edge)| value.map(|v| (bar(self.axis, edge[0], edge[1]), Some(v))))
            .unzip()
    }
}

/// The bar of the bin from `lo` to `hi` along `axis`, in that axis'
/// transform (data along the axis, axes fraction across it).
fn bar(axis: AxisName, lo: f64, hi: f64) -> Path {
    let corners = [(lo, 0.0), (lo, DEPTH), (hi, DEPTH), (hi, 0.0)];
    Path::polygon(corners.map(|(along, across)| match axis {
        AxisName::X => Point::new(along, across),
        AxisName::Y => Point::new(across, along),
    }))
}

impl Axes {
    /// Adds the bars of both marginals, coloured through the colormap of
    /// the hexagons' `mappable` and styled by the hexagons' `props` with
    /// edges in the face colour; the data limits are left alone. The bars
    /// share the hexagons' norm when it was given as a norm, and get one
    /// of their own fitted to each marginal otherwise: a new norm of the
    /// scale named by `norm`, or a linear one (matplotlib's
    /// `bar.set_norm(norm)` with the hexbin's `norm` argument).
    pub(super) fn add_hexbin_marginals(
        &self,
        marginals: &Marginals<'_>,
        mappable: &ScalarMappable,
        norm: Option<NormSpec>,
        props: &CollectionProps,
    ) {
        let weights: Cow<'_, [f64]> = marginals
            .weights
            .map_or_else(|| Cow::Owned(vec![1.0; marginals.x.len()]), Cow::Borrowed);
        let props = CollectionProps {
            edgecolors: Some(EdgeColors::Face),
            linewidths: None,
            ..props.clone()
        };
        for marginal in marginals.bins() {
            let (paths, values) = marginal.bars(&weights, marginals.reduce);
            let cmap = mappable.cmap().clone();
            let mut mappable = match &norm {
                Some(NormSpec::Norm(_)) => mappable.clone(),
                Some(NormSpec::Name(name)) => ScalarMappable::with_norm(cmap, name.norm()),
                None => ScalarMappable::new(cmap),
            };
            mappable.norm_mut().autoscale_none(&values);
            let mut collection =
                CollectionArtist::new(CollectionKind::Poly, paths, crate::rc::params());
            props.clone().apply(&mut collection);
            collection.base_mut().transform = Some(match marginal.axis {
                AxisName::X => self.trans_xaxis(),
                AxisName::Y => self.trans_yaxis(),
            });
            // Added with `autolim=False`: `relim` leaves the bars out.
            Collection::from_raw(self.adopt(collection, &DataUpdate::default()))
                .set_mapping(ColorMapping { values, mappable });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::{HexGridSize, HexbinOptions};
    use crate::figure::Figure;

    fn marginal(coords: &[f64], scale: HexScale) -> Marginal<'_> {
        Marginal {
            axis: AxisName::X,
            coords,
            range: (1.0, 100.0),
            scale,
            nbins: 2,
        }
    }

    #[test]
    fn bins_are_spaced_linearly_or_in_decades() {
        assert_eq!(
            marginal(&[], HexScale::Linear).edges(),
            vec![1.0, 50.5, 100.0]
        );
        assert_eq!(marginal(&[], HexScale::Log).edges(), vec![1.0, 10.0, 100.0]);
    }

    #[test]
    fn points_on_an_edge_belong_to_the_bin_below_it() {
        let coords = [1.0, 2.0, 10.0, 10.5, 100.0, 500.0];
        let log = marginal(&coords, HexScale::Log);
        let sums = log.reduced(&log.edges(), &[1.0; 6], HexReduce::Sum);
        // 1.0 sits on the lowest edge and 500.0 beyond the highest.
        assert_eq!(sums, vec![Some(2.0), Some(2.0)]);
    }

    #[test]
    fn empty_bins_have_no_bar() {
        let coords = [60.0, 70.0];
        let linear = marginal(&coords, HexScale::Linear);
        let means = linear.reduced(&linear.edges(), &[2.0, 4.0], HexReduce::Mean);
        assert_eq!(means, vec![None, Some(3.0)]);
        let (bars, values) = linear.bars(&[2.0, 4.0], HexReduce::Mean);
        assert_eq!((bars.len(), values), (1, vec![Some(3.0)]));
    }

    #[test]
    fn hexbin_adds_both_marginals_without_changing_the_limits() {
        let x = [0.0, 1.0, 2.0, 2.0];
        let y = [0.0, 3.0, 1.0, 1.0];
        let options = || HexbinOptions {
            gridsize: HexGridSize::Shape(2, 2),
            ..HexbinOptions::default()
        };
        let plain = Figure::new().subplot();
        plain.hexbin(&x, &y, options()).unwrap();
        let ax = Figure::new().subplot();
        let with_marginals = HexbinOptions {
            marginals: true,
            ..options()
        };
        ax.hexbin(&x, &y, with_marginals).unwrap();
        let collections = ax.children_of::<CollectionArtist>();
        assert_eq!(collections.len(), 3, "hexagons, then the x and y bars");
        assert_eq!(ax.data_lim().bbox, plain.data_lim().bbox);
        // Two x bins, both with points. Four y bins from 0 to 3: y = 0 sits
        // on the lowest edge and is left out, as in matplotlib, so only the
        // bins of 1 and 3 get bars.
        let bars = |i: usize| Collection::from_raw(collections[i].clone()).paths();
        assert_eq!((bars(1).len(), bars(2).len()), (2, 2));
        assert_eq!(bars(1)[1].vertices()[1].y, DEPTH);
    }

    #[test]
    fn bars_are_drawn_across_their_axis() {
        let x = bar(AxisName::X, 1.0, 2.0);
        assert_eq!(x.vertices()[2], Point::new(2.0, DEPTH));
        let y = bar(AxisName::Y, 1.0, 2.0);
        assert_eq!(y.vertices()[2], Point::new(DEPTH, 2.0));
    }
}
