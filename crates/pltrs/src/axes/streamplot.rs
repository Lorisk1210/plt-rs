//! Streamline plots of vector fields (matplotlib's `streamplot`).

use std::rc::Rc;

use pltrs_core::Affine2D;

use super::Axes;
use super::children::DataUpdate;
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionArtist, CollectionKind, ColorMapping, EdgeColors};
use crate::colors::{Color, ScalarMappable};
use crate::patches::{ArrowStyle, FancyArrowPatch, Patch, PatchArtist};
use crate::rc::RcParams;
use crate::streamplot::{
    PlannedArrow, StreamColors, StreamField, StreamPlan, StreamWidths, StreamplotError,
    StreamplotOptions, StreamplotSet,
};
use crate::transforms::Transform;

/// The z-order and default width shared by streamlines and arrows.
struct Shared {
    zorder: f64,
    width: f64,
}

/// How one arrow of a streamplot looks.
struct ArrowLook<'a> {
    /// The plot's arrow style.
    style: &'a ArrowStyle,
    /// The scale of the style's sizes, in points.
    mutation_scale: f64,
    /// The edge width in points.
    width: f64,
    /// The face and edge colour.
    color: Color,
    /// The drawing order, the streamlines'.
    zorder: f64,
}

/// How the streamlines of one plot are coloured.
enum LineColors {
    Single(Color),
    Mapped(Box<ScalarMappable>),
}

impl Axes {
    /// Draws streamlines of `field` (matplotlib's `streamplot`): a line
    /// collection of the traced streamlines plus arrows along them.
    ///
    /// ```
    /// use pltrs::numeric::{Grid, linspace};
    /// use pltrs::prelude::*;
    /// use pltrs::streamplot::{StreamField, StreamplotOptions};
    /// let x = linspace(0.0, 1.0, 10);
    /// let field = StreamField::new(x.clone(), x, Grid::filled(10, 10, 1.0), Grid::filled(10, 10, 0.5));
    /// let ax = Figure::new().subplot();
    /// let set = ax.streamplot(&field, StreamplotOptions::default())?;
    /// assert_eq!(set.lines.sticky_edges(), (vec![0.0, 1.0], vec![0.0, 1.0]));
    /// # Ok::<(), pltrs::streamplot::StreamplotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the grid is not evenly spaced and increasing, a field does not
    /// match it, a start point lies outside it, an option is out of range
    /// or the colours cannot be mapped; when the density asks for more
    /// than [`MAX_STREAMLINE_CELLS`] mask cells, the arrows number more
    /// than [`MAX_STREAMLINE_ARROWS`], or tracing takes more than
    /// [`MAX_STREAMLINE_STEPS`] integration steps.
    ///
    /// [`MAX_STREAMLINE_CELLS`]: crate::streamplot::MAX_STREAMLINE_CELLS
    /// [`MAX_STREAMLINE_ARROWS`]: crate::streamplot::MAX_STREAMLINE_ARROWS
    /// [`MAX_STREAMLINE_STEPS`]: crate::streamplot::MAX_STREAMLINE_STEPS
    pub fn streamplot(
        &self,
        field: &StreamField,
        options: StreamplotOptions,
    ) -> Result<StreamplotSet, StreamplotError> {
        let rc = crate::rc::params();
        let mut plan = StreamPlan::new(field, &options)?;
        let transform = options
            .transform
            .clone()
            .unwrap_or_else(|| self.trans_data());
        let shared = Shared {
            zorder: options.zorder.unwrap_or(2.0),
            width: match &options.linewidth {
                Some(StreamWidths::Uniform(width)) => *width,
                _ => rc.lines_linewidth,
            },
        };
        let colors = self.stream_colors(options.color, &rc)?;
        // matplotlib's `add_collection` brings the view up to date before
        // the streamlines count; the arrows are then measured in that view.
        self.raw()
            .figure_state()
            .borrow_mut()
            .unstale_axes(self.id());
        let (lines, arrow_colors) =
            self.add_streamlines(&mut plan, colors, &shared, &transform, Rc::clone(&rc))?;
        let arrows = plan
            .arrows
            .iter()
            .zip(arrow_colors)
            .map(|(arrow, color)| {
                let look = ArrowLook {
                    style: &options.arrowstyle,
                    mutation_scale: 10.0 * options.arrowsize,
                    width: arrow.width.unwrap_or(shared.width),
                    color,
                    zorder: shared.zorder,
                };
                self.add_stream_arrow(arrow, look, &transform, Rc::clone(&rc))
            })
            .collect();
        Ok(StreamplotSet { lines, arrows })
    }

    /// Adds the arrow `arrow` along a streamline, in `transform`, as
    /// matplotlib's `streamplot` adds each with `add_patch`: a
    /// `FancyArrowPatch` coloured and as wide as its streamline there,
    /// whose outline counts in the data limits.
    fn add_stream_arrow(
        &self,
        arrow: &PlannedArrow,
        look: ArrowLook<'_>,
        transform: &Transform,
        rc: Rc<RcParams>,
    ) -> Patch<FancyArrowPatch> {
        let shape = FancyArrowPatch::between(arrow.tail, arrow.head)
            .with_arrowstyle(look.style.clone())
            .with_mutation_scale(look.mutation_scale);
        let mut patch = PatchArtist::new(shape, rc);
        patch.style.facecolor = Some(look.color.clone());
        patch.style.edgecolor = Some(look.color);
        patch.style.linewidth = Some(look.width);
        let base = patch.base_mut();
        base.zorder = look.zorder;
        base.transform = Some(transform.clone());
        let outline = patch.shape.outline_data_points(
            transform,
            look.width,
            1.0,
            &*self.raw().figure_state().borrow(),
            self.id(),
        );
        let update = outline.map_or_else(DataUpdate::default, DataUpdate::data);
        Patch::from_raw(self.adopt_in_autoscale(patch, &update))
    }

    /// Adds the streamlines of `plan` as a line collection in `transform`
    /// that sticks to the grid edges; returns it with the colour of each
    /// planned arrow.
    fn add_streamlines(
        &self,
        plan: &mut StreamPlan,
        colors: LineColors,
        shared: &Shared,
        transform: &Transform,
        rc: Rc<RcParams>,
    ) -> Result<(Collection, Vec<Color>), StreamplotError> {
        let mut lines =
            CollectionArtist::new(CollectionKind::Line, std::mem::take(&mut plan.paths), rc);
        let (arrow_colors, mapping) = match colors {
            LineColors::Single(color) => {
                lines.edgecolors = EdgeColors::Colors(vec![color.clone()]);
                (vec![color; plan.arrows.len()], None)
            }
            LineColors::Mapped(mut mappable) => {
                let values: Vec<Option<f64>> = plan.arrows.iter().map(|a| a.value).collect();
                let rgba = mappable.to_rgba(&values, None)?;
                let mapping = ColorMapping {
                    values: plan.values.take().unwrap_or_default(),
                    mappable: *mappable,
                };
                (rgba.into_iter().map(Color::from).collect(), Some(mapping))
            }
        };
        lines.linewidths = Some(plan.widths.take().unwrap_or_else(|| vec![shared.width]));
        let base = lines.base_mut();
        base.zorder = shared.zorder;
        base.transform = Some(transform.clone());
        let grid = plan.grid;
        base.sticky_edges = (
            vec![grid.origin.x, grid.origin.x + grid.width],
            vec![grid.origin.y, grid.origin.y + grid.height],
        );
        // matplotlib's `Collection.get_datalim`: the paths through
        // `transform - transData`, or nothing outside the data coordinates.
        let update = match transform.affine_before_data(self.id()) {
            _ if lines.paths.is_empty() => DataUpdate::data(Vec::new()),
            Some(to_data) if to_data == Affine2D::IDENTITY => {
                DataUpdate::limits(lines.shape_extents())
            }
            Some(to_data) => DataUpdate::data(
                lines
                    .paths
                    .iter()
                    .flat_map(|path| path.vertices().iter())
                    .map(|&v| to_data.transform_point(v))
                    .collect(),
            ),
            None => DataUpdate::data(Vec::new()),
        };
        let lines = Collection::from_raw(self.adopt_collection(lines, &update));
        if let Some(mapping) = mapping {
            lines.set_mapping(mapping);
        }
        Ok((lines, arrow_colors))
    }

    /// The streamline colours: the next line colour, a single colour, or
    /// values mapped through a colormap and a norm spanning their range.
    fn stream_colors(
        &self,
        colors: Option<StreamColors>,
        rc: &RcParams,
    ) -> Result<LineColors, StreamplotError> {
        Ok(match colors {
            None => LineColors::Single(self.next_line_color()),
            Some(StreamColors::Single(color)) => LineColors::Single(color),
            Some(StreamColors::Mapped(map)) => {
                let finite: Vec<Option<f64>> = map
                    .values
                    .as_slice()
                    .iter()
                    .map(|&v| v.is_finite().then_some(v))
                    .collect();
                let mut mappable = map.colorizer.into_mappable(rc)?;
                mappable.norm_mut().autoscale_none(&finite);
                LineColors::Mapped(Box::new(mappable))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Point;

    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::numeric::{Grid, linspace};
    use crate::patches::ArrowRoute;
    use crate::streamplot::{
        StreamColorMap, StreamColors, StreamField, StreamWidths, StreamplotOptions,
    };
    use crate::testing::RecordingRenderer;

    fn field() -> StreamField {
        let x = linspace(-2.0, 2.0, 15);
        let u = Grid::from_fn(15, 15, |i, _| -x[i]);
        let v = Grid::from_fn(15, 15, |_, j| x[j]);
        StreamField::new(x.clone(), x, u, v)
    }

    #[test]
    fn streamlines_stick_to_the_grid_and_draw() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let set = ax
            .streamplot(&field(), StreamplotOptions::default())
            .unwrap();
        assert_eq!(set.lines.zorder(), 2.0);
        assert_eq!(ax.xlim(), (-2.0, 2.0));
        assert_eq!(set.arrows.len(), set.lines.paths().len());
        let ArrowRoute::Between(tail, head) = set.arrows[0].shape().route else {
            panic!("arrows run between two points");
        };
        assert_ne!(tail, head);
        assert_eq!(
            ax.patches().len(),
            set.arrows.len(),
            "the arrows are patches"
        );
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).unwrap();
    }

    #[test]
    fn stagnant_fields_draw_no_streamlines() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let mut field = field();
        field.u = Grid::filled(15, 15, 0.0);
        field.v = Grid::filled(15, 15, f64::NAN);
        let set = ax.streamplot(&field, StreamplotOptions::default()).unwrap();
        assert_eq!((set.lines.paths().len(), set.arrows.len()), (0, 0));
        // matplotlib 3.11 gives the same view: no data, only the margins.
        assert_eq!(
            ax.xlim(),
            (-0.055_000_000_000_000_01, 0.055_000_000_000_000_01)
        );
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).unwrap();
    }

    #[test]
    fn mapped_colours_and_widths() {
        let field = field();
        let options = StreamplotOptions {
            color: Some(StreamColors::Mapped(Box::new(StreamColorMap::new(
                field.u.clone(),
            )))),
            linewidth: Some(StreamWidths::Varying(Grid::filled(15, 15, 3.0))),
            start_points: Some(vec![Point::new(1.0, 0.0)]),
            ..StreamplotOptions::default()
        };
        let ax = Figure::new().subplot();
        let set = ax.streamplot(&field, options).unwrap();
        assert!(set.lines.paths().len() > 1, "one path per step");
        assert_eq!(set.arrows.len(), 1);
    }
}
