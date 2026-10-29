//! Probe operations for the `mpl_cbook` area: the parts of
//! `matplotlib.cbook` pltrs reproduces, ported from matplotlib's
//! `tests/test_cbook.py` (see `spec/reference/probes/test_mpl_cbook.py`).

use pltrs::lines::{pts_to_midstep, pts_to_poststep, pts_to_prestep};
use pltrs::numeric::{block_perimeters, contiguous_regions};
use pltrs::ticker::{pyfmt, strip_math};

use super::markers::steps;
use super::stats::boxplot_stats;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("cbook.boxplot_stats", boxplot_stats),
    Op::new("cbook.pts_to_prestep", |args| steps(args, pts_to_prestep)),
    Op::new("cbook.pts_to_poststep", |args| steps(args, pts_to_poststep)),
    Op::new("cbook.pts_to_midstep", |args| steps(args, pts_to_midstep)),
    Op::new("cbook.contiguous_regions", regions),
    Op::new("cbook._strip_comment", strip_comment),
    Op::new("cbook.strip_math", math_stripped),
    Op::new("cbook._auto_format_str", auto_format),
    Op::new("cbook._array_patch_perimeters", patch_perimeters),
];

/// An index as a JSON number.
fn index(i: usize) -> Json {
    (i as f64).into()
}

fn regions(args: Args<'_>) -> Outcome {
    let mask = args
        .json("mask")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("mask is not a list".into()))?
        .iter()
        .map(|flag| {
            flag.as_bool()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad flag {flag}")))
        })
        .collect::<Result<Vec<bool>, _>>()?;
    Ok(Json::Array(
        contiguous_regions(&mask)
            .into_iter()
            .map(|(start, stop)| Json::Array(vec![index(start), index(stop)]))
            .collect(),
    ))
}

fn strip_comment(args: Args<'_>) -> Outcome {
    pltrs::rc::strip_comment(args.str("line")?)
        .map(Json::from)
        .map_err(ProbeError::rejected)
}

fn math_stripped(args: Args<'_>) -> Outcome {
    Ok(strip_math(args.str("s")?).into())
}

fn auto_format(args: Args<'_>) -> Outcome {
    pyfmt::auto_format(args.str("fmt")?, args.f64("value")?)
        .map(Json::from)
        .map_err(ProbeError::rejected)
}

/// The perimeters of `np.arange(rows * cols).reshape(rows, cols)`.
fn patch_perimeters(args: Args<'_>) -> Outcome {
    let cols = args.usize("cols")?;
    let blocks = block_perimeters(
        args.usize("rows")?,
        cols,
        args.usize("rstride")?,
        args.usize("cstride")?,
    );
    Ok(Json::Array(
        blocks
            .into_iter()
            .map(|cells| {
                Json::Array(
                    cells
                        .into_iter()
                        .map(|(r, c)| index(r * cols + c))
                        .collect(),
                )
            })
            .collect(),
    ))
}
