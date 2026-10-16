//! Streamline plots of vector fields (matplotlib's `streamplot`).

use std::rc::Rc;

use super::Axes;
use super::children::DataUpdate;
use crate::artist::Artist;
use crate::collections::{Collection, CollectionArtist, CollectionKind, ColorMapping, EdgeColors};
use crate::colors::{Color, Normalize, ScalarMappable, colormaps};
use crate::numeric::{nanmax, nanmin};
use crate::rc::RcParams;
use crate::streamplot::{
    ArrowLook, StreamArrow, StreamArrowArtist, StreamColors, StreamField, StreamPlan, StreamWidths,
    StreamplotError, StreamplotOptions, StreamplotSet,
};

/// The z-order and default width shared by streamlines and arrows.
struct Shared {
    zorder: f64,
    width: f64,
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
    /// use plt::numeric::{Grid, linspace};
    /// use plt::prelude::*;
    /// use plt::streamplot::{StreamField, StreamplotOptions};
    /// let x = linspace(0.0, 1.0, 10);
    /// let field = StreamField::new(x.clone(), x, Grid::filled(10, 10, 1.0), Grid::filled(10, 10, 0.5));
    /// let ax = Figure::new().subplot();
    /// let set = ax.streamplot(&field, StreamplotOptions::default())?;
    /// assert_eq!(set.lines.sticky_edges(), (vec![0.0, 1.0], vec![0.0, 1.0]));
    /// # Ok::<(), plt::streamplot::StreamplotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the grid is not evenly spaced and increasing, a field does not
    /// match it, a start point lies outside it, an option is out of range
    /// or the colours cannot be mapped.
    pub fn streamplot(
        &self,
        field: &StreamField,
        options: StreamplotOptions,
    ) -> Result<StreamplotSet, StreamplotError> {
        let rc = crate::rc::params();
        let mut plan = StreamPlan::new(field, &options)?;
        let shared = Shared {
            zorder: options.zorder.unwrap_or(2.0),
            width: match &options.linewidth {
                Some(StreamWidths::Uniform(width)) => *width,
                _ => rc.lines_linewidth,
            },
        };
        let colors = self.stream_colors(options.color, &rc)?;
        let (lines, arrow_colors) = self.add_streamlines(&mut plan, colors, &shared, rc)?;
        let arrows = plan
            .arrows
            .iter()
            .zip(arrow_colors)
            .map(|(arrow, color)| {
                let look = ArrowLook {
                    style: options.arrowstyle,
                    mutation_scale: 10.0 * options.arrowsize,
                    linewidth: arrow.width.unwrap_or(shared.width),
                    color,
                };
                let artist = StreamArrowArtist::new(
                    (arrow.tail, arrow.head),
                    look,
                    self.trans_data(),
                    shared.zorder,
                );
                StreamArrow::from_raw(self.adopt(artist, &DataUpdate::default()))
            })
            .collect();
        Ok(StreamplotSet { lines, arrows })
    }

    /// Adds the streamlines of `plan` as a line collection that sticks to
    /// the grid edges; returns it with the colour of each planned arrow.
    fn add_streamlines(
        &self,
        plan: &mut StreamPlan,
        colors: LineColors,
        shared: &Shared,
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
        base.transform = Some(self.trans_data());
        let grid = plan.grid;
        base.sticky_edges = (
            vec![grid.origin.x, grid.origin.x + grid.width],
            vec![grid.origin.y, grid.origin.y + grid.height],
        );
        let update = if lines.paths.is_empty() {
            DataUpdate::data(Vec::new())
        } else {
            DataUpdate::limits(lines.shape_extents())
        };
        let lines = Collection::from_raw(self.adopt(lines, &update));
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
                let cmap = match map.cmap {
                    Some(cmap) => cmap,
                    None => colormaps::get(&rc.image_cmap)?,
                };
                let finite: Vec<Option<f64>> = map
                    .values
                    .as_slice()
                    .iter()
                    .map(|&v| v.is_finite().then_some(v))
                    .collect();
                let range = nanmin(finite.iter().flatten().copied())
                    .zip(nanmax(finite.iter().flatten().copied()));
                let norm = map.norm.unwrap_or_else(|| match range {
                    Some((vmin, vmax)) => Box::new(Normalize::new(vmin, vmax)),
                    None => Box::new(Normalize::default()),
                });
                let mut mappable = ScalarMappable::with_norm(cmap, norm);
                mappable.norm_mut().autoscale_none(&finite);
                LineColors::Mapped(Box::new(mappable))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use plt_core::Point;

    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::numeric::{Grid, linspace};
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
        let (tail, head) = set.arrows[0].positions();
        assert_ne!(tail, head);
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
            color: Some(StreamColors::Mapped(Box::new(StreamColorMap {
                values: field.u.clone(),
                cmap: None,
                norm: None,
            }))),
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
