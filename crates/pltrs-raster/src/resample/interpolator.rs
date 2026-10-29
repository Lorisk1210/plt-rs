//! Span interpolators: map each destination pixel of a span to a source
//! position in 24.8 fixed point.
//!
//! * [`LinearInterpolator`] is Agg's `span_interpolator_linear`: it
//!   transforms the span's end points and walks between them with a DDA,
//!   built with matplotlib's `MPL_FIX_AGG_INTERPOLATION_ENDPOINT_BUG`.
//! * [`NearestInterpolator`] is matplotlib's `accurate_interpolator_affine_nn`,
//!   which interpolates in floating point and truncates.
//! * [`MeshInterpolator`] is `span_interpolator_adaptor` with matplotlib's
//!   `lookup_distortion`, used for non-affine transforms (and, without a
//!   mesh, for the plain filter path).

use super::grid::Grid;
use super::numeric::{AggAffine, SUBPIXEL_SCALE, iround, trunc};
use crate::dda::Dda2;

/// Computes source coordinates for consecutive pixels of a span.
pub(super) trait SpanInterpolator {
    /// Starts a span of `len` pixels whose first centre is `(x, y)`.
    fn begin(&mut self, x: f64, y: f64, len: u32);
    /// Source position of the current pixel, in 1/256 pixel units.
    fn coordinates(&self) -> (i32, i32);
    /// Moves to the next pixel of the span.
    fn advance(&mut self);
}

/// `span_interpolator_linear<trans_affine, 8>`.
#[derive(Clone, Debug)]
pub(super) struct LinearInterpolator {
    transform: AggAffine,
    x: Dda2,
    y: Dda2,
}

impl LinearInterpolator {
    /// An interpolator through `transform` (destination → source).
    pub(super) fn new(transform: AggAffine) -> Self {
        Self {
            transform,
            x: Dda2::default(),
            y: Dda2::default(),
        }
    }

    /// The destination-to-source transform.
    pub(super) fn transform(&self) -> &AggAffine {
        &self.transform
    }
}

impl SpanInterpolator for LinearInterpolator {
    fn begin(&mut self, x: f64, y: f64, len: u32) {
        let steps = len.saturating_sub(1);
        let scale = f64::from(SUBPIXEL_SCALE);
        let (tx, ty) = self.transform.transform(x, y);
        let (x1, y1) = (iround(tx * scale), iround(ty * scale));
        let (tx, ty) = self.transform.transform(x + f64::from(steps), y);
        let (x2, y2) = (iround(tx * scale), iround(ty * scale));
        let steps = steps as i32;
        self.x = Dda2::new(x1, x2, steps);
        self.y = Dda2::new(y1, y2, steps);
    }

    fn coordinates(&self) -> (i32, i32) {
        (self.x.y, self.y.y)
    }

    fn advance(&mut self) {
        self.x.step();
        self.y.step();
    }
}

/// matplotlib's `accurate_interpolator_affine_nn`.
#[derive(Clone, Debug)]
pub(super) struct NearestInterpolator {
    transform: AggAffine,
    last: u32,
    current: u32,
    start: (f64, f64),
    end: (f64, f64),
}

impl NearestInterpolator {
    /// An interpolator through `transform` (destination → source).
    pub(super) fn new(transform: AggAffine) -> Self {
        Self {
            transform,
            last: 0,
            current: 0,
            start: (0.0, 0.0),
            end: (0.0, 0.0),
        }
    }
}

impl SpanInterpolator for NearestInterpolator {
    fn begin(&mut self, x: f64, y: f64, len: u32) {
        let scale = f64::from(SUBPIXEL_SCALE);
        self.last = len.saturating_sub(1);
        self.current = 0;
        let (sx, sy) = self.transform.transform(x, y);
        self.start = (sx * scale, sy * scale);
        let (ex, ey) = self.transform.transform(x + f64::from(self.last), y);
        self.end = (ex * scale, ey * scale);
    }

    fn coordinates(&self) -> (i32, i32) {
        if self.current == self.last {
            return (trunc(self.end.0), trunc(self.end.1));
        }
        let (current, last) = (f64::from(self.current), f64::from(self.last));
        // The tiny fudge compensates for precision loss, as in matplotlib.
        let lerp = |a: f64, b: f64| trunc(a + (b - a) * current / last + 1e-8);
        (
            lerp(self.start.0, self.end.0),
            lerp(self.start.1, self.end.1),
        )
    }

    fn advance(&mut self) {
        self.current += 1;
    }
}

/// Where in a source pixel the mesh lookup places samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MeshAlignment {
    /// Nearest-neighbour sampling uses edge-aligned sub-pixels.
    Edge,
    /// Filters use (slightly offset) centre-aligned sub-pixels.
    Centre,
}

/// `span_interpolator_adaptor<span_interpolator_linear, lookup_distortion>`.
#[derive(Clone, Debug)]
pub(super) struct MeshInterpolator<'a> {
    linear: LinearInterpolator,
    mesh: Option<&'a Grid<[f64; 2]>>,
    offset: f64,
}

impl<'a> MeshInterpolator<'a> {
    /// Interpolates through `transform`, then replaces each position that
    /// falls inside `mesh` by the mesh entry of the destination pixel.
    pub(super) fn new(
        transform: AggAffine,
        mesh: Option<&'a Grid<[f64; 2]>>,
        alignment: MeshAlignment,
    ) -> Self {
        Self {
            linear: LinearInterpolator::new(transform),
            mesh,
            offset: match alignment {
                MeshAlignment::Edge => 0.0,
                MeshAlignment::Centre => 0.5,
            },
        }
    }
}

impl SpanInterpolator for MeshInterpolator<'_> {
    fn begin(&mut self, x: f64, y: f64, len: u32) {
        self.linear.begin(x, y, len);
    }

    fn coordinates(&self) -> (i32, i32) {
        let (x, y) = self.linear.coordinates();
        let Some(mesh) = self.mesh else {
            return (x, y);
        };
        let scale = f64::from(SUBPIXEL_SCALE);
        let (dx, dy) = (f64::from(x) / scale, f64::from(y) / scale);
        let inside =
            dx >= 0.0 && dx < mesh.width() as f64 && dy >= 0.0 && dy < mesh.height() as f64;
        match mesh.get(dx as usize, dy as usize).filter(|_| inside) {
            // The fudge term mirrors matplotlib's `lookup_distortion`.
            Some(&[mx, my]) => (
                trunc(mx * scale + self.offset + 1e-8),
                trunc(my * scale + self.offset + 1e-8),
            ),
            None => (x, y),
        }
    }

    fn advance(&mut self) {
        self.linear.advance();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(interp: &mut impl SpanInterpolator, x: f64, y: f64, len: u32) -> Vec<(i32, i32)> {
        interp.begin(x, y, len);
        (0..len)
            .map(|_| {
                let c = interp.coordinates();
                interp.advance();
                c
            })
            .collect()
    }

    fn scaled(sx: f64, sy: f64, tx: f64, ty: f64) -> AggAffine {
        AggAffine {
            sx,
            shy: 0.0,
            shx: 0.0,
            sy,
            tx,
            ty,
        }
    }

    #[test]
    fn linear_identity_is_exact() {
        let mut li = LinearInterpolator::new(AggAffine::IDENTITY);
        let coords = walk(&mut li, 3.5, 2.5, 4);
        assert_eq!(
            coords,
            vec![(896, 640), (1152, 640), (1408, 640), (1664, 640)]
        );
        assert_eq!(walk(&mut li, 0.5, 0.5, 1), vec![(128, 128)]);
    }

    #[test]
    fn nearest_truncates_toward_zero() {
        let mut nn = NearestInterpolator::new(scaled(0.5, 1.0, -1.0, 0.0));
        let coords = walk(&mut nn, 0.5, 0.5, 3);
        // Only the last sample is taken verbatim; the others get a 1e-8
        // fudge and truncate toward zero: −192 + 1e-8 → −191,
        // −64 + 1e-8 → −63.
        assert_eq!(coords, vec![(-191, 128), (-63, 128), (64, 128)]);
    }

    #[test]
    fn mesh_replaces_positions_inside_the_grid() {
        let mesh = Grid::from_vec(2, 1, vec![[5.25, 1.5], [f64::NAN, 0.0]]).expect("mesh");
        let mut mi = MeshInterpolator::new(AggAffine::IDENTITY, Some(&mesh), MeshAlignment::Edge);
        let coords = walk(&mut mi, 0.5, 0.5, 2);
        assert_eq!(coords, vec![(1344, 384), (i32::MIN, 0)]);
        let mut mi = MeshInterpolator::new(AggAffine::IDENTITY, None, MeshAlignment::Centre);
        assert_eq!(walk(&mut mi, 0.5, 0.5, 1), vec![(128, 128)]);
    }
}
