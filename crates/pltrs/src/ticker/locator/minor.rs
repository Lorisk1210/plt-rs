//! `AutoMinorLocator`: evenly subdivides the intervals between major ticks.

use pltrs_core::numeric::remainder;

use super::Locator;
use crate::ticker::pymath::{np_isclose, pow10, round_int, sorted_unique};
use crate::ticker::{
    AxisName, MinorDivisions, ScaleHint, TickContext, TickerError, TickerSettings,
};

/// The most minor ticks `AutoMinorLocator` will generate.
const MAX_MINOR_TICKS: i64 = 10_000_000;

/// Minor ticks dividing each major interval of a linear axis into equal
/// parts (matplotlib's `AutoMinorLocator`).
///
/// It reads the major ticks from the context's axis
/// ([`crate::ticker::AxisInfo::major_ticks`]), so only
/// [`Locator::locate`] is meaningful.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoMinorLocator {
    ndivs: Option<MinorDivisions>,
}

impl AutoMinorLocator {
    /// Subdivides major intervals into `ndivs` parts; `None` reads
    /// `xtick.minor.ndivs`/`ytick.minor.ndivs` when ticks are located.
    pub fn new(ndivs: Option<MinorDivisions>) -> Self {
        Self { ndivs }
    }

    /// The configured subdivisions (`None`: from the rcParams).
    pub fn ndivs(&self) -> Option<MinorDivisions> {
        self.ndivs
    }

    /// The number of subdivisions of a major interval `majorstep`.
    fn divisions(self, majorstep: f64, axis: AxisName, settings: &TickerSettings) -> u32 {
        let ndivs = self.ndivs.unwrap_or(match axis {
            AxisName::Y => settings.ytick_minor_ndivs,
            AxisName::X => settings.xtick_minor_ndivs,
        });
        match ndivs {
            MinorDivisions::Count(n) => n,
            MinorDivisions::Auto => {
                let mantissa = pow10(remainder(majorstep.log10(), 1.0));
                let nice = [1.0, 2.5, 5.0, 10.0]
                    .iter()
                    .any(|&b| np_isclose(mantissa, b, 1e-5, 1e-8));
                if nice { 5 } else { 4 }
            }
        }
    }
}

impl Locator for AutoMinorLocator {
    fn tick_values(&self, _: f64, _: f64, _: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        Err(TickerError::Unsupported(
            "AutoMinorLocator derives minor ticks from the axis' major ticks; use locate",
        ))
    }

    fn locate(&self, ctx: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        let axis = ctx.axis_or_dummy();
        if axis.scale == ScaleHint::Log {
            return Ok(Vec::new());
        }
        let majors = sorted_unique(&axis.major_ticks);
        let [t0, t1, ..] = majors[..] else {
            // At least two major ticks are needed to find the spacing.
            return Ok(Vec::new());
        };
        let majorstep = t1 - t0;
        let ndivs = self.divisions(majorstep, axis.name, ctx.settings());
        if ndivs == 0 {
            return Err(TickerError::invalid("ndivs", "must be positive"));
        }
        let minorstep = majorstep / f64::from(ndivs);
        let (v0, v1) = axis.view_interval;
        let (vmin, vmax) = if v1 < v0 { (v1, v0) } else { (v0, v1) };
        let tmin = round_int((vmin - t0) / minorstep, "minor tick index")?;
        let tmax = round_int((vmax - t0) / minorstep, "minor tick index")? + 1;
        if tmax - tmin > MAX_MINOR_TICKS {
            return Err(TickerError::TooManyTicks((tmax - tmin) as usize));
        }
        Ok((tmin..tmax).map(|t| t as f64 * minorstep + t0).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticker::AxisInfo;

    fn minor(locator: AutoMinorLocator, majors: &[f64], view: (f64, f64)) -> Vec<f64> {
        let axis = AxisInfo {
            view_interval: view,
            major_ticks: majors.to_vec(),
            ..AxisInfo::DUMMY
        };
        let settings = TickerSettings::default();
        locator.locate(&TickContext::new(&axis, &settings)).unwrap()
    }

    #[test]
    fn subdivides_major_intervals() {
        let auto = AutoMinorLocator::default();
        assert_eq!(
            minor(auto, &[0.0, 5.0, 10.0], (0.0, 10.0)),
            [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
        );
        assert_eq!(
            minor(auto, &[0.0, 3.0, 6.0], (0.0, 3.0)),
            [0.0, 0.75, 1.5, 2.25, 3.0]
        );
        let two = AutoMinorLocator::new(Some(MinorDivisions::Count(2)));
        assert_eq!(minor(two, &[0.0, 1.0], (1.0, 0.0)), [0.0, 0.5, 1.0]);
        assert_eq!(minor(auto, &[5.0], (0.0, 10.0)), Vec::<f64>::new());
        assert_eq!(minor(auto, &[], (0.0, 10.0)), Vec::<f64>::new());
        assert!(auto.tick_values(0.0, 1.0, &TickContext::default()).is_err());
    }
}
