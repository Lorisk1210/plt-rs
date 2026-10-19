//! Scatter markers in 3-D: projected offsets drawn back to front, faded
//! with depth (matplotlib's `Path3DCollection`).

use plt_core::{Point, Rgba};

use crate::artist::Artist;
use crate::collections::CollectionArtist;
use crate::mplot3d::camera::Projection;
use crate::mplot3d::math::Vec3;
use crate::mplot3d::shade::depth_alphas;
use crate::numeric::argsort;

/// The 3-D state of a scatter collection.
///
/// The sizes and widths are kept in the order of the points; projecting
/// writes them into the 2-D collection sorted by depth, with its colours
/// (worked out afresh each time, so that later colour changes show)
/// faded and sorted alike.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Points3D {
    /// One position per marker.
    pub(crate) positions: Vec<Vec3>,
    /// Marker areas in points² (one, or one per marker).
    pub(crate) sizes: Option<Vec<f64>>,
    /// Edge widths in points (one, or one per marker).
    pub(crate) linewidths: Option<Vec<f64>>,
    /// Whether markers fade with depth.
    pub(crate) depthshade: bool,
    /// The alpha factor of the deepest markers.
    pub(crate) min_alpha: f64,
}

impl Points3D {
    /// Writes the projected, depth-sorted markers into `collection` and
    /// returns the depth of the nearest one. Colours the collection cannot
    /// resolve are left to be reported when it is drawn.
    pub(crate) fn project(
        &self,
        collection: &mut CollectionArtist,
        projection: &Projection,
    ) -> f64 {
        let projected: Vec<Option<Vec3>> = self
            .positions
            .iter()
            .map(|&p| projection.project_clipped(p))
            .collect();
        let depths: Vec<f64> = projected
            .iter()
            .map(|p| p.map_or(f64::NAN, |p| p[2]))
            .collect();
        let order = deepest_first(&depths);
        let reorder = |values: &[f64]| -> Vec<f64> {
            if values.len() > 1 {
                order.iter().map(|&i| values[i]).collect()
            } else {
                values.to_vec()
            }
        };
        let data_scale = data_scale(&projected);
        let alpha = collection.base().alpha;
        let shade = |colors: &[Rgba]| -> Vec<Rgba> {
            let shaded = self.shaded(colors, &depths, data_scale, alpha);
            if shaded.len() > 1 {
                order.iter().map(|&i| shaded[i]).collect()
            } else {
                shaded
            }
        };
        collection.projected_colors = collection
            .base_colors()
            .ok()
            .map(|(faces, edges)| (shade(&faces), shade(&edges)));
        collection.offsets = order
            .iter()
            .map(|&i| {
                projected[i].map_or(Point::new(f64::NAN, f64::NAN), |p| Point::new(p[0], p[1]))
            })
            .collect();
        collection.sizes = self.sizes.as_deref().map(reorder);
        collection.linewidths = self.linewidths.as_deref().map(reorder);
        depths
            .iter()
            .copied()
            .filter(|z| !z.is_nan())
            .reduce(f64::min)
            .unwrap_or(f64::NAN)
    }

    /// `colors` faded with depth (one colour per marker then) when depth
    /// shading is on, with the collection's `alpha` applied once more on
    /// top, as matplotlib does (`_maybe_depth_shade_and_sort_colors`).
    fn shaded(
        &self,
        colors: &[Rgba],
        depths: &[f64],
        data_scale: f64,
        alpha: Option<f64>,
    ) -> Vec<Rgba> {
        if !self.depthshade || colors.is_empty() || depths.is_empty() {
            return colors.to_vec();
        }
        depth_alphas(depths, self.min_alpha, data_scale)
            .into_iter()
            .enumerate()
            .map(|(i, factor)| {
                let color = colors[if colors.len() == 1 { 0 } else { i }];
                let faded = color.a * factor;
                color.with_alpha(alpha.map_or(faded, |a| a * faded))
            })
            .collect()
    }
}

/// Indices ordering `depths` from the deepest (smallest projected z is
/// nearest) to the nearest, unprojectable points first (numpy's
/// `ma.argsort(vzs)[::-1]`).
fn deepest_first(depths: &[f64]) -> Vec<usize> {
    let filled: Vec<f64> = depths
        .iter()
        .map(|&z| if z.is_nan() { f64::INFINITY } else { z })
        .collect();
    let mut order = argsort(&filled);
    order.reverse();
    order
}

/// The size of the projected point cloud: the length of the diagonal of
/// its bounding box (matplotlib's `_get_data_scale`).
fn data_scale(projected: &[Option<Vec3>]) -> f64 {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut any = false;
    for p in projected.iter().flatten() {
        any = true;
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    if !any {
        return 0.0;
    }
    let ptp = |i: usize| hi[i] - lo[i];
    (ptp(0).powi(2) + ptp(1).powi(2) + ptp(2).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepest_points_come_first() {
        assert_eq!(deepest_first(&[0.2, -0.5, 0.9, f64::NAN]), [3, 2, 0, 1]);
        assert_eq!(deepest_first(&[]), Vec::<usize>::new());
    }

    #[test]
    fn data_scale_is_the_bounding_diagonal() {
        let scale = data_scale(&[Some([0.0, 0.0, 0.0]), None, Some([3.0, 4.0, 0.0])]);
        assert!((scale - 5.0).abs() < 1e-12);
        assert_eq!(data_scale(&[None]), 0.0);
    }

    #[test]
    fn shading_applies_alpha_twice_like_matplotlib() {
        let points = Points3D {
            positions: Vec::new(),
            sizes: None,
            linewidths: None,
            depthshade: true,
            min_alpha: 0.3,
        };
        let red = Rgba::new(1.0, 0.0, 0.0, 0.5);
        let shaded = points.shaded(&[red], &[0.0, 1.0], 2.0, Some(0.5));
        assert_eq!(shaded[0].a, 0.25);
        assert_eq!(shaded[1].a, 0.125);
    }
}
