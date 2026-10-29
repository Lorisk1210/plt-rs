//! Probe operations for the `mpl_patches` area: what matplotlib's own
//! drawing tests of patches assert about them (see
//! `spec/reference/probes/test_mpl_patches.py`, which documents every op).

use pltrs::figure::Figure;
use pltrs::lines::LineStyle;
use pltrs::patches::{Annulus, Rectangle};

use super::fancy;
use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::numbers;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Rectangle.get_angle", angles),
    Op::new("Patch.get_linestyle", linestyle),
    Op::new("Annulus.set_semimajor", radii),
    Op::new("Patch.get_hatch_linewidth", hatch_linewidth),
    Op::new("patches.BoxStyle", fancy::box_style),
];

/// The angle of a rotated rectangle, then of a rectangle before and
/// after `set_angle(30)` once it is in an axes.
fn angles(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let reference = fig.subplot().add_patch(Rectangle {
            angle: 30.0,
            ..Rectangle::new((0.0, 0.0), 1.0, 1.0)
        });
        let test = Rectangle::new((0.0, 0.0), 1.0, 1.0);
        let before = test.angle;
        let test = fig.subplot().add_patch(test);
        test.update_shape(|rect| rect.angle = 30.0);
        Ok(numbers([
            reference.shape().angle,
            before,
            test.shape().angle,
        ]))
    })
}

/// The dash pattern a rectangle reports for `[offset, dashes]`.
fn linestyle(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let (offset, pattern) = match args.json("linestyle")?.as_array() {
            Some([offset, dashes]) => (offset.as_f64(), dashes.as_f64_vec().unwrap_or_default()),
            _ => {
                return Err(ProbeError::BadArgs(
                    "linestyle needs [offset, dashes]".into(),
                ));
            }
        };
        let offset = offset.ok_or_else(|| ProbeError::BadArgs("bad dash offset".into()))?;
        let fig = Figure::new();
        let rect = fig.add_patch(Rectangle::new((0.1, 0.1), 0.8, 0.8));
        rect.set_linewidth(3.0)
            .set_edgecolor("b")
            .set_linestyle(LineStyle::dashes(offset, pattern));
        match rect.style().linestyle {
            LineStyle::Dashes { offset, pattern } => Ok(Json::Array(vec![
                Json::from(offset),
                pattern.map_or(Json::Null, numbers),
            ])),
            other => Ok(Json::from(other.to_string())),
        }
    })
}

/// The radii of an annulus after `set_semimajor` and `set_semiminor`.
fn radii(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let radius = args.json("r")?;
        let (a, b) = match (radius.as_f64(), radius.as_f64_vec().as_deref()) {
            (Some(r), _) => (r, r),
            (None, Some(&[a, b])) => (a, b),
            _ => return Err(ProbeError::BadArgs("r needs a radius or [a, b]".into())),
        };
        let mut ring = Annulus::elliptical((0.0, 0.0), (a, b), args.f64("width")?, 0.0)
            .map_err(ProbeError::rejected)?;
        ring.set_semimajor(args.f64("semimajor")?);
        ring.set_semiminor(args.f64("semiminor")?);
        let (a, b) = ring.semi_axes();
        Ok(numbers([a, b]))
    })
}

/// The hatch line width of a rectangle made under `hatch.linewidth: 2`, or
/// given it with the setter.
fn hatch_linewidth(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let ax = Figure::new().subplot();
        let rect = Rectangle::new((0.0, 0.0), 1.0, 1.0);
        let patch = if args.bool("setter")? {
            let patch = ax.add_patch(rect);
            patch.set_hatch("x").set_hatch_linewidth(2.0);
            patch
        } else {
            pltrs::rc::context(|| {
                pltrs::rc::set("hatch.linewidth", "2").map_err(ProbeError::rejected)?;
                let patch = ax.add_patch(rect);
                patch.set_hatch("x");
                Ok::<_, ProbeError>(patch)
            })?
        };
        Ok(Json::from(patch.style().hatch_linewidth))
    })
}
