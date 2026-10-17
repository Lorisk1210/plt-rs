//! Creating 3-D collections: polygon collections, optionally shaded or
//! colour-mapped, and line collections (matplotlib's `Poly3DCollection`
//! and `Line3DCollection` constructors).

use plt_core::Rgba;

use super::art3d::{Art3D, Polys3D, ZSort};
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::math::Vec3;
use super::shade::{LightSource, polygon_normal, shade_colors};
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, ColorMapping, EdgeColors,
};
use crate::colors::{Color, Colormap, ScalarMappable};

/// A colormap and its limits for colouring faces by their mean z
/// (matplotlib's `cmap`, `vmin` and `vmax` of surfaces).
#[derive(Clone, Debug)]
pub struct FaceColormap {
    /// The colormap.
    pub cmap: Colormap,
    /// The value mapped to the low end (`None` = the smallest face value).
    pub vmin: Option<f64>,
    /// The value mapped to the high end (`None` = the largest face value).
    pub vmax: Option<f64>,
}

impl FaceColormap {
    /// `cmap` autoscaled to the faces.
    pub fn new(cmap: Colormap) -> Self {
        Self {
            cmap,
            vmin: None,
            vmax: None,
        }
    }

    /// `values` mapped through the colormap.
    fn mapping(&self, values: Vec<f64>) -> ColorMapping {
        let mut mappable = ScalarMappable::new(self.cmap.clone());
        mappable.norm_mut().set_vmin(self.vmin);
        mappable.norm_mut().set_vmax(self.vmax);
        ColorMapping {
            values: values
                .into_iter()
                .map(|v| v.is_finite().then_some(v))
                .collect(),
            mappable,
        }
    }
}

/// How the faces of a surface are coloured.
#[derive(Clone, Debug, Default)]
pub enum FaceColors {
    /// The next colour of the line property cycle, shaded by default.
    #[default]
    Cycle,
    /// One colour, shaded by default.
    Single(Color),
    /// The mean z of each face through a colormap, unshaded by default.
    Mapped(Box<FaceColormap>),
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
}

impl FaceColormap {
    /// Faces coloured by their mean z (matplotlib's `avg_z`).
    pub(crate) fn mean_z_faces(&self, faces: &[Vec<Vec3>]) -> PolyFaces {
        let values = faces
            .iter()
            .map(|face| {
                let sum = face.iter().fold(0.0, |sum, p| sum + p[2]);
                sum / face.len() as f64
            })
            .collect();
        PolyFaces::Mapped(Box::new(self.mapping(values)))
    }
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
        let (facecolors, edgecolors) = raw
            .with_mut::<CollectionArtist, _>(|c| {
                let colors = c.resolved_colors();
                // The sorted colours are written at every projection.
                c.mapping = None;
                colors
            })
            .map_err(Axes3DError::Style)?;
        self.register(
            raw.id(),
            Art3D::Polys(Polys3D {
                faces: polys.faces,
                zsort: polys.zsort,
                sort_zpos: None,
                facecolors,
                edgecolors,
            }),
        );
        Ok(handle)
    }

    /// Adds a line collection of 3-D polylines (matplotlib's
    /// `Line3DCollection` plus `add_collection`); a `color` in `props`
    /// colours the lines only.
    pub(crate) fn add_segments(
        &self,
        segments: Vec<Vec<Vec3>>,
        mut props: CollectionProps,
    ) -> Collection {
        if let Some(color) = props.color.take() {
            props.edgecolors = Some(EdgeColors::Colors(vec![color]));
        }
        let mut collection =
            CollectionArtist::new(CollectionKind::Line, Vec::new(), crate::rc::params());
        props.apply(&mut collection);
        let raw = self.adopt_projected(collection);
        self.register(raw.id(), Art3D::Segments(segments));
        Collection::from_raw(raw)
    }
}

/// Resolves colours without alpha changes.
pub(crate) fn resolve_all(colors: &[Color]) -> Result<Vec<Rgba>, Axes3DError> {
    colors
        .iter()
        .map(|c| c.resolve().map_err(Axes3DError::from))
        .collect()
}
