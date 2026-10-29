//! Tracing streamlines through a vector field on a grid (matplotlib's
//! `_get_integrator`, `_integrate_rk12`, `_euler_step` and `interpgrid`).

use std::cell::Cell;

use pltrs_core::Point;

use super::domain::DomainMap;
use crate::numeric::Grid;

/// The most integration steps all the streamlines of a plot take. Steps
/// are rarely more than a few per cell of the occupancy mask, but a field
/// whose direction flips within a cell makes a streamline dither there
/// until its maximum length, for each cell (matplotlib takes hours).
pub const MAX_STREAMLINE_STEPS: usize = 1 << 23;

/// Which way streamlines are traced from their start points (matplotlib's
/// `integration_direction`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IntegrationDirection {
    /// Along the flow.
    Forward,
    /// Against the flow.
    Backward,
    /// Both ways, each limited to half the maximum length.
    #[default]
    Both,
}

/// Limits of the traced streamlines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TraceLimits {
    /// Shortest kept streamline, in axes-fraction lengths.
    pub(crate) minlength: f64,
    /// Longest streamline per direction, in axes-fraction lengths.
    pub(crate) maxlength: f64,
    pub(crate) direction: IntegrationDirection,
    /// Whether streamlines stop where they meet another one.
    pub(crate) broken: bool,
    /// Multiplier on the largest integration step.
    pub(crate) max_step_scale: f64,
    /// Multiplier on the largest accepted integration error.
    pub(crate) max_error_scale: f64,
}

/// Why integration stopped.
enum Stop {
    /// The point left the grid.
    OutOfBounds,
    /// The speed vanished or the field is masked there.
    Terminate,
}

/// The velocity field in grid coordinates and the speed in axes-fraction
/// lengths, both bilinearly interpolated; NaN marks masked values.
#[derive(Debug)]
pub(crate) struct Integrator {
    u: Grid<f64>,
    v: Grid<f64>,
    speed: Grid<f64>,
    limits: TraceLimits,
    /// The integration steps left of [`MAX_STREAMLINE_STEPS`]; `None` once
    /// a step was refused.
    steps_left: Cell<Option<usize>>,
}

impl Integrator {
    /// The integrator of the data-space velocities `u`, `v` (non-finite
    /// values are masked).
    pub(crate) fn new(dmap: &DomainMap, u: &Grid<f64>, v: &Grid<f64>, limits: TraceLimits) -> Self {
        let mask = |value: f64| if value.is_finite() { value } else { f64::NAN };
        let rows = u.rows();
        let cols = u.cols();
        let grid_u = Grid::from_fn(rows, cols, |i, j| dmap.data2grid(mask(u[(i, j)]), 0.0).0);
        let grid_v = Grid::from_fn(rows, cols, |i, j| dmap.data2grid(0.0, mask(v[(i, j)])).1);
        let (nx, ny) = (dmap.grid.nx as f64, dmap.grid.ny as f64);
        let speed = Grid::from_fn(rows, cols, |i, j| {
            let u_ax = grid_u[(i, j)] / (nx - 1.0);
            let v_ax = grid_v[(i, j)] / (ny - 1.0);
            (u_ax * u_ax + v_ax * v_ax).sqrt()
        });
        Self {
            u: grid_u,
            v: grid_v,
            speed,
            limits,
            steps_left: Cell::new(Some(MAX_STREAMLINE_STEPS)),
        }
    }

    /// Whether the streamlines needed more than [`MAX_STREAMLINE_STEPS`]
    /// (the last ones were cut short).
    pub(crate) fn exhausted(&self) -> bool {
        self.steps_left.get().is_none()
    }

    /// Takes one integration step from the budget; `false` when none is
    /// left.
    fn take_step(&self) -> bool {
        let left = self.steps_left.get().and_then(|left| left.checked_sub(1));
        self.steps_left.set(left);
        left.is_some()
    }

    /// The direction of travel per unit of axes-fraction length at grid
    /// coordinates (`sign` -1 traces backwards).
    fn step_direction(
        &self,
        dmap: &DomainMap,
        p: (f64, f64),
        sign: f64,
    ) -> Result<(f64, f64), Stop> {
        let (xi, yi) = p;
        if !dmap.grid.contains(xi, yi) {
            return Err(Stop::OutOfBounds);
        }
        let ds_dt = interpolate(&self.speed, xi, yi).ok_or(Stop::Terminate)?;
        if ds_dt == 0.0 {
            return Err(Stop::Terminate);
        }
        let dt_ds = 1.0 / ds_dt;
        let ui = interpolate(&self.u, xi, yi).ok_or(Stop::Terminate)?;
        let vi = interpolate(&self.v, xi, yi).ok_or(Stop::Terminate)?;
        Ok((sign * (ui * dt_ds), sign * (vi * dt_ds)))
    }

    /// The trajectory through grid coordinates `start`, or `None` if its
    /// cell is taken or it is shorter than the minimum length (its cells
    /// are then freed again).
    pub(crate) fn trace(&self, dmap: &mut DomainMap, start: (f64, f64)) -> Option<Vec<(f64, f64)>> {
        let (x0, y0) = start;
        let limits = &self.limits;
        if !dmap.start_trajectory(x0, y0, limits.broken) {
            return None;
        }
        let mut total = 0.0;
        let mut trajectory = Vec::new();
        if limits.direction != IntegrationDirection::Forward {
            let (length, mut backward) = self.rk12(dmap, start, -1.0);
            total += length;
            backward.reverse();
            trajectory.extend(backward);
        }
        if limits.direction != IntegrationDirection::Backward {
            dmap.reset_start_point(x0, y0);
            let (length, forward) = self.rk12(dmap, start, 1.0);
            total += length;
            trajectory.extend(forward.into_iter().skip(1));
        }
        if total > limits.minlength {
            Some(trajectory)
        } else {
            dmap.mask.undo_trajectory();
            None
        }
    }

    /// Second-order Runge-Kutta (Heun) integration with an adaptive step,
    /// from `start` until the grid edge, a taken cell, a stagnation point
    /// or the maximum length. Returns the length travelled and the points.
    fn rk12(&self, dmap: &mut DomainMap, start: (f64, f64), sign: f64) -> (f64, Vec<(f64, f64)>) {
        let limits = &self.limits;
        // Below what matching an RK4 integrator would need, for looks:
        // smaller errors make corners jagged.
        let max_error = 0.003 * limits.max_error_scale;
        let (mask_rows, mask_cols) = dmap.mask.shape();
        // Never skip a mask cell.
        let max_ds = (1.0 / mask_cols as f64)
            .min(1.0 / mask_rows as f64)
            .min(0.1)
            * limits.max_step_scale;
        let (nx, ny) = (dmap.grid.nx as f64, dmap.grid.ny as f64);
        let mut ds = max_ds;
        let mut total = 0.0;
        let (mut xi, mut yi) = start;
        let mut points = Vec::new();
        loop {
            if !dmap.grid.contains(xi, yi) || !self.take_step() {
                break;
            }
            points.push((xi, yi));
            let k1 = match self.step_direction(dmap, (xi, yi), sign) {
                Ok(k1) => k1,
                Err(Stop::OutOfBounds) => {
                    total += self.euler_step(dmap, &mut points, sign);
                    break;
                }
                Err(Stop::Terminate) => break,
            };
            let k2 = match self.step_direction(dmap, (xi + ds * k1.0, yi + ds * k1.1), sign) {
                Ok(k2) => k2,
                Err(Stop::OutOfBounds) => {
                    total += self.euler_step(dmap, &mut points, sign);
                    break;
                }
                Err(Stop::Terminate) => break,
            };
            let (dx1, dy1) = (ds * k1.0, ds * k1.1);
            let (dx2, dy2) = (ds * 0.5 * (k1.0 + k2.0), ds * 0.5 * (k1.1 + k2.1));
            // The error in axes-fraction lengths.
            let error = ((dx2 - dx1) / (nx - 1.0)).hypot((dy2 - dy1) / (ny - 1.0));
            if error < max_error {
                xi += dx2;
                yi += dy2;
                if !dmap.update_trajectory(xi, yi, limits.broken) {
                    break;
                }
                if total + ds > limits.maxlength {
                    break;
                }
                total += ds;
            }
            ds = if error == 0.0 {
                max_ds
            } else {
                max_ds.min(0.85 * ds * (max_error / error).powf(0.5))
            };
        }
        (total, points)
    }

    /// Extends the trajectory to the grid edge with one Euler step from its
    /// last point, returning the length added.
    fn euler_step(&self, dmap: &DomainMap, points: &mut Vec<(f64, f64)>, sign: f64) -> f64 {
        let Some(&(xi, yi)) = points.last() else {
            return 0.0;
        };
        let Ok((cx, cy)) = self.step_direction(dmap, (xi, yi), sign) else {
            return 0.0;
        };
        let (nx, ny) = (dmap.grid.nx as f64, dmap.grid.ny as f64);
        let to_edge = |c: f64, at: f64, n: f64| {
            if c == 0.0 {
                f64::INFINITY
            } else if c < 0.0 {
                at / -c
            } else {
                (n - 1.0 - at) / c
            }
        };
        let ds = to_edge(cx, xi, nx).min(to_edge(cy, yi, ny));
        points.push((xi + cx * ds, yi + cy * ds));
        ds
    }
}

/// Bilinear interpolation of `a` at fractional grid indices (matplotlib's
/// `interpgrid`); `None` where a surrounding value is masked (NaN).
pub(crate) fn interpolate(a: &Grid<f64>, xi: f64, yi: f64) -> Option<f64> {
    let (x, y) = (xi as usize, yi as usize);
    let xn = if x + 1 >= a.cols() { x } else { x + 1 };
    let yn = if y + 1 >= a.rows() { y } else { y + 1 };
    let (a00, a01) = (*a.get(y, x)?, *a.get(y, xn)?);
    let (a10, a11) = (*a.get(yn, x)?, *a.get(yn, xn)?);
    if [a00, a01, a10, a11].iter().any(|v| v.is_nan()) {
        return None;
    }
    let (xt, yt) = (xi - x as f64, yi - y as f64);
    let a0 = a00 * (1.0 - xt) + a01 * xt;
    let a1 = a10 * (1.0 - xt) + a11 * xt;
    Some(a0 * (1.0 - yt) + a1 * yt)
}

/// `points` (grid coordinates) in data coordinates.
pub(crate) fn to_data(dmap: &DomainMap, points: &[(f64, f64)]) -> Vec<Point> {
    points
        .iter()
        .map(|&(xg, yg)| dmap.grid2data(xg, yg))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streamplot::domain::UniformGrid;
    use crate::streamplot::mask::StreamMask;

    fn limits() -> TraceLimits {
        TraceLimits {
            minlength: 0.1,
            maxlength: 2.0,
            direction: IntegrationDirection::Both,
            broken: true,
            max_step_scale: 1.0,
            max_error_scale: 1.0,
        }
    }

    fn domain() -> DomainMap {
        let x: Vec<f64> = (0..5).map(f64::from).collect();
        DomainMap::new(
            UniformGrid::new(&x, &x).unwrap(),
            StreamMask::new((1.0, 1.0)).unwrap(),
        )
    }

    #[test]
    fn interpolation_is_bilinear_and_respects_masks() {
        let a = Grid::from_fn(2, 2, |i, j| (i * 2 + j) as f64);
        assert_eq!(interpolate(&a, 0.5, 0.5), Some(1.5));
        assert_eq!(interpolate(&a, 1.0, 1.0), Some(3.0), "upper edge");
        let mut masked = a.clone();
        masked[(1, 1)] = f64::NAN;
        assert_eq!(interpolate(&masked, 0.0, 0.0), None, "any masked corner");
    }

    #[test]
    fn uniform_flow_traces_straight_lines_to_the_edges() {
        let mut dmap = domain();
        let u = Grid::filled(5, 5, 1.0);
        let v = Grid::filled(5, 5, 0.0);
        let integrator = Integrator::new(&dmap, &u, &v, limits());
        let line = integrator
            .trace(&mut dmap, (2.0, 2.0))
            .expect("long enough");
        assert_eq!(line.first(), Some(&(0.0, 2.0)));
        assert_eq!(line.last(), Some(&(4.0, 2.0)));
        assert!(line.iter().all(|&(_, y)| (y - 2.0).abs() < 1e-12));
        // The cells it crossed are taken now.
        assert!(integrator.trace(&mut dmap, (1.0, 2.0)).is_none());
    }

    #[test]
    fn stagnant_or_masked_fields_trace_nothing() {
        let mut dmap = domain();
        let zero = Grid::filled(5, 5, 0.0);
        let integrator = Integrator::new(&dmap, &zero, &zero, limits());
        assert!(integrator.trace(&mut dmap, (2.0, 2.0)).is_none());
        let nan = Grid::filled(5, 5, f64::NAN);
        let integrator = Integrator::new(&dmap, &nan, &zero, limits());
        assert!(integrator.trace(&mut dmap, (2.0, 2.0)).is_none());
        let (rows, cols) = dmap.mask.shape();
        let free = (0..rows).all(|r| (0..cols).all(|c| dmap.mask.is_free(c, r)));
        assert!(free, "rejected trajectories free their cells");
    }

    #[test]
    fn forward_traces_skip_the_start_point() {
        let mut dmap = domain();
        let u = Grid::filled(5, 5, 1.0);
        let v = Grid::filled(5, 5, 0.0);
        let forward = TraceLimits {
            direction: IntegrationDirection::Forward,
            ..limits()
        };
        let integrator = Integrator::new(&dmap, &u, &v, forward);
        let line = integrator
            .trace(&mut dmap, (2.0, 2.0))
            .expect("long enough");
        assert!(line[0].0 > 2.0);
        assert_eq!(line.last(), Some(&(4.0, 2.0)));
    }
}
