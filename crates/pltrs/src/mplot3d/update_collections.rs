//! Replacing the geometry, colours and depth order of 3-D collections
//! already made (matplotlib's `Line3DCollection.set_segments`,
//! `Poly3DCollection.set_verts`, `set_verts_and_codes` and
//! `set_3d_properties`, and `set_sort_zpos` of the line, path, patch and
//! polygon collections).
//!
//! Like matplotlib's, the new geometry is shown from the next draw on and
//! leaves the limits alone.

use pltrs_core::{Path, PathCode, Point};

use super::art3d::{Art3D, ZSort};
use super::error::{Axes3DError, check_len};
use super::handle::Axes3D;
use super::math::Vec3;
use crate::artist::ArtistHandle;
use crate::collections::{Collection, CollectionArtist};

impl Axes3D {
    /// Sets the z position a collection of these axes is ordered by among
    /// the others in computed z-order, or forgets it with `None`
    /// (matplotlib's `set_sort_zpos`): a polygon collection then takes the
    /// depth of the point `(0, 0, zpos)` in place of that of its nearest
    /// vertex. Line, scatter and patch collections keep the position
    /// without being ordered by it, as matplotlib's do.
    ///
    /// ```
    /// use pltrs::collections::CollectionProps;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let floor = vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]];
    /// let polys = ax.add_poly_collection3d(floor, CollectionProps::default(), true)?;
    /// ax.set_sort_zpos(&polys, Some(5.0))?;
    /// assert_eq!(ax.sort_zpos(&polys)?, Some(5.0));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `collection` is not a line, scatter, patch or polygon
    /// collection of these axes.
    pub fn set_sort_zpos(
        &self,
        collection: &Collection,
        zpos: Option<f64>,
    ) -> Result<&Self, Axes3DError> {
        self.edit_art(collection, |art| {
            *art.sort_zpos_mut()? = zpos;
            Some(())
        })?;
        Ok(self)
    }

    /// The z position a collection of these axes was given to be ordered
    /// by: with [`Axes3D::set_sort_zpos`], or the lowest position of a
    /// collection lifted with [`Axes3D::add_collection3d`] (matplotlib's
    /// `_sort_zpos`).
    ///
    /// # Errors
    ///
    /// When `collection` is not a line, scatter, patch or polygon
    /// collection of these axes.
    pub fn sort_zpos(&self, collection: &Collection) -> Result<Option<f64>, Axes3DError> {
        self.read_art(collection, |art| art.sort_zpos().copied())
    }

    /// Replaces the polylines of a line collection of these axes
    /// (matplotlib's `Line3DCollection.set_segments`); the polylines need
    /// not have equal lengths.
    ///
    /// ```
    /// use pltrs::collections::CollectionProps;
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// let lines = ax.add_line_collection3d(vec![vec![[0.0; 3], [1.0; 3]]], CollectionProps::default(), true)?;
    /// ax.set_segments(&lines, vec![vec![[0.0; 3], [0.5; 3], [1.0, 0.0, 0.0]]])?;
    /// fig.layout();
    /// assert_eq!(lines.paths()[0].len(), 3);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `collection` is not a line collection of these axes.
    pub fn set_segments(
        &self,
        collection: &Collection,
        segments: Vec<Vec<Vec3>>,
    ) -> Result<&Self, Axes3DError> {
        self.edit_art(collection, |art| match art {
            Art3D::Segments(old) => {
                old.lines = segments;
                Some(())
            }
            _ => None,
        })?;
        Ok(self)
    }

    /// Replaces the polygons of a polygon collection of these axes by
    /// polygons through `faces`, closed or open (matplotlib's
    /// `Poly3DCollection.set_verts`); the path codes of polygons lifted
    /// from a 2-D collection go. The colours stay those of the polygons in
    /// order: per-polygon colours given for another number of polygons
    /// repeat as matplotlib repeats them.
    ///
    /// ```
    /// use pltrs::collections::CollectionProps;
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// let triangle = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    /// let polys = ax.add_poly_collection3d(vec![triangle.clone()], CollectionProps::default(), true)?;
    /// let square = vec![[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]];
    /// ax.set_poly_verts(&polys, vec![triangle, square], true)?;
    /// fig.layout();
    /// assert_eq!(polys.paths().len(), 2);
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `collection` is not a polygon collection of these axes.
    pub fn set_poly_verts(
        &self,
        collection: &Collection,
        faces: Vec<Vec<Vec3>>,
        closed: bool,
    ) -> Result<&Self, Axes3DError> {
        self.edit_polys(collection, faces, None, closed)
    }

    /// Replaces the polygons of a polygon collection of these axes by
    /// paths through `faces` with the codes `codes`, one list of codes per
    /// face and one code per vertex (matplotlib's
    /// `Poly3DCollection.set_verts_and_codes`).
    ///
    /// # Errors
    ///
    /// When `collection` is not a polygon collection of these axes, or the
    /// codes do not match the faces or do not start with a move.
    pub fn set_poly_verts_and_codes(
        &self,
        collection: &Collection,
        faces: Vec<Vec<Vec3>>,
        codes: Vec<Vec<PathCode>>,
    ) -> Result<&Self, Axes3DError> {
        check_len("codes", faces.len(), codes.len())?;
        let templates = (faces.iter().zip(codes))
            .map(|(face, codes)| {
                if face.is_empty() {
                    return Ok(Path::polyline(Vec::new()));
                }
                Path::new(vec![Point::ORIGIN; face.len()], codes)
            })
            .collect::<Result<_, _>>()?;
        self.edit_polys(collection, faces, Some(templates), false)
    }

    /// Takes the colours of a polygon collection of these axes as they
    /// are now as its colours in 3-D, orders its polygons by the mean
    /// depth of their vertices and forgets its sort position
    /// (matplotlib's `Poly3DCollection.set_3d_properties`): how colours
    /// set on the collection after it was made reach the polygons.
    ///
    /// ```
    /// use pltrs::collections::CollectionProps;
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_3d();
    /// let triangle = vec![vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]];
    /// let polys = ax.add_poly_collection3d(triangle, CollectionProps::default(), true)?;
    /// polys.set_facecolor("red");
    /// ax.set_poly_3d_properties(&polys)?;
    /// fig.layout();
    /// assert_eq!(polys.colors().expect("colours").faces[0], pltrs::colors::Rgba::opaque(1.0, 0.0, 0.0));
    /// # Ok::<(), pltrs::mplot3d::Axes3DError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `collection` is not a polygon collection of these axes, or its
    /// colours cannot be resolved.
    pub fn set_poly_3d_properties(&self, collection: &Collection) -> Result<&Self, Axes3DError> {
        self.read_art(collection, |art| {
            matches!(art, Art3D::Polys(_)).then_some(())
        })?;
        let (facecolors, edgecolors) = collection
            .raw()
            .with::<CollectionArtist, _>(CollectionArtist::base_colors)
            .map_err(Axes3DError::Style)?;
        self.edit_art(collection, |art| {
            let Art3D::Polys(polys) = art else {
                return None;
            };
            polys.facecolors = facecolors;
            polys.edgecolors = edgecolors;
            polys.zsort = ZSort::Average;
            polys.sort_zpos = None;
            Some(())
        })?;
        Ok(self)
    }

    /// Replaces the faces of a polygon collection, with the paths whose
    /// codes they keep (`None`: polygons made of the faces, closed or
    /// not).
    fn edit_polys(
        &self,
        collection: &Collection,
        faces: Vec<Vec<Vec3>>,
        templates: Option<Vec<Path>>,
        closed: bool,
    ) -> Result<&Self, Axes3DError> {
        self.edit_art(collection, |art| {
            let Art3D::Polys(polys) = art else {
                return None;
            };
            polys.faces = faces;
            polys.templates = templates;
            polys.closed = closed;
            Some(())
        })?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::PathCode;

    use super::*;
    use crate::collections::CollectionProps;
    use crate::figure::Figure;

    fn triangle(z: f64) -> Vec<Vec3> {
        vec![[0.0, 0.0, z], [1.0, 0.0, z], [0.0, 1.0, z]]
    }

    #[test]
    fn sort_positions_order_polygons_only() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let low = ax
            .add_poly_collection3d(vec![triangle(0.0)], CollectionProps::default(), true)
            .unwrap();
        let high = ax
            .add_poly_collection3d(vec![triangle(1.0)], CollectionProps::default(), true)
            .unwrap();
        fig.layout();
        // Seen from above, the high triangle is nearer and drawn last.
        assert!(high.zorder() > low.zorder());
        ax.set_sort_zpos(&high, Some(-5.0)).unwrap();
        fig.layout();
        assert!(high.zorder() < low.zorder(), "ordered by the position");
        let lines = ax
            .add_line_collection3d(vec![triangle(2.0)], CollectionProps::default(), true)
            .unwrap();
        fig.layout();
        let before = lines.zorder();
        ax.set_sort_zpos(&lines, Some(-50.0)).unwrap();
        fig.layout();
        assert_eq!(lines.zorder(), before, "line collections keep their depth");
        assert_eq!(ax.sort_zpos(&lines), Ok(Some(-50.0)));
        let line = ax.plot([0.0], [0.0], [0.0]).unwrap();
        let other = crate::collections::Collection::from_raw(line.raw().clone());
        assert_eq!(ax.sort_zpos(&other), Err(Axes3DError::Not3DArtist));
    }

    #[test]
    fn new_polygons_keep_their_colours_in_order() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let props = CollectionProps {
            facecolors: Some(vec!["red".into(), "blue".into()]),
            ..CollectionProps::default()
        };
        let polys = ax
            .add_poly_collection3d(vec![triangle(0.0), triangle(1.0)], props, true)
            .unwrap();
        ax.set_poly_verts(&polys, vec![triangle(2.0)], false)
            .unwrap();
        fig.layout();
        let paths = polys.paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 3, "an open polygon");
        let red = crate::colors::Rgba::opaque(1.0, 0.0, 0.0);
        assert_eq!(polys.colors().unwrap().faces, [red], "matplotlib's repeat");
        let codes = vec![vec![PathCode::MoveTo, PathCode::LineTo, PathCode::LineTo]];
        ax.set_poly_verts_and_codes(&polys, vec![triangle(3.0)], codes)
            .unwrap();
        fig.layout();
        assert_eq!(polys.paths()[0].codes().map(<[PathCode]>::len), Some(3));
        let bad = vec![vec![PathCode::LineTo; 3]];
        assert!(matches!(
            ax.set_poly_verts_and_codes(&polys, vec![triangle(3.0)], bad),
            Err(Axes3DError::Path(_))
        ));
        assert!(
            ax.set_poly_verts_and_codes(&polys, vec![triangle(3.0)], Vec::new())
                .is_err()
        );
    }
}
