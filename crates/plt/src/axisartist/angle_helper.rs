//! Grid levels and labels for angles in degrees or hours, with minutes
//! and seconds, and the extreme finder of cyclic coordinates
//! (matplotlib's `axisartist.angle_helper`).

use plt_core::{Bbox, Point};

use super::aux_transform::AuxTransform;
use super::grid_finder::{
    ExtremeFinder, GridError, GridFormatter, GridLevels, GridLocator, padded_extent, sample_grid,
};
use crate::spines::SpineSide;
use crate::ticker::pyfmt::{PrintfFormat, integer, repr};
use crate::ticker::pymath::{divmod, np_sign, pow10, py_max, py_min};

/// Whether angles are measured in degrees (cycle 360) or hours (cycle
/// 24).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AngleUnit {
    /// Degrees, minutes and seconds of arc.
    #[default]
    Degree,
    /// Hours, minutes and seconds of time.
    Hour,
}

impl AngleUnit {
    /// The length of a full turn.
    const fn cycle(self) -> f64 {
        match self {
            Self::Degree => 360.0,
            Self::Hour => 24.0,
        }
    }
}

/// The steps of `select_step_degree` and `select_step_hour`: limits on
/// the step size with their step and factor, from seconds up to whole
/// units.
fn step_table(unit: AngleUnit) -> Vec<(f64, f64, f64)> {
    let (unit_limits, unit_steps, minsec_limits, minsec_steps): (&[f64], &[f64], &[f64], &[f64]) =
        match unit {
            AngleUnit::Degree => (
                &[1.5, 3.0, 7.0, 13.0, 20.0, 40.0, 70.0, 120.0, 270.0, 520.0],
                &[1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 45.0, 90.0, 180.0, 360.0],
                &[1.5, 2.5, 3.5, 8.0, 11.0, 18.0, 25.0, 45.0],
                &[1.0, 2.0, 3.0, 5.0, 10.0, 15.0, 20.0, 30.0],
            ),
            AngleUnit::Hour => (
                &[1.5, 2.5, 3.5, 5.0, 7.0, 10.0, 15.0, 21.0, 36.0],
                &[1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 18.0, 24.0],
                &[1.5, 2.5, 3.5, 4.5, 5.5, 8.0, 11.0, 14.0, 18.0, 25.0, 45.0],
                &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 10.0, 12.0, 15.0, 20.0, 30.0],
            ),
        };
    let scaled = |divisor: f64| {
        minsec_limits
            .iter()
            .zip(minsec_steps)
            .map(move |(&limit, &step)| (limit / divisor, step, divisor))
    };
    scaled(3600.0)
        .chain(scaled(60.0))
        .chain(
            unit_limits
                .iter()
                .zip(unit_steps)
                .map(|(&limit, &step)| (limit, step, 1.0)),
        )
        .collect()
}

/// The step and factor of the first table entry whose limit is at least
/// `dv` (numpy's `searchsorted`).
fn select_from_table(unit: AngleUnit, dv: f64) -> Result<(f64, f64), GridError> {
    step_table(unit)
        .into_iter()
        .find(|&(limit, _, _)| dv <= limit)
        .map(|(_, step, factor)| (step, factor))
        .ok_or(GridError::StepOutOfRange(dv))
}

/// A step of whole degrees, minutes or seconds for intervals of `dv`
/// degrees: `(step, factor)` with levels at multiples of `step / factor`
/// (matplotlib's `select_step_degree`).
pub fn select_step_degree(dv: f64) -> Result<(f64, f64), GridError> {
    select_from_table(AngleUnit::Degree, dv)
}

/// A step of whole hours, minutes or seconds for intervals of `dv`
/// hours (matplotlib's `select_step_hour`).
pub fn select_step_hour(dv: f64) -> Result<(f64, f64), GridError> {
    select_from_table(AngleUnit::Hour, dv)
}

/// A step of 1, 2 or 5 in the decade of `dv` for sub-second intervals
/// (matplotlib's `select_step_sub`).
pub fn select_step_sub(dv: f64) -> (f64, f64) {
    let tmp = pow10(dv.log10().trunc() - 1.0);
    let factor = 1.0 / tmp;
    if 1.5 * tmp >= dv {
        (1.0, factor)
    } else if 3.0 * tmp >= dv {
        (2.0, factor)
    } else if 7.0 * tmp >= dv {
        (5.0, factor)
    } else {
        (1.0, 0.1 * factor)
    }
}

/// Levels at a round angular step for about `nv` intervals between `v1`
/// and `v2`; a range spanning a full turn yields one turn of levels, the
/// level closing the turn kept when `include_last` (matplotlib's
/// `select_step`). Steps below `1 / threshold_factor` are sub-second
/// steps.
pub fn select_step(
    v1: f64,
    v2: f64,
    nv: usize,
    unit: AngleUnit,
    include_last: bool,
    threshold_factor: f64,
) -> Result<GridLevels, GridError> {
    let (v1, v2) = if v1 > v2 { (v2, v1) } else { (v1, v2) };
    let dv = (v2 - v1) / nv as f64;
    let (step, factor) = if dv > 1.0 / threshold_factor {
        select_from_table(unit, dv)?
    } else {
        let (step, factor) = select_step_sub(dv * threshold_factor);
        (step, factor * threshold_factor)
    };
    let first = (v1 * factor / step).floor();
    let stop = (v2 * factor / step).ceil() + 0.5;
    if !(first.is_finite() && stop.is_finite()) {
        return Err(GridError::InvalidLevels(format!(
            "no levels between {v1} and {v2}"
        )));
    }
    let count = (stop - first).ceil().max(0.0) as usize;
    let mut levels: Vec<f64> = (0..count).map(|i| (first + i as f64) * step).collect();
    let cycle = unit.cycle();
    #[expect(clippy::float_cmp, reason = "matplotlib tests the factor exactly")]
    let whole_units = factor == 1.0;
    if let (true, Some(&low), Some(&high)) = (whole_units, levels.first(), levels.last())
        && high >= low + cycle
    {
        let turn = (cycle / step) as usize + usize::from(include_last);
        levels = (0..turn).map(|i| low + i as f64 * step).collect();
    }
    Ok(GridLevels {
        n: levels.len(),
        levels,
        factor,
    })
}

/// [`select_step`] for angles in degrees measured in hours: the levels
/// are degrees, the factor counts hours (matplotlib's `select_step24`).
pub fn select_step24(
    v1: f64,
    v2: f64,
    nv: usize,
    include_last: bool,
    threshold_factor: f64,
) -> Result<GridLevels, GridError> {
    let mut levels = select_step(
        v1 / 15.0,
        v2 / 15.0,
        nv,
        AngleUnit::Hour,
        include_last,
        threshold_factor,
    )?;
    for level in &mut levels.levels {
        *level *= 15.0;
    }
    Ok(levels)
}

/// [`select_step`] in degrees (matplotlib's `select_step360`).
pub fn select_step360(
    v1: f64,
    v2: f64,
    nv: usize,
    include_last: bool,
    threshold_factor: f64,
) -> Result<GridLevels, GridError> {
    select_step(
        v1,
        v2,
        nv,
        AngleUnit::Degree,
        include_last,
        threshold_factor,
    )
}

/// Defines an angle locator: about `nbins` intervals at a round step of
/// `$unit`, sub-steps below `1 / $threshold`.
macro_rules! angle_locator {
    ($(#[$doc:meta])* $name:ident, $select:ident, $threshold:expr) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name {
            nbins: usize,
            include_last: bool,
        }

        impl $name {
            /// A locator aiming at `nbins` intervals; over a full turn the
            /// level closing the turn is included.
            pub const fn new(nbins: usize) -> Self {
                Self {
                    nbins,
                    include_last: true,
                }
            }

            /// The same locator, including the level closing a full turn
            /// or not (matplotlib's `include_last`).
            #[must_use]
            pub const fn with_include_last(self, include_last: bool) -> Self {
                Self {
                    include_last,
                    ..self
                }
            }

            /// Changes the number of intervals (matplotlib's
            /// `set_params(nbins=...)`).
            pub fn set_params(&mut self, nbins: usize) {
                self.nbins = nbins;
            }
        }

        impl GridLocator for $name {
            fn levels(&self, v1: f64, v2: f64) -> Result<GridLevels, GridError> {
                $select(v1, v2, self.nbins, self.include_last, $threshold)
            }
        }
    };
}

angle_locator!(
    /// Levels in degrees at whole degrees, minutes or seconds of arc, or
    /// finer (matplotlib's `LocatorDMS`).
    LocatorDms,
    select_step360,
    3600.0
);
angle_locator!(
    /// Levels in degrees at whole degrees or minutes of arc, or finer
    /// (matplotlib's `LocatorDM`).
    LocatorDm,
    select_step360,
    60.0
);
angle_locator!(
    /// Levels in degrees at whole degrees, or finer (matplotlib's
    /// `LocatorD`).
    LocatorD,
    select_step360,
    1.0
);
angle_locator!(
    /// Levels in degrees at whole hours, minutes or seconds of time, or
    /// finer (matplotlib's `LocatorHMS`).
    LocatorHms,
    select_step24,
    3600.0
);
angle_locator!(
    /// Levels in degrees at whole hours or minutes of time, or finer
    /// (matplotlib's `LocatorHM`).
    LocatorHm,
    select_step24,
    60.0
);
angle_locator!(
    /// Levels in degrees at whole hours, or finer (matplotlib's
    /// `LocatorH`).
    LocatorH,
    select_step24,
    1.0
);

/// The marks written after the units, minutes and seconds.
struct Marks {
    unit: &'static str,
    minute: &'static str,
    second: &'static str,
}

/// Degrees, minutes and seconds of arc.
const DMS: Marks = Marks {
    unit: r"^{\circ}",
    minute: r"^{\prime}",
    second: r"^{\prime\prime}",
};

/// Hours, minutes and seconds of time.
const HMS: Marks = Marks {
    unit: r"^\mathrm{h}",
    minute: r"^\mathrm{m}",
    second: r"^\mathrm{s}",
};

/// matplotlib's `_get_number_fraction`: when the factor is a power of ten
/// times 1, 60 or 3600, that base factor and the number of decimals.
fn number_fraction(factor: f64) -> (f64, Option<i32>) {
    for threshold in [1.0, 60.0, 3600.0] {
        if factor <= threshold {
            break;
        }
        let d = divmod(factor, threshold).0;
        let int_log_d = d.log10().floor() as i32;
        #[expect(
            clippy::float_cmp,
            reason = "matplotlib tests for an exact power of ten"
        )]
        let power_of_ten = pow10(f64::from(int_log_d)) == d && d != 1.0;
        if power_of_ten {
            return (
                divmod(factor, pow10(f64::from(int_log_d))).0,
                Some(int_log_d),
            );
        }
    }
    (factor, None)
}

/// Python's `'%0{width}d' % value`.
fn zero_padded(value: f64, width: i32) -> Result<String, GridError> {
    Ok(PrintfFormat::parse(&format!("%0{width}d"))?.format(value)?)
}

/// The labels of `values` (scaled by `factor`) with `marks` (matplotlib's
/// `FormatterDMS.__call__`).
fn angle_labels(marks: &Marks, factor: f64, values: &[f64]) -> Result<Vec<String>, GridError> {
    if values.is_empty() {
        return Ok(Vec::new());
    }
    let Marks {
        unit,
        minute,
        second,
    } = marks;
    let signs: Vec<f64> = values.iter().map(|&v| np_sign(v)).collect();
    let sign_texts: Vec<&str> = values
        .iter()
        .map(|&v| if v < 0.0 { "-" } else { "" })
        .collect();
    let (factor, fraction) = number_fraction(factor);
    let mut magnitudes: Vec<f64> = values.iter().map(|v| v.abs()).collect();
    let mut fractions = Vec::new();
    if let Some(digits) = fraction {
        let scale = pow10(f64::from(digits));
        for magnitude in &mut magnitudes {
            let (whole, part) = divmod(*magnitude, scale);
            *magnitude = whole;
            fractions.push(zero_padded(part, digits)?);
        }
    }
    let minutes_of = |v: f64| divmod(v, 60.0);
    match factor {
        1.0 => signs
            .iter()
            .zip(&magnitudes)
            .enumerate()
            .map(|(i, (&s, &v))| {
                let degrees = integer(s * v.trunc());
                Ok(match fractions.get(i) {
                    None => format!("${degrees}{unit}$"),
                    Some(f) => format!("${degrees}.{f}{unit}$"),
                })
            })
            .collect(),
        60.0 => sign_texts
            .iter()
            .zip(&magnitudes)
            .enumerate()
            .map(|(i, (s, &v))| {
                let (d, m) = minutes_of(v);
                let (d, m) = (integer(d), zero_padded(m, 2)?);
                Ok(match fractions.get(i) {
                    None => format!("${s}{d}{unit}\\,{m}{minute}$"),
                    Some(f) => format!("${s}{d}{unit}\\,{m}.{f}{minute}$"),
                })
            })
            .collect(),
        3600.0 => {
            let reversed = signs.last() == Some(&-1.0);
            if reversed {
                magnitudes.reverse();
            }
            let mut sign_texts = sign_texts;
            if reversed {
                sign_texts.reverse();
            }
            let mut labels = Vec::with_capacity(magnitudes.len());
            let mut previous = String::new();
            for (i, (s, &v)) in sign_texts.iter().zip(&magnitudes).enumerate() {
                let (d, rest) = divmod(v, 3600.0);
                let (m, sec) = minutes_of(rest);
                let seconds = match fractions.get(i) {
                    None => format!("{}{second}$", zero_padded(sec, 2)?),
                    Some(f) => format!("{}.{f}{second}$", zero_padded(sec, 2)?),
                };
                let head = format!(
                    "${s}{}{unit}\\,{}{minute}\\,",
                    integer(d),
                    zero_padded(m, 2)?
                );
                labels.push(if head == previous {
                    format!("${s}{seconds}")
                } else {
                    let label = format!("{head}{seconds}");
                    previous = head;
                    label
                });
            }
            if reversed {
                labels.reverse();
            }
            Ok(labels)
        }
        _ => Ok(signs
            .iter()
            .zip(&magnitudes)
            .map(|(&s, &v)| format!("${}^{{\\circ}}$", repr(s * v)))
            .collect()),
    }
}

/// Labels levels in degrees as degrees, minutes and seconds of arc in
/// mathtext (matplotlib's `FormatterDMS`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormatterDms;

impl GridFormatter for FormatterDms {
    fn labels(&self, _: SpineSide, factor: f64, values: &[f64]) -> Result<Vec<String>, GridError> {
        angle_labels(&DMS, factor, values)
    }
}

/// Labels levels in degrees as hours, minutes and seconds of time in
/// mathtext (matplotlib's `FormatterHMS`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormatterHms;

impl GridFormatter for FormatterHms {
    fn labels(&self, _: SpineSide, factor: f64, values: &[f64]) -> Result<Vec<String>, GridError> {
        let hours: Vec<f64> = values.iter().map(|v| v / 15.0).collect();
        angle_labels(&HMS, factor, &hours)
    }
}

/// An extreme finder for coordinates taken modulo 360 or restricted to a
/// range (matplotlib's `ExtremeFinderCycle`): samples are unwrapped so
/// that jumps stay below half a turn, the span is limited to one turn,
/// and the box is clipped to the given ranges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtremeFinderCycle {
    /// The number of samples along x.
    pub nx: usize,
    /// The number of samples along y.
    pub ny: usize,
    /// The cycle of the first coordinate (matplotlib assumes 360).
    pub lon_cycle: Option<f64>,
    /// The cycle of the second coordinate.
    pub lat_cycle: Option<f64>,
    /// The range the first coordinate is clipped to.
    pub lon_minmax: Option<(f64, f64)>,
    /// The range the second coordinate is clipped to.
    pub lat_minmax: Option<(f64, f64)>,
}

impl ExtremeFinderCycle {
    /// matplotlib's defaults: longitudes cycle over 360, latitudes are
    /// clipped to `[-90, 90]`.
    pub const fn new(nx: usize, ny: usize) -> Self {
        Self {
            nx,
            ny,
            lon_cycle: Some(360.0),
            lat_cycle: None,
            lon_minmax: None,
            lat_minmax: Some((-90.0, 90.0)),
        }
    }
}

/// Moves the values more than half a turn above the smallest one down by
/// a turn (NaNs ignored).
fn unwrap(values: &mut [f64]) {
    let lowest = values
        .iter()
        .copied()
        .filter(|v| !v.is_nan())
        .fold(
            f64::NAN,
            |low, v| if low.is_nan() || v < low { v } else { low },
        );
    for value in values {
        if *value - lowest > 180.0 {
            *value -= 360.0;
        }
    }
}

/// `(low, high)` clipped to `minmax`, if given.
fn clipped((low, high): (f64, f64), minmax: Option<(f64, f64)>) -> (f64, f64) {
    minmax.map_or((low, high), |(min0, max0)| {
        (py_max(min0, low), py_min(max0, high))
    })
}

/// `high` limited to one `cycle` above `low` (unless the cycle is unset
/// or zero).
fn one_turn((low, high): (f64, f64), cycle: Option<f64>) -> (f64, f64) {
    match cycle {
        Some(cycle) if cycle != 0.0 => (low, py_min(high, low + cycle)),
        _ => (low, high),
    }
}

impl ExtremeFinder for ExtremeFinderCycle {
    fn find_transformed_bbox(&self, transform: &AuxTransform, bbox: Bbox) -> Bbox {
        let (mut lon, mut lat): (Vec<f64>, Vec<f64>) = sample_grid(bbox, self.nx, self.ny)
            .into_iter()
            .map(|p| {
                let q = transform.transform_point(p);
                (q.x, q.y)
            })
            .unzip();
        if self.lon_cycle.is_some() {
            unwrap(&mut lon);
        }
        if self.lat_cycle.is_some() {
            unwrap(&mut lat);
        }
        let points: Vec<Point> = lon
            .into_iter()
            .zip(lat)
            .map(|(x, y)| Point::new(x, y))
            .collect();
        let padded = padded_extent(&points, self.nx, self.ny);
        let lon = one_turn(padded.interval_x(), self.lon_cycle);
        let lat = one_turn(padded.interval_y(), self.lat_cycle);
        let (lon_min, lon_max) = clipped(lon, self.lon_minmax);
        let (lat_min, lat_max) = clipped(lat, self.lat_minmax);
        Bbox::from_extents(lon_min, lat_min, lon_max, lat_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(result: Result<GridLevels, GridError>) -> (Vec<f64>, usize, f64) {
        let GridLevels { levels, n, factor } = result.unwrap();
        (levels, n, factor)
    }

    #[test]
    fn steps_cover_one_turn_or_subdivide_seconds() {
        let (turn, n, factor) = levels(select_step360(0.0, 360.0, 12, true, 3600.0));
        assert_eq!((turn.len(), n, factor), (13, 13, 1.0));
        assert_eq!(turn[12], 360.0);
        let (without_last, ..) = levels(select_step360(0.0, 360.0, 12, false, 3600.0));
        assert_eq!(without_last.len(), 12);
        let (wide, ..) = levels(select_step360(0.0, 3000.0, 12, true, 3600.0));
        assert_eq!(wide, [0.0, 180.0, 360.0]);
        assert_eq!(
            levels(select_step360(0.001, 0.0021, 4, true, 3600.0)),
            (vec![3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 6, 3600.0)
        );
        assert_eq!(
            levels(select_step360(0.0, 1e-5, 3, true, 3600.0)),
            (vec![0.0, 1.0, 2.0, 3.0, 4.0], 5, 360_000.0)
        );
        assert_eq!(
            select_step_hour(100.0),
            Err(GridError::StepOutOfRange(100.0))
        );
        assert!(select_step360(f64::NAN, 1.0, 4, true, 3600.0).is_err());
    }

    #[test]
    fn hours_are_levels_in_degrees() {
        let (hours, _, factor) = levels(LocatorHms::new(4).levels(120.0, 210.0));
        assert_eq!(factor, 1.0);
        assert_eq!(hours, [120.0, 135.0, 150.0, 165.0, 180.0, 195.0, 210.0]);
    }

    #[test]
    fn labels_write_degrees_minutes_and_seconds() {
        let minutes = FormatterDms
            .labels(SpineSide::Bottom, 60.0, &[0.0, 90.0, -45.0])
            .unwrap();
        assert_eq!(
            minutes,
            [
                r"$0^{\circ}\,00^{\prime}$",
                r"$1^{\circ}\,30^{\prime}$",
                r"$-0^{\circ}\,45^{\prime}$"
            ]
        );
        // Repeated degrees and minutes are left out; tenths of seconds
        // come from a factor of 36000 (matplotlib's labels).
        let seconds = FormatterDms
            .labels(
                SpineSide::Bottom,
                36000.0,
                &[0.0, 5.0, 15.0, -25.0, 36015.0],
            )
            .unwrap();
        assert_eq!(
            seconds,
            [
                r"$0^{\circ}\,00^{\prime}\,00.0^{\prime\prime}$",
                r"$00.5^{\prime\prime}$",
                r"$01.5^{\prime\prime}$",
                r"$-0^{\circ}\,00^{\prime}\,02.5^{\prime\prime}$",
                r"$1^{\circ}\,00^{\prime}\,01.5^{\prime\prime}$"
            ]
        );
        let hours = FormatterHms
            .labels(SpineSide::Left, 1.0, &[120.0, 135.0])
            .unwrap();
        assert_eq!(hours, [r"$8^\mathrm{h}$", r"$9^\mathrm{h}$"]);
        assert_eq!(
            FormatterDms.labels(SpineSide::Top, 1.0, &[]),
            Ok(Vec::new())
        );
    }

    #[test]
    fn cyclic_extremes_unwrap_and_span_one_turn() {
        let mut values = vec![350.0, 10.0, f64::NAN];
        unwrap(&mut values);
        assert_eq!(values[..2], [-10.0, 10.0]);
        // Samples at 0, 500 and 1000 unwrap to 0, 140 and 640, padded to
        // [-213.3, 853.3] and cut to one turn (matplotlib's values).
        let finder = ExtremeFinderCycle {
            lat_minmax: None,
            ..ExtremeFinderCycle::new(3, 2)
        };
        let bbox = finder.find_transformed_bbox(
            &AuxTransform::identity(),
            Bbox::from_extents(0.0, 0.0, 1000.0, 1.0),
        );
        assert!((bbox.x0() + 640.0 / 3.0).abs() < 1e-9);
        assert!((bbox.x1() - 440.0 / 3.0).abs() < 1e-9);
    }
}
