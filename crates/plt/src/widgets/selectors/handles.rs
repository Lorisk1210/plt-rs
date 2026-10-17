//! The handles interactive selectors draw to resize their shape
//! (matplotlib's `ToolLineHandles`).

use plt_core::Point;

use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::lines::{Line2D, LineProps};
use crate::widgets::Orientation;

/// Lines across the axes at the edges of a span (matplotlib's
/// `ToolLineHandles`): vertical lines for a horizontal span, horizontal
/// lines for a vertical one.
#[derive(Debug)]
pub(super) struct LineHandles {
    ax: Axes,
    direction: Orientation,
    lines: Vec<Line2D>,
}

impl LineHandles {
    /// Hidden handle lines at data `positions` along `direction`.
    pub(super) fn new(
        ax: &Axes,
        positions: [f64; 2],
        direction: Orientation,
        props: &LineProps,
    ) -> Self {
        let props = props.clone().visible(false);
        let lines = positions
            .iter()
            .map(|&p| match direction {
                Orientation::Horizontal => ax.axvline(p, props.clone()),
                Orientation::Vertical => ax.axhline(p, props.clone()),
            })
            .collect();
        Self {
            ax: ax.clone(),
            direction,
            lines,
        }
    }

    /// The lines.
    pub(super) fn lines(&self) -> &[Line2D] {
        &self.lines
    }

    /// The data positions of the handles.
    pub(super) fn positions(&self) -> Vec<f64> {
        self.lines
            .iter()
            .filter_map(|line| match self.direction {
                Orientation::Horizontal => line.xdata().first().copied(),
                Orientation::Vertical => line.ydata().first().copied(),
            })
            .collect()
    }

    /// Moves the handles to data `positions`.
    pub(super) fn set_data(&self, positions: (f64, f64)) {
        for (line, p) in self.lines.iter().zip([positions.0, positions.1]) {
            match self.direction {
                Orientation::Horizontal => line.set_xdata(vec![p, p]),
                Orientation::Vertical => line.set_ydata(vec![p, p]),
            };
        }
    }

    /// Shows or hides the handles.
    pub(super) fn set_visible(&self, visible: bool) {
        for line in &self.lines {
            line.set_visible(visible);
        }
    }

    /// Sets every property `props` sets on each handle.
    pub(super) fn update_props(&self, props: &LineProps) {
        for line in &self.lines {
            line.update_props(props.clone());
        }
    }

    /// Removes the handles from the axes.
    pub(super) fn remove(&self) {
        for line in &self.lines {
            line.remove();
        }
    }

    /// The handle closest to display point `(x, y)` along the direction,
    /// and its distance in pixels (matplotlib's `closest`).
    pub(super) fn closest(&self, x: f64, y: f64) -> Option<(usize, f64)> {
        let trans = self
            .ax
            .figure()
            .resolve_transform(&self.ax.trans_data())
            .ok()?;
        self.positions()
            .iter()
            .map(|&p| match self.direction {
                Orientation::Horizontal => (trans.transform_point(Point::new(p, 0.0)).x - x).abs(),
                Orientation::Vertical => (trans.transform_point(Point::new(0.0, p)).y - y).abs(),
            })
            .enumerate()
            .reduce(|best, next| if next.1 < best.1 { next } else { best })
    }
}
