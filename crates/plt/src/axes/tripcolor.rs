//! Pseudocolour plots on triangular grids (matplotlib's `tripcolor`).

use std::fmt;

use plt_core::{Path, Point};

use super::Axes;
use super::children::DataUpdate;
use super::pcolor::pcolor_edge_defaults;
use crate::artist::Artist;
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, ColorMapping, TriMesh,
    TriMeshSpec,
};
use crate::colors::{ColorizerOptions, impl_colorize};
use crate::image::ImageError;
use crate::tri::Triangulation;

/// The values a [`Axes::tripcolor`] plot colours.
#[derive(Clone, Debug, PartialEq)]
pub enum TriValues {
    /// One value per point or, failing that, one per triangle (matplotlib's
    /// positional `c`; points win when both counts agree).
    Auto(Vec<f64>),
    /// One value per triangle, masked ones included (`facecolors=`).
    Faces(Vec<f64>),
}

/// How the triangles of a [`Axes::tripcolor`] plot are coloured
/// (matplotlib's `shading`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TriShading {
    /// One colour per triangle: its value, or the mean of its points'.
    #[default]
    Flat,
    /// Colours interpolated across each triangle from its points'.
    Gouraud,
}

/// Options of [`Axes::tripcolor`].
#[derive(Debug, Default)]
pub struct TripcolorOptions {
    /// Colormap, norm and limits.
    pub colorizer: ColorizerOptions,
    /// Flat or Gouraud shading.
    pub shading: TriShading,
    /// Edge colours and widths, anti-aliasing and z-order; the opacity
    /// defaults to 1 and the edges to `pcolor`'s.
    pub props: CollectionProps,
}

impl_colorize!(TripcolorOptions);

/// The artist of a [`Axes::tripcolor`] plot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tripcolor {
    /// Flat shading: one polygon per unmasked triangle.
    Flat(Collection),
    /// Gouraud shading: a triangle mesh.
    Gouraud(TriMesh),
}

/// Why a [`Axes::tripcolor`] plot cannot be drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum TripcolorError {
    /// Values fitting neither the points nor the triangles.
    ValueCount {
        /// Number of values.
        values: usize,
        /// Number of points.
        points: usize,
        /// Number of triangles.
        triangles: usize,
    },
    /// Face values whose number differs from the number of triangles.
    FaceCount {
        /// Number of values.
        values: usize,
        /// Number of triangles.
        triangles: usize,
    },
    /// Gouraud shading of values given per triangle.
    GouraudFaces,
    /// Invalid colormap options.
    Colors(ImageError),
}

impl fmt::Display for TripcolorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueCount {
                values,
                points,
                triangles,
            } => write!(
                f,
                "the length of c ({values}) must match either the number of points \
                 ({points}) or the number of triangles ({triangles})"
            ),
            Self::FaceCount { values, triangles } => write!(
                f,
                "the length of facecolors ({values}) must match the number of triangles \
                 ({triangles})"
            ),
            Self::GouraudFaces => f.write_str(
                "shading='gouraud' can only be used when the colors are specified at the \
                 points, not at the faces",
            ),
            Self::Colors(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for TripcolorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Colors(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ImageError> for TripcolorError {
    fn from(err: ImageError) -> Self {
        Self::Colors(err)
    }
}

/// Where the colour values sit, after matplotlib's argument parsing.
enum Placement {
    Points(Vec<f64>),
    Faces(Vec<f64>),
}

impl Placement {
    fn resolve(tri: &Triangulation, values: TriValues) -> Result<Self, TripcolorError> {
        let (points, triangles) = (tri.npoints(), tri.ntri());
        match values {
            TriValues::Faces(v) if v.len() != triangles => Err(TripcolorError::FaceCount {
                values: v.len(),
                triangles,
            }),
            TriValues::Faces(v) => Ok(Self::Faces(v)),
            TriValues::Auto(v) if v.len() == points => Ok(Self::Points(v)),
            TriValues::Auto(v) if v.len() == triangles => Ok(Self::Faces(v)),
            TriValues::Auto(v) => Err(TripcolorError::ValueCount {
                values: v.len(),
                points,
                triangles,
            }),
        }
    }
}

impl Axes {
    /// Colours the triangles of `tri` by `values` (matplotlib's
    /// `tripcolor`): flat triangles take their own value or the mean of
    /// their points' values; Gouraud shading interpolates point values.
    /// Turns the grid off and fits the view to all points.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::{TriValues, Tripcolor, TripcolorOptions};
    /// use plt::tri::Triangulation;
    /// let ax = Figure::new().subplot();
    /// let tri = Triangulation::delaunay(&[0.0, 1.0, 0.0, 1.0], &[0.0, 0.0, 1.0, 1.0])?;
    /// let values = TriValues::Auto(vec![0.0, 1.0, 2.0, 3.0]);
    /// let plot = ax.tripcolor(&tri, values, TripcolorOptions::default())?;
    /// assert!(matches!(plot, Tripcolor::Flat(_)));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the values fit neither the points nor the triangles, Gouraud
    /// shading meets face values, or the colormap options are invalid.
    pub fn tripcolor(
        &self,
        tri: &Triangulation,
        values: TriValues,
        options: TripcolorOptions,
    ) -> Result<Tripcolor, TripcolorError> {
        let placement = Placement::resolve(tri, values)?;
        let mappable = options
            .colorizer
            .into_mappable(&crate::rc::params())
            .map_err(ImageError::from)?;
        let mut props = options.props;
        pcolor_edge_defaults(&mut props);
        props.base.alpha.get_or_insert(1.0);
        let artist = match (options.shading, placement) {
            (TriShading::Gouraud, Placement::Faces(_)) => return Err(TripcolorError::GouraudFaces),
            (TriShading::Gouraud, Placement::Points(values)) => {
                let mapping = ColorMapping {
                    values: masked(values),
                    mappable,
                };
                let mut mesh =
                    TriMeshSpec::new(tri, mapping).map_err(|err| TripcolorError::ValueCount {
                        values: err.values,
                        points: err.points,
                        triangles: tri.ntri(),
                    })?;
                let base = mesh.artist.base_mut();
                props.base.apply(base);
                base.transform.get_or_insert_with(|| self.trans_data());
                Tripcolor::Gouraud(TriMesh::from_raw(self.add_unscaled(mesh.artist, tri)))
            }
            (TriShading::Flat, placement) => {
                let colors = match placement {
                    Placement::Points(values) => tri
                        .drawn_triangles()
                        .iter()
                        .map(|t| (values[t[0]] + values[t[1]] + values[t[2]]) / 3.0)
                        .collect(),
                    Placement::Faces(values) => tri.unmasked().map(|t| values[t]).collect(),
                };
                let paths = tri
                    .drawn_triangles()
                    .iter()
                    .map(|t| Path::polygon(t.map(|i| tri.point(i))))
                    .collect();
                let mut collection =
                    CollectionArtist::new(CollectionKind::Poly, paths, crate::rc::params());
                props.apply(&mut collection);
                collection
                    .base_mut()
                    .transform
                    .get_or_insert_with(|| self.trans_data());
                let handle = Collection::from_raw(self.add_unscaled(collection, tri));
                handle.set_mapping(ColorMapping {
                    values: masked(colors),
                    mappable,
                });
                Tripcolor::Flat(handle)
            }
        };
        Ok(artist)
    }

    /// matplotlib's tail of `tripcolor`: no grid, data limits over every
    /// point of `tri`, an immediate autoscale, then `artist` added without
    /// touching the limits.
    fn add_unscaled(&self, artist: impl Artist, tri: &Triangulation) -> crate::artist::RawHandle {
        self.grid(false);
        let corners = |values: &[f64]| {
            let min = values.iter().copied().fold(f64::INFINITY, f64::min);
            let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        };
        let ((x0, x1), (y0, y1)) = (corners(tri.x()), corners(tri.y()));
        self.update_data(&DataUpdate {
            points: vec![Point::new(x0, y0), Point::new(x1, y1)],
            update: (true, true),
            autoscale: (false, false),
        });
        self.autoscale_view(None);
        self.adopt(artist, &DataUpdate::default())
    }
}

/// The values with non-finite ones masked (`set_array`).
fn masked(values: Vec<f64>) -> Vec<Option<f64>> {
    values
        .into_iter()
        .map(|v| v.is_finite().then_some(v))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    fn square() -> Triangulation {
        Triangulation::delaunay(&[0.0, 2.0, 0.0, 2.0], &[0.0, 0.0, 1.0, 1.0]).unwrap()
    }

    #[test]
    fn values_are_placed_by_count() {
        let tri = square();
        assert!(matches!(
            Placement::resolve(&tri, TriValues::Auto(vec![0.0; 4])),
            Ok(Placement::Points(_))
        ));
        assert!(matches!(
            Placement::resolve(&tri, TriValues::Auto(vec![0.0; 2])),
            Ok(Placement::Faces(_))
        ));
        assert!(matches!(
            Placement::resolve(&tri, TriValues::Auto(vec![0.0; 3])),
            Err(TripcolorError::ValueCount { values: 3, .. })
        ));
        assert!(matches!(
            Placement::resolve(&tri, TriValues::Faces(vec![0.0; 4])),
            Err(TripcolorError::FaceCount {
                values: 4,
                triangles: 2
            })
        ));
    }

    #[test]
    fn flat_triangles_fit_the_view_without_a_grid() {
        let ax = Figure::new().subplot();
        let plot = ax
            .tripcolor(
                &square(),
                TriValues::Faces(vec![1.0, f64::NAN]),
                TripcolorOptions::default(),
            )
            .unwrap();
        let Tripcolor::Flat(collection) = plot else {
            panic!("flat shading draws polygons")
        };
        assert_eq!(collection.alpha(), Some(1.0));
        assert_eq!(ax.xlim(), (-0.1, 2.1));
        assert_eq!(ax.ylim(), (-0.05, 1.05));
    }

    #[test]
    fn gouraud_shading_needs_point_values() {
        let ax = Figure::new().subplot();
        let options = || TripcolorOptions {
            shading: TriShading::Gouraud,
            ..TripcolorOptions::default()
        };
        assert_eq!(
            ax.tripcolor(&square(), TriValues::Faces(vec![0.0; 2]), options()),
            Err(TripcolorError::GouraudFaces)
        );
        let plot = ax.tripcolor(&square(), TriValues::Auto(vec![0.0; 4]), options());
        assert!(matches!(plot, Ok(Tripcolor::Gouraud(_))));
    }
}
