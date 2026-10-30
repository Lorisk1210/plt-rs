//! The grid ticks of geographic axes: gridlines every so many degrees
//! between the fixed limits, labelled in whole degrees (matplotlib's
//! `GeoAxes.ThetaFormatter`, `set_longitude_grid` and
//! `set_latitude_grid`).

use crate::numeric::arange;
use crate::ticker::{FixedLocator, FormattedTicks, Formatter, TickContext, TickerError, pyfmt};

/// Radians labelled as degrees rounded to a multiple of `round_to`,
/// shown without decimals and with a degree sign (matplotlib's
/// `GeoAxes.ThetaFormatter`).
///
/// ```
/// use pltrs::geo::GeoThetaFormatter;
/// let formatter = GeoThetaFormatter::new(15.0);
/// assert_eq!(formatter.label(-30f64.to_radians()), "-30°");
/// assert_eq!(formatter.label(0.3), "15°");
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoThetaFormatter {
    round_to: f64,
}

pltrs_core::archive_struct!(GeoThetaFormatter { round_to });

impl Default for GeoThetaFormatter {
    /// Rounding to whole degrees, matplotlib's default.
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl GeoThetaFormatter {
    /// Labels rounded to multiples of `round_to` degrees.
    pub const fn new(round_to: f64) -> Self {
        Self { round_to }
    }

    /// The multiple of degrees labels are rounded to.
    pub const fn round_to(&self) -> f64 {
        self.round_to
    }

    /// The label of `radians`: Python's `round` (half to even) of the
    /// angle in units of `round_to`, scaled back and printed as
    /// `'%0.0f°'`.
    pub fn label(&self, radians: f64) -> String {
        let units = (radians.to_degrees() / self.round_to).round_ties_even();
        // Python's `round` returns an integer, which has no negative zero.
        let units = if units == 0.0 { 0.0 } else { units };
        format!("{}\u{b0}", pyfmt::fixed(units * self.round_to, 0))
    }
}

impl Formatter for GeoThetaFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        _: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        Ok(FormattedTicks {
            labels: locs.iter().map(|&x| self.label(x)).collect(),
            offset: String::new(),
        })
    }

    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(self.label(value))
    }
}

/// Gridlines every `degrees` strictly between `-limit` and `limit`
/// degrees, in radians (the limits themselves are the edges of the map).
pub(crate) fn grid_locator(degrees: f64, limit: f64) -> FixedLocator {
    let grid = arange(-limit + degrees, limit, degrees);
    FixedLocator::new(grid.into_iter().map(f64::to_radians).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticker::TickerSettings;

    #[test]
    fn labels_round_half_to_even_multiples() {
        let formatter = GeoThetaFormatter::new(20.0);
        assert_eq!(formatter.label(50f64.to_radians()), "40°");
        assert_eq!(formatter.label(70f64.to_radians()), "80°");
        assert_eq!(formatter.label(-1e-17), "0°");
        assert_eq!(
            GeoThetaFormatter::default().label(0.5f64.to_radians()),
            "0°"
        );
        let ctx = TickContext::detached(&TickerSettings::DEFAULT);
        let labels = formatter.format_ticks(&[0.0, -1.0], &ctx).unwrap();
        assert_eq!(labels.labels, ["0°", "-60°"]);
        assert_eq!(formatter.format_data(1.0, &ctx).unwrap(), "60°");
    }

    #[test]
    fn grids_skip_the_fixed_limits() {
        let locator = grid_locator(30.0, 180.0);
        let degrees: Vec<f64> = locator
            .locs()
            .iter()
            .map(|r| r.to_degrees().round())
            .collect();
        assert_eq!(
            degrees,
            [
                -150.0, -120.0, -90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0, 120.0, 150.0
            ]
        );
        assert_eq!(grid_locator(15.0, 90.0).locs().len(), 11);
        assert_eq!(grid_locator(200.0, 90.0).locs(), []);
    }
}
