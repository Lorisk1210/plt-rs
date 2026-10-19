//! Probe operations for the `contour_generator` area: contourpy's quad
//! contour generators, called directly with each algorithm, as
//! `spec/reference/probes/test_contour_generator.py` documents.

use plt::contour::{ContourAlgorithm, QuadContourGenerator};
use plt::numeric::Grid;

use crate::probes::encode;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("QuadContourGenerator.lines", lines),
    Op::new("QuadContourGenerator.filled", filled),
];

/// A 2-D argument as a grid.
fn grid<T>(rows: Vec<Vec<T>>) -> Result<Grid<T>, ProbeError> {
    Grid::from_rows(rows).map_err(ProbeError::rejected)
}

/// The generator of the probe's grid; NaN and infinite z are masked, as
/// contourpy masks invalid values.
fn generator(args: Args<'_>) -> Result<QuadContourGenerator, ProbeError> {
    let algorithm: ContourAlgorithm = args
        .str("algorithm")?
        .parse()
        .map_err(ProbeError::rejected)?;
    let (x, y, z) = (
        grid(args.rows("x")?)?,
        grid(args.rows("y")?)?,
        grid(args.rows("z")?)?,
    );
    let mut mask = z.map(|v| !v.is_finite());
    if args.get("mask").is_some() {
        let given = grid(args.bool_rows("mask")?)?;
        if given.shape() != mask.shape() {
            return Err(ProbeError::rejected(
                "the mask does not have the shape of z",
            ));
        }
        mask = Grid::from_fn(z.shape().0, z.shape().1, |i, j| {
            mask[(i, j)] || given[(i, j)]
        });
    }
    let chunk = usize::try_from(args.i64("chunk_size")?).map_err(ProbeError::rejected)?;
    QuadContourGenerator::with_algorithm(
        algorithm,
        &x,
        &y,
        &z,
        Some(&mask),
        args.bool("corner_mask")?,
        (chunk, chunk),
    )
    .map_err(ProbeError::rejected)
}

fn lines(args: Args<'_>) -> Outcome {
    let level = args.f64("level")?;
    Ok(encode::path(&generator(args)?.lines(level)))
}

fn filled(args: Args<'_>) -> Outcome {
    let (lower, upper) = (args.f64("lower")?, args.f64("upper")?);
    let path = generator(args)?
        .filled(lower, upper)
        .map_err(ProbeError::rejected)?;
    Ok(encode::path(&path))
}
