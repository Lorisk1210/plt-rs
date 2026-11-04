//! Collections: many similar shapes drawn in one batch
//! (matplotlib.collections).
//!
//! One [`Collection`] artist covers matplotlib's `PathCollection` (scatter
//! markers sized in points²), `PolyCollection`, `LineCollection`,
//! `PatchCollection` and friends; the [`CollectionKind`] selects the
//! default colours, widths and z-order of each flavour. A
//! [`CollectionSpec`] builds one of matplotlib's collection classes
//! (patches, lines, polygons, ellipses, circles, regular polygons, events)
//! for [`Axes::add_collection`](crate::axes::Axes::add_collection);
//! [`TriMeshSpec`] builds a Gouraud-shaded [`TriMesh`]. Event and ellipse
//! collections downcast to the [`EventCollection`] and
//! [`EllipseCollection`] handles, which change their events or ellipses.

mod array;
mod copy;
mod depth;
mod draw;
mod ellipses;
mod events;
mod gaps;
mod handle;
mod items;
mod legend_elements;
mod mapped;
mod mesh_cells;
mod props;
mod quad_mesh;
mod readback;
mod spec;
mod style;
mod trimesh;

pub use array::ArrayError;
pub(crate) use depth::{DepthSortError, DepthSorted};
pub use ellipses::EllipseCollection;
pub use events::EventCollection;
pub(crate) use events::EventLines;
pub use handle::{Collection, CollectionColorError, CollectionColors};
pub use legend_elements::{
    ElementCount, LegendElements, LegendElementsOptions, LegendProperty, LegendValueFn,
};
pub(crate) use mapped::Mapped;
pub(crate) use mesh_cells::unmasked_cells;
pub use props::{CollectionProps, CollectionStyle};
use quad_mesh::MeshGeometry;
pub use quad_mesh::MeshShading;
pub(crate) use spec::Ellipses;
pub use spec::{CollectionPatch, CollectionSpec, EllipseUnits};
pub use style::HatchColors;
use style::MappedSides;
pub(crate) use trimesh::TriMeshArtist;
pub use trimesh::{TriMesh, TriMeshError, TriMeshSpec};

use std::any::Any;
use std::borrow::Cow;
use std::cell::Cell;
use std::fmt::Debug;
use std::rc::Rc;

use pltrs_core::path::{CollectionGeometry, DataLimits};
use pltrs_core::{Affine2D, Bbox, Path, Point, Rgba};
use pltrs_render::{CapStyle, JoinStyle};

use crate::artist::{
    Artist, ArtistBase, ArtistId, ArtistStore, DrawContext, DrawErrorKind, LayoutContext,
    LimitPoints, format_values,
};
use crate::axes::FillGeometry;
use crate::colorizer::ColorMapped;
use crate::colors::{Color, NormError, ScalarMappable};
use crate::lines::LineStyle;
use crate::numeric::Grid;
use crate::rc::RcParams;
use crate::transforms::Transform;

/// Which matplotlib collection class the defaults follow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CollectionKind {
    /// A generic `Collection` (patch defaults, z-order 1).
    #[default]
    Generic,
    /// The outlines of patches (`PatchCollection`, patch defaults, z-order
    /// 1).
    Patch,
    /// Markers sized in points² at data offsets (`PathCollection`).
    Path,
    /// Polygons (`PolyCollection`, e.g. `fill_between`).
    Poly,
    /// Polylines without faces (`LineCollection`, line defaults, z-order 2).
    Line,
    /// A grid of quadrilaterals (`QuadMesh`, patch defaults, z-order 1).
    QuadMesh,
}

pltrs_core::archive_enum!(CollectionKind {
    Generic,
    Patch,
    Path,
    Poly,
    Line,
    QuadMesh,
});

/// The edge colours of a collection.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum EdgeColors {
    /// matplotlib's default rule: `patch.edgecolor` when edges are forced,
    /// faces are not drawn, or the kind draws edges by default; else none.
    #[default]
    Default,
    /// The same colours as the faces.
    Face,
    /// Explicit colours, cycled over the items.
    Colors(Vec<Color>),
}

pltrs_core::archive_enum!(EdgeColors {
    Default,
    Face,
    Colors(v0),
});

/// Values mapped to colours through a norm and colormap.
#[derive(Clone, Debug)]
pub struct ColorMapping {
    /// One value per item (`None` = masked).
    pub values: Vec<Option<f64>>,
    /// The norm and colormap.
    pub mappable: ScalarMappable,
}

pltrs_core::archive_struct!(ColorMapping { values, mappable });

/// Shapes recomputed from the figure geometry at every draw, for
/// collections whose paths depend on the axes size or the resolution
/// (matplotlib's `Quiver`, which builds its arrows in `draw`).
pub(crate) trait DrawTimeShapes: Debug + Any {
    /// The shapes for the current geometry and the transform from their
    /// coordinates to display space.
    fn shapes(&self, ctx: LayoutContext<'_>) -> Result<(Vec<Path>, Transform), String>;

    /// Whether the collection's data limits are its offsets alone, its
    /// shapes left out (matplotlib's `Quiver.get_datalim`).
    fn limits_at_offsets(&self) -> bool {
        false
    }

    /// The generator as [`Any`], for artists that read its state.
    fn as_any(&self) -> &dyn Any;
}

/// State of a collection.
#[derive(Clone, Debug)]
pub(crate) struct CollectionArtist {
    base: ArtistBase,
    pub(crate) kind: CollectionKind,
    pub(crate) paths: Vec<Path>,
    /// Whether `paths` holds the one path the program gave where a
    /// sequence of paths is expected ([`Collection::set_single_path`]),
    /// which drawing refuses.
    single_path: bool,
    /// Explicit per-item transforms (cycled); see also `sizes`.
    pub(crate) transforms: Vec<Affine2D>,
    /// Marker areas in points² (cycled); scale each item at draw time.
    pub(crate) sizes: Option<Vec<f64>>,
    /// Extra scale of the sizes (matplotlib's `_factor`: 1/√π for circles
    /// and regular polygons, whose sizes are circle areas).
    pub(crate) size_factor: f64,
    /// Rotation of sized items in radians (regular polygons).
    pub(crate) size_rotation: f64,
    /// Ellipses scaled at draw time (`EllipseCollection`); replaces
    /// `transforms` and `sizes`.
    pub(crate) ellipses: Option<Ellipses>,
    /// The layout of an event collection's lines (`None` for other
    /// collections).
    pub(crate) events: Option<EventLines>,
    pub(crate) offsets: Vec<Point>,
    /// Coordinate system of `offsets` (`None` = no offsets).
    pub(crate) offset_transform: Option<Transform>,
    /// Whether the offsets are a masked array (scatter's are), which loses
    /// the values a non-linear scale cannot map instead of clipping them.
    pub(crate) offsets_masked: bool,
    /// Face colours (`None` = the kind's default).
    pub(crate) facecolors: Option<Vec<Color>>,
    pub(crate) edgecolors: EdgeColors,
    /// Edge widths in points (`None` = the kind's default).
    pub(crate) linewidths: Option<Vec<f64>>,
    pub(crate) linestyles: Vec<LineStyle>,
    /// One opacity per item, replacing the artist's single alpha
    /// (matplotlib's array `alpha`).
    pub(crate) alphas: Option<Vec<f64>>,
    pub(crate) antialiased: Option<Vec<bool>>,
    pub(crate) hatch: Option<String>,
    /// The colours of the hatches (`hatch.color` when the collection was
    /// made, unless set).
    pub(crate) hatchcolors: HatchColors,
    /// The width of the hatch lines in points (`hatch.linewidth` when the
    /// collection was made, unless set).
    pub(crate) hatch_linewidth: f64,
    pub(crate) capstyle: Option<CapStyle>,
    pub(crate) joinstyle: Option<JoinStyle>,
    /// Colours drawn in the gaps of dashed edges (`None` = empty gaps).
    pub(crate) gapcolors: Option<Vec<Color>>,
    /// Links of the items in vector output, cycled (matplotlib's
    /// `_urls`; empty = `[None]`).
    pub(crate) urls: Vec<Option<String>>,
    /// How a `fill_between` collection makes its polygons, kept for new
    /// curves (`None` for other collections).
    pub(crate) fill_geometry: Option<FillGeometry>,
    /// The item styles worked out by a 3-D projection, replacing the
    /// collection's own when set.
    pub(crate) depth_sorted: Option<DepthSorted>,
    pub(crate) mapping: Option<Mapped>,
    /// The sides whose colours matplotlib's getters report as mapped: those
    /// the mapping coloured at the last draw, until their colours are set
    /// again (matplotlib keeps the colours as set until
    /// `update_scalarmappable` maps them, and the mapped ones until the
    /// next `set_facecolor`, `set_edgecolor` or `set_alpha`).
    pub(crate) mapped_shown: Cell<MappedSides>,
    /// The norm and colormap of a colour mapping removed by
    /// [`Collection::clear_array`], which the next
    /// [`Collection::set_array`] maps through again.
    pub(crate) parked_mappable: Option<ScalarMappable>,
    /// How far from an item's outline, in pixels, a pick still hits it; a
    /// pick must fall inside an item when it is not positive.
    pickradius: f64,
    /// The vertex grid of a quad mesh, drawn as a mesh.
    mesh: Option<MeshGeometry>,
    /// The vertex grid of a quad mesh drawn as polygons (`pcolor`'s
    /// `PolyQuadMesh`), which its coordinates are read from.
    pub(crate) polygon_grid: Option<Grid<Point>>,
    /// Replaces `paths` and the artist transform when drawing (shared by
    /// copies of the collection: a generator is replaced, never changed).
    pub(crate) draw_time: Option<Rc<dyn DrawTimeShapes>>,
    /// matplotlib's class of the collection where it is not the one of
    /// its kind (`Line3DCollection`, `GridlinesCollection`), which names
    /// its group in SVG output.
    pub(crate) class_name: Option<&'static str>,
    rc: Rc<RcParams>,
}

pltrs_core::archive_struct!(CollectionArtist {
    base,
    kind,
    paths,
    single_path,
    transforms,
    sizes,
    size_factor,
    size_rotation,
    ellipses,
    events,
    offsets,
    offset_transform,
    offsets_masked,
    facecolors,
    edgecolors,
    linewidths,
    linestyles,
    alphas,
    antialiased,
    hatch,
    hatchcolors,
    hatch_linewidth,
    capstyle,
    joinstyle,
    gapcolors,
    urls,
    fill_geometry,
    depth_sorted,
    mapping,
    mapped_shown,
    parked_mappable,
    pickradius,
    mesh,
    polygon_grid,
    draw_time,
    class_name,
    rc
});

impl CollectionArtist {
    /// An empty collection of the given kind with rc defaults.
    pub(crate) fn new(kind: CollectionKind, paths: Vec<Path>, rc: Rc<RcParams>) -> Self {
        let zorder = if kind == CollectionKind::Line {
            2.0
        } else {
            1.0
        };
        Self {
            base: ArtistBase::with_zorder(zorder),
            kind,
            paths,
            single_path: false,
            transforms: Vec::new(),
            sizes: None,
            size_factor: 1.0,
            size_rotation: 0.0,
            ellipses: None,
            events: None,
            offsets: Vec::new(),
            offset_transform: None,
            offsets_masked: false,
            facecolors: None,
            edgecolors: EdgeColors::Default,
            linewidths: None,
            linestyles: vec![LineStyle::Solid],
            alphas: None,
            antialiased: None,
            hatch: None,
            hatchcolors: HatchColors::from_rc(&rc),
            hatch_linewidth: rc.hatch_linewidth,
            capstyle: None,
            joinstyle: None,
            gapcolors: None,
            urls: Vec::new(),
            fill_geometry: None,
            depth_sorted: None,
            mapping: None,
            mapped_shown: Cell::default(),
            parked_mappable: None,
            pickradius: 5.0,
            mesh: None,
            polygon_grid: None,
            draw_time: None,
            class_name: None,
            rc,
        }
    }

    fn transform(&self) -> Transform {
        self.base.transform.clone().unwrap_or(Transform::Display)
    }

    /// Replaces the item shapes with a sequence of paths.
    pub(crate) fn replace_paths(&mut self, paths: Vec<Path>) {
        self.paths = paths;
        self.single_path = false;
    }

    /// The extent of the shapes placed at their offsets, all in data
    /// coordinates (matplotlib's `get_datalim` for collections whose shapes
    /// live in data space).
    pub(crate) fn shape_extents(&self) -> DataLimits {
        let shapes = self.shapes();
        let geometry = CollectionGeometry {
            master_transform: Affine2D::IDENTITY,
            paths: &shapes,
            transforms: &self.transforms,
            // matplotlib collections without offsets have one at the origin.
            offsets: if self.offsets.is_empty() {
                &[Point::ORIGIN]
            } else {
                &self.offsets
            },
            offset_transform: Affine2D::IDENTITY,
        };
        geometry.extents().unwrap_or(DataLimits::NULL)
    }

    pub(crate) fn rc(&self) -> &RcParams {
        &self.rc
    }

    /// The edge widths in points, or the kind's default width
    /// (`lines.linewidth` for line collections, else `patch.linewidth`).
    pub(crate) fn linewidths_or_default(&self) -> Vec<f64> {
        self.linewidths.clone().unwrap_or_else(|| {
            vec![if self.kind == CollectionKind::Line {
                self.rc.lines_linewidth
            } else {
                self.rc.patch_linewidth
            }]
        })
    }

    /// The edge widths to draw: those of the last 3-D projection, or else
    /// [`Self::linewidths_or_default`].
    pub(crate) fn drawn_linewidths(&self) -> Vec<f64> {
        self.depth_sorted
            .as_ref()
            .and_then(|sorted| sorted.linewidths.clone())
            .unwrap_or_else(|| self.linewidths_or_default())
    }

    /// The offsets to draw at: those of the last 3-D projection in their
    /// drawing order, or else the collection's own.
    pub(crate) fn drawn_offsets(&self) -> &[Point] {
        self.depth_sorted
            .as_ref()
            .and_then(|sorted| sorted.offsets.as_deref())
            .unwrap_or(&self.offsets)
    }

    /// The marker areas to draw: those of the last 3-D projection, or
    /// else the collection's own.
    pub(crate) fn drawn_sizes(&self) -> Option<&[f64]> {
        self.depth_sorted
            .as_ref()
            .and_then(|sorted| sorted.sizes.as_deref())
            .or(self.sizes.as_deref())
    }

    /// The shape transforms of the items at `dpi`: those scaling markers
    /// of the areas `sizes` (points²), else the collection's own
    /// (matplotlib's `_CollectionWithSizes.set_sizes(sizes, dpi)`, then
    /// `get_transforms`).
    pub(crate) fn item_transforms(&self, sizes: Option<&[f64]>, dpi: f64) -> Vec<Affine2D> {
        let Some(sizes) = sizes else {
            return self.transforms.clone();
        };
        sizes
            .iter()
            .map(|area| {
                let scale = area.sqrt() * dpi / 72.0 * self.size_factor;
                let scaled = Affine2D::scaling(scale, scale);
                if self.size_rotation == 0.0 {
                    scaled
                } else {
                    scaled.rotate(-self.size_rotation)
                }
            })
            .collect()
    }

    /// The face and edge colours after defaults, colour mapping and alpha
    /// (matplotlib's `get_facecolor` and `get_edgecolor`), or those of the
    /// last 3-D projection.
    pub(crate) fn resolved_colors(&self) -> Result<(Vec<Rgba>, Vec<Rgba>), String> {
        style::resolve_colors(self)
    }

    /// The face and edge colours as matplotlib's getters report them (see
    /// [`CollectionArtist::mapped_shown`]), or those of the last 3-D
    /// projection.
    pub(crate) fn reported_colors(&self) -> Result<(Vec<Rgba>, Vec<Rgba>), String> {
        style::resolve_reported_colors(self)
    }

    /// Maps the values onto the sides the mapping colours, for the getters
    /// (matplotlib's `update_scalarmappable`, which drawing runs).
    pub(crate) fn update_scalarmappable(&self) {
        self.mapped_shown.set(MappedSides::of(self));
    }

    /// The face and edge colours after defaults, colour mapping and
    /// alpha, ignoring any 3-D projection: what a projection shades.
    pub(crate) fn base_colors(&self) -> Result<(Vec<Rgba>, Vec<Rgba>), String> {
        style::resolve_base_colors(self)
    }

    /// The shapes to draw now and their transform: the stored paths, or
    /// those of the draw-time generator.
    fn current_shapes(
        &self,
        ctx: LayoutContext<'_>,
    ) -> Result<(Cow<'_, [Path]>, Transform), String> {
        match &self.draw_time {
            Some(generator) => {
                let (paths, transform) = generator.shapes(ctx)?;
                Ok((Cow::Owned(paths), transform))
            }
            None => Ok((self.shapes(), self.transform())),
        }
    }

    /// The draw-time generator as a `T`, if it is one.
    pub(crate) fn draw_time_state<T: Any>(&self) -> Option<&T> {
        self.draw_time.as_ref()?.as_any().downcast_ref()
    }

    /// Draws the collection, returning what kept it from being drawn
    /// (also used for collections drawn on behalf of another artist).
    pub(crate) fn render(&self, ctx: &mut DrawContext<'_>) -> Result<(), String> {
        match &self.mesh {
            Some(mesh) => quad_mesh::draw_quad_mesh(self, mesh, ctx),
            None => draw::draw_collection(self, ctx),
        }
    }
}

impl ColorMapped for CollectionArtist {
    fn mapping_mut(&mut self) -> Option<&mut ScalarMappable> {
        self.mapping
            .as_mut()
            .map(|mapped| &mut mapped.source.mappable)
    }

    /// Problems normalising the values are reported when drawing.
    fn remap(&mut self, _others: &mut ArtistStore) -> Result<(), NormError> {
        if let Some(mapped) = &mut self.mapping {
            mapped.refresh();
        }
        Ok(())
    }
}

pub(crate) fn cycled<T>(items: &[T], i: usize) -> Option<&T> {
    (!items.is_empty()).then(|| &items[i % items.len()])
}

impl Artist for CollectionArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        self.update_scalarmappable();
        if self.single_path {
            // matplotlib's renderers index the paths as a sequence.
            ctx.report_kind(
                self.base.id(),
                DrawErrorKind::InvalidType,
                "Object of type 'Path' is not an instance of 'sequence'",
            );
        } else if let Some(problem) = self.depth_sorted.as_ref().and_then(|s| s.problem) {
            ctx.report_kind(self.base.id(), problem.kind(), problem);
        } else if let Some(Err(problem)) = self.mapping.as_ref().map(|m| &m.normalized) {
            // matplotlib's norms raise a `ValueError` for values or limits
            // they cannot normalise.
            ctx.report_kind(self.base.id(), DrawErrorKind::InvalidValue, problem);
        } else if let Err(problem) = self.render(ctx) {
            ctx.report(self.base.id(), problem);
        }
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let resolved = ctx.resolve(&self.transform())?;
        let offsets = match &self.offset_transform {
            Some(t) => {
                let r = ctx.resolve(t)?;
                self.offsets.iter().map(|&p| r.transform_point(p)).collect()
            }
            None => Vec::new(),
        };
        let shape_box = self
            .shapes()
            .iter()
            .map(|p| resolved.transform_path(p).extents())
            .fold(Bbox::NULL, |acc, b| acc.union(&b));
        if offsets.is_empty() {
            return Some(shape_box);
        }
        Some(Bbox::from_points(offsets))
    }

    /// matplotlib's `Collection.contains`: the items whose outline passes
    /// within the pick radius of `at`, or that enclose it when the radius
    /// is not positive.
    fn pick_hit(
        &self,
        ctx: &LayoutContext<'_>,
        at: Point,
        radius: Option<f64>,
    ) -> Option<Vec<usize>> {
        if !self.base.visible {
            return None;
        }
        let radius = radius.unwrap_or(self.pickradius);
        let (shapes, transform) = self.current_shapes(*ctx).ok()?;
        let geometry =
            draw::Geometry::new(self, *ctx, (&shapes, &transform), &self.offsets).ok()?;
        let hits = geometry.hits(at, radius, radius <= 0.0);
        (!hits.is_empty()).then_some(hits)
    }

    /// matplotlib's `_update_collection_limits`.
    fn limit_points(&self, axes: ArtistId) -> Option<LimitPoints> {
        let (limits, directions) = self.data_limits(axes);
        LimitPoints::from_limits(limits, directions)
    }

    /// matplotlib's `QuadMesh.get_cursor_data`: the values of the cells
    /// under `at` (other collections have none).
    fn cursor_text(&self, ctx: &LayoutContext<'_>, at: Point) -> Option<String> {
        let values = self.cursor_values(ctx, at)?;
        Some(format_values(values.into_iter().flatten()))
    }
}

pltrs_core::archive_kinds!(dyn DrawTimeShapes {
    "colorbar-dividers" => crate::colorbar::Dividers,
    "quiver" => crate::quiver::QuiverGeometry,
    "axis-artist-gridlines" => crate::axisartist::Gridlines,
});
