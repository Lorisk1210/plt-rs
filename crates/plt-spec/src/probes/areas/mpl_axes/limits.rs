//! View limits and axes rectangles that matplotlib refuses: NaN or
//! infinite limits, rectangles with a NaN entry or a negative size.

use plt::figure::Figure;

use super::{made, numbers, on_axes, pair};
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError};

pub(super) fn set_xlim(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        made(ax.set_xlim(args.f64("left")?, args.f64("right")?))?;
        Ok(pair(ax.xlim()))
    })
}

pub(super) fn add_axes(args: Args<'_>) -> Outcome {
    styled(args, || {
        let rect: [f64; 4] = args
            .f64_vec("rect")?
            .try_into()
            .map_err(|rect| ProbeError::BadArgs(format!("not a rectangle: {rect:?}")))?;
        let (left, bottom, width, height) = made(Figure::new().add_axes(rect))?.position().bounds();
        Ok(numbers([left, bottom, width, height]))
    })
}
