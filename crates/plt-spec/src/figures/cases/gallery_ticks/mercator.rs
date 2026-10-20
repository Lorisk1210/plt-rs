//! `custom_scale`: a scale of its own for latitudes in a Mercator
//! projection, with fixed ticks every ten degrees.

use std::rc::Rc;

use plt::axes::AxisName;
use plt::prelude::*;
use plt::scale::{AxisTickers, Scale};
use plt::ticker::{FixedLocator, FuncFormatter, NullLocator, TickerSettings, pyfmt};
use plt::transforms::AxisMap;

use crate::figures::{CaseResult, FigureCase};

/// Latitudes in radians scaled as in a Mercator projection,
/// `ln |tan y + sec y|`, with nothing shown beyond ±`thresh` where the
/// scale runs off to infinity (the example's `MercatorLatitudeScale`).
#[derive(Clone, Copy, Debug)]
struct MercatorLatitudeScale {
    thresh: f64,
}

impl Default for MercatorLatitudeScale {
    /// Cropped at ±85°.
    fn default() -> Self {
        Self {
            thresh: 85_f64.to_radians(),
        }
    }
}

/// The map of [`MercatorLatitudeScale`] (`MercatorLatitudeTransform` and
/// its inverse).
#[derive(Clone, Copy, Debug)]
struct MercatorLatitudeMap {
    thresh: f64,
}

impl AxisMap for MercatorLatitudeMap {
    /// Latitudes beyond the threshold are masked.
    fn forward(&self, value: f64) -> f64 {
        if (-self.thresh..=self.thresh).contains(&value) {
            (value.tan() + 1.0 / value.cos()).abs().ln()
        } else {
            f64::NAN
        }
    }

    fn inverse(&self, value: f64) -> f64 {
        value.sinh().atan()
    }
}

/// Labels a latitude in radians with whole degrees.
fn degrees() -> FuncFormatter {
    FuncFormatter::new(|x, _| format!("{}\u{b0}", pyfmt::fixed(x.to_degrees(), 0)))
}

impl Scale for MercatorLatitudeScale {
    fn name(&self) -> &'static str {
        "mercator"
    }

    fn axis_map(&self) -> Option<Rc<dyn AxisMap>> {
        Some(Rc::new(MercatorLatitudeMap {
            thresh: self.thresh,
        }))
    }

    /// The axis never extends beyond the threshold.
    fn limit_range_for_scale(&self, vmin: f64, vmax: f64, _minpos: f64) -> (f64, f64) {
        (vmin.max(-self.thresh), vmax.min(self.thresh))
    }

    /// Ticks every ten degrees from -90° to 80°.
    fn default_tickers(&self, _axis: AxisName, _settings: &TickerSettings) -> AxisTickers {
        let ticks = (-90..90)
            .step_by(10)
            .map(|deg| f64::from(deg).to_radians())
            .collect();
        AxisTickers {
            major_locator: Box::new(FixedLocator::new(ticks)),
            minor_locator: Box::new(NullLocator),
            major_formatter: Box::new(degrees()),
            minor_formatter: Box::new(degrees()),
        }
    }
}

fn custom_scale(fig: &Figure) -> CaseResult {
    let t = arange(-180.0, 180.0, 0.1);
    let s: Vec<f64> = t.iter().map(|t| t.to_radians() / 2.0).collect();
    let ax = fig.subplot();
    ax.plot_with(&t, s, LineProps::fmt("-")?.linewidth(2.0))?;
    ax.set_scale(AxisName::Y, Box::new(MercatorLatitudeScale::default()));
    ax.set_xlabel("Longitude");
    ax.set_ylabel("Latitude");
    ax.set_title("Mercator projection");
    ax.grid(true);
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] =
    &[FigureCase::new("gallery_ticks/custom_scale", custom_scale)];
