//! Tick helpers of polar axes: θ ticks at round multiples of degrees
//! labelled in degrees (matplotlib's `ThetaLocator`, `ThetaFormatter`),
//! and radial ticks kept off the inside of the origin (`RadialLocator`).

use std::any::Any;
use std::f64::consts::PI;
use std::rc::Rc;

use super::transform::full_circle_deg;
use crate::scale::Scale;
use crate::ticker::{
    AxisInfo, FormattedTicks, Formatter, Locator, LogitLocator, MaxNLocator, Nbins, TickContext,
    TickerError, TickerSettings, pyfmt,
};
use crate::transforms::nonsingular;

/// θ steps matplotlib's `ThetaAxis` gives its `AutoLocator`, in degrees.
const THETA_STEPS: [f64; 6] = [1.0, 1.5, 3.0, 4.5, 9.0, 10.0];

/// The default major locator and formatter of the θ axis (matplotlib's
/// `ThetaAxis._wrap_locator_formatter` after `_set_scale("linear")`).
pub(crate) fn theta_tickers(settings: &TickerSettings) -> (Rc<dyn Locator>, Rc<dyn Formatter>) {
    let nbins = if settings.classic_mode {
        Nbins::Fixed(9)
    } else {
        Nbins::Auto
    };
    let base = MaxNLocator::new(nbins)
        .with_steps(&THETA_STEPS)
        .expect("the theta steps are increasing within [1, 10]");
    (
        Rc::new(ThetaLocator::wrapping(Rc::new(base))),
        Rc::new(ThetaFormatter),
    )
}

/// The axis as the θ locator's base sees it: intervals in degrees.
fn in_degrees(axis: &AxisInfo) -> AxisInfo {
    let degrees = |(a, b): (f64, f64)| (a.to_degrees(), b.to_degrees());
    AxisInfo {
        view_interval: degrees(axis.view_interval),
        data_interval: degrees(axis.data_interval),
        minpos: axis.minpos.to_degrees(),
        shared_minpos: axis.shared_minpos.map(f64::to_degrees),
        ..axis.clone()
    }
}

/// θ ticks every 45° on a full circle, else the base locator's ticks
/// computed in degrees (matplotlib's `ThetaLocator`).
#[derive(Debug)]
pub struct ThetaLocator {
    base: Rc<dyn Locator>,
}

pltrs_core::archive_struct!(ThetaLocator { base });

impl ThetaLocator {
    /// The θ locator around `base`, which works in degrees.
    pub fn wrapping(base: Rc<dyn Locator>) -> Self {
        Self { base }
    }

    /// The locator placing the ticks of sectors, in degrees (matplotlib's
    /// `ThetaLocator.base`).
    pub fn base(&self) -> &dyn Locator {
        self.base.as_ref()
    }
}

impl Locator for ThetaLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        let degrees = self
            .base
            .tick_values(vmin.to_degrees(), vmax.to_degrees(), ctx)?;
        Ok(degrees.into_iter().map(f64::to_radians).collect())
    }

    fn locate(&self, ctx: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        let axis = in_degrees(ctx.axis_or_dummy());
        let (low, high) = axis.view_interval;
        if full_circle_deg(low, high) {
            let start = low.min(high).to_radians();
            return Ok((0..8)
                .map(|i| start + f64::from(2 * i) * PI / 8.0)
                .collect());
        }
        let degrees = self.base.locate(&TickContext::new(&axis, ctx.settings()))?;
        Ok(degrees.into_iter().map(f64::to_radians).collect())
    }

    fn view_limits(
        &self,
        dmin: f64,
        dmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        let (low, high) = self
            .base
            .view_limits(dmin.to_degrees(), dmax.to_degrees(), ctx)?;
        Ok((low.to_radians(), high.to_radians()))
    }

    fn calls_program(&self) -> bool {
        self.base.calls_program()
    }
}

/// Angles in radians labelled in degrees with a degree sign, with more
/// digits for narrower views (matplotlib's `ThetaFormatter`): the default
/// formatter of the θ axis, and one a unit converter may give an axis of
/// angles.
///
/// ```
/// use pltrs::polar::ThetaFormatter;
/// use pltrs::prelude::*;
/// let ax = Figure::new().subplot();
/// ax.set_xlim(0.0, std::f64::consts::PI)?;
/// ax.xaxis().set_major_formatter(ThetaFormatter);
/// assert_eq!(ax.xaxis().ticklabels(false)?[1], "29°", "half a radian");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct ThetaFormatter;

pltrs_core::archive_struct!(ThetaFormatter);

impl ThetaFormatter {
    /// The number of decimals for a view `span` radians wide; Python's
    /// errors of matplotlib's `int` for a span that is no finite non-zero
    /// number, after numpy's warning for the logarithm of no span at all.
    fn digits(span: f64) -> Result<usize, TickerError> {
        let degrees = span.abs().to_degrees();
        if degrees == 0.0 {
            crate::warnings::warn_runtime("divide by zero encountered in log10");
        }
        let order = degrees.log10() - 1.5;
        if !order.is_finite() {
            return Err(TickerError::NotAnInteger {
                infinite: order.is_infinite(),
            });
        }
        // Python's `max(-int(order), 0)`; `order` is finite here.
        Ok((-order.trunc()).max(0.0) as usize)
    }
}

impl Formatter for ThetaFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let (vmin, vmax) = ctx.view_interval();
        let digits = Self::digits(vmax - vmin)?;
        Ok(FormattedTicks {
            labels: locs
                .iter()
                .map(|&x| format!("{}\u{b0}", pyfmt::fixed(x.to_degrees(), digits)))
                .collect(),
            offset: String::new(),
        })
    }

    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        let (vmin, vmax) = ctx.view_interval();
        let digits = Self::digits(vmax - vmin)?;
        Ok(format!(
            "{}\u{b0}",
            pyfmt::fixed(value.to_degrees(), digits)
        ))
    }
}

/// Wraps the radial axis' major locator: the initial view is `(0, 1)` on
/// scales that include zero, and autoscaled views start at zero
/// (matplotlib's `RadialLocator`). Ticks inside the origin of full
/// circles are dropped by the axis, which knows the polar geometry.
#[derive(Debug)]
pub struct RadialLocator {
    base: Rc<dyn Locator>,
    /// Whether zero is a valid radius under the axis scale.
    zero_in_bounds: bool,
}

pltrs_core::archive_struct!(RadialLocator {
    base,
    zero_in_bounds
});

impl RadialLocator {
    /// `locator` wrapped for a radial axis under `scale`; an already
    /// wrapped locator is returned as is.
    pub(crate) fn wrap(locator: Rc<dyn Locator>, scale: &dyn Scale) -> Rc<dyn Locator> {
        let any: &dyn Any = locator.as_ref();
        if any.is::<Self>() {
            return locator;
        }
        let (low, _) = scale.limit_range_for_scale(0.0, 1.0, 1e-5);
        Rc::new(Self {
            base: locator,
            zero_in_bounds: low == 0.0,
        })
    }

    /// The wrapped locator (matplotlib's `RadialLocator.base`).
    pub fn base(&self) -> &dyn Locator {
        self.base.as_ref()
    }

    /// The wrapped locator, unless it places logit ticks, which polar
    /// axes refuse (matplotlib's `LogitLocator` raises on a polar axis).
    fn placing(&self) -> Result<&dyn Locator, TickerError> {
        let any: &dyn Any = self.base.as_ref();
        if any.is::<LogitLocator>() {
            Err(TickerError::PolarLogit)
        } else {
            Ok(self.base.as_ref())
        }
    }
}

impl Locator for RadialLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        self.placing()?.tick_values(vmin, vmax, ctx)
    }

    fn locate(&self, ctx: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        self.placing()?.locate(ctx)
    }

    fn nonsingular(&self, v0: f64, v1: f64, ctx: &TickContext<'_>) -> (f64, f64) {
        if self.zero_in_bounds && v0 == f64::NEG_INFINITY && v1 == f64::INFINITY {
            (0.0, 1.0)
        } else {
            self.base.nonsingular(v0, v1, ctx)
        }
    }

    fn view_limits(
        &self,
        dmin: f64,
        dmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        let (mut low, high) = self.base.view_limits(dmin, dmax, ctx)?;
        if self.zero_in_bounds && high > low {
            low = low.min(0.0);
        }
        Ok(nonsingular(low, high, 0.001, 1e-15, true))
    }

    fn calls_program(&self) -> bool {
        self.base.calls_program()
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::TAU;

    use super::*;
    use crate::scale::{LinearScale, LogScale};
    use crate::ticker::AutoLocator;

    fn context(axis: &AxisInfo) -> TickContext<'_> {
        TickContext::new(axis, &TickerSettings::DEFAULT)
    }

    #[test]
    fn full_circle_ticks_every_45_degrees() {
        let (locator, formatter) = theta_tickers(&TickerSettings::DEFAULT);
        let axis = AxisInfo::with_view(0.0, TAU);
        let ticks = locator.locate(&context(&axis)).unwrap();
        assert_eq!(ticks.len(), 8);
        assert!((ticks[2] - PI / 2.0).abs() < 1e-15);
        let labels = formatter.format_ticks(&ticks, &context(&axis)).unwrap();
        assert_eq!(labels.labels[..3], ["0°", "45°", "90°"]);
    }

    #[test]
    fn sectors_tick_at_degree_multiples_with_more_digits_when_narrow() {
        let (locator, formatter) = theta_tickers(&TickerSettings::DEFAULT);
        let axis = AxisInfo {
            tick_space: crate::ticker::TickSpace::of(6),
            ..AxisInfo::with_view(45f64.to_radians(), 135f64.to_radians())
        };
        let ticks = locator.locate(&context(&axis)).unwrap();
        let degrees: Vec<f64> = ticks.iter().map(|t| t.to_degrees().round()).collect();
        assert_eq!(degrees, [45.0, 60.0, 75.0, 90.0, 105.0, 120.0, 135.0]);
        let narrow = AxisInfo::with_view(0.0, 1f64.to_radians());
        let labels = formatter
            .format_ticks(&[0.5f64.to_radians()], &context(&narrow))
            .unwrap();
        assert_eq!(labels.labels, ["0.5°"]);
    }

    #[test]
    fn spans_of_no_number_fail_as_pythons_int() {
        assert_eq!(
            ThetaFormatter::digits(f64::NAN),
            Err(TickerError::NotAnInteger { infinite: false })
        );
        assert_eq!(
            ThetaFormatter::digits(0.0),
            Err(TickerError::NotAnInteger { infinite: true })
        );
        assert_eq!(ThetaFormatter::digits(TAU), Ok(0));
    }

    #[test]
    fn no_span_warns_as_numpys_logarithm_of_zero() {
        let warned =
            |span: f64| crate::warnings::capturing_runtime(|| drop(ThetaFormatter::digits(span))).1;
        assert_eq!(warned(0.0), ["divide by zero encountered in log10"]);
        assert!(warned(f64::NAN).is_empty() && warned(TAU).is_empty());
    }

    #[test]
    fn radial_views_start_at_zero_on_linear_scales() {
        let linear = RadialLocator::wrap(Rc::new(AutoLocator::new()), &LinearScale);
        let ctx = TickContext::detached(&TickerSettings::DEFAULT);
        assert_eq!(
            linear.nonsingular(f64::NEG_INFINITY, f64::INFINITY, &ctx),
            (0.0, 1.0)
        );
        assert_eq!(linear.view_limits(0.5, 2.0, &ctx).unwrap().0, 0.0);
        let log = RadialLocator::wrap(Rc::new(AutoLocator::new()), &LogScale::new(10.0).unwrap());
        assert_eq!(log.view_limits(0.5, 2.0, &ctx).unwrap().0, 0.5);
        let again = RadialLocator::wrap(Rc::clone(&linear), &LinearScale);
        assert!(Rc::ptr_eq(&again, &linear));
    }
}
