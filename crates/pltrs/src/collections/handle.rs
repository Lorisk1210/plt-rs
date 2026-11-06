//! [`Collection`], the handle to a collection artist: its styling,
//! colour mapping, offsets and resolved colours.

use std::fmt;

use pltrs_core::{Path, Point, Rgba};
use pltrs_render::{CapStyle, JoinStyle};

use super::{
    ArrayError, CollectionArtist, CollectionKind, CollectionStyle, ColorMapping, EdgeColors,
    HatchColors, KeptColors, Mapped, style,
};
use crate::artist::{ArtistHandle, RawHandle, TypedHandle};
use crate::colorizer::{self, MappingChange, change_mapping};
use crate::colors::{Color, Colormap, Norm, ScalarMappable, colormaps};
use crate::lines::LineStyle;
use crate::transforms::Transform;

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

pltrs_core::archive_struct!(Collection { raw });

/// The colours of `faces_and_edges` with the hatch colours they give.
fn with_hatches(
    c: &CollectionArtist,
    faces_and_edges: Result<(Vec<Rgba>, Vec<Rgba>), String>,
) -> Result<CollectionColors, CollectionColorError> {
    let (faces, edges) = faces_and_edges.map_err(CollectionColorError)?;
    let hatches = style::resolve_hatch_colors(c, &edges)
        .map_err(|err| CollectionColorError(err.to_string()))?;
    Ok(CollectionColors {
        faces,
        edges,
        hatches,
    })
}

impl ArtistHandle for Collection {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for Collection {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<CollectionArtist>()
            .then(|| Self { raw: raw.clone() })
    }
}

impl Collection {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// The kind of collection (matplotlib's subclass of `Collection`).
    pub fn kind(&self) -> CollectionKind {
        self.raw.with::<CollectionArtist, _>(|c| c.kind)
    }

    /// The name of matplotlib's 3-D class of a collection lifted to 3-D
    /// (`"Poly3DCollection"`, `"Line3DCollection"`, ...), which its SVG
    /// id carries; `None` for one of its [`kind`](Self::kind)'s class.
    pub fn class_name(&self) -> Option<&'static str> {
        self.raw.with::<CollectionArtist, _>(|c| c.class_name)
    }

    /// Changes the collection artist.
    pub(super) fn update(&self, f: impl FnOnce(&mut CollectionArtist)) -> &Self {
        self.raw.with_mut::<CollectionArtist, _>(f);
        self
    }

    /// Sets one face colour for every item.
    pub fn set_facecolor(&self, color: impl Into<Color>) -> &Self {
        self.set_facecolors(vec![color.into()])
    }

    /// Sets per-item face colours (cycled).
    pub fn set_facecolors(&self, colors: Vec<Color>) -> &Self {
        self.update(|c| {
            c.facecolors = Some(colors);
            c.unmap_faces();
            c.kept_colors.forget_faces();
        })
    }

    /// Gives the items their kind's default face colour again, which a
    /// colour mapping overrides (matplotlib's `set_facecolor(None)`).
    pub fn reset_facecolor(&self) -> &Self {
        self.update(|c| {
            c.facecolors = None;
            c.unmap_faces();
            c.kept_colors.forget_faces();
        })
    }

    /// Sets the edge colours.
    pub fn set_edgecolors(&self, edges: EdgeColors) -> &Self {
        self.update(|c| {
            c.edgecolors = edges;
            c.unmap_edges();
            c.kept_colors.forget_edges();
        })
    }

    /// Sets one edge colour for every item.
    pub fn set_edgecolor(&self, color: impl Into<Color>) -> &Self {
        self.set_edgecolors(EdgeColors::Colors(vec![color.into()]))
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
        self.set_alpha_array(alphas, None)
    }

    /// [`set_alphas`](Self::set_alphas) from an array of `shape` (row by
    /// row), which matplotlib's errors about it name.
    ///
    /// # Errors
    ///
    /// matplotlib's `ValueError` message when the alphas cannot be
    /// broadcast onto the colours given to the faces, the edges or the gaps
    /// of dashed edges; they are set all the same, as matplotlib sets them
    /// before it finds out.
    ///
    /// ```
    /// use pltrs::colors::Color;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let dots = ax.scatter(vec![0.0, 1.0], vec![0.0, 1.0])?;
    /// dots.set_facecolors(vec![Color::from("r"), Color::from("g")]);
    /// let refused = dots.try_set_alpha_array(vec![0.1, 0.2, 0.3], None).unwrap_err();
    /// assert_eq!(refused, "could not broadcast input array from shape (3,) into shape (2,)");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// The colours the getters report ([`reported_colors`]) stay those
    /// they reported before, on the sides the alphas do not fit, until
    /// that side's colours are set again, as matplotlib keeps them.
    ///
    /// [`reported_colors`]: Self::reported_colors
    pub fn try_set_alpha_array(
        &self,
        alphas: Vec<f64>,
        shape: Option<Vec<usize>>,
    ) -> Result<&Self, String> {
        let before = self.raw.with::<CollectionArtist, _>(|c| {
            let (faces, edges) = c.reported_sides();
            KeptColors::of(faces, edges)
        });
        self.set_alpha_array(alphas, shape);
        let problem = self
            .raw
            .with::<CollectionArtist, _>(CollectionArtist::alpha_colors_problem);
        match problem {
            Some(problem) => {
                self.update(|c| c.kept_colors = before);
                Err(problem)
            }
            None => Ok(self),
        }
    }

    /// [`set_alphas`](Self::set_alphas) from an array of `shape` (`None`
    /// for one dimension).
    fn set_alpha_array(&self, alphas: Vec<f64>, shape: Option<Vec<usize>>) -> &Self {
        self.update(|c| {
            c.alphas = Some(alphas);
            c.alpha_shape = shape.filter(|shape| shape.len() > 1);
            c.unmap_faces();
            c.unmap_edges();
            c.kept_colors = KeptColors::default();
        })
    }

    /// Sets per-item antialiasing (cycled).
    pub fn set_antialiased(&self, antialiased: Vec<bool>) -> &Self {
        self.update(|c| c.antialiased = Some(antialiased))
    }

    /// Sets the marker areas in points² (cycled).
    pub fn set_sizes(&self, sizes: Vec<f64>) -> &Self {
        self.update(|c| c.sizes = Some(sizes))
    }

    /// The marker areas in points² (cycled), if set (matplotlib's
    /// `get_sizes`).
    pub fn sizes(&self) -> Option<Vec<f64>> {
        self.raw.with::<CollectionArtist, _>(|c| c.sizes.clone())
    }

    /// Sets the cap style of every edge.
    pub fn set_capstyle(&self, capstyle: CapStyle) -> &Self {
        self.update(|c| c.capstyle = Some(capstyle))
    }

    /// Sets the join style of every edge.
    pub fn set_joinstyle(&self, joinstyle: JoinStyle) -> &Self {
        self.update(|c| c.joinstyle = Some(joinstyle))
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

    /// Links the items to `urls` in vector output, cycled over the items
    /// (matplotlib's `set_urls`); `None` leaves an item unlinked.
    pub fn set_urls(&self, urls: Vec<Option<String>>) -> &Self {
        self.update(|c| c.urls = urls)
    }

    /// The links of the items (matplotlib's `get_urls`).
    pub fn urls(&self) -> Vec<Option<String>> {
        self.raw.with::<CollectionArtist, _>(|c| c.urls.clone())
    }

    /// Sets a hatch pattern for every item.
    pub fn set_hatch(&self, pattern: impl Into<String>) -> &Self {
        let pattern = pattern.into();
        self.update(|c| c.hatch = Some(pattern))
    }

    /// Removes the hatch pattern (matplotlib's `set_hatch(None)`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let points = Figure::new().subplot().scatter([0.0], [0.0])?;
    /// points.set_hatch("//").clear_hatch();
    /// assert_eq!(points.style().hatch, None);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn clear_hatch(&self) -> &Self {
        self.update(|c| c.hatch = None)
    }

    /// Sets the hatch colours: the edge colours, or explicit colours
    /// cycled over the items (matplotlib's `set_hatchcolor`).
    pub fn set_hatchcolors(&self, colors: HatchColors) -> &Self {
        self.update(|c| c.hatchcolors = colors)
    }

    /// Sets the width of the hatch lines in points (matplotlib's
    /// `set_hatch_linewidth`).
    pub fn set_hatch_linewidth(&self, width: f64) -> &Self {
        self.update(|c| c.hatch_linewidth = width)
    }

    /// The look of the items as set: colours, widths, line styles, hatch,
    /// caps and joins (matplotlib's `get_linewidth`, `get_linestyle`,
    /// `get_hatch`, `get_hatch_linewidth`, ...).
    pub fn style(&self) -> CollectionStyle {
        self.raw.with::<CollectionArtist, _>(CollectionStyle::of)
    }

    /// The edge widths in points, the kind's default when none were set:
    /// `lines.linewidth` for line collections, else `patch.linewidth`
    /// (matplotlib's `get_linewidth`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let points = ax.scatter([0.0, 1.0], [0.0, 1.0])?;
    /// assert_eq!(points.linewidths(), [pltrs::rc::params().patch_linewidth]);
    /// points.set_linewidths(vec![0.5, 2.0]);
    /// assert_eq!(points.linewidths(), [0.5, 2.0]);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn linewidths(&self) -> Vec<f64> {
        self.raw
            .with::<CollectionArtist, _>(CollectionArtist::linewidths_or_default)
    }

    /// The values mapped to colours, `None` without a colour mapping
    /// (matplotlib's `get_array`).
    pub fn array(&self) -> Option<Vec<Option<f64>>> {
        self.mapped_values(<[Option<f64>]>::to_vec)
    }

    /// What `read` makes of the values mapped to colours (masked ones are
    /// `None`), without copying them first; `None` without a colour
    /// mapping.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let dots = ax.scatter(vec![1.0, 2.0], vec![3.0, 4.0])?;
    /// dots.set_array(vec![Some(0.5), None])?;
    /// let nan_masked = dots.mapped_values(|values| {
    ///     values.iter().map(|v| v.unwrap_or(f64::NAN)).collect::<Vec<_>>()
    /// });
    /// assert!(nan_masked.unwrap()[1].is_nan());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn mapped_values<R>(&self, read: impl FnOnce(&[Option<f64>]) -> R) -> Option<R> {
        self.raw.with::<CollectionArtist, _>(|c| {
            c.mapping.as_ref().map(|mapped| read(&mapped.source.values))
        })
    }

    /// Maps `values` to colours with the collection's norm and colormap,
    /// or with a linear norm and `image.cmap` if it maps none yet
    /// (matplotlib's `set_array`); unset limits of the norm come from the
    /// values, and the collection's colorbars follow.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let dots = ax.scatter(vec![1.0, 2.0], vec![3.0, 4.0])?;
    /// assert_eq!(dots.array(), None);
    /// dots.set_array(vec![Some(0.5), None])?;
    /// assert_eq!(dots.array(), Some(vec![Some(0.5), None]));
    /// assert_eq!(dots.clim(), (Some(0.5), Some(0.5)));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// A mesh drawn cell by cell (`pcolor`'s) given one value per cell
    /// draws the cells left unmasked.
    ///
    /// # Errors
    ///
    /// When the collection is a quad mesh and the values do not match its
    /// cells (or, Gouraud-shaded, its vertices), or when it maps no colours
    /// yet and `image.cmap` names no colormap.
    pub fn set_array(&self, values: Vec<Option<f64>>) -> Result<&Self, ArrayError> {
        let expected = self
            .raw
            .with::<CollectionArtist, _>(CollectionArtist::mesh_value_count);
        if let Some(expected) = expected.filter(|&n| n != values.len()) {
            return Err(ArrayError::MeshSize {
                expected,
                actual: values.len(),
            });
        }
        let values = self
            .raw
            .with_mut::<CollectionArtist, _>(|c| c.mask_polygon_cells(values));
        let mappable = match self.raw.with::<CollectionArtist, _>(|c| {
            c.mapping
                .as_ref()
                .map(|mapped| mapped.source.mappable.clone())
                .or_else(|| c.parked_mappable.clone())
        }) {
            Some(mappable) => mappable,
            None => ScalarMappable::new(colormaps::get(&crate::rc::params().image_cmap)?),
        };
        Ok(self.set_mapping(ColorMapping { values, mappable }))
    }

    /// Stops mapping values to colours (matplotlib's `set_array(None)`):
    /// the items take their own colours again, and the norm and colormap,
    /// which colorbars keep showing, wait for the next
    /// [`Collection::set_array`].
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let dots = ax.scatter(vec![1.0, 2.0], vec![3.0, 4.0])?;
    /// dots.set_array(vec![Some(0.0), Some(1.0)])?.clear_array();
    /// assert_eq!((dots.array(), dots.cmap()), (None, None));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn clear_array(&self) -> &Self {
        self.update(|c| {
            if let Some(mapped) = c.mapping.take() {
                c.parked_mappable = Some(mapped.source.mappable);
            }
        })
    }

    /// The colormap of the colour mapping, if any (matplotlib's
    /// `get_cmap`).
    pub fn cmap(&self) -> Option<Colormap> {
        self.raw.with::<CollectionArtist, _>(|c| {
            c.mapping
                .as_ref()
                .map(|mapped| mapped.source.mappable.cmap().clone())
        })
    }

    /// The norm of the colour mapping, if any (matplotlib's `norm`).
    pub fn norm(&self) -> Option<Box<dyn Norm>> {
        self.raw.with::<CollectionArtist, _>(|c| {
            c.mapping
                .as_ref()
                .map(|mapped| mapped.source.mappable.norm().clone_box())
        })
    }

    /// Maps `values` to face (or, for line collections, edge) colours; the
    /// collection's colorbars follow the new norm and colormap.
    pub fn set_mapping(&self, mapping: ColorMapping) -> &Self {
        let mapped = Mapped::new(mapping);
        self.update(|c| c.mapping = Some(mapped));
        colorizer::notify(&self.raw, true);
        self
    }

    /// Hands a norm of the program's the values of the colour mapping as
    /// an array of `shape`, as matplotlib's collection holds them (a
    /// mesh's rows and columns).
    pub(crate) fn set_mapping_shape(&self, shape: Vec<usize>) -> &Self {
        self.update(|c| {
            if let Some(mapped) = &mut c.mapping {
                mapped.set_shape(shape);
            }
        });
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

    /// Gives the norm of the colour mapping, if the collection has one,
    /// new parameters, `norm` standing for the same norm changed in place
    /// (matplotlib setting `vmin`, `vmax`, ... of the norm object a
    /// collection keeps): unlike [`set_norm`](Self::set_norm), its
    /// colorbars keep their tickers and scale.
    pub fn retune_norm(&self, norm: Box<dyn Norm>) -> &Self {
        change_mapping::<CollectionArtist>(&self.raw, MappingChange::Retune(norm));
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
        self.raw
            .with::<CollectionArtist, _>(|c| with_hatches(c, c.resolved_colors()))
    }

    /// The colours of the items as matplotlib's `get_facecolor`,
    /// `get_edgecolor` and `get_hatchcolor` report them: the colours as set
    /// until a draw maps the values onto them (matplotlib's
    /// `update_scalarmappable`), and the mapped colours from then on, until
    /// the faces' or edges' colours or the alpha are set again; without a
    /// colour mapping, [`Collection::colors`].
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let dots = fig.subplot().scatter(vec![1.0, 2.0], vec![3.0, 4.0])?;
    /// dots.set_array(vec![Some(0.0), Some(1.0)])?;
    /// assert_eq!(dots.reported_colors()?.faces.len(), 1, "the default colour");
    /// dots.update_scalarmappable();
    /// assert_eq!(dots.reported_colors()?.faces.len(), 2, "mapped");
    /// dots.set_facecolor(Color::from("red"));
    /// assert_eq!(dots.reported_colors()?.faces.len(), 1, "as set");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a colour is invalid or the norm cannot map the values.
    pub fn reported_colors(&self) -> Result<CollectionColors, CollectionColorError> {
        self.raw
            .with::<CollectionArtist, _>(|c| with_hatches(c, c.reported_colors()))
    }

    /// Maps the values onto the faces and edges the colour mapping colours,
    /// as a draw does, for [`Collection::reported_colors`] (matplotlib's
    /// `update_scalarmappable`).
    pub fn update_scalarmappable(&self) -> &Self {
        self.raw
            .with::<CollectionArtist, _>(CollectionArtist::update_scalarmappable);
        self
    }

    /// Sets (or clears) the opacity of every item, and the colours as set
    /// again for [`Collection::reported_colors`] (matplotlib's
    /// `Collection.set_alpha` with a single value).
    pub fn set_uniform_alpha(&self, alpha: Option<f64>) -> &Self {
        self.set_alpha(alpha);
        self.update(|c| {
            c.unmap_faces();
            c.unmap_edges();
            c.kept_colors = KeptColors::default();
        })
    }

    /// The item offsets.
    pub fn offsets(&self) -> Vec<Point> {
        self.raw.with::<CollectionArtist, _>(|c| c.offsets.clone())
    }

    /// The coordinates of the offsets, `None` when the items are placed
    /// without offsets (matplotlib's `get_offset_transform`).
    pub fn offset_transform(&self) -> Option<Transform> {
        self.raw
            .with::<CollectionArtist, _>(|c| c.offset_transform.clone())
    }

    /// Places the offsets in other coordinates, or drops them with `None`
    /// (matplotlib's `set_offset_transform`).
    pub fn set_offset_transform(&self, transform: Option<Transform>) -> &Self {
        self.update(|c| c.offset_transform = transform)
    }

    /// Moves the items to `offsets`; the data limits of the axes follow
    /// at the next [`Axes::relim`](crate::axes::Axes::relim) (matplotlib's
    /// `set_offsets`). A collection made without offsets takes them in
    /// display pixels, matplotlib's default offset transform.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let dots = ax.scatter(vec![1.0, 2.0], vec![3.0, 4.0])?;
    /// dots.set_offsets(vec![Point::new(10.0, 20.0), Point::new(30.0, 40.0)]);
    /// assert_eq!(ax.data_lim().bbox.p1, Point::new(2.0, 4.0), "until relim");
    /// ax.relim(false);
    /// assert_eq!(ax.data_lim().bbox.p1, Point::new(30.0, 40.0));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_offsets(&self, offsets: Vec<Point>) -> &Self {
        self.raw.with_mut::<CollectionArtist, _>(|c| {
            c.offsets = offsets;
            c.offset_transform.get_or_insert(Transform::Display);
        });
        self
    }

    /// The item shapes.
    pub fn paths(&self) -> Vec<Path> {
        self.raw
            .with::<CollectionArtist, _>(|c| c.shapes().into_owned())
    }

    /// Replaces the items with polylines through the points of each
    /// segment (matplotlib's `LineCollection.set_segments`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let lines = ax.vlines([0.0], [0.0], [1.0])?;
    /// lines.set_segments(vec![vec![Point::new(2.0, 0.0), Point::new(2.0, 3.0)]]);
    /// assert_eq!(lines.paths()[0].vertices()[1], Point::new(2.0, 3.0));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_segments(&self, segments: Vec<Vec<Point>>) -> &Self {
        self.update(|c| c.replace_paths(segments.into_iter().map(Path::polyline).collect()))
    }

    /// Replaces the items with closed polygons through the points of each
    /// (matplotlib's `PolyCollection.set_verts`).
    ///
    /// ```
    /// use pltrs::collections::CollectionSpec;
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let triangle = vec![Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(0.0, 1.0)];
    /// let polys = ax.add_collection(CollectionSpec::polygons(vec![triangle]));
    /// polys.set_verts(vec![vec![Point::new(0.0, 0.0), Point::new(2.0, 0.0), Point::new(2.0, 2.0)]]);
    /// assert_eq!(polys.paths()[0].vertices()[1], Point::new(2.0, 0.0));
    /// ```
    pub fn set_verts(&self, polygons: Vec<Vec<Point>>) -> &Self {
        self.update(|c| c.replace_paths(polygons.into_iter().map(Path::polygon).collect()))
    }

    /// Replaces the item shapes (matplotlib's `Collection.set_paths`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::path::Path;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let lines = ax.vlines([0.0], [0.0], [1.0])?;
    /// lines.set_paths(vec![Path::polyline(vec![Point::new(1.0, 1.0), Point::new(2.0, 2.0)])]);
    /// assert_eq!(lines.paths()[0].vertices()[0], Point::new(1.0, 1.0));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_paths(&self, paths: Vec<Path>) -> &Self {
        self.update(|c| c.replace_paths(paths))
    }

    /// Gives the collection the one path `path` where a sequence of paths
    /// is expected, as matplotlib's `set_paths(path)` accepts:
    /// [`Collection::paths`] holds it, and drawing the collection fails
    /// with [`DrawErrorKind::InvalidType`](crate::artist::DrawErrorKind)
    /// (matplotlib's renderers index the paths as a sequence) until its
    /// paths are set again.
    ///
    /// ```
    /// use pltrs::artist::DrawErrorKind;
    /// use pltrs::prelude::*;
    /// use pltrs::path::Path;
    /// use pltrs::transforms::Point;
    /// let fig = Figure::new();
    /// let points = fig.subplot().scatter(vec![1.0], vec![1.0])?;
    /// points.set_single_path(Path::polyline(vec![Point::new(0.0, 1.0), Point::new(2.0, 3.0)]));
    /// assert!(points.has_single_path());
    /// let errors = fig.draw_without_rendering().unwrap_err();
    /// assert_eq!(errors.0[0].kind, DrawErrorKind::InvalidType);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_single_path(&self, path: Path) -> &Self {
        self.update(|c| {
            c.replace_paths(vec![path]);
            c.single_path = true;
        })
    }

    /// Whether the paths are the one path given by
    /// [`Collection::set_single_path`].
    pub fn has_single_path(&self) -> bool {
        self.raw.with::<CollectionArtist, _>(|c| c.single_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::LogNorm;
    use crate::figure::Figure;

    #[test]
    fn style_reads_back_what_was_set() {
        let ax = Figure::new().subplot();
        let lines = ax.vlines([0.0, 1.0], [0.0], [1.0]).expect("lines");
        let before = lines.style();
        assert_eq!(before.linestyles, [LineStyle::Solid]);
        assert_eq!(before.hatch_linewidth, crate::rc::params().hatch_linewidth);
        lines
            .set_linewidths(vec![2.0, 3.0])
            .set_linestyles(vec![LineStyle::Dashed])
            .set_hatch("//")
            .set_hatch_linewidth(0.5)
            .set_capstyle(CapStyle::Round)
            .set_alphas(vec![0.5]);
        let style = lines.style();
        assert_eq!(style.linewidths, Some(vec![2.0, 3.0]));
        assert_eq!(style.linestyles, [LineStyle::Dashed]);
        assert_eq!(
            (style.hatch.as_deref(), style.hatch_linewidth),
            (Some("//"), 0.5)
        );
        assert_eq!(
            (style.capstyle, style.joinstyle),
            (Some(CapStyle::Round), None)
        );
        assert_eq!(style.alphas, Some(vec![0.5]));
        assert_eq!(style.edgecolors, before.edgecolors, "unchanged");
    }

    #[test]
    fn linewidths_default_by_kind_and_handles_downcast() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let rc = crate::rc::params();
        let lines = ax.vlines([0.0], [0.0], [1.0]).expect("lines");
        let dots = ax.scatter(vec![0.0], vec![0.0]).expect("dots");
        dots.set_linewidths(vec![2.5]);
        assert_eq!(lines.linewidths(), [rc.lines_linewidth]);
        assert_eq!(dots.linewidths(), [2.5]);
        assert_eq!(dots.raw().downcast::<Collection>(), Some(dots.clone()));
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("line");
        assert_eq!(line.raw().downcast::<Collection>(), None);
    }

    #[test]
    fn arrays_map_through_the_mapping_or_a_new_one() {
        let ax = Figure::new().subplot();
        let dots = ax
            .scatter(vec![1.0, 2.0, 3.0], vec![1.0, 2.0, 3.0])
            .expect("dots");
        assert_eq!(
            (dots.array(), dots.cmap().is_some(), dots.norm().is_some()),
            (None, false, false)
        );
        dots.set_array(vec![Some(1.0), Some(10.0), None])
            .expect("viridis");
        assert_eq!(dots.array(), Some(vec![Some(1.0), Some(10.0), None]));
        assert_eq!(
            dots.cmap().map(|cmap| cmap.name().to_owned()).as_deref(),
            Some("viridis")
        );
        assert_eq!(dots.clim(), (Some(1.0), Some(10.0)));
        dots.set_norm(Box::new(LogNorm::new(1.0, 100.0)));
        dots.set_array(vec![Some(5.0), Some(50.0), Some(500.0)])
            .expect("a mapping");
        assert_eq!(dots.clim(), (Some(1.0), Some(100.0)), "the norm stays");
        let colors = dots.colors().expect("colours");
        assert_eq!(colors.faces.len(), 3);
    }

    #[test]
    fn meshes_take_one_value_per_cell() {
        use crate::axes::{Mesh, PcolorOptions};
        let ax = Figure::new().subplot();
        let values = crate::numeric::Grid::from_fn(2, 3, |r, c| (r + c) as f64);
        let mesh = ax
            .pcolormesh(Mesh::new(values), PcolorOptions::default())
            .unwrap();
        assert_eq!(
            mesh.set_array(vec![Some(1.0); 5]).unwrap_err(),
            ArrayError::MeshSize {
                expected: 6,
                actual: 5
            }
        );
        assert!(mesh.set_array(vec![Some(1.0); 6]).is_ok());
    }

    #[test]
    fn arrays_cleared_map_again_through_the_same_colormap() {
        let ax = Figure::new().subplot();
        let dots = ax.scatter(vec![1.0, 2.0], vec![1.0, 2.0]).expect("dots");
        dots.set_array(vec![Some(1.0), Some(2.0)]).expect("viridis");
        dots.set_cmap(crate::colors::colormaps::get("hot").expect("hot"));
        dots.clear_array();
        assert_eq!(dots.array(), None);
        assert_eq!(
            dots.colors().expect("colours").faces,
            [crate::rc::params()
                .patch_facecolor
                .resolve()
                .expect("a colour")],
            "unmapped faces"
        );
        dots.set_facecolor("none").reset_facecolor();
        assert_eq!(dots.style().facecolors, None, "the default again");
        dots.set_array(vec![Some(3.0), Some(4.0)])
            .expect("a mapping");
        assert_eq!(
            dots.cmap().map(|c| c.name().to_owned()).as_deref(),
            Some("hot")
        );
    }

    #[test]
    fn offsets_can_move_to_other_coordinates() {
        let ax = Figure::new().subplot();
        let dots = ax.scatter(vec![1.0], vec![1.0]).expect("dots");
        assert_eq!(dots.offset_transform(), Some(ax.trans_data()));
        dots.set_offset_transform(Some(ax.trans_axes()));
        assert_eq!(dots.offset_transform(), Some(ax.trans_axes()));
        dots.set_offset_transform(None);
        assert_eq!(dots.offset_transform(), None);
        dots.set_offsets(vec![Point::new(5.0, 5.0)]);
        assert_eq!(
            dots.offset_transform(),
            Some(Transform::Display),
            "offsets without coordinates are in pixels"
        );
    }
}
