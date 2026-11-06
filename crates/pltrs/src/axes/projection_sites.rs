//! Where matplotlib maps the parts of an axis through the transforms of a
//! projection of the program's (the gallery's Hammer projection): its
//! `Tick` objects are `Text` and `Line2D` artists placed by the
//! transforms the class gives (`get_xaxis_transform`,
//! `get_xaxis_text1_transform` and friends), which call the program's
//! code each time they are measured or drawn:
//!
//! * a label maps its position through the label transform of its side
//!   once per measure (`_get_ticklabel_bboxes`) and per draw;
//! * a gridline or tick mark (`Line2D.draw`, a new `TransformedPath`, as
//!   the tick update moved the line) maps its path, then its vertices as
//!   points.
//!
//! The sites are reached as those of the program's scales (see
//! `transform_sites` and `tick_sites`), the figure lent meanwhile; pltrs
//! lays out and draws the same parts its own way, answered from these
//! calls (see [`program_calls`]).

use pltrs_core::Path;

use super::AxesState;
use super::axis_layout::AxisPainter;
use super::transform_sites::TickDraw;
use crate::artist::{ArtistId, LayoutContext};
use crate::figure::FigureState;
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, TransformContext, program_calls};

impl FigureState {
    /// The label of the tick at `loc` on the first (or second) side of
    /// axis `name` of `axes`, placed through the axes' projection: the
    /// program's label transform called as matplotlib's `Text` maps its
    /// position.
    pub(super) fn projection_label(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        loc: f64,
        first_side: bool,
    ) {
        let Some(display) = self.axes_bbox(axes) else {
            return;
        };
        // Held apart from the figure, which the program may change.
        let Some(held) = self.store.shared_as::<AxesState>(axes) else {
            return;
        };
        let side = SpineSide::of(name, !first_side);
        self.lend(|_| program_calls::observed(|| held.custom_tick_label(side, loc, &display)));
    }

    /// The parts of ticks `drawn` of axis `name` of `axes` mapped through
    /// the transforms of its projection, as `Tick.draw` maps them: the
    /// gridlines through the grid transform, the marks through the
    /// transform of their side, the labels through theirs.
    pub(super) fn projection_tick_draws(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        drawn: &[TickDraw],
    ) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let painter = AxisPainter::new(axes, state, name);
        let layout = LayoutContext::new(self);
        let grid = layout.resolve(&painter.grid_transform());
        let marks = [false, true].map(|high| layout.resolve(&painter.side_transform(layout, high)));
        let jobs: Vec<Job> = (drawn.iter())
            .filter_map(|part| match part {
                TickDraw::Gridline(_, path) => Some(Job::Line(grid.clone()?, path.clone())),
                &TickDraw::Mark(loc, high) => {
                    let resolved = marks[usize::from(high)].clone()?;
                    Some(Job::Line(resolved, painter.tick_mark_path(loc, high)))
                }
                &TickDraw::Label(loc, high) => Some(Job::Label(loc, !high)),
            })
            .collect();
        for job in jobs {
            match job {
                Job::Line(resolved, path) => self.projection_line(&resolved, &path),
                Job::Label(loc, first_side) => self.projection_label(axes, name, loc, first_side),
            }
        }
    }

    /// `path`, a line of a tick, mapped through `resolved` as matplotlib's
    /// `Line2D.draw` maps it when it calls the program: the path, then its
    /// vertices as points, the figure lent meanwhile.
    fn projection_line(&mut self, resolved: &ResolvedTransform, path: &Path) {
        if !resolved.calls_program() {
            return;
        }
        self.lend(|_| {
            program_calls::observed(|| {
                resolved.split_path(path);
                resolved.split_points(path);
            });
        });
    }
}

/// What a part of a tick maps through a projection: a line through its
/// transform, or the label at a location on its first (or second) side.
enum Job {
    Line(ResolvedTransform, Path),
    Label(f64, bool),
}
