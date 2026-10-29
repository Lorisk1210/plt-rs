//! The colour blocks of a colorbar: their boundaries and values, the
//! lengths of the extensions and the block edges along the long axis
//! (matplotlib's `_process_values`, `_proportional_y`, `_uniform_y`,
//! `_get_extension_lengths`, `_mesh`, `_locate` and
//! `_forward_boundaries`).

use std::ops::Range;

use super::{ColorbarError, ExtendFrac, Spacing};
use crate::colors::{Extend, Norm, NormError, NormKind, Normalized};
use crate::image::ScalarKind;
use crate::numeric::{interp, linspace};
use crate::transforms::nonsingular;

/// The norm families a colorbar distinguishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NormClass {
    /// Exactly `Normalize`.
    Linear,
    /// `BoundaryNorm`.
    Boundary,
    /// `NoNorm`.
    Identity,
    /// A norm made from a scale (`LogNorm`, ...).
    Scaled,
    /// Any other norm.
    Other,
}

impl NormClass {
    /// The class of `norm`.
    pub(crate) fn of(norm: &dyn Norm) -> Self {
        match norm.kind() {
            NormKind::Linear => Self::Linear,
            NormKind::Boundary(_) => Self::Boundary,
            NormKind::Identity => Self::Identity,
            NormKind::Scaled(_) => Self::Scaled,
            NormKind::Other => Self::Other,
        }
    }
}

/// What the blocks are derived from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BlockSpec<'a> {
    /// The number of colours of the colormap.
    pub(crate) ncolors: usize,
    /// Explicit block edges (user boundaries or contour levels).
    pub(crate) boundaries: Option<&'a [f64]>,
    /// Explicit block values.
    pub(crate) values: Option<&'a [f64]>,
    /// What the explicit values hold: integers (contour colour indices)
    /// index the colormap under `NoNorm`.
    pub(crate) value_kind: ScalarKind,
    /// The extended ends.
    pub(crate) extend: Extend,
    /// Whether the long axis is inverted (extensions swap ends).
    pub(crate) inverted: bool,
    /// The extension lengths.
    pub(crate) extendfrac: ExtendFrac,
    /// The spacing of discrete colorbars.
    pub(crate) spacing: Spacing,
}

/// The colour blocks of a colorbar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Blocks {
    /// The block edges, extensions included (`_boundaries`).
    pub(crate) boundaries: Vec<f64>,
    /// The value coloured in each block (`_values`).
    pub(crate) values: Vec<f64>,
    /// What the values hold: integers index the colormap under `NoNorm`
    /// (numpy's integer `_values`).
    pub(crate) kind: ScalarKind,
    /// The edges inside the extensions (`_inside`).
    pub(crate) inside: Range<usize>,
    /// Whether the lower end is extended.
    pub(crate) lower: bool,
    /// Whether the upper end is extended.
    pub(crate) upper: bool,
    /// The norm's family.
    pub(crate) class: NormClass,
    /// Whether the edges were given explicitly.
    pub(crate) explicit: bool,
    spacing: Spacing,
    extendfrac: ExtendFrac,
}

impl Blocks {
    /// matplotlib's `_process_values`: the blocks, with the limits of
    /// `norm` made non-singular (or 0 and 1 when unset).
    pub(crate) fn new(spec: &BlockSpec<'_>, norm: &mut dyn Norm) -> Result<Self, ColorbarError> {
        let (lower, upper) = Self::extended_ends(spec.extend, spec.inverted);
        let mut blocks = Self {
            boundaries: Vec::new(),
            values: Vec::new(),
            kind: ScalarKind::Float,
            inside: 0..0,
            lower,
            upper,
            class: NormClass::of(norm),
            explicit: spec.boundaries.is_some(),
            spacing: spec.spacing,
            extendfrac: spec.extendfrac,
        };
        blocks.process(spec, norm)?;
        // matplotlib's `_inside` follows `extend`, whatever the direction.
        let len = blocks.boundaries.len();
        let start = usize::from(spec.extend.extends_min()).min(len);
        let end = len
            .saturating_sub(usize::from(spec.extend.extends_max()))
            .max(start);
        blocks.inside = start..end;
        if blocks.inside.len() < 2 {
            return Err(ColorbarError::TooFewBoundaries(blocks.inside.len()));
        }
        Ok(blocks)
    }

    /// Whether the lower and the upper end of the long axis are extended
    /// (matplotlib's `_extend_lower` and `_extend_upper`): an inverted
    /// axis shows the minimum's extension at its upper end.
    pub(crate) fn extended_ends(extend: Extend, inverted: bool) -> (bool, bool) {
        let (lower_end, upper_end) = if inverted {
            (Extend::Max, Extend::Min)
        } else {
            (Extend::Min, Extend::Max)
        };
        (
            extend == Extend::Both || extend == lower_end,
            extend == Extend::Both || extend == upper_end,
        )
    }

    fn process(&mut self, spec: &BlockSpec<'_>, norm: &mut dyn Norm) -> Result<(), ColorbarError> {
        if let Some(values) = spec.values {
            self.values = values.to_vec();
            self.kind = spec.value_kind;
            self.boundaries = match spec.boundaries {
                Some(boundaries) => boundaries.to_vec(),
                None => bracket(values)?,
            };
            return Ok(());
        }
        let mut b = match (norm.kind(), spec.boundaries) {
            (NormKind::Boundary(boundary), _) => boundary.boundaries().to_vec(),
            (NormKind::Identity, _) => (0..=spec.ncolors).map(|i| i as f64 - 0.5).collect(),
            (_, Some(boundaries)) => boundaries.to_vec(),
            _ => self.uniform_y(spec.ncolors + 1)?,
        };
        if b.is_empty() {
            return Err(ColorbarError::TooFewBoundaries(0));
        }
        if self.lower {
            b.insert(0, b[0] - 1.0);
        }
        if self.upper {
            b.push(b[b.len() - 1] + 1.0);
        }
        if !norm.scaled() {
            norm.set_vmin(Some(0.0));
            norm.set_vmax(Some(1.0));
        }
        let (vmin, vmax) = norm.vmin().zip(norm.vmax()).ok_or(NormError::Unscaled)?;
        let (vmin, vmax) = nonsingular(vmin, vmax, 0.1, 1e-15, true);
        norm.set_vmin(Some(vmin));
        norm.set_vmax(Some(vmax));
        if self.class != NormClass::Boundary && !self.explicit {
            b = norm
                .inverse(&b)?
                .into_iter()
                .map(|v| v.unwrap_or(f64::NAN))
                .collect();
        }
        self.values = b.windows(2).map(|w| f64::midpoint(w[0], w[1])).collect();
        if self.class == NormClass::Identity {
            // numpy's `(values + 1e-5).astype(np.int16)` truncates.
            for value in &mut self.values {
                *value = (*value + 0.00001).trunc();
            }
            self.kind = ScalarKind::Integer;
        }
        self.boundaries = b;
        Ok(())
    }

    /// The edges inside the extensions.
    pub(crate) fn inside(&self) -> &[f64] {
        &self.boundaries[self.inside.clone()]
    }

    /// The first and last edge inside the extensions (the colorbar's
    /// `vmin` and `vmax`).
    pub(crate) fn limits(&self) -> (f64, f64) {
        let inside = self.inside();
        (inside[0], inside[inside.len() - 1])
    }

    /// Whether the edges are positioned by interpolation between them
    /// rather than through the norm.
    fn discrete(&self) -> bool {
        self.explicit || self.class == NormClass::Boundary
    }

    /// matplotlib's `_uniform_y`: `n` evenly spaced edges in `[0, 1]`
    /// (validating the extension lengths, as matplotlib does).
    fn uniform_y(&self, n: usize) -> Result<Vec<f64>, ColorbarError> {
        let auto = 1.0 / (n as f64 - 1.0);
        extension_lengths(self.extendfrac, auto, auto)?;
        Ok(linspace(0.0, 1.0, n))
    }

    /// matplotlib's `_proportional_y`: the inside edges in `[0, 1]` along
    /// the colorbar, and the lower and upper extension lengths.
    pub(crate) fn proportional_y(
        &self,
        norm: &mut dyn Norm,
    ) -> Result<(Vec<f64>, [f64; 2]), ColorbarError> {
        let (y, yscaled) = if self.discrete() {
            let (low, high) = self.limits();
            let y: Vec<f64> = self
                .boundaries
                .iter()
                .map(|b| (b - low) / (high - low))
                .collect();
            let yscaled = match self.spacing {
                Spacing::Uniform => forward_boundaries(&self.boundaries, &self.boundaries),
                Spacing::Proportional => y.clone(),
            };
            (y, yscaled)
        } else {
            let y = normalized(norm, &self.boundaries, None, f64::NAN)?;
            (y.clone(), y)
        };
        let y = rescaled(&y[self.inside.clone()])?;
        let yscaled = rescaled(&yscaled[self.inside.clone()])?;
        // `new` guarantees at least two inside edges.
        let last = yscaled.len() - 1;
        let automin = yscaled[1] - yscaled[0];
        let automax = yscaled[last] - yscaled[last - 1];
        let lengths = if self.lower || self.upper {
            extension_lengths(self.extendfrac, automin, automax)?
        } else {
            [0.0, 0.0]
        };
        Ok((y, lengths))
    }

    /// matplotlib's `_mesh`: the block edges along the long axis in data
    /// coordinates (`_y`).
    pub(crate) fn edges(&self, norm: &mut dyn Norm) -> Result<Vec<f64>, ColorbarError> {
        let (y, _) = self.proportional_y(norm)?;
        let (vmin, vmax) = self.limits();
        if self.discrete() || self.class == NormClass::Identity {
            return Ok(y.iter().map(|y| y * (vmax - vmin) + vmin).collect());
        }
        let mut scaled = norm.clone_box();
        scaled.set_vmin(Some(vmin));
        scaled.set_vmax(Some(vmax));
        Ok(scaled
            .inverse(&y)?
            .into_iter()
            .map(|v| v.unwrap_or(f64::NAN))
            .collect())
    }

    /// matplotlib's `_locate`: where data values `x` fall along the long
    /// axis, given the block `edges`.
    pub(crate) fn locate(
        &self,
        norm: &mut dyn Norm,
        edges: &[f64],
        x: &[f64],
    ) -> Result<Vec<f64>, ColorbarError> {
        let (b, xn) = if matches!(self.class, NormClass::Identity | NormClass::Boundary) {
            (self.boundaries.clone(), x.to_vec())
        } else {
            // numpy's `filled()` default for masked floats.
            const FILL: f64 = 1e20;
            (
                normalized(norm, &self.boundaries, Some(false), FILL)?,
                normalized(norm, x, Some(false), FILL)?,
            )
        };
        let inside = &b[self.inside.clone()];
        Ok(xn.iter().map(|&v| interp(v, inside, edges, None)).collect())
    }
}

/// Edges halfway between `values`, extrapolated at both ends as matplotlib
/// does (evaluated in its order on a zero-filled array).
fn bracket(values: &[f64]) -> Result<Vec<f64>, ColorbarError> {
    let n = values.len();
    if n < 2 {
        return Err(ColorbarError::TooFewValues(n));
    }
    let mut b = vec![0.0; n + 1];
    for (i, w) in values.windows(2).enumerate() {
        b[i + 1] = f64::midpoint(w[0], w[1]);
    }
    b[0] = 2.0 * b[1] - b[2];
    b[n] = 2.0 * b[n - 1] - b[n - 2];
    Ok(b)
}

/// matplotlib's `_get_extension_lengths`.
fn extension_lengths(
    frac: ExtendFrac,
    automin: f64,
    automax: f64,
) -> Result<[f64; 2], ColorbarError> {
    const DEFAULT: f64 = 0.05;
    let lengths = match frac {
        ExtendFrac::Default => [DEFAULT, DEFAULT],
        ExtendFrac::Auto => [automin, automax],
        ExtendFrac::Both(f) => [f, f],
        ExtendFrac::Each(low, high) => [low, high],
    };
    let given = matches!(frac, ExtendFrac::Both(_) | ExtendFrac::Each(..));
    if given && lengths.iter().any(|v| v.is_nan()) {
        return Err(ColorbarError::InvalidExtendFrac);
    }
    Ok(lengths)
}

/// `y` mapped linearly from `[y[0], y[-1]]` onto `[0, 1]` (matplotlib's
/// `Normalize(y[0], y[-1])(y)`).
#[expect(
    clippy::float_cmp,
    reason = "matplotlib maps everything to 0 when the limits are equal"
)]
fn rescaled(y: &[f64]) -> Result<Vec<f64>, ColorbarError> {
    let (Some(&low), Some(&high)) = (y.first(), y.last()) else {
        return Ok(Vec::new());
    };
    if low == high {
        return Ok(vec![0.0; y.len()]);
    }
    if low > high {
        return Err(NormError::InvertedLimits {
            vmin: low,
            vmax: high,
        }
        .into());
    }
    Ok(y.iter().map(|v| (v - low) / (high - low)).collect())
}

/// `norm(values, clip)` with masked results replaced by `fill`.
fn normalized(
    norm: &mut dyn Norm,
    values: &[f64],
    clip: Option<bool>,
    fill: f64,
) -> Result<Vec<f64>, ColorbarError> {
    let data: Vec<Option<f64>> = values.iter().copied().map(Some).collect();
    Ok(norm
        .normalize_masked(&data, clip)?
        .into_iter()
        .map(|v| match v {
            Normalized::Fraction(x) => x,
            Normalized::Index(i) => i as f64,
            Normalized::Masked => fill,
        })
        .collect())
}

/// matplotlib's `_forward_boundaries`: positions spread evenly between the
/// `boundaries`, with values well outside them sent out of the way.
pub(crate) fn forward_boundaries(boundaries: &[f64], x: &[f64]) -> Vec<f64> {
    let positions = linspace(0.0, 1.0, boundaries.len());
    let (first, last) = (boundaries[0], boundaries[boundaries.len() - 1]);
    let eps = (last - first) * 1e-6;
    x.iter()
        .map(|&v| {
            if v < first - eps {
                -1.0
            } else if v > last + eps {
                2.0
            } else {
                interp(v, boundaries, &positions, None)
            }
        })
        .collect()
}

/// matplotlib's `_inverse_boundaries`.
pub(crate) fn inverse_boundaries(boundaries: &[f64], y: f64) -> f64 {
    interp(y, &linspace(0.0, 1.0, boundaries.len()), boundaries, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{BoundaryNorm, LogNorm, NoNorm, Normalize};

    fn spec(ncolors: usize) -> BlockSpec<'static> {
        BlockSpec {
            ncolors,
            boundaries: None,
            values: None,
            value_kind: ScalarKind::Float,
            extend: Extend::Neither,
            inverted: false,
            extendfrac: ExtendFrac::Default,
            spacing: Spacing::Uniform,
        }
    }

    #[test]
    fn linear_norms_split_the_range_per_colour() {
        let mut norm = Normalize::new(-1.0, 1.0);
        let blocks = Blocks::new(&spec(4), &mut norm).unwrap();
        assert_eq!(blocks.boundaries, [-1.0, -0.5, 0.0, 0.5, 1.0]);
        assert_eq!(blocks.values, [-0.75, -0.25, 0.25, 0.75]);
        assert_eq!(blocks.limits(), (-1.0, 1.0));
        assert_eq!(blocks.edges(&mut norm).unwrap(), blocks.boundaries);
        let (_, lengths) = blocks.proportional_y(&mut norm).unwrap();
        assert_eq!(lengths, [0.0, 0.0]);
    }

    #[test]
    fn extensions_add_outer_blocks() {
        let mut norm = Normalize::new(0.0, 1.0);
        let both = BlockSpec {
            extend: Extend::Both,
            extendfrac: ExtendFrac::Auto,
            ..spec(4)
        };
        let blocks = Blocks::new(&both, &mut norm).unwrap();
        assert_eq!(blocks.boundaries, [-1.0, 0.0, 0.25, 0.5, 0.75, 1.0, 2.0]);
        assert_eq!(blocks.inside, 1..6);
        assert_eq!(blocks.values[0], -0.5);
        let (y, lengths) = blocks.proportional_y(&mut norm).unwrap();
        assert_eq!(y, [0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_eq!(lengths, [0.25, 0.25]);
        let min = BlockSpec {
            extend: Extend::Min,
            extendfrac: ExtendFrac::Both(0.3),
            ..spec(2)
        };
        let blocks = Blocks::new(&min, &mut norm).unwrap();
        assert_eq!((blocks.lower, blocks.upper), (true, false));
        assert_eq!(blocks.proportional_y(&mut norm).unwrap().1, [0.3, 0.3]);
        let inverted = BlockSpec {
            inverted: true,
            ..min
        };
        let blocks = Blocks::new(&inverted, &mut norm).unwrap();
        assert_eq!((blocks.lower, blocks.upper), (false, true));
    }

    #[test]
    fn unscaled_and_singular_norms_get_usable_limits() {
        let mut unset = Normalize::default();
        let blocks = Blocks::new(&spec(2), &mut unset).unwrap();
        assert_eq!(blocks.boundaries, [0.0, 0.5, 1.0]);
        let mut flat = Normalize::new(2.0, 2.0);
        Blocks::new(&spec(2), &mut flat).unwrap();
        assert_eq!((flat.vmin(), flat.vmax()), (Some(1.8), Some(2.2)));
    }

    #[test]
    fn boundary_norms_keep_their_edges() {
        let mut norm = BoundaryNorm::new([-1.0, -0.4, 0.4, 1.0], 3, Extend::Neither).unwrap();
        let blocks = Blocks::new(&spec(3), &mut norm).unwrap();
        assert_eq!(blocks.boundaries, [-1.0, -0.4, 0.4, 1.0]);
        assert_eq!(blocks.class, NormClass::Boundary);
        // The edges stay in data units; the axis scale spreads them.
        let (y, _) = blocks.proportional_y(&mut norm).unwrap();
        assert!((y[1] - 0.3).abs() < 1e-15);
        let edges = blocks.edges(&mut norm).unwrap();
        assert!((edges[1] + 0.4).abs() < 1e-15);
        assert_eq!(
            blocks.locate(&mut norm, &edges, &[0.4]).unwrap()[0],
            edges[2]
        );
    }

    #[test]
    fn identity_norms_centre_blocks_on_integers() {
        let mut norm = NoNorm::default();
        let blocks = Blocks::new(&spec(3), &mut norm).unwrap();
        assert_eq!(blocks.boundaries, [-0.5, 0.5, 1.5, 2.5]);
        assert_eq!(blocks.values, [0.0, 1.0, 2.0]);
        assert_eq!(blocks.kind, ScalarKind::Integer, "the values index colours");
    }

    #[test]
    fn log_norms_space_edges_geometrically() {
        let mut norm = LogNorm::new(1.0, 100.0);
        let blocks = Blocks::new(&spec(2), &mut norm).unwrap();
        assert!((blocks.boundaries[1] - 10.0).abs() < 1e-12);
        let edges = blocks.edges(&mut norm).unwrap();
        assert!((edges[1] - 10.0).abs() < 1e-12);
    }

    #[test]
    fn explicit_values_are_bracketed() {
        let mut norm = Normalize::default();
        let values = [1.0, 2.0, 4.0];
        let given = BlockSpec {
            values: Some(&values),
            ..spec(256)
        };
        // matplotlib extrapolates from a zero-filled array, in its order.
        let blocks = Blocks::new(&given, &mut norm).unwrap();
        assert_eq!(blocks.boundaries, [0.0, 1.5, 3.0, 4.5]);
        assert_eq!(blocks.kind, ScalarKind::Float);
        assert_eq!(bracket(&[1.0, 3.0]).unwrap(), [4.0, 2.0, 0.0]);
        let one = BlockSpec {
            values: Some(&values[..1]),
            ..spec(256)
        };
        assert_eq!(
            Blocks::new(&one, &mut norm),
            Err(ColorbarError::TooFewValues(1))
        );
    }

    #[test]
    fn invalid_extension_fractions_are_rejected() {
        let mut norm = Normalize::new(0.0, 1.0);
        let nan = BlockSpec {
            extendfrac: ExtendFrac::Each(0.1, f64::NAN),
            ..spec(4)
        };
        assert_eq!(
            Blocks::new(&nan, &mut norm),
            Err(ColorbarError::InvalidExtendFrac)
        );
    }

    #[test]
    fn boundary_positions_send_outliers_away() {
        let b = [0.0, 1.0, 3.0];
        assert_eq!(
            forward_boundaries(&b, &[-1.0, 0.5, 2.0, 4.0]),
            [-1.0, 0.25, 0.75, 2.0]
        );
        assert_eq!(inverse_boundaries(&b, 0.75), 2.0);
    }
}
