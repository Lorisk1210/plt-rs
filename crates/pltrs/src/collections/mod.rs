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

mod alpha_fit;
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
mod program_colors;
mod props;
mod quad_mesh;
mod readback;
mod spec;
mod style;
mod trimesh;
mod update;

use alpha_fit::KeptColors;
pub use array::ArrayError;
pub(crate) use depth::{DepthSortError, DepthSorted, without_last};
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
use trimesh::TriangleMesh;
pub use trimesh::{TriMesh, TriMeshError, TriMeshSpec};

use std::any::Any;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;

use pltrs_core::path::{CollectionGeometry, DataLimits};
use pltrs_core::{Affine2D, Bbox, Path, Point, Rgba};
use pltrs_render::{CapStyle, JoinStyle};

use crate::artist::{
    Artist, ArtistBase, ArtistId, ArtistStore, DrawContext, DrawErrorKind, LayoutContext,
    LimitPoints, ProgramShaping, format_values,
};
use crate::axes::FillGeometry;
use crate::colorizer::ColorMapped;
use crate::colors::{Color, NormError, ScalarMappable};
use crate::lines::LineStyle;
use crate::numeric::Grid;
use crate::rc::RcParams;
use crate::transforms::{Transform, TransformContext};

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
    /// The triangles of a triangulation, Gouraud-shaded (`TriMesh`, patch
    /// defaults, z-order 1).
    TriMesh,
}

pltrs_core::archive_enum!(CollectionKind {
    Generic,
    Patch,
    Path,
    Poly,
    Line,
    QuadMesh,
    TriMesh,
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

    /// The shapes to draw now, as [`shapes`](Self::shapes) gives them,
    /// recording what the generator keeps of its draws.
    fn drawn_shapes(&self, ctx: LayoutContext<'_>) -> Result<(Vec<Path>, Transform), String> {
        self.shapes(ctx)
    }

    /// Whether the collection's data limits are its offsets alone, its
    /// shapes left out (matplotlib's `Quiver.get_datalim`).
    fn limits_at_offsets(&self) -> bool {
        false
    }

    /// A problem drawing meets before it asks for the shapes, with the kind
    /// of exception matplotlib raises for it (a setting it checks only
    /// then); none by default.
    fn problem(&self) -> Option<(DrawErrorKind, String)> {
        None
    }

    /// The generator as [`Any`], for artists that read its state.
    fn as_any(&self) -> &dyn Any;
}

/// A way of asking a draw-time generator for its shapes.
type GenerateShapes =
    fn(&dyn DrawTimeShapes, LayoutContext<'_>) -> Result<(Vec<Path>, Transform), String>;

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
    /// The shape of the array the per-item alphas were given as, when it
    /// is not one-dimensional (matplotlib's `_alpha.shape`, which its
    /// errors name).
    pub(crate) alpha_shape: Option<Vec<usize>>,
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
    /// The colour mapping the last `update_scalarmappable` mapped with,
    /// once the mapping changed since (its values, norm or colormap): the
    /// getters report its colours until the next one, as matplotlib's
    /// `_facecolors` and `_edgecolors` only change there.
    pub(crate) mapping_shown: RefCell<Option<Mapped>>,
    /// Whether [`EdgeColors::Default`] gives the items edges, as the rule
    /// decided when it was last applied (matplotlib's `_set_edgecolor(None)`
    /// picks the colour then, from the faces of that time); `None` until
    /// then, the rule applying to the colours as they are.
    pub(crate) default_edges: Cell<Option<bool>>,
    /// The colours the getters keep reporting for a side that per-item
    /// alphas set since do not fit.
    pub(crate) kept_colors: KeptColors,
    /// The norm and colormap of a colour mapping removed by
    /// [`Collection::clear_array`], which the next
    /// [`Collection::set_array`] maps through again.
    pub(crate) parked_mappable: Option<ScalarMappable>,
    /// The colours of the items a colour mapping of the program's gave
    /// (see [`Collection::set_program_colors`]), which take the place of
    /// those of `mapping`.
    pub(crate) program_colors: Option<Vec<Rgba>>,
    /// How far from an item's outline, in pixels, a pick still hits it; a
    /// pick must fall inside an item when it is not positive.
    pickradius: f64,
    /// The vertex grid of a quad mesh, drawn as a mesh.
    mesh: Option<MeshGeometry>,
    /// The triangles of a triangle mesh, drawn Gouraud-shaded.
    pub(crate) triangle_mesh: Option<TriangleMesh>,
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
    /// The parameters the default face and edge colours were resolved
    /// with at the last draw that resolved them again (matplotlib's
    /// `update_scalarmappable` sets the colours as given again, with the
    /// parameters of the moment, at a collection's first draw, and at
    /// each draw while values are mapped); `None` before, the colours
    /// then following the parameters the collection was made with.
    colors_rc: RefCell<Option<Rc<RcParams>>>,
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
    alpha_shape,
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
    default_edges,
    kept_colors,
    parked_mappable,
    program_colors,
    pickradius,
    mesh,
    triangle_mesh,
    polygon_grid,
    draw_time,
    class_name,
    rc
} skip { colors_rc, mapping_shown });

impl CollectionArtist {
    /// An empty collection of the given kind with rc defaults.
    pub(crate) fn new(kind: CollectionKind, paths: Vec<Path>, rc: Rc<RcParams>) -> Self {
        let zorder = if kind == CollectionKind::Line {
            2.0
        } else {
            1.0
        };
        let mut base = ArtistBase::with_zorder(zorder);
        // matplotlib's collections start without the rc's path effects
        // (`Collection.__init__` sets `_path_effects` to None).
        base.path_effects.clear();
        Self {
            base,
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
            alpha_shape: None,
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
            mapping_shown: RefCell::default(),
            default_edges: Cell::default(),
            kept_colors: KeptColors::default(),
            parked_mappable: None,
            program_colors: None,
            pickradius: 5.0,
            mesh: None,
            triangle_mesh: None,
            polygon_grid: None,
            draw_time: None,
            class_name: None,
            rc,
            colors_rc: RefCell::new(None),
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

    /// The rcParams the collection was made with.
    pub(crate) fn rc(&self) -> &RcParams {
        &self.rc
    }

    /// The parameters the default face and edge colours follow now (see
    /// `colors_rc`).
    pub(crate) fn colors_rc(&self) -> Rc<RcParams> {
        self.colors_rc
            .borrow()
            .clone()
            .unwrap_or_else(|| Rc::clone(&self.rc))
    }

    /// Resolves the default colours again with the parameters of the
    /// moment, as matplotlib's draw does the first time and, while values
    /// are mapped, each time (`update_scalarmappable`).
    pub(crate) fn resolve_colors_again(&self) {
        let first = self.colors_rc.borrow().is_none();
        if first || MappedSides::of(self) != MappedSides::default() {
            *self.colors_rc.borrow_mut() = Some(crate::rc::params());
        }
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

    /// [`reported_colors`](Self::reported_colors), the faces and the
    /// edges each resolved on its own.
    pub(crate) fn reported_sides(&self) -> (style::Sided, style::Sided) {
        style::resolve_reported_sides(self)
    }

    /// Maps the values onto the sides the mapping colours, for the getters
    /// (matplotlib's `update_scalarmappable`, which drawing runs). When it
    /// maps a side, or the sides it maps changed, edges it does not colour
    /// take their colours as set again, with the parameters of the moment
    /// ([`CollectionArtist::resolve_colors_again`]), the default rule
    /// deciding anew from the faces as they are (matplotlib's
    /// `_set_edgecolor(self._original_edgecolor)` there).
    pub(crate) fn update_scalarmappable(&self) {
        if self.update_decides_default_edges() {
            self.resolve_colors_again();
            self.decide_default_edges();
        }
        self.mapped_shown.set(MappedSides::of(self));
        self.mapping_shown.take();
    }

    /// Keeps the colour mapping as the getters' until the next
    /// `update_scalarmappable`, before it changes, when they report
    /// mapped colours and none is kept yet (see
    /// [`CollectionArtist::mapping_shown`]).
    pub(crate) fn keep_shown_mapping(&mut self) {
        let shown = self.mapped_shown.get();
        let kept = self.mapping_shown.get_mut();
        if (shown.faces || shown.edges) && kept.is_none() {
            kept.clone_from(&self.mapping);
        }
    }

    /// Replaces the colour mapping, the one replaced kept for the getters
    /// (see [`keep_shown_mapping`](Self::keep_shown_mapping)); returns it
    /// when it is not.
    pub(crate) fn replace_mapping(&mut self, mapping: Option<Mapped>) -> Option<Mapped> {
        let shown = self.mapped_shown.get();
        let replaced = std::mem::replace(&mut self.mapping, mapping);
        let kept = self.mapping_shown.get_mut();
        if (shown.faces || shown.edges) && kept.is_none() {
            *kept = replaced;
            return None;
        }
        replaced
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
        self.shapes_through(ctx, DrawTimeShapes::shapes)
    }

    /// The shapes a draw draws and their transform, as
    /// [`current_shapes`](Self::current_shapes), the draw-time generator
    /// recording the draw.
    fn drawn_shapes(&self, ctx: LayoutContext<'_>) -> Result<(Cow<'_, [Path]>, Transform), String> {
        self.shapes_through(ctx, DrawTimeShapes::drawn_shapes)
    }

    /// The stored paths and transform, or what `generate` asks the
    /// draw-time generator for.
    fn shapes_through(
        &self,
        ctx: LayoutContext<'_>,
        generate: GenerateShapes,
    ) -> Result<(Cow<'_, [Path]>, Transform), String> {
        match &self.draw_time {
            Some(generator) => {
                let (paths, transform) = generate(&**generator, ctx)?;
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
        match (&self.mesh, &self.triangle_mesh) {
            (Some(mesh), _) => quad_mesh::draw_quad_mesh(self, mesh, ctx),
            (None, Some(mesh)) => trimesh::draw_tri_mesh(self, mesh, ctx),
            (None, None) => draw::draw_collection(self, ctx),
        }
    }
}

impl ColorMapped for CollectionArtist {
    fn mapping_mut(&mut self) -> Option<&mut ScalarMappable> {
        self.keep_shown_mapping();
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

/// Item `i` of `items`, which repeat as often as needed (as matplotlib
/// cycles a collection's per-element properties); `None` when there are
/// none.
pub(crate) fn cycled<T>(items: &[T], i: usize) -> Option<&T> {
    (!items.is_empty()).then(|| &items[i % items.len()])
}

/// The extent of `paths`, in their own coordinates, as a collection of
/// them without item transforms or offsets has it (matplotlib's
/// `get_path_collection_extents`: non-finite vertices left out).
pub(crate) fn paths_extents(paths: &[Path]) -> DataLimits {
    CollectionGeometry {
        master_transform: Affine2D::IDENTITY,
        paths,
        transforms: &[],
        // matplotlib collections without offsets have one at the origin.
        offsets: &[Point::ORIGIN],
        offset_transform: Affine2D::IDENTITY,
    }
    .extents()
    .unwrap_or(DataLimits::NULL)
}

impl Artist for CollectionArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw_hidden(&self, ctx: &mut DrawContext<'_>) {
        // matplotlib's `EllipseCollection.draw` makes the ellipses'
        // transforms before `Collection.draw` asks whether it is visible.
        self.warn_transforming_ellipses(ctx);
        if let Some(problem) = self.ellipses.as_ref().and_then(Ellipses::broadcast_problem) {
            ctx.report_kind(self.base.id(), DrawErrorKind::InvalidValue, problem);
        }
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        if let Some(mapped) = self.mapping.as_ref().filter(|mapped| mapped.at_draw()) {
            // The program's norm normalises with the figure free.
            ctx.free();
            mapped.normalize_drawn();
        }
        self.update_scalarmappable();
        self.warn_transforming_ellipses(ctx);
        if self.single_path {
            // matplotlib's renderers index the paths as a sequence.
            ctx.report_kind(
                self.base.id(),
                DrawErrorKind::InvalidType,
                "Object of type 'Path' is not an instance of 'sequence'",
            );
        } else if let Some(problem) = self.ellipses.as_ref().and_then(Ellipses::broadcast_problem) {
            // matplotlib's `EllipseCollection.draw` makes their transforms
            // first, with numpy.
            ctx.report_kind(self.base.id(), DrawErrorKind::InvalidValue, problem);
        } else if let Some(problem) = self.depth_sorted.as_ref().and_then(|s| s.problem) {
            ctx.report_kind(self.base.id(), problem.kind(), problem);
        } else if let Some(problem) = self.mapping.as_ref().and_then(Mapped::problem) {
            // matplotlib's norms raise a `ValueError` for values or limits
            // they cannot normalise.
            ctx.report_kind(self.base.id(), DrawErrorKind::InvalidValue, problem);
        } else if let Some(problem) = self.alpha_problem() {
            ctx.report_kind(self.base.id(), DrawErrorKind::InvalidValue, problem);
        } else if let Some((kind, problem)) = self.draw_time.as_ref().and_then(|g| g.problem()) {
            ctx.report_kind(self.base.id(), kind, problem);
        } else if let Err(problem) = self.render(ctx) {
            ctx.report(self.base.id(), problem);
        }
    }

    /// matplotlib's `get_window_extent`, its `get_datalim` in display
    /// coordinates: the shapes of a quad mesh, a `fill_between` collection,
    /// a 3-D or a draw-time collection (whose classes measure their data),
    /// the shapes or offsets of one placed in display coordinates (its
    /// transform ending in one that changes nothing, as the offsets an
    /// aux-transform box clears to measure it), and the null box for any
    /// other (matplotlib's collections cannot place their data without the
    /// data transform it is given).
    /// A norm or a colormap of the program's mapping the values, as
    /// matplotlib's `update_scalarmappable` does at every draw (where the figure's
    /// draws are not walked for the program's tickers already, which
    /// normalise them there: see `axes::tick_sites`).
    fn program_shaping(&self, layout: &LayoutContext<'_>) -> Option<ProgramShaping> {
        let mapping = self.mapping.as_ref()?;
        let alpha = style::collection_alpha(self);
        if layout.figure_state().ticks_by_program() {
            return mapping.predraw_mapping(alpha);
        }
        mapping.predraw_normalization(alpha)
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let measures_data = self.mesh.is_some()
            || self.fill_geometry.is_some()
            || self.draw_time.is_some()
            || self.depth_sorted.is_some();
        let offsets_in_display = (self.offset_transform.as_ref()).is_none_or(ends_in_display);
        let placed = ends_in_display(&self.transform()) || !self.offsets.is_empty();
        if !(measures_data || offsets_in_display && placed) {
            return Some(Bbox::NULL);
        }
        let resolved = ctx.resolve(&self.transform())?;
        let offsets = match &self.offset_transform {
            Some(t) => {
                // All at once: a map of the program's is called once, as
                // matplotlib maps the offsets array.
                let mut offsets = self.offsets.clone();
                ctx.resolve(t)?.transform_points(&mut offsets);
                offsets
            }
            None => Vec::new(),
        };
        // The union of the shapes' extents, null for none: a union with
        // the null box would be the whole plane.
        let shape_box = self
            .shapes()
            .iter()
            .map(|p| resolved.transform_path(p).extents())
            .filter(|b| !b.is_null())
            .reduce(|acc, b| acc.union(&b))
            .unwrap_or(Bbox::NULL);
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
    fn limit_points(&self, axes: ArtistId, _ctx: &dyn TransformContext) -> Option<LimitPoints> {
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

/// Whether `transform` ends in one that changes nothing (matplotlib's
/// `contains_branch(IdentityTransform())`, which an affine transform of the
/// identity matrix equals).
fn ends_in_display(transform: &Transform) -> bool {
    match transform {
        Transform::Display => true,
        Transform::Affine(affine) => affine.is_identity(),
        Transform::Composite(_, last) => ends_in_display(last),
        _ => false,
    }
}

pltrs_core::archive_kinds!(dyn DrawTimeShapes {
    "quiver" => crate::quiver::QuiverGeometry,
    "axis-artist-gridlines" => crate::axisartist::Gridlines,
});
