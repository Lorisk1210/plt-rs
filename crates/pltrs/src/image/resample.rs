//! matplotlib's `_resample`: keeps huge data within Agg's limits, picks
//! the kernel for adaptive interpolation, breaks nearest-neighbour ties
//! towards the left/upper pixel and runs the Agg resampler of
//! `pltrs-raster`; and matplotlib's public `image.resample`, which runs the
//! resampler as it is.

use std::borrow::Cow;

use pltrs_core::{Affine2D, Point};
use pltrs_raster::resample::{self as raster, Pixel, PixelTransform, ResampleParams};

use super::{ImageError, Interpolation, Origin};
use crate::numeric::Grid;
use crate::transforms::ResolvedTransform;

/// The settings of [`resample`] (the keyword arguments of matplotlib's
/// `image.resample`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResampleOptions {
    /// The kernel; the adaptive choices and `none` sample the nearest
    /// pixel (nearest by default).
    pub interpolation: Interpolation,
    /// Agg's full resampling for affine transforms, whose kernel grows
    /// with the downsampling factor (off by default).
    pub resample: bool,
    /// The opacity, in `[0, 1]`, of every pixel written (1 by default).
    pub alpha: f64,
    /// Normalised kernel weights (off by default).
    pub norm: bool,
    /// The kernel radius of `sinc`, `lanczos` and `blackman` (1 by
    /// default).
    pub radius: f64,
}

impl Default for ResampleOptions {
    fn default() -> Self {
        Self {
            interpolation: Interpolation::Nearest,
            resample: false,
            alpha: 1.0,
            norm: false,
            radius: 1.0,
        }
    }
}

/// Resamples `data` into `out` through `transform`, from source to output
/// pixel coordinates, blending what it writes over `out` (matplotlib's
/// `image.resample`). Affine transforms map pixels directly; others go
/// through a lookup mesh of the source position of every output pixel
/// centre. Unlike images drawn on axes, nothing is adjusted: the kernel is
/// used as given, and nearest-neighbour ties go the way Agg breaks them.
///
/// ```
/// use pltrs::image::{ResampleOptions, resample};
/// use pltrs::numeric::Grid;
/// use pltrs::transforms::{Affine2D, ResolvedTransform};
/// let data = Grid::from_vec(1, 3, vec![0.1, 0.3, 0.2])?;
/// let mut out = Grid::filled(1, 10, 0.0);
/// let stretch = ResolvedTransform::affine(Affine2D::scaling(10.0 / 3.0, 1.0));
/// resample(&data, &mut out, &stretch, ResampleOptions::default())?;
/// assert_eq!(out.row(0), &[0.1, 0.1, 0.1, 0.3, 0.3, 0.3, 0.3, 0.2, 0.2, 0.2]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the transform cannot be inverted, a grid is empty or too large for
/// Agg, or `alpha` or `radius` is out of range.
pub fn resample(
    data: &Grid<f64>,
    out: &mut Grid<f64>,
    transform: &ResolvedTransform,
    options: ResampleOptions,
) -> Result<(), ImageError> {
    let (out_rows, out_cols) = out.shape();
    let map = PixelMap::from_resolved(transform.clone());
    resample_raster(data, out, map.pixel_transform(out_cols, out_rows)?, options)
}

/// Where [`resample_pixels`] takes the output pixels from.
#[derive(Clone, Debug, PartialEq)]
pub enum PixelMapping {
    /// An affine map from source to output pixel coordinates.
    Affine(Affine2D),
    /// The source position of the centre of every output pixel, in a grid
    /// of the output's shape: what matplotlib computes for a non-affine
    /// transform by mapping the centres back through its inverse.
    Mesh(Grid<[f64; 2]>),
}

/// [`resample`] for any of the pixel types of matplotlib's
/// `_image.resample`: scalars (`u8`, `f32`, `f64`) or RGBA pixels (`[u8;
/// 4]`, `[f32; 4]`, `[f64; 4]`), through an affine map or the lookup mesh
/// matplotlib computes for other transforms.
///
/// ```
/// use pltrs::image::{PixelMapping, ResampleOptions, resample_pixels};
/// use pltrs::numeric::Grid;
/// use pltrs::transforms::Affine2D;
/// let data = Grid::from_vec(1, 2, vec![[255_u8, 0, 0, 255], [0, 0, 255, 255]])?;
/// let mut out = Grid::filled(1, 4, [0; 4]);
/// let stretch = PixelMapping::Affine(Affine2D::scaling(2.0, 1.0));
/// resample_pixels(&data, &mut out, &stretch, ResampleOptions::default())?;
/// assert_eq!(out.row(0)[1], [255, 0, 0, 255]);
/// assert_eq!(out.row(0)[2], [0, 0, 255, 255]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// As [`resample`]; also when a mesh does not have the output's shape.
pub fn resample_pixels<P: Pixel>(
    data: &Grid<P>,
    out: &mut Grid<P>,
    mapping: &PixelMapping,
    options: ResampleOptions,
) -> Result<(), ImageError> {
    let transform = match mapping {
        PixelMapping::Affine(affine) => PixelTransform::Affine(*affine),
        PixelMapping::Mesh(mesh) => {
            let (rows, cols) = mesh.shape();
            if (rows, cols) != out.shape() {
                return Err(ImageError::ShapeMismatch {
                    what: "mesh",
                    expected: out.shape(),
                    actual: (rows, cols),
                });
            }
            PixelTransform::Mesh(raster::Grid::from_vec(
                cols,
                rows,
                mesh.as_slice().to_vec(),
            )?)
        }
    };
    resample_raster(data, out, transform, options)
}

/// Runs Agg's resampler from `data` into `out` as `options` say.
fn resample_raster<P: Pixel>(
    data: &Grid<P>,
    out: &mut Grid<P>,
    transform: PixelTransform,
    options: ResampleOptions,
) -> Result<(), ImageError> {
    let (rows, cols) = data.shape();
    let (out_rows, out_cols) = out.shape();
    let src = raster::GridRef::new(cols, rows, data.as_slice())?;
    let mut dst = raster::Grid::from_vec(out_cols, out_rows, out.as_slice().to_vec())?;
    let params = ResampleParams::new(transform)
        .with_interpolation(options.interpolation.kernel())
        .with_resample(options.resample)
        .with_alpha(options.alpha)
        .with_norm(options.norm)
        .with_radius(options.radius);
    raster::resample(src, &mut dst, &params)?;
    out.as_mut_slice().copy_from_slice(dst.as_slice());
    Ok(())
}

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

pltrs_core::archive_struct!(Resampling {
    interpolation,
    resample,
    filternorm,
    filterrad,
    origin
});

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

    /// The map as an affine transformation, when it is one.
    pub(crate) fn affine(&self) -> Option<Affine2D> {
        match self {
            Self::Affine(affine) => Some(*affine),
            Self::General(_) => None,
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
        let (data, mut map) = within_agg_limits(data, map);
        let (rows, cols) = data.shape();
        let interpolation = self.resolved_interpolation(rows, cols, &map);
        let mut flipped = None;
        // Agg picks the right-hand pixel on exact ties; flipping the data
        // and the transform makes it pick the left (and upper) one.
        if interpolation == Interpolation::Nearest {
            let upper = self.origin == Origin::Upper;
            let mut pixels = Vec::with_capacity(rows * cols);
            for r in 0..rows {
                let row = if upper { rows - 1 - r } else { r };
                pixels.extend(data.row(row).iter().rev());
            }
            flipped = Some(pixels);
            map = map.after(Affine2D::translation(-(cols as f64), 0.0).scale(-1.0, 1.0));
            if upper {
                map = map.after(Affine2D::translation(0.0, -(rows as f64)).scale(1.0, -1.0));
            }
        }
        let source = flipped.as_deref().unwrap_or(data.as_slice());
        let (out_rows, out_cols) = out_shape;
        let src = raster::GridRef::new(cols, rows, source)?;
        let mut dst = raster::Grid::filled(out_cols, out_rows, P::default())?;
        // matplotlib's wrapper passes the radius as a C float.
        let radius = f64::from(self.filterrad as f32);
        let params = ResampleParams::new(map.pixel_transform(out_cols, out_rows)?)
            .with_interpolation(interpolation.kernel())
            .with_resample(resample.unwrap_or(self.resample))
            .with_norm(self.filternorm)
            .with_radius(radius);
        raster::resample(src, &mut dst, &params)?;
        Ok(Grid::from_vec(out_rows, out_cols, dst.into_vec())
            .expect("the destination grid has the requested shape"))
    }
}

/// The most columns and rows Agg resamples: its coordinates are 24-bit
/// signed integers.
const AGG_LIMITS: (usize, usize) = (1 << 24, 1 << 23);

/// `data` with only every `step`-th row and column kept where it has more
/// rows or columns than Agg handles, and `map` scaled to match
/// (matplotlib's `_resample` downsamples such data, with a warning).
fn within_agg_limits<'a, P: Pixel>(
    data: &'a Grid<P>,
    map: &PixelMap,
) -> (Cow<'a, Grid<P>>, PixelMap) {
    let (rows, cols) = data.shape();
    let step = |n: usize, limit: usize| if n > limit { n.div_ceil(limit) } else { 1 };
    let (row_step, col_step) = (step(rows, AGG_LIMITS.0), step(cols, AGG_LIMITS.1));
    if (row_step, col_step) == (1, 1) {
        return (Cow::Borrowed(data), map.clone());
    }
    let kept = Grid::from_fn(rows.div_ceil(row_step), cols.div_ceil(col_step), |r, c| {
        data[(r * row_step, c * col_step)]
    });
    let scaled = map
        .clone()
        .after(Affine2D::scaling(col_step as f64, row_step as f64));
    (Cow::Owned(kept), scaled)
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
    fn data_beyond_agg_limits_keeps_every_other_column() {
        // Odd columns are 1: keeping only the even ones leaves zeros.
        let cols = (1 << 23) + 2;
        let data = Grid::from_fn(1, cols, |_, c| (c % 2) as f64);
        let map = PixelMap::Affine(Affine2D::scaling(8.0 / cols as f64, 1.0));
        let nearest = settings(Interpolation::Nearest, Origin::Lower);
        let out = nearest.run(&data, (1, 8), &map, None).unwrap();
        assert_eq!(out.row(0), &[0.0; 8]);
        let (kept, scaled) = within_agg_limits(&data, &map);
        assert_eq!(kept.shape(), (1, (1 << 22) + 1));
        assert_eq!(scaled.apply(Point::new(1.0, 1.0)).x, 16.0 / cols as f64);
    }

    #[test]
    fn public_resampling_takes_non_affine_transforms() {
        // matplotlib's test_resample_nonaffine: the identity in front of
        // the affine stretch makes no difference to nearest sampling.
        #[derive(Debug)]
        struct Identity;
        impl crate::transforms::PointMap for Identity {
            fn forward(&self, point: Point) -> Point {
                point
            }
            fn inverse(&self, point: Point) -> Point {
                point
            }
        }
        let data = Grid::from_vec(1, 6, vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6]).unwrap();
        let stretch = Affine2D::scaling(1.5, 1.0);
        let general = ResolvedTransform::with_step(
            crate::transforms::Step::Plane(std::rc::Rc::new(Identity)),
            stretch,
        );
        let mut out = Grid::filled(1, 9, 0.0);
        resample(&data, &mut out, &general, ResampleOptions::default()).unwrap();
        assert_eq!(out.row(0), &[0.1, 0.2, 0.2, 0.3, 0.4, 0.4, 0.5, 0.6, 0.6]);
        let mut bad = Grid::filled(1, 9, 0.0);
        let options = ResampleOptions {
            alpha: 2.0,
            ..ResampleOptions::default()
        };
        assert!(
            resample(
                &data,
                &mut bad,
                &ResolvedTransform::affine(stretch),
                options
            )
            .is_err()
        );
    }

    #[test]
    fn pixels_of_every_kind_resample_through_affine_maps_and_meshes() {
        let data = Grid::from_vec(1, 3, vec![0.1, 0.3, 0.2]).unwrap();
        let stretch = Affine2D::scaling(10.0 / 3.0, 1.0);
        let mut affine = Grid::filled(1, 10, 0.0);
        resample_pixels(
            &data,
            &mut affine,
            &PixelMapping::Affine(stretch),
            ResampleOptions::default(),
        )
        .unwrap();
        // The mesh of the same stretch: every output centre mapped back.
        let inverse = stretch.inverted().unwrap();
        let mesh = Grid::from_fn(1, 10, |r, c| {
            let p = inverse.transform_point(Point::new(c as f64 + 0.5, r as f64 + 0.5));
            [p.x, p.y]
        });
        let mut meshed = Grid::filled(1, 10, 0.0);
        resample_pixels(
            &data,
            &mut meshed,
            &PixelMapping::Mesh(mesh),
            ResampleOptions::default(),
        )
        .unwrap();
        assert_eq!(affine, meshed);
        assert_eq!(affine.row(0)[..4], [0.1, 0.1, 0.1, 0.3]);
        let colours = Grid::from_vec(1, 1, vec![[10_u8, 20, 30, 255]]).unwrap();
        let mut out = Grid::filled(2, 2, [0; 4]);
        let doubled = PixelMapping::Affine(Affine2D::scaling(2.0, 2.0));
        resample_pixels(&colours, &mut out, &doubled, ResampleOptions::default()).unwrap();
        assert!(out.as_slice().iter().all(|&p| p == [10, 20, 30, 255]));
    }

    #[test]
    fn a_mesh_must_have_the_output_shape() {
        let data = Grid::filled(2, 2, 1.0_f32);
        let mut out = Grid::filled(2, 3, 0.0_f32);
        let mesh = PixelMapping::Mesh(Grid::filled(1, 2, [0.5, 0.5]));
        let err = resample_pixels(&data, &mut out, &mesh, ResampleOptions::default());
        assert!(matches!(
            err,
            Err(ImageError::ShapeMismatch {
                what: "mesh",
                expected: (2, 3),
                actual: (1, 2),
            })
        ));
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
