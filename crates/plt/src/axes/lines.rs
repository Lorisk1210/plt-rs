//! Line plotting: `plot`.

use std::fmt;

use plt_core::Point;

use super::{Axes, AxesState};
use crate::artist::{ArtistHandle, RawHandle};
use crate::lines::{Line2D, LineArtist};
use crate::rc::CycleValue;
use crate::transforms::Transform;

/// Invalid data passed to a plotting method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlotError {
    /// x and y have different lengths.
    LengthMismatch {
        /// Number of x values.
        x: usize,
        /// Number of y values.
        y: usize,
    },
}

impl fmt::Display for PlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { x, y } => write!(
                f,
                "x and y must have same first dimension, but have shapes ({x},) and ({y},)"
            ),
        }
    }
}

impl std::error::Error for PlotError {}

impl Axes {
    /// Plots `y` against `x` as a line using the next property-cycle style
    /// (matplotlib's `ax.plot(x, y)`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().add_axes([0.1, 0.1, 0.8, 0.8]);
    /// let line = ax.plot([1.0, 2.0, 3.0], [1.0, 4.0, 9.0])?;
    /// line.set_label("squares");
    /// assert_eq!(ax.xlim(), (0.9, 3.1));
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn plot(&self, x: impl AsRef<[f64]>, y: impl AsRef<[f64]>) -> Result<Line2D, PlotError> {
        let (x, y) = (x.as_ref(), y.as_ref());
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch { x: x.len(), y: y.len() });
        }
        Ok(self.add_line(x.to_vec(), y.to_vec()))
    }

    /// Plots `y` against its indices `0, 1, 2, ...` (matplotlib's `ax.plot(y)`).
    pub fn plot_y(&self, y: impl AsRef<[f64]>) -> Line2D {
        let y = y.as_ref().to_vec();
        let x = (0..y.len()).map(|i| i as f64).collect();
        self.add_line(x, y)
    }

    fn add_line(&self, x: Vec<f64>, y: Vec<f64>) -> Line2D {
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let axes_id = self.id();
        let mut line = LineArtist::new(x, y, crate::rc::params());
        let style = state
            .store
            .get_as_mut::<AxesState>(axes_id)
            .map(|axes| {
                let index = axes.line_cycle;
                axes.line_cycle += 1;
                axes.prop_cycle.entry(index).map(|(k, v)| (k.to_owned(), v.clone())).collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for (key, value) in style {
            apply_cycle(&mut line, &key, value);
        }
        let base = crate::artist::Artist::base_mut(&mut line);
        base.transform = Some(Transform::Data(axes_id));
        base.clip_box = Some(axes_id);
        let points: Vec<Point> = line.drawn_points();
        let id = state.adopt(axes_id, Box::new(line));
        if let Some(axes) = state.store.get_as_mut::<AxesState>(axes_id) {
            axes.add_data(&points, true, true);
        }
        Line2D::from_raw(RawHandle::new(std::rc::Rc::clone(figure), id))
    }
}

/// Applies one property-cycle entry to a new line.
fn apply_cycle(line: &mut LineArtist, key: &str, value: CycleValue) {
    match (key, value) {
        ("color", CycleValue::Color(color)) => line.color = color,
        ("linestyle", CycleValue::LineStyle(style)) => line.linestyle = style,
        ("linewidth", CycleValue::Number(width)) => line.linewidth = width,
        ("marker", CycleValue::Marker(marker)) => line.marker = marker,
        ("markersize", CycleValue::Number(size)) => line.markersize = size,
        ("markeredgewidth", CycleValue::Number(width)) => line.markeredgewidth = width,
        ("markeredgecolor", CycleValue::Color(color)) => line.markeredgecolor = color.into(),
        ("markerfacecolor", CycleValue::Color(color)) => line.markerfacecolor = color.into(),
        ("alpha", CycleValue::Number(alpha)) => crate::artist::Artist::base_mut(line).alpha = Some(alpha),
        ("dashes", CycleValue::Dashes(pattern)) => line.linestyle = crate::lines::LineStyle::dashes(0.0, pattern),
        _ => {}
    }
}
