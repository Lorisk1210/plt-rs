//! matplotlib's `_resample`: picks the kernel for adaptive interpolation,
//! breaks nearest-neighbour ties towards the left/upper pixel and runs
//! the Agg resampler of `plt-raster`.

use plt_core::{Affine2D, Point};
use plt_raster::resample::{self as raster, Pixel, PixelTransform, ResampleParams};

use super::{ImageError, Interpolation, Origin};
use crate::numeric::Grid;
use crate::transforms::ResolvedTransform;

/// The resampling settings of an image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Resampling {
    /// The interpolation as set on the image.
    pub(crate) interpolation: Interpolation,
    /// Agg's full resampling for downsampling (matplotlib's `resample`).
    pub(crate) resample: bool,
    /// Normalised kernel weights (matplotlib's `filternorm`).
    pub(crate) filternorm: bool,
    /// Kernel radius of the sinc family (matplotlib's `filterrad`).
    pub(crate) filterrad: f64,
    /// Where row 0 of the array is drawn.
    pub(crate) origin: Origin,
}

/// The map from array indices (column, row) to output pixels: affine, or
/// passing through a non-affine data transform.
#[derive(Clone, Debug)]
pub(crate) enum PixelMap {
    /// An affine map, composed exactly as matplotlib's affine chain.
    Affine(Affine2D),
    /// A map with a non-affine part (log axes, projections).
    General(ResolvedTransform),
}

impl PixelMap {
    /// The map of a resolved data transform.
    pub(crate) fn from_resolved(transform: ResolvedTransform) -> Self {
        if transform.is_affine() {
            Self::Affine(transform.affine_part())
        } else {
            Self::General(transform)
        }
    }

    /// `pre` followed by this map (matplotlib's `pre + self`).
    #[must_use]
    pub(crate) fn after(self, pre: Affine2D) -> Self {
        match self {
            Self::Affine(affine) => Self::Affine(pre.then(affine)),
            Self::General(transform) => {
                Self::General(ResolvedTransform::affine(pre).then(transform))
            }
        }
    }

    /// This map followed by `post` (matplotlib's `self + post`).
    #[must_use]
    pub(crate) fn then(self, post: Affine2D) -> Self {
        match self {
            Self::Affine(affine) => Self::Affine(affine.then(post)),
            Self::General(transform) => {
                Self::General(transform.then(ResolvedTransform::affine(post)))
            }
        }
    }

    /// Maps one point.
    pub(crate) fn apply(&self, p: Point) -> Point {
        match self {
            Self::Affine(affine) => affine.transform_point(p),
            Self::General(transform) => transform.transform_point(p),
        }
    }

    /// The resampler's transform for a `width × height` output: the affine
    /// map, or a lookup mesh of the source position of every output pixel
    /// centre (matplotlib's `_get_transform_mesh`).
    fn pixel_transform(&self, width: usize, height: usize) -> Result<PixelTransform, ImageError> {
        let transform = match self {
            Self::Affine(affine) => return Ok(PixelTransform::Affine(*affine)),
            Self::General(transform) => transform,
        };
        let inverse = transform
            .inverted()
            .ok_or(raster::ResampleError::SingularTransform)?;
        let mesh = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| {
                let p = inverse.transform_point(Point::new(x as f64 + 0.5, y as f64 + 0.5));
                [p.x, p.y]
            })
            .collect();
        Ok(PixelTransform::Mesh(raster::Grid::from_vec(
            width, height, mesh,
        )?))
    }
}

impl Resampling {
    /// The kernel for `data` of `rows × cols` mapped by `map`: adaptive
    /// interpolation samples nearest-neighbour when the image is magnified
    /// more than three times or by exactly one or two (in both
    /// directions), and Hanning-filters otherwise.
    #[expect(
        clippy::float_cmp,
        reason = "matplotlib tests for exact integer magnifications"
    )]
    fn resolved_interpolation(&self, rows: usize, cols: usize, map: &PixelMap) -> Interpolation {
        if !self.interpolation.is_adaptive() {
            return self.interpolation;
        }
        let (rows, cols) = (rows as f64, cols as f64);
        let a = map.apply(Point::ORIGIN);
        let b = map.apply(Point::new(cols, rows));
        let (dispx, dispy) = ((b.x - a.x).abs(), (b.y - a.y).abs());
        let fits = |disp: f64, n: f64| disp > 3.0 * n || disp == n || disp == 2.0 * n;
        if fits(dispx, cols) && fits(dispy, rows) {
            Interpolation::Nearest
        } else {
            Interpolation::Hanning
        }
    }

    /// Resamples `data` to an output of `(rows, cols)` pixels through
    /// `map` (matplotlib's `_resample`); `resample` overrides the image's
    /// own flag.
    pub(crate) fn run<P: Pixel>(
        &self,
        data: &Grid<P>,
        out_shape: (usize, usize),
        map: &PixelMap,
        resample: Option<bool>,
    ) -> Result<Grid<P>, ImageError> {
        let (rows, cols) = data.shape();
        let interpolation = self.resolved_interpolation(rows, cols, map);
        let mut map = map.clone();
        let mut flipped = None;
        // Agg picks the right-hand pixel on exact ties; flipping the data
        // and the transform makes it pick the left (and upper) one.
        if interpolation == Interpolation::Nearest {
            let upper = self.origin == Origin::Upper;
            flipped = Some(Grid::from_fn(rows, cols, |r, c| {
                let row = if upper { rows - 1 - r } else { r };
                data[(row, cols - 1 - c)]
            }));
            map = map.after(Affine2D::translation(-(cols as f64), 0.0).scale(-1.0, 1.0));
            if upper {
                map = map.after(Affine2D::translation(0.0, -(rows as f64)).scale(1.0, -1.0));
            }
        }
        let source = flipped.as_ref().unwrap_or(data);
        let (out_rows, out_cols) = out_shape;
        let src = raster::Grid::from_vec(cols, rows, source.as_slice().to_vec())?;
        let mut dst = raster::Grid::filled(out_cols, out_rows, P::default())?;
        // matplotlib's wrapper passes the radius as a C float.
        let radius = f64::from(self.filterrad as f32);
        let params = ResampleParams::new(map.pixel_transform(out_cols, out_rows)?)
            .with_interpolation(interpolation.kernel())
            .with_resample(resample.unwrap_or(self.resample))
            .with_norm(self.filternorm)
            .with_radius(radius);
        raster::resample(&src, &mut dst, &params)?;
        Ok(Grid::from_vec(out_rows, out_cols, dst.into_vec())
            .expect("the destination grid has the requested shape"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(interpolation: Interpolation, origin: Origin) -> Resampling {
        Resampling {
            interpolation,
            resample: true,
            filternorm: true,
            filterrad: 4.0,
            origin,
        }
    }

    #[test]
    fn adaptive_interpolation_depends_on_magnification() {
        let r = settings(Interpolation::Auto, Origin::Upper);
        let map = |s: f64| PixelMap::Affine(Affine2D::scaling(s, s));
        assert_eq!(
            r.resolved_interpolation(10, 10, &map(2.0)),
            Interpolation::Nearest
        );
        assert_eq!(
            r.resolved_interpolation(10, 10, &map(3.5)),
            Interpolation::Nearest
        );
        assert_eq!(
            r.resolved_interpolation(10, 10, &map(2.5)),
            Interpolation::Hanning
        );
        assert_eq!(
            r.resolved_interpolation(10, 10, &map(0.5)),
            Interpolation::Hanning
        );
        let fixed = settings(Interpolation::Bicubic, Origin::Upper);
        assert_eq!(
            fixed.resolved_interpolation(10, 10, &map(0.5)),
            Interpolation::Bicubic
        );
    }

    #[test]
    fn nearest_ties_pick_the_left_and_upper_pixel() {
        let data = Grid::from_vec(2, 2, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        // Three output pixels across two source pixels: the middle one
        // sits exactly on the boundary.
        let map = PixelMap::Affine(Affine2D::scaling(1.5, 1.5));
        let lower = settings(Interpolation::Nearest, Origin::Lower);
        let out = lower.run(&data, (3, 3), &map, None).unwrap();
        assert_eq!(out.row(0), &[1.0, 1.0, 2.0]);
        // With origin lower only columns are flipped: rows keep Agg's rule.
        assert_eq!(out.row(1), &[3.0, 3.0, 4.0]);
        let upper = settings(Interpolation::Nearest, Origin::Upper);
        let out = upper.run(&data, (3, 3), &map, None).unwrap();
        assert_eq!(out.row(1), &[1.0, 1.0, 2.0]);
        let none = settings(Interpolation::None, Origin::Lower);
        let out = none.run(&data, (3, 3), &map, None).unwrap();
        assert_eq!(out.row(0), &[1.0, 2.0, 2.0], "none keeps Agg's tie rule");
    }

    #[test]
    fn general_maps_use_a_lookup_mesh() {
        let affine = Affine2D::scaling(2.0, 1.0);
        let general = PixelMap::General(
            ResolvedTransform::affine(Affine2D::identity()).then(ResolvedTransform::affine(affine)),
        );
        let mesh = general.pixel_transform(2, 1).unwrap();
        let PixelTransform::Mesh(mesh) = mesh else {
            unreachable!("general maps give meshes")
        };
        assert_eq!(mesh.as_slice(), &[[0.25, 0.5], [0.75, 0.5]]);
        let shifted = PixelMap::Affine(affine).after(Affine2D::translation(1.0, 0.0));
        assert_eq!(shifted.apply(Point::ORIGIN), Point::new(2.0, 0.0));
        let then = PixelMap::Affine(affine).then(Affine2D::translation(1.0, 0.0));
        assert_eq!(then.apply(Point::ORIGIN), Point::new(1.0, 0.0));
    }
}
