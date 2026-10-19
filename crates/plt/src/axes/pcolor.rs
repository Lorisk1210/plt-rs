//! Pseudocolour plots on quadrilateral grids: `pcolor` (one polygon per
//! cell, matplotlib's `PolyQuadMesh`) and `pcolormesh` (a `QuadMesh`).

use std::fmt;
use std::str::FromStr;

use plt_core::{Path, Point};

use super::Axes;
use super::children::DataUpdate;
use crate::artist::Artist;
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, CollectionProps, ColorMapping, EdgeColors,
    MeshShading,
};
use crate::colors::{Color, ScalarMappable};
use crate::image::{ColorizerOptions, ImageError, MultiField, ScalarField, UnknownName};
use crate::numeric::Grid;

/// Where the colour values of a mesh sit (matplotlib's `shading`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Shading {
    /// `Flat` when the grid is one larger than the values, else `Nearest`.
    #[default]
    Auto,
    /// Values fill the cells between the grid vertices.
    Flat,
    /// Values sit at the grid points; cells are centred on them.
    Nearest,
    /// Values sit at the grid points and are interpolated across cells.
    Gouraud,
}

impl Shading {
    const fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Flat => "flat",
            Self::Nearest => "nearest",
            Self::Gouraud => "gouraud",
        }
    }
}

impl fmt::Display for Shading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Shading {
    type Err = UnknownName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        [Self::Auto, Self::Flat, Self::Nearest, Self::Gouraud]
            .into_iter()
            .find(|shading| shading.name() == s.to_ascii_lowercase())
            .ok_or_else(|| UnknownName {
                kind: "shading",
                name: s.to_owned(),
            })
    }
}

/// The coordinates of a mesh along one direction.
#[derive(Clone, Debug, PartialEq)]
pub enum MeshCoords {
    /// One value per column (for x) or per row (for y).
    Vector(Vec<f64>),
    /// One value per grid point.
    Grid(Grid<f64>),
}

impl From<Vec<f64>> for MeshCoords {
    fn from(values: Vec<f64>) -> Self {
        Self::Vector(values)
    }
}

impl From<&[f64]> for MeshCoords {
    fn from(values: &[f64]) -> Self {
        Self::Vector(values.to_vec())
    }
}

impl<const N: usize> From<[f64; N]> for MeshCoords {
    fn from(values: [f64; N]) -> Self {
        Self::Vector(values.to_vec())
    }
}

impl From<Grid<f64>> for MeshCoords {
    fn from(values: Grid<f64>) -> Self {
        Self::Grid(values)
    }
}

/// The colour-mapped values of a pseudocolour plot (matplotlib's `C`).
#[derive(Clone, Debug, PartialEq)]
pub enum MeshValues {
    /// Scalars, mapped through a norm and a colormap.
    Scalar(ScalarField),
    /// Several variates, mapped through one norm each and a bivariate or
    /// multivariate colormap (`pcolormesh` only, as in matplotlib 3.11).
    Multivariate(MultiField),
}

impl MeshValues {
    /// `(rows, columns)`.
    pub fn shape(&self) -> (usize, usize) {
        match self {
            Self::Scalar(field) => field.values().shape(),
            Self::Multivariate(field) => field.shape(),
        }
    }
}

impl From<ScalarField> for MeshValues {
    fn from(field: ScalarField) -> Self {
        Self::Scalar(field)
    }
}

impl From<Grid<f64>> for MeshValues {
    fn from(values: Grid<f64>) -> Self {
        Self::Scalar(values.into())
    }
}

impl From<MultiField> for MeshValues {
    fn from(field: MultiField) -> Self {
        Self::Multivariate(field)
    }
}

/// The data of a pseudocolour plot: values and, optionally, their x and
/// y coordinates (matplotlib's `[X, Y,] C`).
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// The coordinates; `None` uses the column and row indices.
    pub coords: Option<(MeshCoords, MeshCoords)>,
    /// The colour-mapped values, `(rows, columns)`.
    pub values: MeshValues,
}

impl Mesh {
    /// Values on the grid of column and row indices.
    pub fn new(values: impl Into<MeshValues>) -> Self {
        Self {
            coords: None,
            values: values.into(),
        }
    }

    /// Values at explicit coordinates.
    pub fn with_coords(
        x: impl Into<MeshCoords>,
        y: impl Into<MeshCoords>,
        values: impl Into<MeshValues>,
    ) -> Self {
        Self {
            coords: Some((x.into(), y.into())),
            values: values.into(),
        }
    }
}

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
    /// Edge colours and widths, anti-aliasing, opacity and z-order.
    pub props: CollectionProps,
}

/// A mesh after matplotlib's `_pcolorargs`: full vertex grids and the
/// resolved shading.
pub(super) struct PreparedMesh {
    pub(super) x: Grid<f64>,
    pub(super) y: Grid<f64>,
    pub(super) values: MeshValues,
    pub(super) shading: MeshShading,
}

impl PreparedMesh {
    /// Vertex `(r, c)`.
    pub(super) fn point(&self, r: usize, c: usize) -> Point {
        Point::new(self.x[(r, c)], self.y[(r, c)])
    }

    /// Every finite vertex, row by row.
    pub(super) fn finite_points(&self) -> Vec<Point> {
        let (rows, cols) = self.x.shape();
        (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (r, c)))
            .map(|(r, c)| self.point(r, c))
            .filter(|p| p.is_finite())
            .collect()
    }
}

/// matplotlib's `_pcolorargs`: builds the vertex grids, resolves `auto`
/// shading and turns `nearest` into flat cells around the points;
/// `pcolormesh` rejects non-finite coordinates.
pub(super) fn pcolor_args(
    mesh: Mesh,
    shading: Shading,
    finite_coords: bool,
) -> Result<PreparedMesh, ImageError> {
    let (nrows, ncols) = mesh.values.shape();
    let (x, y, shading) = match mesh.coords {
        None => {
            let (w, h, shading) = match shading {
                Shading::Gouraud | Shading::Nearest => (ncols, nrows, shading),
                Shading::Auto | Shading::Flat => (ncols + 1, nrows + 1, Shading::Flat),
            };
            let x = Grid::from_fn(h, w, |_, c| c as f64);
            let y = Grid::from_fn(h, w, |r, _| r as f64);
            (x, y, shading)
        }
        Some((x, y)) => {
            let finite = |coords: &MeshCoords| match coords {
                MeshCoords::Vector(v) => v.iter().all(|v| v.is_finite()),
                MeshCoords::Grid(g) => g.as_slice().iter().all(|v| v.is_finite()),
            };
            if finite_coords && !(finite(&x) && finite(&y)) {
                return Err(ImageError::InvalidCoordinates);
            }
            let (x, y) = full_grids(x, y)?;
            (x, y, shading)
        }
    };
    let (ny, nx) = x.shape();
    let shading = match shading {
        Shading::Auto if ncols == nx && nrows == ny => Shading::Nearest,
        Shading::Auto => Shading::Flat,
        other => other,
    };
    let mismatch = |shading: Shading| ImageError::MeshShape {
        shading: shading.name(),
        values: (nrows, ncols),
        nx,
        ny,
    };
    let (x, y, shading) = match shading {
        Shading::Flat => {
            if (nx, ny) != (ncols + 1, nrows + 1) {
                return Err(mismatch(shading));
            }
            (x, y, MeshShading::Flat)
        }
        Shading::Nearest | Shading::Gouraud | Shading::Auto => {
            if (nx, ny) != (ncols, nrows) {
                return Err(mismatch(shading));
            }
            if shading == Shading::Gouraud {
                (x, y, MeshShading::Gouraud)
            } else {
                let (x, y) = (interp_columns(&x), interp_columns(&y));
                let x = interp_columns(&x.transposed()).transposed();
                let y = interp_columns(&y.transposed()).transposed();
                (x, y, MeshShading::Flat)
            }
        }
    };
    Ok(PreparedMesh {
        x,
        y,
        values: mesh.values,
        shading,
    })
}

/// Repeats vector coordinates into grids of the same shape (x along the
/// rows, y along the columns).
fn full_grids(x: MeshCoords, y: MeshCoords) -> Result<(Grid<f64>, Grid<f64>), ImageError> {
    let nx = match &x {
        MeshCoords::Vector(v) => v.len(),
        MeshCoords::Grid(g) => g.cols(),
    };
    let ny = match &y {
        MeshCoords::Vector(v) => v.len(),
        MeshCoords::Grid(g) => g.rows(),
    };
    let x = match x {
        MeshCoords::Grid(g) if g.rows() != 1 => g,
        MeshCoords::Grid(g) => Grid::from_fn(ny, nx, |_, c| g[(0, c)]),
        MeshCoords::Vector(v) => Grid::from_fn(ny, nx, |_, c| v[c]),
    };
    let y = match y {
        MeshCoords::Grid(g) if g.cols() != 1 => g,
        MeshCoords::Grid(g) => Grid::from_fn(ny, nx, |r, _| g[(r, 0)]),
        MeshCoords::Vector(v) => Grid::from_fn(ny, nx, |r, _| v[r]),
    };
    if x.shape() == y.shape() {
        Ok((x, y))
    } else {
        Err(ImageError::IncompatibleCoordinates)
    }
}

/// matplotlib's `_interp_grid`: cell edges halfway between the columns,
/// extrapolated by half a step at both ends (a single column is doubled).
fn interp_columns(a: &Grid<f64>) -> Grid<f64> {
    let (rows, cols) = a.shape();
    if cols <= 1 {
        return Grid::from_fn(rows, 2 * cols, |r, c| a[(r, c % cols.max(1))]);
    }
    Grid::from_fn(rows, cols + 1, |r, c| {
        let half = |c: usize| (a[(r, c + 1)] - a[(r, c)]) * 0.5;
        match c {
            0 => a[(r, 0)] - half(0),
            c if c == cols => a[(r, cols - 1)] + half(cols - 2),
            c => a[(r, c - 1)] + half(c - 1),
        }
    })
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
        let shading = default_shading(options.shading);
        let mesh = pcolor_args(mesh, shading, true)?;
        let mut props = options.props;
        props
            .edgecolors
            .get_or_insert(EdgeColors::Colors(vec![Color::none()]));
        props.antialiased.get_or_insert(vec![false]);
        let snap = options.snap.unwrap_or(crate::rc::params().pcolormesh_snap);
        match &mesh.values {
            MeshValues::Scalar(values) => {
                let mappable = mesh_mappable(options.colors, values)?;
                let values = values.masked_values();
                let artist = self.quad_mesh_artist(&mesh, props, Some(snap));
                let handle = self.add_mesh(artist, &mesh);
                handle.set_mapping(ColorMapping { values, mappable });
                Ok(handle)
            }
            MeshValues::Multivariate(field) => {
                if mesh.shading == MeshShading::Gouraud {
                    return Err(ImageError::MultivariateNotSupported("gouraud shading"));
                }
                multivariate_faces(field, options.colors, &mut props)?;
                let artist = self.quad_mesh_artist(&mesh, props, Some(snap));
                Ok(self.add_mesh(artist, &mesh))
            }
        }
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
        base.transform = Some(self.trans_data());
        collection
    }

    /// matplotlib's `_update_pcolor_lims`: the vertices become the data
    /// limits and sticky edges, and autoscaling is requested.
    fn add_mesh(&self, mut collection: CollectionArtist, mesh: &PreparedMesh) -> Collection {
        let points = mesh.finite_points();
        let span = |coord: fn(&Point) -> f64| {
            let values = points.iter().map(coord);
            let min = values.clone().fold(f64::INFINITY, f64::min);
            let max = values.fold(f64::NEG_INFINITY, f64::max);
            vec![min, max]
        };
        let base = collection.base_mut();
        base.transform = Some(self.trans_data());
        base.sticky_edges = (span(|p| p.x), span(|p| p.y));
        Collection::from_raw(self.adopt(collection, &DataUpdate::data(points)))
    }
}

/// The shading, `pcolor.shading` when unset.
fn default_shading(shading: Option<Shading>) -> Shading {
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

    fn values(rows: usize, cols: usize) -> ScalarField {
        ScalarField::new(Grid::from_fn(rows, cols, |r, c| (r * cols + c) as f64))
    }

    #[test]
    fn auto_shading_follows_the_grid_size() {
        let flat = pcolor_args(
            Mesh::with_coords([0.0, 1.0, 2.0], [0.0, 1.0], values(1, 2)),
            Shading::Auto,
            true,
        )
        .unwrap();
        assert_eq!(flat.x.shape(), (2, 3));
        let nearest = pcolor_args(
            Mesh::with_coords([1.0, 2.0, 4.0, 8.0], [0.0, 1.0, 3.0], values(3, 4)),
            Shading::Auto,
            true,
        )
        .unwrap();
        assert_eq!(nearest.shading, MeshShading::Flat);
        assert_eq!(nearest.x.row(0), &[0.5, 1.5, 3.0, 6.0, 10.0]);
        assert_eq!(nearest.y.column(0), vec![-0.5, 0.5, 2.0, 4.0]);
    }

    #[test]
    fn mismatched_grids_are_rejected() {
        let err = pcolor_args(
            Mesh::with_coords([0.0, 1.0], [0.0, 1.0], values(3, 3)),
            Shading::Flat,
            true,
        );
        assert!(matches!(
            err,
            Err(ImageError::MeshShape {
                shading: "flat",
                ..
            })
        ));
        let x = Grid::filled(2, 3, 0.0);
        let y = Grid::filled(3, 2, 0.0);
        assert_eq!(
            full_grids(MeshCoords::Grid(x), MeshCoords::Grid(y)).err(),
            Some(ImageError::IncompatibleCoordinates)
        );
        let nan = pcolor_args(
            Mesh::with_coords([0.0, f64::NAN, 2.0], [0.0, 1.0], values(1, 2)),
            Shading::Flat,
            true,
        );
        assert_eq!(nan.err(), Some(ImageError::InvalidCoordinates));
    }

    #[test]
    fn single_columns_are_doubled() {
        let a = Grid::from_vec(2, 1, vec![3.0, 4.0]).unwrap();
        assert_eq!(interp_columns(&a).as_slice(), &[3.0, 3.0, 4.0, 4.0]);
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
