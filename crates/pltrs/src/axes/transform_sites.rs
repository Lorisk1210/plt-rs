//! Where matplotlib maps the parts of an axis through the axes' scales,
//! for the axes whose scales call the program: matplotlib's `Text`,
//! `Line2D` and `Spine` artists map their positions through the axes'
//! transforms each time they are measured or drawn, so a scale of the
//! program's is called at each of these sites, as often and with the
//! values matplotlib hands over (see `tick_sites` for where they are
//! reached):
//!
//! * `Axis._get_ticklabel_bboxes` measures the first label of every tick
//!   drawn, then the second, each once (`Text.get_window_extent`);
//! * `Tick.draw` draws the gridline, then each tick mark, each mapping
//!   its path, then its points (a new `TransformedPath`, as the tick
//!   update moved the line), then each label with text, mapping its
//!   position;
//! * `Spine.get_window_extent` and `Spine.draw` map the spine's path;
//! * `Line2D.draw` maps the line's path, then its points, unless it kept
//!   them (see `lines::program_path`).
//!
//! An axes maps the view limits through its scales once
//! (`TransformedBbox(viewLim, transScale)`) where the affine part of its
//! transforms is first needed after the limits or the scales changed.
//!
//! The parts of rectilinear axes are placed by blends of `transData` and
//! `transAxes`, whose non-affine part maps the x coordinates of every
//! point through the x scale and their y coordinates through the y scale,
//! whatever the blend takes them for. pltrs lays out and draws the same
//! parts its own way, answered from these calls (see
//! [`program_calls`](crate::transforms::program_calls)).

use std::rc::Rc;

use pltrs_core::Point;

use super::AxesState;
use super::axis_layout::AxisPainter;
use crate::artist::{Artist, ArtistId, LayoutContext};
use crate::figure::FigureState;
use crate::lines::LineArtist;
use crate::spines::{SpineArtist, SpinePosition};
use crate::ticker::AxisName;
use crate::transforms::{AxisMap, TransformContext, program_calls};

/// The maps of the scales of an axes that call the program.
#[derive(Clone)]
struct ProgramMaps {
    x: Option<Rc<dyn AxisMap>>,
    y: Option<Rc<dyn AxisMap>>,
}

impl ProgramMaps {
    /// The non-affine part of the axes' transforms applied to `points`
    /// (matplotlib's `transScale.transform_non_affine`): the x scale
    /// called with their x coordinates, then the y scale with their y
    /// coordinates.
    fn map(&self, points: &[Point]) {
        if points.is_empty() {
            return;
        }
        program_calls::observed(|| {
            for (map, coordinate) in [(&self.x, AxisName::X), (&self.y, AxisName::Y)] {
                if let Some(map) = map {
                    let mut values: Vec<f64> = (points.iter())
                        .map(|p| match coordinate {
                            AxisName::X => p.x,
                            AxisName::Y => p.y,
                        })
                        .collect();
                    program_calls::forward_all(map, &mut values);
                }
            }
        });
    }
}

/// The view limits and maps an axes mapped its view through last
/// (matplotlib's `TransformedBbox(viewLim, transScale)`, valid while
/// neither changes).
#[derive(Clone, Debug)]
pub(crate) struct ScaleBox {
    view: [u64; 4],
    maps: [Option<Rc<dyn AxisMap>>; 2],
}

impl ScaleBox {
    /// Whether the box was mapped from `view` through `maps`.
    fn is(&self, view: [u64; 4], maps: &[Option<Rc<dyn AxisMap>>; 2]) -> bool {
        let same = |a: &Option<Rc<dyn AxisMap>>, b: &Option<Rc<dyn AxisMap>>| match (a, b) {
            (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            (None, None) => true,
            _ => false,
        };
        self.view == view && same(&self.maps[0], &maps[0]) && same(&self.maps[1], &maps[1])
    }
}

impl FigureState {
    /// The maps of the scales of `axes` that call the program, when it is
    /// a rectilinear 2-D axes with at least one.
    fn program_maps(&self, axes: ArtistId) -> Option<ProgramMaps> {
        let state = self.store.get_as::<AxesState>(axes)?;
        if !state.projection.is_rectilinear() || crate::mplot3d::is_axes3d(&self.store, axes) {
            return None;
        }
        let program = |name: AxisName| {
            state
                .axis(name)
                .map()
                .filter(|map| map.calls_program())
                .cloned()
        };
        let maps = ProgramMaps {
            x: program(AxisName::X),
            y: program(AxisName::Y),
        };
        (maps.x.is_some() || maps.y.is_some()).then_some(maps)
    }

    /// `points` mapped through the transforms of `axes` with the
    /// program's scales, the non-affine part, then the affine part (as
    /// matplotlib's `transform` and `transform_path` evaluate them), the
    /// figure lent to the program meanwhile.
    fn map_through(&mut self, axes: ArtistId, maps: &ProgramMaps, points: &[Point]) {
        self.lend(|_| maps.map(points));
        self.program_scale_box(axes, maps);
    }

    /// matplotlib's `TransformedBbox(viewLim, transScale)` of `axes`,
    /// mapped again when the view limits or the scales changed since:
    /// the x scale called with the x coordinates of its four corners,
    /// then the y scale with their y coordinates.
    fn program_scale_box(&mut self, axes: ArtistId, maps: &ProgramMaps) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let view = state.view_lim;
        let key = [view.x0(), view.x1(), view.y0(), view.y1()].map(f64::to_bits);
        let all = [AxisName::X, AxisName::Y].map(|name| state.axis(name).map().cloned());
        if state
            .scale_box
            .borrow()
            .as_ref()
            .is_some_and(|b| b.is(key, &all))
        {
            return;
        }
        state.scale_box.replace(Some(ScaleBox {
            view: key,
            maps: all,
        }));
        let (x0, x1, y0, y1) = (view.x0(), view.x1(), view.y0(), view.y1());
        let corners = [
            Point::new(x0, y0),
            Point::new(x1, y0),
            Point::new(x0, y1),
            Point::new(x1, y1),
        ];
        self.lend(|_| maps.map(&corners));
    }

    /// The ticks of axis `name` of `axes`, which pltrs ticks itself,
    /// placed where the axes is now, as matplotlib's `_update_ticks`
    /// places them, when the axes maps through the program's scales (whose
    /// calls depend on the ticks measured and drawn).
    pub(crate) fn native_ticks_as_updated(&mut self, axes: ArtistId, name: AxisName) {
        if self.program_maps(axes).is_none() || self.program_ticker(axes, name).is_some() {
            return;
        }
        let Some(extent) = self.axes_bbox(axes) else {
            return;
        };
        let dpi = self.dpi();
        let tick_space = self.shared_tick_spaces(axes)[usize::from(name == AxisName::Y)];
        if let Some(state) = self.store.get_as_mut::<AxesState>(axes) {
            state.update_axis_ticks(name, &extent, dpi, tick_space);
        }
    }

    /// `_get_ticklabel_bboxes` of the ticks axis `name` of `axes` draws,
    /// as far as it calls the program's scales: the first label of each
    /// tick shown measured, then the second.
    pub(crate) fn program_label_extents(&mut self, axes: ArtistId, name: AxisName) {
        let Some(maps) = self.program_maps(axes) else {
            return;
        };
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let painter = AxisPainter::new(axes, state, name);
        let located = painter.located_ticks();
        let mut positions = Vec::new();
        for first_side in [true, false] {
            for &(tick, minor, index) in &located {
                if label_shown(state, name, (minor, index), first_side) {
                    positions.push(painter.tick_point(tick.loc, across(!first_side)));
                }
            }
        }
        for position in positions {
            self.map_through(axes, &maps, &[position]);
        }
    }

    /// `Tick.draw` of every tick axis `name` of `axes` draws, as far as it
    /// calls the program's scales.
    pub(crate) fn program_tick_draws(&mut self, axes: ArtistId, name: AxisName) {
        let Some(maps) = self.program_maps(axes) else {
            return;
        };
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let painter = AxisPainter::new(axes, state, name);
        // What each tick maps, in order: a line maps its path, then its
        // points.
        let mut drawn: Vec<(Vec<Point>, usize)> = Vec::new();
        for (tick, minor, index) in painter.located_ticks() {
            let style = state.axis(name).tick_style_at(minor, index);
            let point = |high: bool| painter.tick_point(tick.loc, across(high));
            if style.grid_on {
                drawn.push((vec![point(false), point(true)], 2));
            }
            for (on, high) in [(style.ticks.first, false), (style.ticks.second, true)] {
                // A mark of no size skips its markers, and so its path.
                if on && style.size > 0.0 {
                    drawn.push((vec![point(high)], 2));
                }
            }
            // `Text.draw` draws nothing for a label without text.
            for high in [false, true] {
                if !tick.label.is_empty() && label_shown(state, name, (minor, index), !high) {
                    drawn.push((vec![point(high)], 1));
                }
            }
        }
        for (points, times) in drawn {
            for _ in 0..times {
                self.lend(|_| maps.map(&points));
            }
            self.program_scale_box(axes, &maps);
        }
    }

    /// The path of `spine` mapped through the program's scales of its
    /// axes, as `Spine.get_window_extent` (`drawn` false) or `Spine.draw`
    /// maps it.
    pub(crate) fn program_spine_path(&mut self, spine: ArtistId, drawn: bool) {
        let Some(spine_state) = self.store.get_as::<SpineArtist>(spine) else {
            return;
        };
        let Some(axes) = spine_state.base().parent else {
            return;
        };
        if drawn && !spine_state.base().visible {
            return;
        }
        let Some(maps) = self.program_maps(axes) else {
            return;
        };
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let Some(points) = spine_points(spine_state, state) else {
            return;
        };
        self.map_through(axes, &maps, &points);
    }

    /// `Line2D.draw` of the line `line` of `axes`, as far as it maps the
    /// line through a transform calling the program (see
    /// [`crate::lines`]'s `program_path`), then the affine part.
    pub(crate) fn program_line_drawn(&mut self, axes: ArtistId, line: ArtistId) {
        let Some(artist) = self.store.shared_as::<LineArtist>(line) else {
            return;
        };
        if let Some(mapping) = artist.program_mapping(LayoutContext::new(self)) {
            self.lend(|_| program_calls::observed(mapping));
        }
        if let Some(maps) = self.program_maps(axes) {
            self.program_scale_box(axes, &maps);
        }
    }
}

/// Whether the label on the first or the second side of the major (or
/// `minor`) tick at `index` of axis `name` is shown (matplotlib's
/// `label1On`, `label2On` and the label's own visibility).
fn label_shown(
    axes: &AxesState,
    name: AxisName,
    (minor, index): (bool, usize),
    first: bool,
) -> bool {
    let style = axes.axis(name).tick_label_style_at(minor, index, first);
    if first {
        style.labels.first
    } else {
        style.labels.second
    }
}

/// The position across an axis of its first (0) or second (1) side.
fn across(high: bool) -> f64 {
    f64::from(u8::from(high))
}

/// The vertices of the path of a spine of a rectilinear axes, in the
/// coordinates its blend of data and axes fractions maps (matplotlib's
/// `_adjust_location`); `None` for a spine placed in data coordinates,
/// whose blend maps them twice.
fn spine_points(spine: &SpineArtist, axes: &AxesState) -> Option<Vec<Point>> {
    let (low, high) = spine
        .bounds
        .unwrap_or_else(|| axes.spine_interval(spine.side));
    let at = match spine.position {
        SpinePosition::Outward(_) => across(spine.side.is_high()),
        SpinePosition::Axes(fraction) => fraction,
        SpinePosition::Data(_) => return None,
    };
    Some(match spine.side.axis() {
        AxisName::X => vec![Point::new(low, at), Point::new(high, at)],
        AxisName::Y => vec![Point::new(at, low), Point::new(at, high)],
    })
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    use pltrs_core::archive::External;

    use crate::axes::{Axes, SpineAxis, TickLevels, TickUpdater, UpdatedLevel, UpdatedTicks};
    use crate::figure::Figure;
    use crate::scale::{ArrayLayout, FuncScale, FuncTransform};
    use crate::ticker::{AxisInfo, AxisName, TickerError};

    /// Ticks the program's axes at 1 and 4, labelled.
    #[derive(Debug)]
    struct Ticking;

    impl TickUpdater for Ticking {
        fn update_ticks(
            &self,
            _: &Axes,
            _: AxisName,
            _: &AxisInfo,
            levels: TickLevels,
        ) -> Result<UpdatedTicks, TickerError> {
            Ok(UpdatedTicks {
                major: levels.major.then(|| UpdatedLevel {
                    locs: vec![1.0, 4.0],
                    labels: vec!["1".to_owned(), "4".to_owned()],
                    drawn: vec![true, true],
                }),
                minor: None,
            })
        }

        fn offset(&self, _: &Axes, _: AxisName, _: &AxisInfo) -> String {
            String::new()
        }

        fn spines(&self, _: &Axes) -> Vec<SpineAxis> {
            Vec::new()
        }
    }

    /// The arrays a square-root scale of the program's on the y axis of
    /// a figure is called with while the figure is drawn twice.
    fn drawn_twice() -> [Vec<Vec<f64>>; 2] {
        let calls: Arc<Mutex<Vec<Vec<f64>>>> = Arc::default();
        let seen = Arc::clone(&calls);
        let forward = FuncTransform::on_arrays(
            move |values: &mut [f64], _: ArrayLayout| {
                seen.lock().expect("not poisoned").push(values.to_vec());
                for v in values.iter_mut() {
                    *v = v.sqrt();
                }
            },
            |values: &mut [f64], _: ArrayLayout| values.iter_mut().for_each(|v| *v *= *v),
            (External::new("forward"), External::new("inverse")),
        );
        let fig = Figure::with_size(3.0, 2.0, 50.0);
        let ax = fig.subplot();
        ax.plot([1.0, 2.0], [1.0, 4.0]).expect("a line");
        ax.set_xticks([1.0]);
        ax.set_scale(AxisName::Y, Box::new(FuncScale::from_transform(forward)));
        fig.set_tick_updater(Some(Rc::new(Ticking) as Rc<dyn TickUpdater>));
        [(); 2].map(|()| {
            calls.lock().expect("not poisoned").clear();
            fig.to_png().expect("drawn");
            calls.lock().expect("not poisoned").clone()
        })
    }

    #[test]
    fn a_draw_maps_each_part_through_the_program_s_scale_as_matplotlib() {
        // matplotlib 3.11's calls of the scale with rows (those with
        // columns are its tick updates) for the same figure: the x tick
        // label measured (its y, 0, through the scale), the view's four
        // corners, the x label measured again, the bottom spine, the y
        // labels measured twice, the y tick marks (path, then points) and
        // labels drawn, the left spine measured, the line (path, then
        // points), then the four spines drawn.
        let first: [&[f64]; 24] = [
            &[0.0],
            &[0.85, 0.85, 4.15, 4.15],
            &[0.0],
            &[0.0],
            &[0.0],
            &[0.0],
            &[0.0, 0.0],
            &[1.0],
            &[4.0],
            &[1.0],
            &[1.0],
            &[1.0],
            &[4.0],
            &[4.0],
            &[4.0],
            &[1.0],
            &[4.0],
            &[0.85, 4.15],
            &[1.0, 4.0],
            &[1.0, 4.0],
            &[0.85, 4.15],
            &[0.85, 4.15],
            &[0.0, 0.0],
            &[1.0, 1.0],
        ];
        let [drawn, again] = drawn_twice();
        let rounded = |calls: Vec<Vec<f64>>| -> Vec<Vec<f64>> {
            (calls.into_iter())
                .map(|call| call.iter().map(|v| (v * 1e4).round() / 1e4).collect())
                .collect()
        };
        assert_eq!(rounded(drawn), first.map(<[f64]>::to_vec));
        // The second draw keeps the view and the line mapped.
        let second: Vec<Vec<f64>> = (first.iter())
            .filter(|call| call.len() != 4 && **call != [1.0, 4.0])
            .map(|call| call.to_vec())
            .collect();
        assert_eq!(rounded(again), second);
    }
}
