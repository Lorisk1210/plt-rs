//! The slider and range slider ops.

use pltrs::artist::ArtistHandle;
use pltrs::axes::Axes;
use pltrs::figure::Figure;
use pltrs::transforms::{Bbox, Point};
use pltrs::widgets::{Orientation, RangeSlider, Slider, SliderOptions, ValStep};

use super::steps::unknown;
use super::{malformed, numbers};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The extent of `artist` in the axes coordinates of `ax` (matplotlib's
/// `artist.get_extents().transformed(ax.transAxes.inverted())`).
fn axes_extent(ax: &Axes, artist: &impl ArtistHandle) -> Result<Bbox, ProbeError> {
    let extent = artist
        .window_extent()
        .ok_or_else(|| ProbeError::rejected("the slider shows nothing"))?;
    let inverse = ax
        .figure()
        .resolve_transform(&ax.trans_axes())
        .map_err(ProbeError::rejected)?
        .inverted()
        .ok_or_else(|| ProbeError::rejected("singular axes transform"))?;
    let (lower, upper) = (
        inverse.transform_point(Point::new(extent.x0(), extent.y0())),
        inverse.transform_point(Point::new(extent.x1(), extent.y1())),
    );
    Ok(Bbox::from_extents(lower.x, lower.y, upper.x, upper.y))
}

/// The probe's `orientation` (horizontal by default).
fn orientation(args: Args<'_>) -> Result<Orientation, ProbeError> {
    args.opt_str("orientation")?
        .map_or(Ok(Orientation::Horizontal), |name| {
            name.parse().map_err(ProbeError::rejected)
        })
}

/// `Slider(ax, "", valmin, valmax, valinit=..., valstep=...,
/// orientation=...)` after `set_val` and `reset()` when asked: `{val,
/// poly}`, the bounds of its patch in axes coordinates.
pub(super) fn slider(args: Args<'_>) -> Outcome {
    styled(args, || {
        let ax = Figure::new().subplot();
        let valstep = match args.get("valstep") {
            None => None,
            Some(Json::Array(_)) => Some(ValStep::Values(args.f64_vec("valstep")?)),
            Some(_) => Some(ValStep::Step(args.f64("valstep")?)),
        };
        let options = SliderOptions {
            valinit: args.f64("valinit")?,
            valstep,
            orientation: orientation(args)?,
            ..SliderOptions::default()
        };
        let slider = Slider::new(&ax, "", args.f64("valmin")?, args.f64("valmax")?, options)
            .map_err(ProbeError::rejected)?;
        if let Some(val) = args.opt_f64("set_val")? {
            slider.set_val(val);
        }
        if args.opt_bool("reset")?.unwrap_or(false) {
            slider.reset();
        }
        let poly = axes_extent(&ax, slider.poly())?;
        Ok(encode::object([
            ("val", Json::from(slider.val())),
            (
                "poly",
                numbers([poly.x0(), poly.y0(), poly.width(), poly.height()]),
            ),
        ]))
    })
}

/// A 0 to 24 slider starting at `valinit`, bounded by a slider of value
/// `other` from below (`slidermin`) or above (`slidermax`): its value.
fn bounded(args: Args<'_>, below: bool) -> Outcome {
    styled(args, || {
        let ax = Figure::new().subplot();
        let at = |valinit| SliderOptions {
            valinit,
            ..SliderOptions::default()
        };
        let bound = Slider::new(&ax, "", 0.0, 24.0, at(args.f64("other")?))
            .map_err(ProbeError::rejected)?;
        let slider = Slider::new(&ax, "", 0.0, 24.0, at(args.f64("valinit")?))
            .map_err(ProbeError::rejected)?;
        let slider = if below {
            slider.with_slidermin(&bound)
        } else {
            slider.with_slidermax(&bound)
        };
        Ok(Json::from(slider.val()))
    })
}

/// `Slider(..., slidermin=other)`.
pub(super) fn slidermin(args: Args<'_>) -> Outcome {
    bounded(args, true)
}

/// `Slider(..., slidermax=other)`.
pub(super) fn slidermax(args: Args<'_>) -> Outcome {
    bounded(args, false)
}

/// `RangeSlider(ax, "", 0, 1, valinit=..., orientation=...)`: `{val,
/// handles, poly}` after construction and after each step.
pub(super) fn range_slider(args: Args<'_>) -> Outcome {
    styled(args, || {
        let ax = Figure::new().subplot();
        let direction = orientation(args)?;
        let valinit = match args.f64_vec("valinit")?.as_slice() {
            &[low, high] => (low, high),
            _ => return Err(malformed("valinit")),
        };
        let options = SliderOptions {
            orientation: direction,
            ..SliderOptions::default()
        };
        let slider = RangeSlider::new(&ax, "", 0.0, 1.0, Some(valinit), options)
            .map_err(ProbeError::rejected)?;
        let state = || -> Result<Json, ProbeError> {
            let (low, high) = slider.val();
            let handles: Vec<f64> = slider
                .handles()
                .iter()
                .filter_map(|handle| match direction {
                    Orientation::Vertical => handle.ydata().first().copied(),
                    Orientation::Horizontal => handle.xdata().first().copied(),
                })
                .collect();
            let poly = axes_extent(&ax, slider.poly())?;
            Ok(encode::object([
                ("val", numbers([low, high])),
                ("handles", numbers(&handles)),
                (
                    "poly",
                    numbers([poly.x0(), poly.y0(), poly.x1(), poly.y1()]),
                ),
            ]))
        };
        let mut states = vec![state()?];
        for step in args
            .json("steps")?
            .as_array()
            .ok_or_else(|| malformed("steps"))?
        {
            let parts = step.as_array().ok_or_else(|| malformed("step"))?;
            match (parts.first().and_then(Json::as_str), parts.get(1)) {
                (Some("set_val"), Some(value)) => match value.as_f64_vec().as_deref() {
                    Some(&[a, b]) => {
                        slider.set_val((a, b));
                    }
                    _ => return Err(malformed("set_val")),
                },
                (Some("reset"), None) => slider.reset(),
                (name, _) => return Err(unknown("step", name.unwrap_or_default())),
            }
            states.push(state()?);
        }
        Ok(Json::Array(states))
    })
}
