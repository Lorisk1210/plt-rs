//! The handles interactive selectors draw to resize their shape
//! (matplotlib's `ToolLineHandles` and `ToolHandles`).

use pltrs_core::Point;

use crate::artist::{Artist, ArtistHandle};
use crate::axes::Axes;
use crate::colors::Color;
use crate::lines::{Line2D, LineArtist, LineProps, LineStyle};
use crate::markers::MarkerSymbol;
use crate::orientation::Orientation;
use crate::rc::ColorOrAuto;

/// The display transform of the data of `ax`, for distances in pixels.
fn data_to_display(ax: &Axes) -> Option<crate::transforms::ResolvedTransform> {
    ax.figure().resolve_transform(&ax.trans_data()).ok()
}

/// The index and value of the smallest of `distances` (the first on ties,
/// like `numpy.argmin`).
fn closest(distances: impl Iterator<Item = f64>) -> Option<(usize, f64)> {
    distances
        .enumerate()
        .reduce(|best, next| if next.1 < best.1 { next } else { best })
}

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
        let trans = data_to_display(&self.ax)?;
        closest(self.positions().iter().map(|&p| match self.direction {
            Orientation::Horizontal => (trans.transform_point(Point::new(p, 0.0)).x - x).abs(),
            Orientation::Vertical => (trans.transform_point(Point::new(0.0, p)).y - y).abs(),
        }))
    }
}

/// Markers at the corners, edges or centre of a shape (matplotlib's
/// `ToolHandles`): one line of markers without a line.
#[derive(Debug)]
pub(super) struct MarkerHandles {
    ax: Axes,
    markers: Line2D,
}

impl MarkerHandles {
    /// Hidden `marker`s at data points `(x, y)`, half transparent with white
    /// faces unless `props` say otherwise.
    pub(super) fn new(
        ax: &Axes,
        (x, y): (Vec<f64>, Vec<f64>),
        marker: MarkerSymbol,
        props: &LineProps,
    ) -> Self {
        let defaults = LineProps::default()
            .marker(marker)
            .markersize(7.0)
            .markerfacecolor(ColorOrAuto::Color(Color::from("w")))
            .linestyle(LineStyle::None)
            .alpha(0.5)
            .visible(false)
            .label("_nolegend_");
        let mut line = LineArtist::new(x, y, crate::rc::params());
        defaults.overlay(props.clone()).apply(&mut line);
        line.base_mut().transform = Some(ax.trans_data());
        Self {
            ax: ax.clone(),
            markers: ax.add_line_artist(line),
        }
    }

    /// The line drawing the markers.
    pub(super) fn line(&self) -> &Line2D {
        &self.markers
    }

    /// Moves the markers to data points `(x, y)`.
    pub(super) fn set_data(&self, (x, y): (Vec<f64>, Vec<f64>)) {
        self.markers.set_data(&x, &y);
    }

    /// Shows or hides the markers.
    pub(super) fn set_visible(&self, visible: bool) {
        self.markers.set_visible(visible);
    }

    /// Sets every property `props` sets on the markers.
    pub(super) fn update_props(&self, props: &LineProps) {
        self.markers.update_props(props.clone());
    }

    /// The marker closest to display point `(x, y)` and its distance in
    /// pixels (matplotlib's `closest`).
    pub(super) fn closest(&self, x: f64, y: f64) -> Option<(usize, f64)> {
        let trans = data_to_display(&self.ax)?;
        let (xs, ys) = (self.markers.xdata(), self.markers.ydata());
        closest(xs.iter().zip(&ys).map(|(&px, &py)| {
            let p = trans.transform_point(Point::new(px, py));
            (p.x - x).hypot(p.y - y)
        }))
    }
}
