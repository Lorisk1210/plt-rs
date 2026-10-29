//! `skewt`: the gallery's SkewT-logP diagram, its skewed x axis written
//! as user code and registered as `skewx`: the example's `SkewXAxes`
//! transforms, the ticks of `SkewXAxis` and `SkewXTick` that cover the
//! upper and the lower x limits, and the top `SkewSpine`.

use pltrs::axes::{AxisName, Sides};
use pltrs::prelude::*;
use pltrs::projections::{self, Projection, ProjectionView};
use pltrs::spines::SpineSide;
use pltrs::ticker::{MultipleLocator, NullFormatter, ScalarFormatter};
use pltrs::transforms::{Affine2D, Point, ResolvedTransform, interval_contains};

use crate::figures::{CaseResult, FigureCase};

/// How far the x axis leans, in degrees (`rot`).
const ROT: f64 = 30.0;

/// The example's `SkewXAxes`: rectilinear axes whose data is skewed in
/// axes coordinates, so that lines of constant x lean by [`ROT`].
#[derive(Debug, Default)]
struct SkewXAxes;

impl SkewXAxes {
    /// The skew, in axes coordinates.
    fn skew() -> ResolvedTransform {
        ResolvedTransform::affine(Affine2D::identity().skew_deg(ROT, 0.0))
    }

    /// `lower_xlim`: the x view interval, spanned at the bottom.
    fn lower_xlim(view: &ProjectionView<'_>) -> (f64, f64) {
        view.interval(AxisName::X)
    }

    /// `upper_xlim`: the x data interval spanned at the top, where the
    /// skew has moved the data over (the lower one when the view is too
    /// degenerate to invert).
    fn upper_xlim(&self, view: &ProjectionView<'_>) -> (f64, f64) {
        match self.data_to_axes(view).inverted() {
            Some(to_data) => (
                to_data.transform_point(Point::new(0.0, 1.0)).x,
                to_data.transform_point(Point::new(1.0, 1.0)).x,
            ),
            None => Self::lower_xlim(view),
        }
    }
}

impl Projection for SkewXAxes {
    fn name(&self) -> &'static str {
        "skewx"
    }

    /// `transDataToAxes`: scales and limits, then the skew.
    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        view.rectilinear().then(Self::skew())
    }

    /// `_xaxis_transform`: x through the scale and limits, the fraction
    /// across as it is, both skewed; the y axis keeps the default.
    fn axis_transform(
        &self,
        view: &ProjectionView<'_>,
        axis: AxisName,
    ) -> Option<ResolvedTransform> {
        let across = ResolvedTransform::blend(view.rectilinear(), ResolvedTransform::identity());
        (axis == AxisName::X).then(|| across.then(Self::skew()))
    }

    /// `SkewXAxis.get_view_interval`: from the upper interval's start to
    /// the lower one's end.
    fn tick_interval(&self, view: &ProjectionView<'_>, axis: AxisName) -> (f64, f64) {
        match axis {
            AxisName::X => (self.upper_xlim(view).0, Self::lower_xlim(view).1),
            AxisName::Y => view.interval(axis),
        }
    }

    /// `SkewXTick.draw`: the lower mark and label within the lower
    /// interval, the upper ones within the upper interval.
    fn tick_sides(&self, view: &ProjectionView<'_>, axis: AxisName, loc: f64) -> Sides {
        match axis {
            AxisName::X => Sides {
                first: interval_contains(Self::lower_xlim(view), loc),
                second: interval_contains(self.upper_xlim(view), loc),
            },
            AxisName::Y => Sides::BOTH,
        }
    }

    /// `_gen_axes_spines`: the top spine first.
    fn spine_order(&self) -> [SpineSide; 4] {
        [
            SpineSide::Top,
            SpineSide::Bottom,
            SpineSide::Left,
            SpineSide::Right,
        ]
    }

    /// `SkewSpine._adjust_location`: the top spine spans the upper
    /// interval.
    fn spine_interval(&self, view: &ProjectionView<'_>, side: SpineSide) -> Option<(f64, f64)> {
        (side == SpineSide::Top).then(|| self.upper_xlim(view))
    }
}

/// The example's sounding: pressure (hPa), height (m), temperature and
/// dew point (°C).
const SOUNDING: &str = "
    978.0    345    7.8    0.8
    971.0    404    7.2    0.2
    946.7    610    5.2   -1.8
    944.0    634    5.0   -2.0
    925.0    798    3.4   -2.6
    911.8    914    2.4   -2.7
    906.0    966    2.0   -2.7
    877.9   1219    0.4   -3.2
    850.0   1478   -1.3   -3.7
    841.0   1563   -1.9   -3.8
    823.0   1736    1.4   -0.7
    813.6   1829    4.5    1.2
    809.0   1875    6.0    2.2
    798.0   1988    7.4   -0.6
    791.0   2061    7.6   -1.4
    783.9   2134    7.0   -1.7
    755.1   2438    4.8   -3.1
    727.3   2743    2.5   -4.4
    700.5   3048    0.2   -5.8
    700.0   3054    0.2   -5.8
    698.0   3077    0.0   -6.0
    687.0   3204   -0.1   -7.1
    648.9   3658   -3.2  -10.9
    631.0   3881   -4.7  -12.7
    600.7   4267   -6.4  -16.7
    592.0   4381   -6.9  -17.9
    577.6   4572   -8.1  -19.6
    555.3   4877  -10.0  -22.3
    536.0   5151  -11.7  -24.7
    533.8   5182  -11.9  -25.0
    500.0   5680  -15.9  -29.9
    472.3   6096  -19.7  -33.4
    453.0   6401  -22.4  -36.0
    400.0   7310  -30.7  -43.7
    399.7   7315  -30.8  -43.8
    387.0   7543  -33.1  -46.1
    382.7   7620  -33.8  -46.8
    342.0   8398  -40.5  -53.5
    320.4   8839  -43.7  -56.7
    318.0   8890  -44.1  -57.1
    310.0   9060  -44.7  -58.7
    306.1   9144  -43.9  -57.9
    305.0   9169  -43.7  -57.7
    300.0   9280  -43.5  -57.5
    292.0   9462  -43.7  -58.7
    276.0   9838  -47.1  -62.1
    264.0  10132  -47.5  -62.5
    251.0  10464  -49.7  -64.7
    250.0  10490  -49.7  -64.7
    247.0  10569  -48.7  -63.7
    244.0  10649  -48.9  -63.9
    243.3  10668  -48.9  -63.9
    220.0  11327  -50.3  -65.3
    212.0  11569  -50.5  -65.5
    210.0  11631  -49.7  -64.7
    200.0  11950  -49.9  -64.9
    194.0  12149  -49.9  -64.9
    183.0  12529  -51.3  -66.3
    164.0  13233  -55.3  -68.3
    152.0  13716  -56.5  -69.5
    150.0  13800  -57.1  -70.1
    136.0  14414  -60.5  -72.5
    132.0  14600  -60.1  -72.1
    131.4  14630  -60.2  -72.2
    128.0  14792  -60.9  -72.9
    125.0  14939  -60.1  -72.1
    119.0  15240  -62.2  -73.8
    112.0  15616  -64.9  -75.9
    108.0  15838  -64.1  -75.1
    107.8  15850  -64.1  -75.1
    105.0  16010  -64.7  -75.7
    103.0  16128  -62.9  -73.9
    100.0  16310  -62.5  -73.5
";

/// The sounding's columns (`np.loadtxt(..., unpack=True)`).
fn sounding() -> Result<[Vec<f64>; 4], std::num::ParseFloatError> {
    let values = SOUNDING
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<Vec<f64>, _>>()?;
    Ok(std::array::from_fn(|column| {
        values.iter().skip(column).step_by(4).copied().collect()
    }))
}

fn skewt(fig: &Figure) -> CaseResult {
    let [p, _height, t, td] = sounding()?;
    let kind = projections::register(SkewXAxes::default)?;
    let ax = fig.subplot_with_projection(kind)?;
    ax.grid(true);
    ax.semilogy(&t, &p)?.set_color("C3");
    ax.semilogy(&td, &p)?.set_color("C2");
    ax.axvline(0.0, LineProps::default().color("C0"));
    ax.yaxis().set_major_formatter(ScalarFormatter::default());
    ax.yaxis().set_minor_formatter(NullFormatter);
    ax.set_yticks(linspace(100.0, 1000.0, 10));
    ax.set_ylim(1050.0, 100.0)?;
    ax.xaxis().set_major_locator(MultipleLocator::new(10.0)?);
    ax.set_xlim(-50.0, 50.0)?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] =
    &[FigureCase::new("gallery_misc/skewt", skewt).sized(6.5875, 6.2125, 100.0)];

#[cfg(test)]
mod tests {
    use pltrs::axes::ProjectionKind;

    use super::*;

    #[test]
    fn ticks_cover_the_upper_and_lower_limits() {
        let fig = Figure::new();
        skewt(&fig).unwrap();
        let ax = &fig.axes()[0];
        assert_eq!(ax.projection(), ProjectionKind::Registered("skewx"));
        let ticks = ax.ticks(AxisName::X).unwrap().major;
        assert_eq!(ticks.first().map(|t| t.loc), Some(-100.0));
        assert_eq!(ticks.last().map(|t| t.loc), Some(50.0));
        assert_eq!(sounding().unwrap()[0].len(), 73);
    }
}
