//! Tracing the streamlines of a field and cutting them into the paths,
//! widths, colour values and arrows streamplot draws (the body of
//! matplotlib's `streamplot`).

use plt_core::{Path, Point};

use super::domain::{DomainMap, UniformGrid};
use super::integrate::{Integrator, TraceLimits, interpolate, to_data};
use super::mask::{StreamMask, starting_points};
use super::{
    IntegrationDirection, StreamColors, StreamField, StreamWidths, StreamplotError,
    StreamplotOptions,
};
use crate::numeric::Grid;

/// One arrow along a streamline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PlannedArrow {
    /// Tail and head in data coordinates.
    pub(crate) tail: Point,
    pub(crate) head: Point,
    /// The width at the tail, for varying widths.
    pub(crate) width: Option<f64>,
    /// The colour value at the tail (`None` = masked), for mapped colours.
    pub(crate) value: Option<f64>,
}

/// The traced streamlines, ready to draw.
#[derive(Debug)]
pub(crate) struct StreamPlan {
    pub(crate) grid: UniformGrid,
    /// One path per streamline, or one per step when widths or colours
    /// vary along them.
    pub(crate) paths: Vec<Path>,
    /// The width of every path, for varying widths.
    pub(crate) widths: Option<Vec<f64>>,
    /// The colour value of every path (`None` = masked), for mapped
    /// colours.
    pub(crate) values: Option<Vec<Option<f64>>>,
    pub(crate) arrows: Vec<PlannedArrow>,
}

impl StreamPlan {
    /// Validates the input and traces the streamlines.
    pub(crate) fn new(
        field: &StreamField,
        options: &StreamplotOptions,
    ) -> Result<Self, StreamplotError> {
        let grid = UniformGrid::new(&field.x, &field.y)?;
        let mask = StreamMask::new(options.density)?;
        check_scales(options)?;
        let color_values = match &options.color {
            Some(StreamColors::Mapped(map)) => Some(&map.values),
            _ => None,
        };
        let width_values = match &options.linewidth {
            Some(StreamWidths::Varying(widths)) => Some(widths),
            _ => None,
        };
        check_shapes(
            &grid,
            [
                ("color", color_values),
                ("linewidth", width_values),
                ("u", Some(&field.u)),
                ("v", Some(&field.v)),
            ],
        )?;
        let limits = TraceLimits {
            minlength: options.minlength,
            maxlength: if options.integration_direction == IntegrationDirection::Both {
                options.maxlength / 2.0
            } else {
                options.maxlength
            },
            direction: options.integration_direction,
            broken: options.broken_streamlines,
            max_step_scale: options.integration_max_step_scale,
            max_error_scale: options.integration_max_error_scale,
        };
        let mut dmap = DomainMap::new(grid, mask);
        let integrator = Integrator::new(&dmap, &field.u, &field.v, limits);
        let trajectories = match &options.start_points {
            None => seeded(&integrator, &mut dmap),
            Some(points) => from_points(&integrator, &mut dmap, points)?,
        };
        let mut plan = Self {
            grid,
            paths: Vec::new(),
            widths: width_values.map(|_| Vec::new()),
            values: color_values.map(|_| Vec::new()),
            arrows: Vec::new(),
        };
        let samplers = Samplers {
            widths: width_values,
            colors: color_values,
        };
        for trajectory in &trajectories {
            plan.add(&dmap, trajectory, &samplers, options.num_arrows);
        }
        Ok(plan)
    }

    /// Adds one streamline (grid coordinates) with its arrows.
    fn add(
        &mut self,
        dmap: &DomainMap,
        trajectory: &[(f64, f64)],
        samplers: &Samplers<'_>,
        num_arrows: usize,
    ) {
        let points = to_data(dmap, trajectory);
        // The values at every point but the last, one per step.
        let steps = trajectory.len().saturating_sub(1);
        let sample = |grid: &Grid<f64>| -> Vec<Option<f64>> {
            trajectory[..steps]
                .iter()
                .map(|&(xg, yg)| interpolate(grid, xg, yg))
                .collect()
        };
        let widths: Option<Vec<f64>> = samplers.widths.map(|grid| {
            sample(grid)
                .into_iter()
                .map(|w| w.unwrap_or(f64::NAN))
                .collect()
        });
        let values = samplers.colors.map(sample);
        if widths.is_some() || values.is_some() {
            self.paths
                .extend(points.windows(2).map(|pair| Path::polyline(pair.to_vec())));
        } else {
            self.paths.push(Path::polyline(points.clone()));
        }
        // The distance travelled after each step.
        let distances: Vec<f64> = points
            .windows(2)
            .scan(0.0, |total, pair| {
                *total += (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
                Some(*total)
            })
            .collect();
        if let Some(&length) = distances.last() {
            for k in 1..=num_arrows {
                let target = length * (k as f64 / (num_arrows + 1) as f64);
                let idx = distances.partition_point(|&s| s < target);
                let tail = points[idx];
                let next = points.get(idx + 1).copied().unwrap_or(tail);
                self.arrows.push(PlannedArrow {
                    tail,
                    head: Point::new(f64::midpoint(tail.x, next.x), f64::midpoint(tail.y, next.y)),
                    width: widths.as_ref().and_then(|w| w.get(idx).copied()),
                    value: values.as_ref().and_then(|v| v.get(idx).copied().flatten()),
                });
            }
        }
        if let (Some(all), Some(new)) = (&mut self.widths, widths) {
            all.extend(new);
        }
        if let (Some(all), Some(new)) = (&mut self.values, values) {
            all.extend(new);
        }
    }
}

/// Checks that the integration scales are positive.
fn check_scales(options: &StreamplotOptions) -> Result<(), StreamplotError> {
    let scales = [
        (
            "integration_max_step_scale",
            options.integration_max_step_scale,
        ),
        (
            "integration_max_error_scale",
            options.integration_max_error_scale,
        ),
    ];
    match scales.into_iter().find(|&(_, value)| value <= 0.0) {
        Some((name, value)) => Err(StreamplotError::InvalidScale { name, value }),
        None => Ok(()),
    }
}

/// Checks that the given fields have the grid's shape.
fn check_shapes<const N: usize>(
    grid: &UniformGrid,
    fields: [(&'static str, Option<&Grid<f64>>); N],
) -> Result<(), StreamplotError> {
    for (field, values) in fields {
        if let Some(values) = values.filter(|values| !grid.fits(values)) {
            return Err(StreamplotError::ShapeMismatch {
                field,
                shape: (values.rows(), values.cols()),
                grid: grid.shape(),
            });
        }
    }
    Ok(())
}

/// The grids streamline widths and colour values are interpolated from.
struct Samplers<'a> {
    widths: Option<&'a Grid<f64>>,
    colors: Option<&'a Grid<f64>>,
}

/// The streamlines started from every free mask cell, spiralling inwards
/// from the domain corner.
fn seeded(integrator: &Integrator, dmap: &mut DomainMap) -> Vec<Vec<(f64, f64)>> {
    let mut trajectories = Vec::new();
    for (xm, ym) in starting_points(dmap.mask.shape()) {
        if dmap.mask.is_free(xm, ym) {
            let start = dmap.mask2grid(xm, ym);
            trajectories.extend(integrator.trace(dmap, start));
        }
    }
    trajectories
}

/// The streamlines through the data points `points`.
fn from_points(
    integrator: &Integrator,
    dmap: &mut DomainMap,
    points: &[Point],
) -> Result<Vec<Vec<(f64, f64)>>, StreamplotError> {
    let grid = dmap.grid;
    let inside = |p: &Point| {
        (grid.origin.x..=grid.origin.x + grid.width).contains(&p.x)
            && (grid.origin.y..=grid.origin.y + grid.height).contains(&p.y)
    };
    if let Some(&outside) = points.iter().find(|p| !inside(p)) {
        return Err(StreamplotError::StartPointOutside(outside));
    }
    let mut trajectories = Vec::new();
    for p in points {
        let (xg, yg) = dmap.data2grid(p.x - grid.origin.x, p.y - grid.origin.y);
        // Round-off can put points on the upper edges just outside.
        let start = (
            xg.clamp(0.0, (grid.nx - 1) as f64),
            yg.clamp(0.0, (grid.ny - 1) as f64),
        );
        trajectories.extend(integrator.trace(dmap, start));
    }
    Ok(trajectories)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::linspace;
    use crate::streamplot::StreamColorMap;

    fn rotation(n: usize) -> StreamField {
        let x = linspace(-1.0, 1.0, n);
        let u = Grid::from_fn(n, n, |i, _| -x[i]);
        let v = Grid::from_fn(n, n, |_, j| x[j]);
        StreamField::new(x.clone(), x, u, v)
    }

    #[test]
    fn seeded_streamlines_have_one_arrow_each() {
        let plan = StreamPlan::new(&rotation(20), &StreamplotOptions::default()).unwrap();
        assert_ne!(plan.paths.len(), 0);
        assert_eq!(plan.paths.len(), plan.arrows.len());
        assert!(plan.widths.is_none() && plan.values.is_none());
    }

    #[test]
    fn varying_colours_split_streamlines_into_steps() {
        let field = rotation(20);
        let options = StreamplotOptions {
            color: Some(StreamColors::Mapped(Box::new(StreamColorMap {
                values: field.u.clone(),
                cmap: None,
                norm: None,
            }))),
            num_arrows: 2,
            ..StreamplotOptions::default()
        };
        let plan = StreamPlan::new(&field, &options).unwrap();
        let values = plan.values.as_ref().expect("mapped");
        assert_eq!(values.len(), plan.paths.len());
        assert!(plan.paths.iter().all(|p| p.vertices().len() == 2));
        assert!(plan.arrows.iter().all(|a| a.value.is_some()));
    }

    #[test]
    fn start_points_must_be_inside() {
        let options = StreamplotOptions {
            start_points: Some(vec![Point::new(0.5, 0.0), Point::new(2.0, 0.0)]),
            ..StreamplotOptions::default()
        };
        assert_eq!(
            StreamPlan::new(&rotation(10), &options).unwrap_err(),
            StreamplotError::StartPointOutside(Point::new(2.0, 0.0))
        );
        let options = StreamplotOptions {
            start_points: Some(vec![Point::new(0.5, 0.0)]),
            ..StreamplotOptions::default()
        };
        let plan = StreamPlan::new(&rotation(10), &options).unwrap();
        assert_eq!(plan.paths.len(), 1);
    }

    #[test]
    fn invalid_input_is_reported() {
        let mut field = rotation(10);
        field.v = Grid::filled(3, 3, 0.0);
        assert!(matches!(
            StreamPlan::new(&field, &StreamplotOptions::default()),
            Err(StreamplotError::ShapeMismatch { field: "v", .. })
        ));
        let options = StreamplotOptions {
            integration_max_step_scale: 0.0,
            ..StreamplotOptions::default()
        };
        assert!(StreamPlan::new(&rotation(10), &options).is_err());
    }
}
