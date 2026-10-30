//! The numeric helpers of the `axisartist` toolkit (matplotlib's
//! `mpl_toolkits.axisartist.angle_helper` and `grid_finder`): angular grid
//! steps, angle labels and where grid lines cross a box.

use pltrs::axisartist::angle_helper::{
    self, AngleUnit, ExtremeFinderCycle, FormatterDms, FormatterHms, select_step_degree,
    select_step_hour, select_step_sub,
};
use pltrs::axisartist::grid_finder::{
    self, GridError, GridFormatter, GridLevels, line_box_crossings,
};
use pltrs::spines::SpineSide;
use pltrs::transforms::{Bbox, Point};

use crate::binding::prelude::*;

impl From<GridError> for PyErr {
    /// matplotlib's `IndexError` for intervals beyond the step tables,
    /// `ValueError` otherwise.
    fn from(err: GridError) -> Self {
        let kind = match err {
            GridError::StepOutOfRange(_) => ExceptionKind::IndexError,
            _ => ExceptionKind::ValueError,
        };
        PyErr::new(kind, err.to_string())
    }
}

/// `(levels, n, factor)`, as matplotlib's step selectors return them.
fn levels_tuple(levels: GridLevels) -> (NdArray, usize, f64) {
    (NdArray::from_vec(levels.levels), levels.n, levels.factor)
}

/// `select_step(v1, v2, nv, hour, include_last, threshold_factor)`:
/// levels at a round step of degrees (or hours) for about `nv` intervals.
fn select_step(
    v1: f64,
    v2: f64,
    nv: usize,
    hour: bool,
    include_last: bool,
    threshold_factor: f64,
) -> PyResult<(NdArray, usize, f64)> {
    let unit = if hour {
        AngleUnit::Hour
    } else {
        AngleUnit::Degree
    };
    Ok(levels_tuple(angle_helper::select_step(
        v1,
        v2,
        nv,
        unit,
        include_last,
        threshold_factor,
    )?))
}

/// `select_step24(v1, v2, nv, include_last, threshold_factor)`: levels
/// in degrees at a round step of hours.
fn select_step24(
    v1: f64,
    v2: f64,
    nv: usize,
    include_last: bool,
    threshold_factor: f64,
) -> PyResult<(NdArray, usize, f64)> {
    Ok(levels_tuple(angle_helper::select_step24(
        v1,
        v2,
        nv,
        include_last,
        threshold_factor,
    )?))
}

/// `select_step360(v1, v2, nv, include_last, threshold_factor)`: levels
/// in degrees at a round step of degrees.
fn select_step360(
    v1: f64,
    v2: f64,
    nv: usize,
    include_last: bool,
    threshold_factor: f64,
) -> PyResult<(NdArray, usize, f64)> {
    Ok(levels_tuple(angle_helper::select_step360(
        v1,
        v2,
        nv,
        include_last,
        threshold_factor,
    )?))
}

/// `(step, factor)` of whole degrees, minutes or seconds for `dv`.
fn step_degree(dv: f64) -> PyResult<(f64, f64)> {
    Ok(select_step_degree(dv)?)
}

/// `(step, factor)` of whole hours, minutes or seconds for `dv`.
fn step_hour(dv: f64) -> PyResult<(f64, f64)> {
    Ok(select_step_hour(dv)?)
}

/// `(step, factor)` of 1, 2 or 5 in the decade of `dv`.
fn step_sub(dv: f64) -> PyResult<(f64, f64)> {
    Ok(select_step_sub(dv))
}

/// The labels of grid levels `values` (scaled by `factor`) in degrees,
/// minutes and seconds of arc (`FormatterDMS.__call__`).
fn format_dms(factor: f64, values: Vec<f64>) -> PyResult<Vec<String>> {
    Ok(FormatterDms.labels(SpineSide::Left, factor, &values)?)
}

/// The labels of grid levels `values` (degrees, scaled by `factor`) in
/// hours, minutes and seconds (`FormatterHMS.__call__`).
fn format_hms(factor: f64, values: Vec<f64>) -> PyResult<Vec<String>> {
    Ok(FormatterHms.labels(SpineSide::Left, factor, &values)?)
}

/// One crossing: `((x, y), angle in degrees)`.
type Crossing = ((f64, f64), f64);

/// The crossings with the left, right, bottom and top sides of a box.
type SideCrossings = (Vec<Crossing>, Vec<Crossing>, Vec<Crossing>, Vec<Crossing>);

/// A range a coordinate is clipped to, if any.
type Range = Option<(f64, f64)>;

/// Where the polyline `xys` (an `N × 2` array) crosses the left, right,
/// bottom and top sides of the box `(x0, y0, x1, y1)`
/// (`_find_line_box_crossings`).
fn find_line_box_crossings(
    xys: NdArray,
    (x0, y0, x1, y1): (f64, f64, f64, f64),
) -> PyResult<SideCrossings> {
    if xys.shape.len() != 2 || xys.shape[1] != 2 {
        return Err(PyErr::value_error(format!(
            "xys must be an N x 2 array, not of shape {:?}",
            xys.shape
        )));
    }
    let points: Vec<Point> = xys
        .data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[x, y]| Point::new(x, y))
        .collect();
    let [left, right, bottom, top] =
        line_box_crossings(&points, Bbox::from_extents(x0, y0, x1, y1)).map(|side| {
            side.into_iter()
                .map(|(point, angle)| ((point.x, point.y), angle))
                .collect()
        });
    Ok((left, right, bottom, top))
}

/// A box `(x0, y0, x1, y1)`.
type Extents = (f64, f64, f64, f64);

/// The extents of `bbox`.
fn extents(bbox: Bbox) -> Extents {
    (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())
}

/// The `nx × ny` points an extreme finder samples in the box `bbox`, as
/// an `N × 2` array (`ExtremeFinderSimple._find_transformed_bbox`'s
/// grid).
fn sample_grid(bbox: Extents, nx: usize, ny: usize) -> PyResult<NdArray> {
    let (x0, y0, x1, y1) = bbox;
    let points = grid_finder::sample_grid(Bbox::from_extents(x0, y0, x1, y1), nx, ny);
    let count = points.len();
    let data = points.into_iter().flat_map(|p| [p.x, p.y]).collect();
    NdArray::with_shape(data, vec![count, 2])
}

/// The box of transformed samples `lon` and `lat`, padded by one
/// sampling step of an `nx × ny` grid (`ExtremeFinderSimple`).
fn simple_extent(lon: Vec<f64>, lat: Vec<f64>, nx: usize, ny: usize) -> PyResult<Extents> {
    let points: Vec<Point> = lon
        .into_iter()
        .zip(lat)
        .map(|(x, y)| Point::new(x, y))
        .collect();
    Ok(extents(grid_finder::padded_extent(&points, nx, ny)))
}

/// The box of transformed samples `lon` and `lat` for coordinates taken
/// modulo a cycle or clipped to a range (`ExtremeFinderCycle`).
fn cycle_extent(
    lon: Vec<f64>,
    lat: Vec<f64>,
    (nx, ny): (usize, usize),
    (lon_cycle, lat_cycle): (Option<f64>, Option<f64>),
    (lon_minmax, lat_minmax): (Range, Range),
) -> PyResult<Extents> {
    let finder = ExtremeFinderCycle {
        nx,
        ny,
        lon_cycle,
        lat_cycle,
        lon_minmax,
        lat_minmax,
    };
    Ok(extents(finder.extent(lon, lat)))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("sample_grid", sample_grid)?;
    module.add("simple_extent", simple_extent)?;
    module.add("cycle_extent", cycle_extent)?;
    module.add("select_step", select_step)?;
    module.add("select_step24", select_step24)?;
    module.add("select_step360", select_step360)?;
    module.add("select_step_degree", step_degree)?;
    module.add("select_step_hour", step_hour)?;
    module.add("select_step_sub", step_sub)?;
    module.add("format_dms", format_dms)?;
    module.add("format_hms", format_hms)?;
    module.add("find_line_box_crossings", find_line_box_crossings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_and_labels_follow_matplotlib() {
        let (levels, n, factor) = select_step(-180.0, 180.0, 10, false, true, 3600.0).unwrap();
        assert_eq!((levels.data.len(), n, factor), (13, 13, 1.0));
        assert_eq!(levels.data[0], -180.0);
        let (_, _, hours) = select_step24(-12.0, 12.0, 10, true, 3600.0).unwrap();
        assert_eq!(hours, 60.0);
        assert_eq!(
            format_dms(60.0, vec![0.0, 90.0]).unwrap(),
            [r"$0^{\circ}\,00^{\prime}$", r"$1^{\circ}\,30^{\prime}$"]
        );
        assert_eq!(format_hms(1.0, vec![120.0]).unwrap(), [r"$8^\mathrm{h}$"]);
        assert!(step_hour(100.0).is_err());
        assert_eq!(step_degree(10.0).unwrap(), (10.0, 1.0));
        assert_eq!(step_sub(0.15).unwrap(), (1.0, 10.0));
    }

    #[test]
    fn crossings_and_extents_cover_the_box() {
        let xys = NdArray::with_shape(vec![-3.0, 0.0, 3.0, 1.0], vec![2, 2]).unwrap();
        let (left, right, bottom, top) =
            find_line_box_crossings(xys, (-2.0, -1.0, 2.0, 2.0)).unwrap();
        assert_eq!((left.len(), right.len()), (1, 1));
        assert!(bottom.is_empty() && top.is_empty());
        assert_eq!(left[0].0.0, -2.0);
        let flat = NdArray::from_vec(vec![1.0, 2.0]);
        assert!(find_line_box_crossings(flat, (0.0, 0.0, 1.0, 1.0)).is_err());
        let grid = sample_grid((0.0, 0.0, 1.0, 1.0), 3, 2).unwrap();
        assert_eq!(grid.shape, [6, 2]);
        let (x0, _, x1, _) = simple_extent(vec![0.0, 1.0], vec![0.0, 1.0], 2, 2).unwrap();
        assert_eq!((x0, x1), (-0.5, 1.5));
        let (lon0, lat0, lon1, lat1) = cycle_extent(
            vec![350.0, 10.0],
            vec![-100.0, 100.0],
            (20, 20),
            (Some(360.0), None),
            (None, Some((-90.0, 90.0))),
        )
        .unwrap();
        assert!(lon0 < -10.0 && lon1 > 10.0 && lon1 - lon0 < 360.0);
        assert_eq!((lat0, lat1), (-90.0, 90.0));
    }
}
