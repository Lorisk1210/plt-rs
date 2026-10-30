//! The hexagon lattice of a hexbin plot (matplotlib's `hexbin`): two
//! interleaved rectangular grids over the binned region, the hexagon each
//! point falls into, and the counts or reduced values per hexagon.

use pltrs_core::{Path, Point};

use super::hexbin::{HexReduce, HexScale};
use crate::image::{Extent, ImageError};
use crate::transforms::nonsingular;

/// The hexagon lattice: two interleaved rectangular grids.
pub(super) struct Lattice {
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
    pub(super) fn centre(&self, k: usize) -> Point {
        let (nx1, ny1) = (self.nx + 1, self.ny + 1);
        let (x, y) = if k < nx1 * ny1 {
            ((k / ny1) as f64, (k % ny1) as f64)
        } else {
            let k = k - nx1 * ny1;
            ((k / self.ny) as f64 + 0.5, (k % self.ny) as f64 + 0.5)
        };
        Point::new(x * self.sx + self.xmin, y * self.sy + self.ymin)
    }

    pub(super) fn len(&self) -> usize {
        (self.nx + 1) * (self.ny + 1) + self.nx * self.ny
    }

    /// The outline of a hexagon around the origin, in data units.
    pub(super) fn hexagon(&self, xscale: HexScale, yscale: HexScale) -> Path {
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
    pub(super) fn corners(&self, xscale: HexScale, yscale: HexScale) -> [Point; 2] {
        [
            Point::new(unscale(self.xmin, xscale), unscale(self.ymin, yscale)),
            Point::new(unscale(self.xmax, xscale), unscale(self.ymax, yscale)),
        ]
    }
}

/// The data in binning coordinates (decades for log binning).
pub(super) fn scaled(values: &[f64], scale: HexScale) -> Result<Vec<f64>, ImageError> {
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
pub(super) fn lattice(
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
pub(super) fn accumulate(
    lattice: &Lattice,
    tx: &[f64],
    ty: &[f64],
    values: Option<&[f64]>,
    reduce: &HexReduce,
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let counts = accumulate(&lattice, &x, &y, None, &HexReduce::Mean, None);
        assert_eq!(counts[0], 2.0);
        assert_eq!(counts[1], 0.0, "empty hexagons are kept without mincnt");
        let sparse = accumulate(&lattice, &x, &y, None, &HexReduce::Mean, Some(2));
        assert!(sparse[6].is_nan());
        let means = accumulate(
            &lattice,
            &x,
            &y,
            Some(&[1.0, 3.0, 5.0]),
            &HexReduce::Mean,
            None,
        );
        assert_eq!((means[0], means[6]), (2.0, 5.0));
        assert!(means[1].is_nan(), "hexagons without points have no value");
    }
}
