//! The artists a colorbar draws in its axes: the colour mesh, the
//! extension patches, the outline, the dividers between colours and lines
//! at given levels (matplotlib's `_add_solids`, `_do_extends`,
//! `_update_dividers` and `add_lines`).

use plt_core::{Path, Point, Rgba};
use plt_render::CapStyle;

use super::ColorbarError;
use super::dividers::Dividers;
use crate::artist::{Artist, ArtistHandle, ArtistId, BaseProps, ClipPathSpec};
use crate::axes::{Axes, DataUpdate, Mesh, MeshCoords, PcolorOptions, RgbaMesh, Shading};
use crate::collections::{CollectionArtist, CollectionKind, CollectionProps, EdgeColors};
use crate::colors::{Color, ColorizerOptions, ScalarMappable};
use crate::image::ScalarField;
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

/// Segments across a colorbar of `orientation` at `positions` along it,
/// in data coordinates.
pub(super) fn across(orientation: Orientation, positions: &[f64]) -> Vec<Path> {
    positions
        .iter()
        .map(|&at| {
            Path::polyline(vec![
                place(orientation, at, 0.0),
                place(orientation, at, 1.0),
            ])
        })
        .collect()
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

/// How one colour block of a colorbar is filled.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Block {
    /// The colour of its value.
    pub(crate) color: Rgba,
    /// Its hatch, for hatched contour sets.
    pub(crate) hatch: Option<String>,
}

/// How the colour blocks of a colorbar's mesh are coloured.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SolidFill<'a> {
    /// Values mapped through a norm and colormap.
    Mapped(&'a [f64], &'a ScalarMappable),
    /// Colours worked out already: integer values indexing the colormap
    /// under `NoNorm` (numpy's integer `_values`), which a mesh of mapped
    /// floats cannot express.
    Colors(&'a [Rgba]),
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

    /// Frames the colorbar with an outline around the interior and the
    /// extensions (matplotlib's `_ColorbarSpine`): moves the `existing`
    /// outline, keeping its style, or hides the spines and makes one.
    pub(crate) fn outline(&self, extensions: &Extensions, existing: Option<ArtistId>) -> ArtistId {
        let path = Path::closed(self.points(&extensions.outline()));
        if let Some(id) = existing {
            let mut figure = self.ax.raw().figure_state().borrow_mut();
            if let Some(outline) = figure.artist_as_mut::<PatchArtist<PathShape>>(id) {
                outline.shape = PathShape(path);
                figure.stale = true;
                return id;
            }
        }
        for spine in self.ax.spines().iter() {
            spine.set_visible(false);
        }
        let rc = crate::rc::params();
        let mut outline = PatchArtist::new(PathShape(path), rc.clone());
        outline.style.facecolor = Some(Color::none());
        outline.style.edgecolor = Some(rc.axes_edgecolor.clone());
        outline.style.linewidth = Some(rc.axes_linewidth);
        outline.style.capstyle = CapStyle::Projecting;
        let base = outline.base_mut();
        base.zorder = 2.5;
        base.transform = Some(self.ax.trans_axes());
        base.clip_on = false;
        self.ax.adopt(outline, &DataUpdate::default()).id()
    }

    /// Fills the extensions with the under and over colours and their
    /// hatches, just behind the axes background (matplotlib's
    /// `_do_extends`).
    pub(crate) fn extension_patches(
        &self,
        extensions: &Extensions,
        lower: Option<Block>,
        upper: Option<Block>,
    ) -> Vec<ArtistId> {
        let mut patches = Vec::new();
        for (is_upper, block) in [(false, lower), (true, upper)] {
            let Some(block) = block else { continue };
            let mut patch = self.block_patch(&extensions.patch(is_upper), block);
            let base = patch.base_mut();
            base.transform = Some(self.ax.trans_axes());
            base.clip_on = false;
            base.zorder = 1.0_f64.next_down();
            patches.push(self.ax.adopt(patch, &DataUpdate::default()).id());
        }
        patches
    }

    /// One hatched patch per colour block between consecutive `edges`
    /// (matplotlib's `_add_solids_patches`, used for hatched contour sets).
    pub(crate) fn solids_patches(&self, edges: &[f64], blocks: Vec<Block>) -> Vec<ArtistId> {
        edges
            .windows(2)
            .zip(blocks)
            .map(|(pair, block)| {
                // matplotlib takes the corners from its mesh, whose x and y
                // swap for horizontal colorbars, so these start across.
                let (near, far) = match self.orientation {
                    Orientation::Vertical => (0.0, 1.0),
                    Orientation::Horizontal => (1.0, 0.0),
                };
                let corners = [
                    (pair[0], near),
                    (pair[0], far),
                    (pair[1], far),
                    (pair[1], near),
                ];
                let mut patch = self.block_patch(&corners, block);
                patch.base_mut().transform = Some(self.ax.trans_data());
                self.ax.adopt(patch, &DataUpdate::default()).id()
            })
            .collect()
    }

    /// An unstroked, aliased patch through the `(along, across)` points
    /// (an open path, as matplotlib's `PathPatch(Path(xy))`), filled and
    /// hatched as `block` says.
    fn block_patch(&self, along_across: &[(f64, f64)], block: Block) -> PatchArtist<PathShape> {
        let path = Path::polyline(self.points(along_across));
        let mut patch = PatchArtist::new(PathShape(path), crate::rc::params());
        patch.style.facecolor = Some(Color::from(block.color));
        patch.style.hatch = block.hatch;
        patch.style.linewidth = Some(0.0);
        patch.style.antialiased = Some(false);
        patch.base_mut().alpha = self.alpha;
        patch
    }

    /// Lines across the colorbar at `positions` along it, in the axes
    /// edge colour (matplotlib's `dividers`).
    pub(crate) fn dividers(&self, placement: Dividers) -> ArtistId {
        let rc = crate::rc::params();
        let mut dividers = CollectionArtist::new(CollectionKind::Line, Vec::new(), rc.clone());
        dividers.edgecolors = EdgeColors::Colors(vec![rc.axes_edgecolor.clone()]);
        dividers.linewidths = Some(vec![0.5 * rc.axes_linewidth]);
        dividers.draw_time = Some(Box::new(placement));
        let base = dividers.base_mut();
        base.transform = Some(self.ax.trans_data());
        base.clip_on = false;
        self.ax.adopt(dividers, &DataUpdate::default()).id()
    }

    /// Replaces where the dividers go.
    pub(crate) fn move_dividers(&self, dividers: ArtistId, placement: Dividers) {
        let mut figure = self.ax.raw().figure_state().borrow_mut();
        figure.stale = true;
        if let Some(dividers) = figure.artist_as_mut::<CollectionArtist>(dividers) {
            dividers.draw_time = Some(Box::new(placement));
        }
    }

    /// Removes artists drawn earlier.
    pub(crate) fn remove(&self, artists: impl IntoIterator<Item = ArtistId>) {
        let mut figure = self.ax.raw().figure_state().borrow_mut();
        for artist in artists {
            figure.remove_artist(artist);
        }
    }

    /// The colour blocks between consecutive `edges`, filled as `fill`
    /// says (matplotlib's `_add_solids`).
    pub(crate) fn solids(
        &self,
        edges: &[f64],
        fill: SolidFill<'_>,
    ) -> Result<ArtistId, ColorbarError> {
        let n = edges.len();
        let along = Grid::from_fn(n, 2, |r, _| edges[r]);
        let across = Grid::from_fn(n, 2, |_, c| c as f64);
        let (x, y) = match self.orientation {
            Orientation::Vertical => (across, along),
            Orientation::Horizontal => (along, across),
        };
        let mut options = PcolorOptions {
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
        let (x, y) = (MeshCoords::Grid(x), MeshCoords::Grid(y));
        let solids = match fill {
            SolidFill::Mapped(values, mapping) => {
                options.colorizer = ColorizerOptions {
                    cmap: Some(mapping.cmap().clone()),
                    norm: Some(mapping.norm().clone_box().into()),
                    ..ColorizerOptions::default()
                };
                let values =
                    Grid::from_vec(values.len(), 1, values.to_vec()).expect("one value per block");
                let mesh = Mesh::from_coords(x, y, ScalarField::new(values));
                self.ax.pcolormesh(mesh, options)?
            }
            SolidFill::Colors(colors) => {
                let colors =
                    Grid::from_vec(colors.len(), 1, colors.to_vec()).expect("one colour per block");
                self.ax
                    .pcolormesh_rgba(RgbaMesh::from_coords(x, y, colors), options)?
            }
        };
        if n >= RASTERIZE_FROM {
            solids.set_rasterized(true);
        }
        Ok(solids.id())
    }

    /// Lines across the colorbar at `positions`, clipped to the axes
    /// stretched by the widest line along the colorbar's y direction
    /// (matplotlib's `add_lines`).
    pub(crate) fn lines(
        &self,
        positions: &[f64],
        colors: Vec<Color>,
        linewidths: Vec<f64>,
    ) -> ArtistId {
        let widest = linewidths.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let clip = self.clip_path(widest / 72.0);
        let mut lines = CollectionArtist::new(
            CollectionKind::Line,
            across(self.orientation, positions),
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
        raw.id()
    }

    /// The axes rectangle grown by `inches` at the bottom and top, in axes
    /// coordinates of the axes' current placement.
    fn clip_path(&self, inches: f64) -> Path {
        let figure = self.ax.raw().figure_state().borrow();
        let height = figure
            .axes_bbox(self.ax.id())
            .map_or(f64::NAN, |extent| extent.height());
        let grow = inches * figure.dpi() / height;
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
        let ax = fig
            .add_axes([0.8, 0.1, 0.05, 0.8])
            .expect("a valid rectangle");
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
