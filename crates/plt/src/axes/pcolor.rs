//! Pseudocolour plots on quadrilateral grids: `pcolor` (one polygon per
//! cell, matplotlib's `PolyQuadMesh`) and `pcolormesh` (a `QuadMesh`).

use plt_core::{Path, Point};

use super::Axes;
use super::children::DataUpdate;
use super::data_coords::grid_points_in_data;
use super::mesh::{Mesh, MeshValues, PreparedMesh, Shading, pcolor_args};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, ColorMapping, EdgeColors,
    MeshShading,
};
use crate::colors::{Color, ScalarMappable};
use crate::image::{ColorizerOptions, ImageError, MultiField, ScalarField};
use crate::numeric::Grid;

/// Options of [`Axes::pcolor`] and [`Axes::pcolormesh`].
#[derive(Debug, Default)]
pub struct PcolorOptions {
    /// Colormap, norm and limits.
    pub colors: ColorizerOptions,
    /// Where the values sit (`None` = `pcolor.shading`).
    pub shading: Option<Shading>,
    /// Snap the cells to pixels (`None` = off for `pcolor`,
    /// `pcolormesh.snap` for `pcolormesh`).
    pub snap: Option<bool>,
    /// Edge colours and widths, anti-aliasing, opacity, z-order and the
    /// coordinates of the mesh (default the axes' data coordinates; when
    /// they end in the data coordinates after a fixed affine map, the data
    /// limits and sticky edges cover the mesh mapped into them).
    pub props: CollectionProps,
}

/// Colours the faces of a mesh of several variates as `colors` say
/// (matplotlib's `update_scalarmappable`, which applies the artist's alpha
/// in the colormap). The alpha moves from the artist into the colours:
/// the faces get it from the colormap, explicit edge colours directly.
pub(super) fn multivariate_faces(
    field: &MultiField,
    colors: ColorizerOptions,
    props: &mut CollectionProps,
) -> Result<(), ImageError> {
    let alpha = props.base.alpha.take();
    let faces = colors.multivariate_colors(field, alpha)?;
    props.facecolors = Some(faces.into_iter().map(Color::from).collect());
    if let (Some(alpha), Some(EdgeColors::Colors(edges))) = (alpha, &mut props.edgecolors) {
        for edge in edges.iter_mut().filter(|edge| !edge.is_none()) {
            if let Ok(rgba) = edge.resolve_alpha(Some(alpha)) {
                *edge = Color::from(rgba);
            }
        }
    }
    Ok(())
}

/// The colormap and norm of a mesh, autoscaled to all its values
/// (matplotlib's `_scale_norm`).
pub(super) fn mesh_mappable(
    colors: ColorizerOptions,
    values: &ScalarField,
) -> Result<ScalarMappable, ImageError> {
    let mut mappable = colors.into_mappable(&crate::rc::params())?;
    mappable.norm_mut().autoscale_none(&values.masked_values());
    Ok(mappable)
}

impl Axes {
    /// A pseudocolour plot drawing every cell as its own polygon
    /// (matplotlib's `pcolor`); cells with a masked value or vertex are
    /// left out.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::{Mesh, PcolorOptions};
    /// use plt::numeric::Grid;
    /// let ax = Figure::new().subplot();
    /// let values = Grid::from_fn(2, 3, |r, c| (r * 3 + c) as f64);
    /// ax.pcolor(Mesh::new(values), PcolorOptions::default())?;
    /// assert_eq!(ax.xlim(), (0.0, 3.0));
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates do not fit the values, Gouraud shading is
    /// requested, the values have several variates (which matplotlib 3.11's
    /// `pcolor` cannot draw), or the colormap options are invalid.
    pub fn pcolor(&self, mesh: Mesh, options: PcolorOptions) -> Result<Collection, ImageError> {
        let shading = default_shading(options.shading);
        let mesh = pcolor_args(mesh, shading, false)?;
        if mesh.shading == MeshShading::Gouraud {
            return Err(ImageError::ShadingNotSupported("gouraud"));
        }
        let MeshValues::Scalar(values) = &mesh.values else {
            return Err(ImageError::MultivariateNotSupported("pcolor"));
        };
        let mappable = mesh_mappable(options.colors, values)?;
        let (rows, cols) = values.values().shape();
        let masked = values.masked_values();
        let (paths, values): (Vec<Path>, Vec<Option<f64>>) = (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (r, c)))
            .filter_map(|(r, c)| {
                let corners = [(r, c), (r + 1, c), (r + 1, c + 1), (r, c + 1), (r, c)]
                    .map(|(r, c)| mesh.point(r, c));
                let value = masked[r * cols + c];
                (value.is_some() && corners.iter().all(|p| p.is_finite()))
                    .then(|| (Path::polygon(corners), value))
            })
            .unzip();
        let mut props = options.props;
        pcolor_edge_defaults(&mut props);
        let mut collection =
            CollectionArtist::new(CollectionKind::Poly, paths, crate::rc::params());
        props.apply(&mut collection);
        collection.base_mut().snap = Some(options.snap.unwrap_or(false));
        let handle = self.add_mesh(collection, &mesh);
        handle.set_mapping(ColorMapping { values, mappable });
        Ok(handle)
    }

    /// A pseudocolour plot drawn as one quadrilateral mesh (matplotlib's
    /// `pcolormesh`): flat cells, cells centred on the points
    /// (`nearest`), or Gouraud-shaded; masked values leave their cells
    /// transparent.
    ///
    /// Values of several variates ([`MeshValues::Multivariate`]) are
    /// coloured through the bivariate or multivariate colormap of
    /// `options.colors.multivariate`, with the norms autoscaled to each
    /// variate; the mesh then carries the resulting face colours.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::{Mesh, PcolorOptions};
    /// use plt::colors::multivar::MultiColormap;
    /// use plt::image::{ColorizerOptions, MultiField, MultivarColorizerOptions};
    /// use plt::numeric::Grid;
    /// let ax = Figure::new().subplot();
    /// let a = Grid::from_fn(2, 3, |r, c| (r + c) as f64);
    /// let b = Grid::from_fn(2, 3, |r, c| (r * c) as f64);
    /// let colors = ColorizerOptions::default()
    ///     .multivariate(MultivarColorizerOptions::new(MultiColormap::get("BiOrangeBlue")?));
    /// let options = PcolorOptions { colors, ..PcolorOptions::default() };
    /// ax.pcolormesh(Mesh::new(MultiField::new([a, b])?), options)?;
    /// assert_eq!(ax.xlim(), (0.0, 3.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates are not finite or do not fit the values, the
    /// colormap options are invalid or do not fit the values, or values of
    /// several variates are Gouraud-shaded (which matplotlib 3.11 cannot
    /// draw).
    pub fn pcolormesh(&self, mesh: Mesh, options: PcolorOptions) -> Result<Collection, ImageError> {
        let mesh = pcolor_args(mesh, default_shading(options.shading), true)?;
        match &mesh.values {
            MeshValues::Scalar(values) => {
                let mappable = mesh_mappable(options.colors, values)?;
                let values = values.masked_values();
                let handle = self.add_pcolormesh(&mesh, options.props, options.snap);
                handle.set_mapping(ColorMapping { values, mappable });
                Ok(handle)
            }
            MeshValues::Multivariate(field) => {
                if mesh.shading == MeshShading::Gouraud {
                    return Err(ImageError::MultivariateNotSupported("gouraud shading"));
                }
                let mut props = options.props;
                multivariate_faces(field, options.colors, &mut props)?;
                Ok(self.add_pcolormesh(&mesh, props, options.snap))
            }
        }
    }

    /// The `QuadMesh` of `pcolormesh` over a prepared grid: without edges
    /// or anti-aliasing unless `props` set them, snapped as `snap` (or
    /// `pcolormesh.snap`) says.
    pub(super) fn add_pcolormesh(
        &self,
        mesh: &PreparedMesh,
        mut props: CollectionProps,
        snap: Option<bool>,
    ) -> Collection {
        props
            .edgecolors
            .get_or_insert(EdgeColors::Colors(vec![Color::none()]));
        props.antialiased.get_or_insert(vec![false]);
        let snap = snap.unwrap_or(crate::rc::params().pcolormesh_snap);
        let artist = self.quad_mesh_artist(mesh, props, Some(snap));
        self.add_mesh(artist, mesh)
    }

    /// A `QuadMesh` in data coordinates over the prepared grid, styled by
    /// `props` and `snap`.
    pub(super) fn quad_mesh_artist(
        &self,
        mesh: &PreparedMesh,
        props: CollectionProps,
        snap: Option<bool>,
    ) -> CollectionArtist {
        let (rows, cols) = mesh.x.shape();
        let coordinates = Grid::from_fn(rows, cols, |r, c| mesh.point(r, c));
        let mut collection =
            CollectionArtist::quad_mesh(coordinates, mesh.shading, crate::rc::params());
        props.apply(&mut collection);
        let base = collection.base_mut();
        base.snap = snap;
        base.transform.get_or_insert_with(|| self.trans_data());
        collection
    }

    /// matplotlib's `_update_pcolor_lims`: the mesh's vertices, in data
    /// coordinates, become the data limits and sticky edges, and
    /// autoscaling is requested.
    fn add_mesh(&self, mut collection: CollectionArtist, mesh: &PreparedMesh) -> Collection {
        let transform = collection
            .base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data())
            .clone();
        let points = grid_points_in_data(Some(&transform), self.id(), mesh.finite_points());
        let span = |coord: fn(&Point) -> f64| {
            let values = points.iter().map(coord);
            let min = values.clone().fold(f64::INFINITY, f64::min);
            let max = values.fold(f64::NEG_INFINITY, f64::max);
            vec![min, max]
        };
        collection.base_mut().sticky_edges = (span(|p| p.x), span(|p| p.y));
        Collection::from_raw(self.adopt(collection, &DataUpdate::data(points)))
    }
}

/// The shading, `pcolor.shading` when unset.
pub(super) fn default_shading(shading: Option<Shading>) -> Shading {
    shading.unwrap_or_else(|| {
        crate::rc::params()
            .pcolor_shading
            .parse()
            .unwrap_or_default()
    })
}

/// `pcolor`'s edge defaults (shared by `tripcolor`): no edges, 0.25 pt
/// wide, and no anti-aliasing unless edges are drawn or it is set.
pub(super) fn pcolor_edge_defaults(props: &mut CollectionProps) {
    let edges_none = props.edgecolors.as_ref().is_none_or(is_none);
    props
        .edgecolors
        .get_or_insert(EdgeColors::Colors(vec![Color::none()]));
    props.linewidths.get_or_insert(vec![0.25]);
    if edges_none {
        props.antialiased.get_or_insert(vec![false]);
    }
}

/// Whether edge colours are matplotlib's `"none"`.
fn is_none(edges: &EdgeColors) -> bool {
    matches!(edges, EdgeColors::Colors(colors) if colors.len() == 1 && colors[0].is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::transforms::Transform;

    fn values(rows: usize, cols: usize) -> ScalarField {
        ScalarField::new(Grid::from_fn(rows, cols, |r, c| (r * cols + c) as f64))
    }

    #[test]
    fn transformed_meshes_bound_the_data_where_they_land() {
        let ax = Figure::new().subplot();
        let times10 = Transform::Affine(plt_core::Affine2D::identity().scale(10.0, 10.0))
            .then(ax.trans_data());
        let options = PcolorOptions {
            props: CollectionProps::default().transform(times10),
            ..PcolorOptions::default()
        };
        let mesh = ax.pcolormesh(Mesh::new(values(2, 3)), options).unwrap();
        assert_eq!(mesh.sticky_edges(), (vec![0.0, 30.0], vec![0.0, 20.0]));
        assert_eq!((ax.xlim(), ax.ylim()), ((0.0, 30.0), (0.0, 20.0)));
    }

    #[test]
    fn pcolor_skips_masked_cells() {
        let ax = Figure::new().subplot();
        let mask = Grid::from_vec(1, 2, vec![false, true]).unwrap();
        let field =
            ScalarField::masked(Grid::from_vec(1, 2, vec![1.0, 2.0]).unwrap(), &mask).unwrap();
        let mesh = ax
            .pcolor(Mesh::new(field), PcolorOptions::default())
            .unwrap();
        assert_eq!(mesh.sticky_edges(), (vec![0.0, 2.0], vec![0.0, 1.0]));
        let gouraud = PcolorOptions {
            shading: Some(Shading::Gouraud),
            ..PcolorOptions::default()
        };
        assert_eq!(
            ax.pcolor(Mesh::new(values(2, 2)), gouraud).err(),
            Some(ImageError::ShadingNotSupported("gouraud"))
        );
    }

    #[test]
    fn pcolormesh_fits_the_view_to_the_grid() {
        let ax = Figure::new().subplot();
        ax.pcolormesh(
            Mesh::with_coords([0.0, 1.0, 3.0], [0.0, 2.0], values(1, 2)),
            PcolorOptions::default(),
        )
        .unwrap();
        assert_eq!(ax.xlim(), (0.0, 3.0));
        assert_eq!(ax.ylim(), (0.0, 2.0));
        assert_eq!("Gouraud".parse(), Ok(Shading::Gouraud));
        assert!("smooth".parse::<Shading>().is_err());
    }

    /// A two-variate ramp and colouring through `BiOrangeBlue`.
    fn bivariate(rows: usize, cols: usize) -> (MultiField, ColorizerOptions) {
        use crate::colors::multivar::MultiColormap;
        use crate::image::MultivarColorizerOptions;
        let field = MultiField::new([values(rows, cols), values(rows, cols)]).unwrap();
        let cmap = MultiColormap::get("BiOrangeBlue").unwrap();
        let colors = ColorizerOptions::default().multivariate(MultivarColorizerOptions::new(cmap));
        (field, colors)
    }

    #[test]
    fn multivariate_meshes_carry_their_colours() {
        let ax = Figure::new().subplot();
        let (field, colors) = bivariate(1, 2);
        let options = PcolorOptions {
            colors,
            props: CollectionProps {
                edgecolors: Some(EdgeColors::Colors(vec!["k".into()])),
                ..CollectionProps::default().alpha(0.5)
            },
            ..PcolorOptions::default()
        };
        let mesh = ax.pcolormesh(Mesh::new(field), options).unwrap();
        let (faces, edges, alpha) = mesh.raw().with::<CollectionArtist, _>(|c| {
            (c.facecolors.clone(), c.edgecolors.clone(), c.base().alpha)
        });
        let faces = faces.expect("mapped face colours");
        assert_eq!(
            faces[1],
            Color::from(plt_core::Rgba::new(1.0, 1.0, 1.0, 0.5))
        );
        assert_eq!(alpha, None, "the opacity moved into the colours");
        assert_eq!(
            edges,
            EdgeColors::Colors(vec![Color::from(plt_core::Rgba::new(0.0, 0.0, 0.0, 0.5))])
        );
        assert_eq!(ax.xlim(), (0.0, 2.0));
    }

    #[test]
    fn multivariate_data_is_drawn_by_pcolormesh_only() {
        let ax = Figure::new().subplot();
        let (field, colors) = bivariate(2, 2);
        let gouraud = PcolorOptions {
            colors,
            shading: Some(Shading::Gouraud),
            ..PcolorOptions::default()
        };
        assert_eq!(
            ax.pcolormesh(Mesh::new(field.clone()), gouraud).err(),
            Some(ImageError::MultivariateNotSupported("gouraud shading"))
        );
        let (_, colors) = bivariate(2, 2);
        let options = PcolorOptions {
            colors,
            ..PcolorOptions::default()
        };
        assert_eq!(
            ax.pcolor(Mesh::new(field.clone()), options).err(),
            Some(ImageError::MultivariateNotSupported("pcolor"))
        );
        assert_eq!(
            ax.pcolormesh(Mesh::new(field), PcolorOptions::default())
                .err(),
            Some(ImageError::ScalarColorizer)
        );
        let (_, colors) = bivariate(2, 2);
        let scalar_with_bivariate = PcolorOptions {
            colors,
            ..PcolorOptions::default()
        };
        assert_eq!(
            ax.pcolormesh(Mesh::new(values(2, 2)), scalar_with_bivariate)
                .err(),
            Some(ImageError::MultivariateColormap)
        );
        assert_eq!(MeshValues::from(values(2, 3)).shape(), (2, 3));
    }
}
