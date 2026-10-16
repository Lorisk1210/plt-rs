//! The fastest pseudocolour plot for each kind of grid (matplotlib's
//! `pcolorfast`): an image for uniform grids, a `PcolorImage` for
//! rectangular ones and a `QuadMesh` for arbitrary quadrilaterals.

use std::cell::RefCell;

use plt_core::Point;

use super::children::DataUpdate;
use super::pcolor::{MeshCoords, PreparedMesh};
use super::{Axes, AxesState};
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionProps, ColorMapping, EdgeColors, MeshShading};
use crate::colors::Color;
use crate::image::{
    AxesImage, AxesImageArtist, ColorizerOptions, Extent, ImageCore, ImageData, ImageError,
    Interpolation, Origin, PcolorImage, PcolorImageArtist, Resampling, ScalarField, rc_defaults,
};
use crate::numeric::Grid;

/// Options of [`Axes::pcolorfast`].
#[derive(Debug, Default)]
pub struct PcolorfastOptions {
    /// Colormap, norm and limits for scalar data.
    pub colors: ColorizerOptions,
    /// Uniform opacity.
    pub alpha: Option<f64>,
}

/// What [`Axes::pcolorfast`] drew, depending on the grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcolorFast {
    /// A uniform rectangular grid, drawn as an image.
    Image(AxesImage),
    /// A non-uniform rectangular grid.
    PcolorImage(PcolorImage),
    /// An arbitrary quadrilateral grid.
    QuadMesh(Collection),
}

/// The grid kinds `pcolorfast` distinguishes.
enum Style {
    Image(Vec<f64>, Vec<f64>),
    PcolorImage(Vec<f64>, Vec<f64>),
    QuadMesh(Grid<f64>, Grid<f64>),
}

impl Axes {
    /// A pseudocolour plot drawn as cheaply as the grid allows (matplotlib's
    /// `pcolorfast`): with no coordinates or `(min, max)` pairs, or evenly
    /// spaced 1-D edges, an image; with other 1-D edges a
    /// [`PcolorImage`]; with 2-D corner grids a quad mesh.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::{MeshCoords, PcolorFast, PcolorfastOptions};
    /// use plt::numeric::Grid;
    /// let ax = Figure::new().subplot();
    /// let c = Grid::from_fn(2, 3, |r, c| (r + c) as f64);
    /// let xy = (MeshCoords::from([0.0, 6.0]), MeshCoords::from([0.0, 4.0]));
    /// let drawn = ax.pcolorfast(Some(xy), c, PcolorfastOptions::default())?;
    /// assert!(matches!(drawn, PcolorFast::Image(_)));
    /// assert_eq!(ax.xlim(), (0.0, 6.0));
    /// # Ok::<(), plt::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the coordinates do not fit the data or the colormap options
    /// are invalid.
    pub fn pcolorfast(
        &self,
        coords: Option<(MeshCoords, MeshCoords)>,
        values: impl Into<ImageData>,
        options: PcolorfastOptions,
    ) -> Result<PcolorFast, ImageError> {
        let data = values.into().normalized();
        let (nr, nc) = data.shape();
        let style = classify(coords, nr, nc)?;
        let scalars = data.scalars().cloned();
        let mut mappable = options.colors.into_mappable(&crate::rc::params())?;
        if let Some(field) = &scalars {
            mappable.norm_mut().autoscale_none(&field.masked_values());
        }
        let (bounds, drawn) = match style {
            Style::QuadMesh(x, y) => {
                if x.shape() != y.shape() {
                    return Err(ImageError::IncompatibleCoordinates);
                }
                if x.shape() != (nr + 1, nc + 1) {
                    return Err(ImageError::MeshShape {
                        shading: "flat",
                        values: (nr, nc),
                        nx: x.cols(),
                        ny: x.rows(),
                    });
                }
                let mesh = PreparedMesh {
                    x,
                    y,
                    values: scalars.unwrap_or_else(|| ScalarField::new(Grid::filled(nr, nc, 0.0))),
                    shading: MeshShading::Flat,
                };
                let mut props = CollectionProps {
                    edgecolors: Some(EdgeColors::Colors(vec![Color::none()])),
                    antialiased: Some(vec![false]),
                    ..CollectionProps::default()
                };
                let colours = mesh_colours(&data, options.alpha);
                if colours.is_none() {
                    props.base.alpha = options.alpha;
                }
                props.facecolors = colours;
                let artist = self.quad_mesh_artist(&mesh, props, None);
                let handle = Collection::from_raw(self.adopt(artist, &DataUpdate::default()));
                if data.scalars().is_some() {
                    let values = mesh.values.masked_values();
                    handle.set_mapping(ColorMapping { values, mappable });
                }
                (
                    bounds_of(&mesh.finite_points()),
                    PcolorFast::QuadMesh(handle),
                )
            }
            Style::Image(x, y) => {
                let extent = edges_extent(&x, &y);
                let image = self.image_for_pcolorfast(data, mappable, extent, options.alpha);
                (extent_bounds(extent), PcolorFast::Image(image))
            }
            Style::PcolorImage(x, y) => {
                let extent = edges_extent(&x, &y);
                let mut artist = PcolorImageArtist::new(data, mappable, x, y);
                artist.base_mut().alpha = options.alpha;
                artist.base_mut().transform = Some(self.trans_data());
                let raw = self.adopt(artist, &DataUpdate::default());
                (
                    extent_bounds(extent),
                    PcolorFast::PcolorImage(PcolorImage::from_raw(raw)),
                )
            }
        };
        self.finish_pcolorfast(&drawn, bounds);
        Ok(drawn)
    }

    /// An `AxesImage` over `extent` with nearest-neighbour sampling, origin
    /// lower and without resampling, as `pcolorfast` builds it.
    fn image_for_pcolorfast(
        &self,
        data: ImageData,
        mappable: crate::colors::ScalarMappable,
        extent: Extent,
        alpha: Option<f64>,
    ) -> AxesImage {
        let (_, stage, _) = rc_defaults(&crate::rc::params());
        let core = ImageCore {
            data,
            mappable: RefCell::new(mappable),
            alpha_grid: None,
            stage,
            resampling: Resampling {
                interpolation: Interpolation::Nearest,
                resample: false,
                filternorm: true,
                filterrad: 4.0,
                origin: Origin::Lower,
            },
        };
        let mut artist = AxesImageArtist::new(core, extent);
        artist.base_mut().alpha = alpha;
        artist.base_mut().transform = Some(self.trans_data());
        AxesImage::from_raw(self.adopt(artist, &DataUpdate::default()))
    }

    /// The common tail of `pcolorfast`: sticky edges, data limits and a
    /// tight autoscale request.
    fn finish_pcolorfast(&self, drawn: &PcolorFast, (xl, xr, yb, yt): (f64, f64, f64, f64)) {
        let raw = match drawn {
            PcolorFast::Image(image) => image.raw(),
            PcolorFast::PcolorImage(image) => image.raw(),
            PcolorFast::QuadMesh(mesh) => mesh.raw(),
        };
        raw.with_base_mut(|base| base.sticky_edges = (vec![xl, xr], vec![yb, yt]));
        self.raw.with_mut::<AxesState, _>(|axes| {
            axes.data_lim
                .update_from_points(&[Point::new(xl, yb), Point::new(xr, yt)], true, true);
            axes.request_autoscale(true, true);
            axes.tight = true;
        });
    }
}

/// Picks the grid kind from the coordinates (matplotlib's style choice).
fn classify(
    coords: Option<(MeshCoords, MeshCoords)>,
    nr: usize,
    nc: usize,
) -> Result<Style, ImageError> {
    let Some((x, y)) = coords else {
        return Ok(Style::Image(vec![0.0, nc as f64], vec![0.0, nr as f64]));
    };
    match (x, y) {
        (MeshCoords::Vector(x), MeshCoords::Vector(y)) => {
            if x.len() == 2 && y.len() == 2 {
                return Ok(Style::Image(x, y));
            }
            for (axis, len, cells) in [("X", x.len(), nc), ("Y", y.len(), nr)] {
                if len != cells + 1 {
                    return Err(ImageError::EdgeCount {
                        axis,
                        len,
                        expected: cells + 1,
                    });
                }
            }
            if is_uniform(&x) && is_uniform(&y) {
                Ok(Style::Image(x, y))
            } else {
                Ok(Style::PcolorImage(x, y))
            }
        }
        (MeshCoords::Grid(x), MeshCoords::Grid(y)) => Ok(Style::QuadMesh(x, y)),
        _ => Err(ImageError::IncompatibleCoordinates),
    }
}

/// Whether the steps between edges vary by less than 1 % of their mean
/// (`np.ptp(dx) < 0.01 * abs(dx.mean())`).
fn is_uniform(edges: &[f64]) -> bool {
    let steps: Vec<f64> = edges.windows(2).map(|w| w[1] - w[0]).collect();
    let max = steps.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = steps.iter().copied().fold(f64::INFINITY, f64::min);
    let mean = steps.iter().sum::<f64>() / steps.len() as f64;
    max - min < 0.01 * mean.abs()
}

/// The face colours of a mesh of RGB(A) cells (matplotlib's `to_rgba(A,
/// alpha)`: `alpha` fills a missing alpha channel, NaN cells become
/// transparent), or `None` for scalar data.
fn mesh_colours(data: &ImageData, alpha: Option<f64>) -> Option<Vec<Color>> {
    let fill = alpha.unwrap_or(1.0);
    let byte = |v: u8| f64::from(f32::from(v) / 255.0);
    let colours: Vec<[f64; 4]> = match data {
        ImageData::Scalar(_) => return None,
        ImageData::Rgb(grid) => grid
            .as_slice()
            .iter()
            .map(|&[r, g, b]| [r, g, b, fill])
            .collect(),
        ImageData::Rgba(grid) => grid.as_slice().to_vec(),
        ImageData::RgbBytes(grid) => {
            let alpha = byte((fill * 255.0) as u8);
            grid.as_slice()
                .iter()
                .map(|&[r, g, b]| [byte(r), byte(g), byte(b), alpha])
                .collect()
        }
        ImageData::RgbaBytes(grid) => grid.as_slice().iter().map(|p| p.map(byte)).collect(),
    };
    Some(
        colours
            .into_iter()
            .map(|c| {
                let [r, g, b, a] = if c.iter().any(|v| v.is_nan()) {
                    [0.0; 4]
                } else {
                    c
                };
                Color::from(plt_core::Rgba::new(r, g, b, a))
            })
            .collect(),
    )
}

/// `(x[0], x[-1], y[0], y[-1])` as an extent.
fn edges_extent(x: &[f64], y: &[f64]) -> Extent {
    let first_last = |v: &[f64]| {
        (
            v.first().copied().unwrap_or(0.0),
            v.last().copied().unwrap_or(0.0),
        )
    };
    let ((left, right), (bottom, top)) = (first_last(x), first_last(y));
    Extent::new(left, right, bottom, top)
}

fn extent_bounds(extent: Extent) -> (f64, f64, f64, f64) {
    (extent.left, extent.right, extent.bottom, extent.top)
}

/// `(x.min(), x.max(), y.min(), y.max())` of the mesh vertices.
fn bounds_of(points: &[Point]) -> (f64, f64, f64, f64) {
    points.iter().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |(xl, xr, yb, yt), p| (xl.min(p.x), xr.max(p.x), yb.min(p.y), yt.max(p.y)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn values(rows: usize, cols: usize) -> Grid<f64> {
        Grid::from_fn(rows, cols, |r, c| (r * cols + c) as f64)
    }

    #[test]
    fn styles_follow_the_coordinates() {
        assert!(matches!(classify(None, 2, 3), Ok(Style::Image(..))));
        let uniform = (
            MeshCoords::from([0.0, 1.0, 2.0]),
            MeshCoords::from([0.0, 2.0]),
        );
        assert!(matches!(
            classify(Some(uniform), 1, 2),
            Ok(Style::Image(..))
        ));
        let uneven = (
            MeshCoords::from([0.0, 1.0, 3.0]),
            MeshCoords::from([0.0, 2.0]),
        );
        assert!(matches!(
            classify(Some(uneven), 1, 2),
            Ok(Style::PcolorImage(..))
        ));
        let short = (
            MeshCoords::from([0.0, 1.0, 3.0]),
            MeshCoords::from([0.0, 2.0, 3.0]),
        );
        assert!(matches!(
            classify(Some(short), 1, 2),
            Err(ImageError::EdgeCount { axis: "Y", .. })
        ));
        let mixed = (
            MeshCoords::from([0.0, 1.0]),
            MeshCoords::Grid(Grid::filled(2, 2, 0.0)),
        );
        assert_eq!(
            classify(Some(mixed), 1, 1).err(),
            Some(ImageError::IncompatibleCoordinates)
        );
    }

    #[test]
    fn every_style_fits_the_view_tightly() {
        let ax = Figure::new().subplot();
        let xy = (
            MeshCoords::from([0.0, 1.0, 3.0]),
            MeshCoords::from([0.0, 2.0]),
        );
        let drawn = ax
            .pcolorfast(Some(xy), values(1, 2), PcolorfastOptions::default())
            .unwrap();
        assert!(matches!(drawn, PcolorFast::PcolorImage(_)));
        assert_eq!((ax.xlim(), ax.ylim()), ((0.0, 3.0), (0.0, 2.0)));
        let ax = Figure::new().subplot();
        let x = Grid::from_fn(2, 3, |r, c| c as f64 + 0.5 * r as f64);
        let y = Grid::from_fn(2, 3, |r, _| r as f64);
        let drawn = ax
            .pcolorfast(
                Some((x.into(), y.into())),
                values(1, 2),
                PcolorfastOptions::default(),
            )
            .unwrap();
        assert!(matches!(drawn, PcolorFast::QuadMesh(_)));
        assert_eq!(ax.xlim(), (0.0, 2.5));
    }
}
