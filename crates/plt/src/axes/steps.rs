//! Step plots: `step` and `stairs`.

use crate::units::AxisValues;
use plt_core::Point;

use super::children::DataUpdate;
use super::data_coords::grid_points_in_data;
use super::{Axes, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::colors::Color;
use crate::lines::{Line2D, LineProps, StepWhere};
use crate::numeric::{Number, to_f64s};
use crate::orientation::Orientation;
use crate::patches::{Patch, PatchArtist, StepBaseline, StepPatch};

/// Options of [`Axes::stairs`] (matplotlib's `stairs` keywords).
#[derive(Clone, Debug, Default)]
pub struct StairsOptions {
    /// Step boundaries (`None` = `0, 1, ..., values.len()`).
    pub edges: Option<Vec<f64>>,
    /// Where the outer edges end.
    pub baseline: StepBaseline,
    /// Whether values grow along y (vertical) or x.
    pub orientation: Orientation,
    /// Fill the area between the steps and the baseline.
    pub fill: bool,
    /// Colour (`None` = the next colour of the line cycle).
    pub color: Option<Color>,
}

impl Axes {
    /// A line through `(x, y)` drawn as steps (matplotlib's `step`).
    pub fn step(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        at: StepWhere,
        props: LineProps,
    ) -> Result<Line2D, PlotError> {
        self.plot_with(x, y, props.drawstyle(at))
    }

    /// A stepwise constant function drawn as a patch (matplotlib's
    /// `stairs`). A baseline is sticky: margins do not extend past it.
    pub fn stairs(
        &self,
        values: impl IntoIterator<Item: Number>,
        options: StairsOptions,
    ) -> Result<Patch<StepPatch>, PlotError> {
        let values = to_f64s(values);
        let edges = options
            .edges
            .unwrap_or_else(|| (0..=values.len()).map(|i| i as f64).collect());
        let first_edge = edges.first().copied().unwrap_or(0.0);
        let low = match &options.baseline {
            StepBaseline::None => None,
            StepBaseline::Value(base) => Some(*base),
            StepBaseline::PerStep(base) => Some(base.iter().copied().fold(f64::INFINITY, f64::min)),
        };
        let shape = StepPatch::new(values, edges)?
            .with_baseline(options.baseline)?
            .with_orientation(options.orientation);
        let color = options.color.unwrap_or_else(|| self.next_line_color());

        let mut patch = PatchArtist::new(shape, crate::rc::params());
        patch.style.fill = options.fill;
        if options.fill {
            patch.style.linewidth = Some(0.0);
            patch.style.facecolor = Some(color);
        } else {
            patch.style.edgecolor = Some(color);
        }
        let mut points = patch.data_points();
        // A baseline is sticky: margins do not extend past it.
        if let Some(low) = low {
            let sticky = &mut patch.base_mut().sticky_edges;
            match options.orientation {
                Orientation::Vertical => sticky.1.push(low),
                Orientation::Horizontal => sticky.0.push(low),
            }
            let (x, y) = options.orientation.place(first_edge, low);
            points.push(Point::new(x, y));
        }
        let transform = patch
            .base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data())
            .clone();
        let points = grid_points_in_data(Some(&transform), self.id(), points);
        Ok(Patch::from_raw(
            self.adopt_in_autoscale(patch, &DataUpdate::data(points)),
        ))
    }
}
