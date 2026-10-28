//! Creating 3-D collections: polygon collections, optionally shaded or
//! colour-mapped, and line collections (matplotlib's `Poly3DCollection`
//! and `Line3DCollection` constructors).

use plt_core::{Path, Rgba};

use super::art3d::{Art3D, Polys3D, Segments3D, ZSort};
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::math::Vec3;
use super::shade::{LightSource, polygon_normal, shade_colors};
use crate::artist::{Artist, RawHandle};
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, ColorMapping, EdgeColors,
};
use crate::colors::{Color, ColorizerOptions, ColormapError, ScalarMappable};

/// How the faces of a surface are coloured.
#[derive(Clone, Debug, Default)]
pub enum FaceColors {
    /// The next colour of the line property cycle, shaded by default.
    #[default]
    Cycle,
    /// One colour, shaded by default.
    Single(Color),
    /// The mean z of each face through a colormap, and a norm or limits,
    /// unshaded by default; a norm replaces the limits, as matplotlib sets
    /// the limits before the norm.
    Mapped(Box<ColorizerOptions>),
}

impl From<ColorizerOptions> for FaceColors {
    fn from(colorizer: ColorizerOptions) -> Self {
        Self::Mapped(Box::new(colorizer))
    }
}

impl FaceColors {
    /// Whether faces are shaded unless asked otherwise (matplotlib's
    /// `shade=cmap is None`).
    pub(crate) fn shaded_by_default(&self) -> bool {
        !matches!(self, Self::Mapped(_))
    }
}

/// The face colours of new polygons before shading.
#[derive(Debug)]
pub(crate) enum PolyFaces {
    /// Explicit colours (one, or one per polygon).
    Colors(Vec<Rgba>),
    /// One value per polygon mapped through a colormap.
    Mapped(Box<ColorMapping>),
}

/// Everything a new polygon collection needs.
#[derive(Debug)]
pub(crate) struct NewPolys {
    pub(crate) faces: Vec<Vec<Vec3>>,
    pub(crate) colors: PolyFaces,
    /// Edge colours that are shaded with the faces (matplotlib's
    /// `edgecolors` given to a shaded `Poly3DCollection`).
    pub(crate) shaded_edges: Option<Vec<Rgba>>,
    /// The light shading the colours, if any.
    pub(crate) light: Option<LightSource>,
    pub(crate) zsort: ZSort,
    /// Further properties, applied after the colours.
    pub(crate) props: CollectionProps,
    /// Whether the vertices outside the view limits are hidden
    /// (matplotlib's `axlim_clip`).
    pub(crate) axlim_clip: bool,
}

/// Faces coloured by their mean z (matplotlib's `avg_z`) through
/// `colorizer`, whose norm replaces its limits (matplotlib's `set_clim`
/// followed by `set_norm`).
pub(crate) fn mean_z_faces(
    colorizer: &ColorizerOptions,
    faces: &[Vec<Vec3>],
) -> Result<PolyFaces, ColormapError> {
    let cmap = colorizer.colormap(&crate::rc::params())?;
    let mappable = if let Some(norm) = &colorizer.norm {
        ScalarMappable::with_norm(cmap, norm.clone().resolve())
    } else {
        let mut mappable = ScalarMappable::new(cmap);
        mappable.norm_mut().set_vmin(colorizer.vmin);
        mappable.norm_mut().set_vmax(colorizer.vmax);
        mappable
    };
    let values = faces
        .iter()
        .map(|face| {
            let mean = face.iter().fold(0.0, |sum, p| sum + p[2]) / face.len() as f64;
            mean.is_finite().then_some(mean)
        })
        .collect();
    Ok(PolyFaces::Mapped(Box::new(ColorMapping {
        values,
        mappable,
    })))
}

impl Axes3D {
    /// Adds a polygon collection (matplotlib's `Poly3DCollection` plus
    /// `add_collection`); the colours are resolved once, like
    /// matplotlib's `_facecolor3d` and `_edgecolor3d`.
    pub(crate) fn add_polys(&self, polys: NewPolys) -> Result<Collection, Axes3DError> {
        let normals: Vec<Vec3> = match polys.light {
            Some(_) => polys.faces.iter().map(|f| polygon_normal(f)).collect(),
            None => Vec::new(),
        };
        let shade = |colors: Vec<Rgba>| -> Vec<Color> {
            let colors = match &polys.light {
                Some(light) => shade_colors(&colors, &normals, light),
                None => colors,
            };
            colors.into_iter().map(Color::from).collect()
        };
        let mut collection =
            CollectionArtist::new(CollectionKind::Poly, Vec::new(), crate::rc::params());
        let mapping = match polys.colors {
            PolyFaces::Colors(colors) => {
                collection.facecolors = Some(shade(colors));
                None
            }
            PolyFaces::Mapped(mapping) => Some(mapping),
        };
        if let Some(edges) = polys.shaded_edges {
            collection.edgecolors = EdgeColors::Colors(shade(edges));
        }
        polys.props.apply(&mut collection);
        let raw = self.adopt_projected(collection);
        let handle = Collection::from_raw(raw.clone());
        if let Some(mapping) = mapping {
            handle.set_mapping(*mapping);
        }
        self.register_polys(&raw, polys.faces, polys.zsort, None, polys.axlim_clip)?;
        Ok(handle)
    }

    /// Gives a polygon collection already added to the plane its 3-D
    /// faces, resolving its colours once (matplotlib's
    /// `set_3d_properties` keeping `_facecolor3d` and `_edgecolor3d`;
    /// colour-mapped ones are mapped again at every projection);
    /// `lifted` holds the 2-D paths and the sort position of polygons
    /// lifted from a 2-D collection, and `axlim_clip` hides the vertices
    /// outside the view limits.
    ///
    /// The collection's alpha fades the faces; the edges keep their own
    /// alpha, as matplotlib's `Poly3DCollection.set_alpha` leaves
    /// `_edgecolor3d` alone, unless they were lifted from a 2-D
    /// collection, whose `set_alpha` faded them already.
    pub(crate) fn register_polys(
        &self,
        raw: &RawHandle,
        faces: Vec<Vec<Vec3>>,
        zsort: ZSort,
        lifted: Option<(Vec<Path>, f64)>,
        axlim_clip: bool,
    ) -> Result<(), Axes3DError> {
        let (facecolors, edgecolors) = raw
            .with_mut::<CollectionArtist, _>(|c| {
                let (faces, edges) = c.resolved_colors()?;
                let edges = if lifted.is_some() {
                    edges
                } else {
                    unfaded_edges(c)?
                };
                Ok((faces, edges))
            })
            .map_err(Axes3DError::Style)?;
        let (templates, sort_zpos) = lifted.unzip();
        self.register(
            raw.id(),
            Art3D::Polys(Polys3D {
                faces,
                zsort,
                sort_zpos,
                facecolors,
                edgecolors,
                templates,
                closed: true,
            }),
            axlim_clip,
        );
        Ok(())
    }

    /// Adds a line collection of 3-D polylines (matplotlib's
    /// `Line3DCollection` plus `add_collection`); a `color` in `props`
    /// colours the lines only, and `axlim_clip` hides the points outside
    /// the view limits.
    pub(crate) fn add_segments(
        &self,
        segments: Vec<Vec<Vec3>>,
        mut props: CollectionProps,
        axlim_clip: bool,
    ) -> Collection {
        if let Some(color) = props.color.take() {
            props.edgecolors = Some(EdgeColors::Colors(vec![color]));
        }
        let mut collection =
            CollectionArtist::new(CollectionKind::Line, Vec::new(), crate::rc::params());
        props.apply(&mut collection);
        let raw = self.adopt_projected(collection);
        self.register(
            raw.id(),
            Art3D::Segments(Segments3D::new(segments)),
            axlim_clip,
        );
        Collection::from_raw(raw)
    }
}

/// The edge colours of `collection` without its alpha.
fn unfaded_edges(collection: &mut CollectionArtist) -> Result<Vec<Rgba>, String> {
    let alpha = collection.base_mut().alpha.take();
    let alphas = collection.alphas.take();
    let edges = collection.resolved_colors().map(|(_, edges)| edges);
    collection.base_mut().alpha = alpha;
    collection.alphas = alphas;
    edges
}

/// Resolves colours without alpha changes.
pub(crate) fn resolve_all(colors: &[Color]) -> Result<Vec<Rgba>, Axes3DError> {
    colors
        .iter()
        .map(|c| c.resolve().map_err(Axes3DError::from))
        .collect()
}
