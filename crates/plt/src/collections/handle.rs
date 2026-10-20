//! [`Collection`], the handle to a collection artist: its styling,
//! colour mapping, offsets and resolved colours.

use std::fmt;

use plt_core::{Path, Point, Rgba};
use plt_render::{CapStyle, JoinStyle};

use super::{CollectionArtist, ColorMapping, EdgeColors, HatchColors, Mapped, style};
use crate::artist::{ArtistHandle, RawHandle};
use crate::colorizer::{self, MappingChange, change_mapping};
use crate::colors::{Color, Colormap, Norm};
use crate::lines::LineStyle;

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

    /// Changes the collection artist.
    pub(super) fn update(&self, f: impl FnOnce(&mut CollectionArtist)) -> &Self {
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

    /// Replaces the items with polylines through the points of each
    /// segment (matplotlib's `LineCollection.set_segments`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let lines = ax.vlines([0.0], [0.0], [1.0])?;
    /// lines.set_segments(vec![vec![Point::new(2.0, 0.0), Point::new(2.0, 3.0)]]);
    /// assert_eq!(lines.paths()[0].vertices()[1], Point::new(2.0, 3.0));
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn set_segments(&self, segments: Vec<Vec<Point>>) -> &Self {
        self.update(|c| c.paths = segments.into_iter().map(Path::polyline).collect())
    }
}
