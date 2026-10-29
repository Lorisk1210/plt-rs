//! Probe operations for the `mpl_textpath` area: `TextPath`, after
//! matplotlib's `tests/test_textpath.py` (see
//! `spec/reference/probes/test_mpl_textpath.py`).

use pltrs::fonts::FontProperties;
use pltrs::rc;
use pltrs_core::Point;

use super::text_metrics::with_rc;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("textpath.TextPath", text_path)];

/// `TextPath(xy, s, size=size)` in the default font.
fn text_path(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let xy = args.f64_vec("xy")?;
        let [x, y] = xy[..] else {
            return Err(ProbeError::BadArgs("xy is not a point".into()));
        };
        let font = FontProperties::from_rc(&rc::params());
        let path = pltrs::text::text_path(
            Point::new(x, y),
            args.str("s")?,
            args.opt_f64("size")?,
            &font,
        )
        .map_err(ProbeError::rejected)?;
        Ok(encode::path(&path))
    })
}
