//! The artists a colorbar draws in its axes: the colour mesh, the
//! extension patches, the outline, the dividers between colours and lines
//! at given levels (matplotlib's `_add_solids`, `_do_extends`,
//! `_update_dividers` and `add_lines`).

use plt_core::{Path, Point, Rgba};
use plt_render::CapStyle;

use super::ColorbarError;
use crate::artist::{Artist, ArtistHandle, BaseProps, ClipPathSpec};
use crate::axes::{Axes, DataUpdate, Mesh, MeshCoords, PcolorOptions, Shading};
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, EdgeColors,
};
use crate::colors::{Color, ScalarMappable};
use crate::image::{ColorizerOptions, ScalarField};
use crate::numeric::Grid;
use crate::orientation::Orientation;
use crate::patches::{PatchArtist, PathShape};
use crate::transforms::TransformContext;

/// The number of colours from which the mesh is rasterized in vector
/// output (matplotlib's `Colorbar.n_rasterize`).
const RASTERIZE_FROM: usize = 50;

/// `(along, across)` as `(x, y)` for a colorbar of `orientation`.
fn place(orientation: Orientation, along: f64, across: f64) -> Point {
    match orientation {
        Orientation::Vertical => Point::new(across, along),
        Orientation::Horizontal => Point::new(along, across),
    }
}

/// The shape of the extensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Extensions {
    /// The lower extension length (0 without one), as a fraction of the
    /// interior.
    pub(crate) lower: f64,
    /// The upper extension length.
    pub(crate) upper: f64,
    /// Rectangles instead of triangles.
    pub(crate) rect: bool,
}

impl Extensions {
    /// The outline around the interior and the extensions, in axes
    /// coordinates, as `(along, across)` pairs.
    fn outline(&self) -> Vec<(f64, f64)> {
        let (bot, top) = (-self.lower, 1.0 + self.upper);
        if self.rect {
            vec![
                (0.0, 0.0),
                (bot, 0.0),
                (bot, 1.0),
                (0.0, 1.0),
                (1.0, 1.0),
                (top, 1.0),
                (top, 0.0),
                (1.0, 0.0),
                (0.0, 0.0),
            ]
        } else {
            vec![
                (0.0, 0.0),
                (bot, 0.5),
                (0.0, 1.0),
                (1.0, 1.0),
                (top, 0.5),
                (1.0, 0.0),
                (0.0, 0.0),
            ]
        }
    }

    /// The patch filling the lower (`upper = false`) or upper extension.
    fn patch(&self, upper: bool) -> Vec<(f64, f64)> {
        let (edge, tip) = if upper {
            (1.0, 1.0 + self.upper)
        } else {
            (0.0, -self.lower)
        };
        if self.rect {
            vec![(edge, 0.0), (tip, 0.0), (tip, 1.0), (edge, 1.0)]
        } else {
            vec![(edge, 0.0), (tip, 0.5), (edge, 1.0)]
        }
    }
}

/// Draws a colorbar's artists into its axes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Painter<'a> {
    /// The colorbar axes.
    pub(crate) ax: &'a Axes,
    /// The colorbar's orientation.
    pub(crate) orientation: Orientation,
    /// The opacity of the colours.
    pub(crate) alpha: Option<f64>,
}

impl Painter<'_> {
    fn points(&self, along_across: &[(f64, f64)]) -> Vec<Point> {
        along_across
            .iter()
            .map(|&(along, across)| place(self.orientation, along, across))
            .collect()
    }

    /// Hides the spines and frames the colorbar with an outline around the
    /// interior and the extensions (matplotlib's `_ColorbarSpine`).
    pub(crate) fn outline(&self, extensions: &Extensions) {
        for spine in self.ax.spines().iter() {
            spine.set_visible(false);
        }
        let rc = crate::rc::params();
        let path = Path::closed(self.points(&extensions.outline()));
        let mut outline = PatchArtist::new(PathShape(path), rc.clone());
        outline.style.facecolor = Some(Color::none());
        outline.style.edgecolor = Some(rc.axes_edgecolor.clone());
        outline.style.linewidth = Some(rc.axes_linewidth);
        outline.style.capstyle = CapStyle::Projecting;
        let base = outline.base_mut();
        base.zorder = 2.5;
        base.transform = Some(self.ax.trans_axes());
        base.clip_on = false;
        self.ax.adopt(outline, &DataUpdate::default());
    }

    /// Fills the extensions with the under and over colours, just behind
    /// the axes background (matplotlib's `_do_extends`).
    pub(crate) fn extension_patches(
        &self,
        extensions: &Extensions,
        lower: Option<Rgba>,
        upper: Option<Rgba>,
    ) {
        for (is_upper, color) in [(false, lower), (true, upper)] {
            let Some(color) = color else { continue };
            let path = Path::polyline(self.points(&extensions.patch(is_upper)));
            let mut patch = PatchArtist::new(PathShape(path), crate::rc::params());
            patch.style.facecolor = Some(Color::from(color));
            patch.style.linewidth = Some(0.0);
            patch.style.antialiased = Some(false);
            let base = patch.base_mut();
            base.alpha = self.alpha;
            base.transform = Some(self.ax.trans_axes());
            base.clip_on = false;
            base.zorder = 1.0_f64.next_down();
            self.ax.adopt(patch, &DataUpdate::default());
        }
    }

    /// Lines across the colorbar at `positions` along it, in the axes
    /// edge colour (matplotlib's `dividers`).
    pub(crate) fn dividers(&self, positions: &[f64]) -> Collection {
        let rc = crate::rc::params();
        let mut dividers =
            CollectionArtist::new(CollectionKind::Line, self.across(positions), rc.clone());
        dividers.edgecolors = EdgeColors::Colors(vec![rc.axes_edgecolor.clone()]);
        dividers.linewidths = Some(vec![0.5 * rc.axes_linewidth]);
        let base = dividers.base_mut();
        base.transform = Some(self.ax.trans_data());
        base.clip_on = false;
        Collection::from_raw(self.ax.adopt(dividers, &DataUpdate::default()))
    }

    /// Replaces the dividers' positions.
    pub(crate) fn move_dividers(&self, dividers: &Collection, positions: &[f64]) {
        let paths = self.across(positions);
        dividers
            .raw()
            .with_mut::<CollectionArtist, _>(|c| c.paths = paths);
    }

    /// Segments across the colorbar at `positions` in data coordinates.
    fn across(&self, positions: &[f64]) -> Vec<Path> {
        positions
            .iter()
            .map(|&at| Path::polyline(self.points(&[(at, 0.0), (at, 1.0)])))
            .collect()
    }

    /// The colour blocks between consecutive `edges`, coloured by
    /// `values` (matplotlib's `_add_solids`).
    pub(crate) fn solids(
        &self,
        edges: &[f64],
        values: &[f64],
        mapping: &ScalarMappable,
    ) -> Result<Collection, ColorbarError> {
        let n = edges.len();
        let along = Grid::from_fn(n, 2, |r, _| edges[r]);
        let across = Grid::from_fn(n, 2, |_, c| c as f64);
        let (x, y) = match self.orientation {
            Orientation::Vertical => (across, along),
            Orientation::Horizontal => (along, across),
        };
        let colors = Grid::from_vec(values.len(), 1, values.to_vec()).expect("one value per block");
        let mesh = Mesh::with_coords(
            MeshCoords::Grid(x),
            MeshCoords::Grid(y),
            ScalarField::new(colors),
        );
        let options = PcolorOptions {
            colors: ColorizerOptions {
                cmap: Some(mapping.cmap().clone()),
                norm: Some(mapping.norm().clone_box()),
                ..ColorizerOptions::default()
            },
            shading: Some(Shading::Flat),
            props: CollectionProps {
                edgecolors: Some(EdgeColors::Colors(vec![Color::none()])),
                base: BaseProps {
                    alpha: self.alpha,
                    ..BaseProps::default()
                },
                ..CollectionProps::default()
            },
            ..PcolorOptions::default()
        };
        let solids = self.ax.pcolormesh(mesh, options)?;
        if n >= RASTERIZE_FROM {
            solids.set_rasterized(true);
        }
        Ok(solids)
    }

    /// Lines across the colorbar at `positions`, clipped to the axes
    /// stretched by the widest line along the colorbar's y direction
    /// (matplotlib's `add_lines`).
    pub(crate) fn lines(
        &self,
        positions: &[f64],
        colors: Vec<Color>,
        linewidths: Vec<f64>,
    ) -> Collection {
        let widest = linewidths.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let clip = self.clip_path(widest / 72.0);
        let mut lines = CollectionArtist::new(
            CollectionKind::Line,
            self.across(positions),
            crate::rc::params(),
        );
        lines.edgecolors = EdgeColors::Colors(colors);
        lines.linewidths = Some(linewidths);
        lines.base_mut().transform = Some(self.ax.trans_data());
        let raw = self.ax.adopt(lines, &DataUpdate::default());
        raw.with_base_mut(|base| {
            base.clip_box = None;
            base.clip_path = Some(ClipPathSpec::Path {
                path: clip,
                transform: self.ax.trans_axes(),
            });
        });
        Collection::from_raw(raw)
    }

    /// The axes rectangle grown by `inches` at the bottom and top, in axes
    /// coordinates of the axes' current placement.
    fn clip_path(&self, inches: f64) -> Path {
        let figure = self.ax.raw().figure_state().borrow();
        let height = figure
            .axes_bbox(self.ax.id())
            .map_or(f64::NAN, |extent| extent.height());
        let grow = inches * figure.dpi / height;
        Path::closed(vec![
            Point::new(0.0, -grow),
            Point::new(1.0, -grow),
            Point::new(1.0, 1.0 + grow),
            Point::new(0.0, 1.0 + grow),
            Point::new(0.0, -grow),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn outlines_trace_triangles_or_rectangles() {
        let triangles = Extensions {
            lower: 0.05,
            upper: 0.0,
            rect: false,
        };
        assert_eq!(
            triangles.outline(),
            [
                (0.0, 0.0),
                (-0.05, 0.5),
                (0.0, 1.0),
                (1.0, 1.0),
                (1.0, 0.5),
                (1.0, 0.0),
                (0.0, 0.0)
            ]
        );
        assert_eq!(
            triangles.patch(false),
            [(0.0, 0.0), (-0.05, 0.5), (0.0, 1.0)]
        );
        let rect = Extensions {
            rect: true,
            upper: 0.1,
            ..triangles
        };
        assert_eq!(rect.outline().len(), 9);
        assert_eq!(rect.patch(true)[1], (1.1, 0.0));
    }

    #[test]
    fn orientation_swaps_coordinates() {
        assert_eq!(place(Orientation::Vertical, 0.2, 1.0), Point::new(1.0, 0.2));
        assert_eq!(
            place(Orientation::Horizontal, 0.2, 1.0),
            Point::new(0.2, 1.0)
        );
    }

    #[test]
    fn line_clips_reach_past_the_ends() {
        let fig = Figure::with_size(5.0, 3.5, 100.0);
        let ax = fig.add_axes([0.8, 0.1, 0.05, 0.8]);
        let painter = Painter {
            ax: &ax,
            orientation: Orientation::Vertical,
            alpha: None,
        };
        // Half an inch is 50 px of the 280 px tall axes.
        let clip = painter.clip_path(0.5);
        let grow = 50.0 / 280.0;
        assert!((clip.vertices()[0].y + grow).abs() < 1e-12);
        assert!((clip.vertices()[2].y - 1.0 - grow).abs() < 1e-12);
    }
}
