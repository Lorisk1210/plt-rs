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
//! [`TriMeshSpec`] builds a Gouraud-shaded [`TriMesh`].

mod draw;
mod legend_elements;
mod props;
mod quad_mesh;
mod spec;
mod style;
mod trimesh;

pub use legend_elements::{ElementCount, LegendElements, LegendElementsOptions, LegendProperty};
pub use props::CollectionProps;
use quad_mesh::MeshGeometry;
pub use quad_mesh::MeshShading;
pub(crate) use spec::Ellipses;
pub use spec::{CollectionPatch, CollectionSpec, EllipseUnits};
pub use style::HatchColors;
pub(crate) use trimesh::TriMeshArtist;
pub use trimesh::{TriMesh, TriMeshError, TriMeshSpec};

use std::any::Any;
use std::borrow::Cow;
use std::fmt::{self, Debug};
use std::rc::Rc;

use plt_core::path::{CollectionGeometry, DataLimits};
use plt_core::{Affine2D, Bbox, Path, Point, Rgba};
use plt_render::{CapStyle, JoinStyle};

use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ArtistStore, DrawContext, LayoutContext,
    LimitPoints, RawHandle, format_values,
};
use crate::colorizer::{self, ColorMapped, MappingChange, change_mapping};
use crate::colors::{Alpha, Color, Colormap, Norm, NormError, Normalized, ScalarMappable};
use crate::lines::LineStyle;
use crate::rc::RcParams;
use crate::transforms::Transform;

/// Which matplotlib collection class the defaults follow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CollectionKind {
    /// A generic `Collection` (patch defaults, z-order 1).
    #[default]
    Generic,
    /// Markers sized in points² at data offsets (`PathCollection`).
    Path,
    /// Polygons (`PolyCollection`, e.g. `fill_between`).
    Poly,
    /// Polylines without faces (`LineCollection`, line defaults, z-order 2).
    Line,
    /// A grid of quadrilaterals (`QuadMesh`, patch defaults, z-order 1).
    QuadMesh,
}

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

/// Values mapped to colours through a norm and colormap.
#[derive(Debug)]
pub struct ColorMapping {
    /// One value per item (`None` = masked).
    pub values: Vec<Option<f64>>,
    /// The norm and colormap.
    pub mappable: ScalarMappable,
}

/// Shapes recomputed from the figure geometry at every draw, for
/// collections whose paths depend on the axes size or the resolution
/// (matplotlib's `Quiver`, which builds its arrows in `draw`).
pub(crate) trait DrawTimeShapes: Debug {
    /// The shapes for the current geometry and the transform from their
    /// coordinates to display space.
    fn shapes(&self, ctx: LayoutContext<'_>) -> Result<(Vec<Path>, Transform), String>;

    /// The generator as [`Any`], for artists that read its state.
    fn as_any(&self) -> &dyn Any;
}

/// A [`ColorMapping`] with its values already normalised (matplotlib
/// autoscales the norm and normalises when the array is set).
#[derive(Debug)]
pub(crate) struct Mapped {
    pub(crate) source: ColorMapping,
    pub(crate) normalized: Result<Vec<Normalized>, String>,
}

impl Mapped {
    fn new(source: ColorMapping) -> Self {
        let mut mapped = Self {
            source,
            normalized: Ok(Vec::new()),
        };
        mapped.refresh();
        mapped
    }

    /// Fills unset limits from the values and normalises them again, after
    /// the norm or its limits changed (matplotlib's `set_array` followed by
    /// `update_scalarmappable`).
    fn refresh(&mut self) {
        let ColorMapping { values, mappable } = &mut self.source;
        mappable.norm_mut().autoscale_none(values);
        self.normalized = mappable.normalize(values).map_err(|e| e.to_string());
    }

    /// The mapped colours with `alpha` applied.
    pub(crate) fn colors(&self, alpha: &Alpha) -> Result<Vec<plt_core::Rgba>, String> {
        let cmap = self.source.mappable.cmap();
        let normalized = self.normalized.as_ref().map_err(Clone::clone)?;
        cmap.map(normalized, alpha).map_err(|err| err.to_string())
    }
}

/// State of a collection.
#[derive(Debug)]
pub(crate) struct CollectionArtist {
    base: ArtistBase,
    pub(crate) kind: CollectionKind,
    pub(crate) paths: Vec<Path>,
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
    pub(crate) offsets: Vec<Point>,
    /// Coordinate system of `offsets` (`None` = no offsets).
    pub(crate) offset_transform: Option<Transform>,
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
    pub(crate) capstyle: Option<CapStyle>,
    pub(crate) joinstyle: Option<JoinStyle>,
    pub(crate) mapping: Option<Mapped>,
    /// How far from an item's outline, in pixels, a pick still hits it; a
    /// pick must fall inside an item when it is not positive.
    pickradius: f64,
    /// The vertex grid of a quad mesh, drawn as a mesh.
    mesh: Option<MeshGeometry>,
    /// Replaces `paths` and the artist transform when drawing.
    pub(crate) draw_time: Option<Box<dyn DrawTimeShapes>>,
    rc: Rc<RcParams>,
}

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
            transforms: Vec::new(),
            sizes: None,
            size_factor: 1.0,
            size_rotation: 0.0,
            ellipses: None,
            offsets: Vec::new(),
            offset_transform: None,
            facecolors: None,
            edgecolors: EdgeColors::Default,
            linewidths: None,
            linestyles: vec![LineStyle::Solid],
            alphas: None,
            antialiased: None,
            hatch: None,
            hatchcolors: HatchColors::from_rc(&rc),
            capstyle: None,
            joinstyle: None,
            mapping: None,
            pickradius: 5.0,
            mesh: None,
            draw_time: None,
            rc,
        }
    }

    fn transform(&self) -> Transform {
        self.base.transform.clone().unwrap_or(Transform::Display)
    }

    /// The extent of the shapes placed at their offsets, all in data
    /// coordinates (matplotlib's `get_datalim` for collections whose shapes
    /// live in data space).
    pub(crate) fn shape_extents(&self) -> DataLimits {
        let geometry = CollectionGeometry {
            master_transform: Affine2D::IDENTITY,
            paths: &self.paths,
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

    /// The face and edge colours after defaults, colour mapping and alpha
    /// (matplotlib's `get_facecolor` and `get_edgecolor`).
    pub(crate) fn resolved_colors(
        &self,
    ) -> Result<(Vec<plt_core::Rgba>, Vec<plt_core::Rgba>), String> {
        style::resolve_colors(self)
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
            None => Ok((Cow::Borrowed(&self.paths), self.transform())),
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
        if let Err(problem) = self.render(ctx) {
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
        let shapes = self
            .paths
            .iter()
            .map(|p| resolved.transform_path(p).extents());
        let shape_box = shapes.fold(Bbox::NULL, |acc, b| acc.union(&b));
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
        let geometry = draw::Geometry::new(self, *ctx, &shapes, &transform).ok()?;
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
        self.mesh.as_ref()?;
        let values = &self.mapping.as_ref()?.source.values;
        let cells = self.pick_hit(ctx, at, None)?;
        Some(format_values(
            cells
                .into_iter()
                .filter_map(|i| values.get(i).copied().flatten()),
        ))
    }
}

/// The face, edge and hatch colours of a collection's items
/// ([`Collection::colors`]).
#[derive(Clone, Debug, PartialEq)]
pub struct CollectionColors {
    /// The face colours, cycled over the items.
    pub faces: Vec<Rgba>,
    /// The edge colours, cycled over the items.
    pub edges: Vec<Rgba>,
    /// The hatch colours, cycled over the items.
    pub hatches: Vec<Rgba>,
}

/// Why the colours of a collection cannot be resolved: an invalid colour,
/// or values the norm cannot map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionColorError(pub String);

impl fmt::Display for CollectionColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot resolve the collection's colours: {}", self.0)
    }
}

impl std::error::Error for CollectionColorError {}

/// A handle to a collection (matplotlib's `Collection` subclasses).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collection {
    raw: RawHandle,
}

impl ArtistHandle for Collection {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl Collection {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    fn update(&self, f: impl FnOnce(&mut CollectionArtist)) -> &Self {
        self.raw.with_mut::<CollectionArtist, _>(f);
        self
    }

    /// Sets one face colour for every item.
    pub fn set_facecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|c| c.facecolors = Some(vec![color]))
    }

    /// Sets per-item face colours (cycled).
    pub fn set_facecolors(&self, colors: Vec<Color>) -> &Self {
        self.update(|c| c.facecolors = Some(colors))
    }

    /// Sets the edge colours.
    pub fn set_edgecolors(&self, edges: EdgeColors) -> &Self {
        self.update(|c| c.edgecolors = edges)
    }

    /// Sets one edge colour for every item.
    pub fn set_edgecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|c| c.edgecolors = EdgeColors::Colors(vec![color]))
    }

    /// Sets the edge widths in points (cycled).
    pub fn set_linewidths(&self, widths: Vec<f64>) -> &Self {
        self.update(|c| c.linewidths = Some(widths))
    }

    /// Sets the edge line styles (cycled).
    pub fn set_linestyles(&self, styles: Vec<LineStyle>) -> &Self {
        self.update(|c| c.linestyles = styles)
    }

    /// Sets one opacity per item (matplotlib's `set_alpha` with an array):
    /// the faces and edges take them, a single colour repeated for each.
    pub fn set_alphas(&self, alphas: Vec<f64>) -> &Self {
        self.update(|c| c.alphas = Some(alphas))
    }

    /// Sets per-item antialiasing (cycled).
    pub fn set_antialiased(&self, antialiased: Vec<bool>) -> &Self {
        self.update(|c| c.antialiased = Some(antialiased))
    }

    /// Sets the marker areas in points² (cycled).
    pub fn set_sizes(&self, sizes: Vec<f64>) -> &Self {
        self.update(|c| c.sizes = Some(sizes))
    }

    /// Sets the cap style of every edge.
    pub fn set_capstyle(&self, capstyle: CapStyle) -> &Self {
        self.update(|c| c.capstyle = Some(capstyle))
    }

    /// Sets how far from an item's outline, in pixels, a pick still hits
    /// it (matplotlib's `set_pickradius`); when not positive, a pick must
    /// fall inside an item.
    pub fn set_pickradius(&self, pickradius: f64) -> &Self {
        self.update(|c| c.pickradius = pickradius)
    }

    /// The pick radius in pixels.
    pub fn pickradius(&self) -> f64 {
        self.raw.with::<CollectionArtist, _>(|c| c.pickradius)
    }

    /// Sets a hatch pattern for every item.
    pub fn set_hatch(&self, pattern: impl Into<String>) -> &Self {
        let pattern = pattern.into();
        self.update(|c| c.hatch = Some(pattern))
    }

    /// Sets the hatch colours: the edge colours, or explicit colours
    /// cycled over the items (matplotlib's `set_hatchcolor`).
    pub fn set_hatchcolors(&self, colors: HatchColors) -> &Self {
        self.update(|c| c.hatchcolors = colors)
    }

    /// Maps `values` to face (or, for line collections, edge) colours; the
    /// collection's colorbars follow the new norm and colormap.
    pub fn set_mapping(&self, mapping: ColorMapping) -> &Self {
        let mapped = Mapped::new(mapping);
        self.update(|c| c.mapping = Some(mapped));
        colorizer::notify(&self.raw, true);
        self
    }

    /// Replaces the colormap of the colour mapping, if the collection has
    /// one (matplotlib's `set_cmap`); its colorbars follow.
    pub fn set_cmap(&self, cmap: Colormap) -> &Self {
        change_mapping::<CollectionArtist>(&self.raw, MappingChange::Cmap(Box::new(cmap)));
        self
    }

    /// Replaces the norm of the colour mapping, if the collection has one
    /// (matplotlib's `set_norm`); unset limits come from the values and
    /// its colorbars follow.
    pub fn set_norm(&self, norm: Box<dyn Norm>) -> &Self {
        change_mapping::<CollectionArtist>(&self.raw, MappingChange::Norm(norm));
        self
    }

    /// Sets the data values mapped to the ends of the colormap, if the
    /// collection has a colour mapping (matplotlib's `set_clim`); `None`
    /// leaves a limit unchanged. Its colorbars follow.
    pub fn set_clim(&self, vmin: Option<f64>, vmax: Option<f64>) -> &Self {
        change_mapping::<CollectionArtist>(&self.raw, MappingChange::Clim(vmin, vmax));
        self
    }

    /// The data values mapped to the ends of the colormap, `(None, None)`
    /// without a colour mapping.
    pub fn clim(&self) -> (Option<f64>, Option<f64>) {
        self.raw.with::<CollectionArtist, _>(|c| {
            c.mapping
                .as_ref()
                .map_or((None, None), |mapped| mapped.source.mappable.clim())
        })
    }

    /// The colours of the items, after defaults, colour mapping and alpha
    /// (matplotlib's `get_facecolor`, `get_edgecolor` and `get_hatchcolor`
    /// after `update_scalarmappable`); a lone `"none"` gives no face or
    /// edge colours.
    ///
    /// # Errors
    ///
    /// When a colour is invalid or the norm cannot map the values.
    pub fn colors(&self) -> Result<CollectionColors, CollectionColorError> {
        self.raw.with::<CollectionArtist, _>(|c| {
            let (faces, edges) = c.resolved_colors().map_err(CollectionColorError)?;
            let hatches = style::resolve_hatch_colors(c, &edges)
                .map_err(|err| CollectionColorError(err.to_string()))?;
            Ok(CollectionColors {
                faces,
                edges,
                hatches,
            })
        })
    }

    /// The item offsets.
    pub fn offsets(&self) -> Vec<Point> {
        self.raw.with::<CollectionArtist, _>(|c| c.offsets.clone())
    }

    /// Moves the items to `offsets`; the data limits of the axes follow
    /// at the next [`Axes::relim`](crate::axes::Axes::relim) (matplotlib's
    /// `set_offsets`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let dots = ax.scatter(vec![1.0, 2.0], vec![3.0, 4.0])?;
    /// dots.set_offsets(vec![Point::new(10.0, 20.0), Point::new(30.0, 40.0)]);
    /// assert_eq!(ax.data_lim().bbox.p1, Point::new(2.0, 4.0), "until relim");
    /// ax.relim(false);
    /// assert_eq!(ax.data_lim().bbox.p1, Point::new(30.0, 40.0));
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn set_offsets(&self, offsets: Vec<Point>) -> &Self {
        self.raw
            .with_mut::<CollectionArtist, _>(|c| c.offsets = offsets);
        self
    }

    /// The item shapes.
    pub fn paths(&self) -> Vec<Path> {
        self.raw.with::<CollectionArtist, _>(|c| c.paths.clone())
    }
}
