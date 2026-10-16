//! The positions and components of a vector field (matplotlib's
//! `_parse_args` and the masking of `set_UVC`).

use plt_core::Point;

use crate::axes::{MappedColors, PlotError};
use crate::collections::ColorMapping;
use crate::numeric::{Grid, meshgrid};
use crate::rc::RcParams;

/// Vector positions and components, optionally with values mapped to
/// colours (the positional `X, Y, U, V, C` of `quiver` and `barbs`).
///
/// The components (and colour values) hold one entry per position or a
/// single entry shared by all. Non-finite entries are masked: nothing is
/// drawn at those positions.
#[derive(Debug)]
pub struct VectorField {
    /// x of every position.
    pub x: Vec<f64>,
    /// y of every position.
    pub y: Vec<f64>,
    /// x components.
    pub u: Vec<f64>,
    /// y components.
    pub v: Vec<f64>,
    /// Values mapped to colours (matplotlib's `C`).
    pub colors: Option<MappedColors>,
}

impl VectorField {
    /// Components `u`, `v` at the positions `x`, `y`.
    pub fn new(
        x: impl Into<Vec<f64>>,
        y: impl Into<Vec<f64>>,
        u: impl Into<Vec<f64>>,
        v: impl Into<Vec<f64>>,
    ) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            u: u.into(),
            v: v.into(),
            colors: None,
        }
    }

    /// Components on a grid placed at its column and row indices
    /// (matplotlib's `quiver(U, V)`).
    pub fn on_index_grid(u: &Grid<f64>, v: &Grid<f64>) -> Self {
        let columns: Vec<f64> = (0..u.cols()).map(|c| c as f64).collect();
        let rows: Vec<f64> = (0..u.rows()).map(|r| r as f64).collect();
        Self::on_axes(&columns, &rows, u, v)
    }

    /// Components on the grid spanned by one x per column and one y per
    /// row (matplotlib's 1-D `X`, `Y` matching the columns and rows).
    pub fn on_axes(x: &[f64], y: &[f64], u: &Grid<f64>, v: &Grid<f64>) -> Self {
        let (xx, yy) = meshgrid(x, y);
        Self::on_mesh(&xx, &yy, u, v)
    }

    /// Components and positions given as grids of the same shape, read
    /// row by row.
    pub fn on_mesh(x: &Grid<f64>, y: &Grid<f64>, u: &Grid<f64>, v: &Grid<f64>) -> Self {
        Self::new(x.as_slice(), y.as_slice(), u.as_slice(), v.as_slice())
    }

    /// Maps `colors` (one value per position) to the arrow colours.
    #[must_use]
    pub fn with_colors(self, colors: MappedColors) -> Self {
        Self {
            colors: Some(colors),
            ..self
        }
    }

    /// Checks the sizes and broadcasts the components to every position
    /// (matplotlib's `_parse_args` plus the size checks of `set_UVC`).
    pub(crate) fn into_points(self, rc: &RcParams) -> Result<FieldPoints, PlotError> {
        let n = self.x.len();
        if self.y.len() != n {
            return Err(PlotError::LengthMismatch {
                x: n,
                y: self.y.len(),
            });
        }
        let u = broadcast("U", self.u, n)?;
        let v = broadcast("V", self.v, n)?;
        let colors = match self.colors {
            Some(colors) => {
                let mut mapping = colors.into_mapping(rc)?;
                if mapping.values.len() == 1 {
                    mapping.values = vec![mapping.values[0]; n];
                } else if mapping.values.len() != n {
                    return Err(PlotError::Broadcast {
                        argument: "C",
                        len: mapping.values.len(),
                        points: n,
                    });
                }
                Some(mapping)
            }
            None => None,
        };
        let xy = self
            .x
            .iter()
            .zip(&self.y)
            .map(|(&x, &y)| Point::new(x, y))
            .collect();
        Ok(FieldPoints { xy, u, v, colors })
    }
}

/// `values` repeated to `n` entries if it holds one, checked otherwise.
fn broadcast(argument: &'static str, values: Vec<f64>, n: usize) -> Result<Vec<f64>, PlotError> {
    match values.len() {
        len if len == n => Ok(values),
        1 => Ok(vec![values[0]; n]),
        len => Err(PlotError::Broadcast {
            argument,
            len,
            points: n,
        }),
    }
}

/// A validated field: one position, component pair and colour value
/// (`None` = masked) per vector.
#[derive(Debug)]
pub(crate) struct FieldPoints {
    pub(crate) xy: Vec<Point>,
    pub(crate) u: Vec<f64>,
    pub(crate) v: Vec<f64>,
    pub(crate) colors: Option<ColorMapping>,
}

impl FieldPoints {
    /// Whether vector `i` is masked: a non-finite component or colour
    /// value (matplotlib's `masked_invalid` and `mask_or`).
    pub(crate) fn masked(&self, i: usize) -> bool {
        !(self.u[i].is_finite() && self.v[i].is_finite())
            || self.colors.as_ref().is_some_and(|c| c.values[i].is_none())
    }

    /// Which vectors are kept when the masked ones and those at
    /// non-finite positions are dropped (matplotlib's
    /// `delete_masked_points`).
    pub(crate) fn unmasked(&self) -> Vec<bool> {
        (0..self.xy.len())
            .map(|i| self.xy[i].is_finite() && !self.masked(i))
            .collect()
    }

    /// The vectors flagged in `keep`.
    pub(crate) fn retain(self, keep: &[bool]) -> Self {
        let colors = self.colors.map(|mut mapping| {
            mapping.values = retained(mapping.values, keep);
            mapping
        });
        Self {
            xy: retained(self.xy, keep),
            u: retained(self.u, keep),
            v: retained(self.v, keep),
            colors,
        }
    }
}

/// The items of `values` flagged in `keep`.
pub(crate) fn retained<T>(values: Vec<T>, keep: &[bool]) -> Vec<T> {
    values
        .into_iter()
        .zip(keep)
        .filter_map(|(value, &k)| k.then_some(value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rc() -> RcParams {
        RcParams::matplotlib()
    }

    #[test]
    fn index_grids_place_vectors_at_columns_and_rows() {
        let u = Grid::filled(2, 3, 1.0);
        let field = VectorField::on_index_grid(&u, &u);
        assert_eq!(field.x, [0.0, 1.0, 2.0, 0.0, 1.0, 2.0]);
        assert_eq!(field.y, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn components_broadcast_and_sizes_are_checked() {
        let points = VectorField::new([0.0, 1.0], [0.0, 1.0], [2.0], [3.0, 4.0])
            .into_points(&rc())
            .unwrap();
        assert_eq!(points.u, [2.0, 2.0]);
        let err = VectorField::new([0.0, 1.0], [0.0, 1.0], [1.0, 2.0, 3.0], [1.0])
            .into_points(&rc())
            .unwrap_err();
        assert!(matches!(
            err,
            PlotError::Broadcast {
                argument: "U",
                len: 3,
                points: 2
            }
        ));
        assert!(
            VectorField::new([0.0], [0.0, 1.0], [1.0], [1.0])
                .into_points(&rc())
                .is_err()
        );
    }

    #[test]
    fn non_finite_vectors_are_masked_and_dropped() {
        let field = VectorField::new(
            [0.0, 1.0, 2.0, f64::NAN],
            [0.0; 4],
            [1.0, f64::NAN, 1.0, 1.0],
            [1.0; 4],
        )
        .with_colors(MappedColors::new(vec![1.0, 2.0, f64::INFINITY, 4.0]));
        let points = field.into_points(&rc()).unwrap();
        assert_eq!(
            (0..4).map(|i| points.masked(i)).collect::<Vec<_>>(),
            [false, true, true, false]
        );
        let keep = points.unmasked();
        assert_eq!(keep, [true, false, false, false]);
        let kept = points.retain(&keep);
        assert_eq!(kept.xy, [Point::new(0.0, 0.0)]);
        assert_eq!(kept.colors.unwrap().values, [Some(1.0)]);
    }

    #[test]
    fn empty_fields_are_valid() {
        let points = VectorField::new(Vec::new(), Vec::new(), Vec::new(), Vec::new())
            .into_points(&rc())
            .unwrap();
        assert_eq!(points.xy.len(), 0);
    }
}
