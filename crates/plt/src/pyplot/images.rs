//! Images and pseudocolour plots (matplotlib's `imshow`, `matshow`,
//! `pcolor`, `pcolormesh`, `spy`, `hexbin`, `hist2d`, `figimage`,
//! `imread` and `imsave`). The plots that colour-map become the current
//! image.

use std::path::Path;

use super::figsize::figaspect_of_shape;
use super::mappables::make_current;
use super::registry::{fignum_exists, figure_num, figure_with_size, gca, gcf};
use crate::axes::{
    Axes, HexbinOptions, Hist2d, Hist2dOptions, ImshowOptions, Mesh, PcolorOptions, Spy, SpyOptions,
};
use crate::collections::Collection;
use crate::figure::Figure;
use crate::image::{
    AxesImage, FigimageOptions, FigureImage, ImageData, ImageError, ImageIoError, ImsaveOptions,
};
use crate::numeric::Grid;

/// Where `matshow` puts the axes of a new figure, in figure fractions.
const MATSHOW_RECT: [f64; 4] = [0.15, 0.09, 0.775, 0.775];

/// New axes for `matshow` at [`MATSHOW_RECT`] in `fig`.
fn matshow_axes(fig: &Figure) -> Axes {
    fig.add_axes(MATSHOW_RECT)
        .expect("the matshow rectangle is finite with a positive size")
}

/// Shows `data` as an image on the current axes and makes it the current
/// image (matplotlib's `imshow`).
///
/// # Errors
///
/// As [`Axes::imshow`](crate::axes::Axes::imshow).
pub fn imshow(data: impl Into<ImageData>, options: ImshowOptions) -> Result<AxesImage, ImageError> {
    let ax = gca();
    let image = ax.imshow(data, options)?;
    make_current(&ax, &image);
    Ok(image)
}

/// Shows a matrix with row 0 on top and makes it the current image
/// (matplotlib's `matshow`): in a new figure shaped like the matrix when
/// `fignum` is `None`, in the current axes for `Some(0)`, else in new axes
/// on figure `fignum` (shaped like the matrix if it is created).
/// `fignum` is matplotlib's pyplot `matshow(A, fignum=None)` argument,
/// which [`Axes::matshow`](crate::axes::Axes::matshow) has no use for.
///
/// # Errors
///
/// As [`Axes::matshow`](crate::axes::Axes::matshow).
pub fn matshow(
    data: impl Into<ImageData>,
    fignum: Option<u32>,
    options: ImshowOptions,
) -> Result<AxesImage, ImageError> {
    let data = data.into();
    let ax = match fignum {
        Some(0) => gca(),
        Some(num) => {
            let created = !fignum_exists(num);
            let fig = figure_num(num);
            if created {
                let (rows, cols) = data.shape();
                let (width, height) = figaspect_of_shape(rows, cols);
                fig.set_size_inches(width, height)
                    .expect("figaspect sizes are finite and positive");
            }
            matshow_axes(&fig)
        }
        None => {
            let (rows, cols) = data.shape();
            let (width, height) = figaspect_of_shape(rows, cols);
            matshow_axes(&figure_with_size(
                width,
                height,
                crate::rc::params().figure_dpi,
            ))
        }
    };
    let image = ax.matshow(data, options)?;
    make_current(&ax, &image);
    Ok(image)
}

/// Draws a pseudocolour plot of a mesh on the current axes and makes it
/// the current image (matplotlib's `pcolor`).
///
/// # Errors
///
/// As [`Axes::pcolor`](crate::axes::Axes::pcolor).
pub fn pcolor(mesh: Mesh, options: PcolorOptions) -> Result<Collection, ImageError> {
    let ax = gca();
    let cells = ax.pcolor(mesh, options)?;
    make_current(&ax, &cells);
    Ok(cells)
}

/// Draws a quadrilateral mesh on the current axes and makes it the
/// current image (matplotlib's `pcolormesh`).
///
/// # Errors
///
/// As [`Axes::pcolormesh`](crate::axes::Axes::pcolormesh).
pub fn pcolormesh(mesh: Mesh, options: PcolorOptions) -> Result<Collection, ImageError> {
    let ax = gca();
    let cells = ax.pcolormesh(mesh, options)?;
    make_current(&ax, &cells);
    Ok(cells)
}

/// Plots the non-zero entries of `z` on the current axes; an image result
/// becomes the current image (matplotlib's `spy`).
///
/// # Errors
///
/// As [`Axes::spy`](crate::axes::Axes::spy).
pub fn spy(z: &Grid<f64>, options: SpyOptions) -> Result<Spy, ImageError> {
    let ax = gca();
    let pattern = ax.spy(z, options)?;
    if let Spy::Image(image) = &pattern {
        make_current(&ax, image);
    }
    Ok(pattern)
}

/// Draws a hexagonal binning of the points on the current axes and makes
/// it the current image (matplotlib's `hexbin`).
///
/// # Errors
///
/// As [`Axes::hexbin`](crate::axes::Axes::hexbin).
pub fn hexbin(x: &[f64], y: &[f64], options: HexbinOptions) -> Result<Collection, ImageError> {
    let ax = gca();
    let cells = ax.hexbin(x, y, options)?;
    make_current(&ax, &cells);
    Ok(cells)
}

/// Draws a 2-D histogram of the points on the current axes and makes its
/// mesh the current image (matplotlib's `hist2d`).
///
/// # Errors
///
/// As [`Axes::hist2d`](crate::axes::Axes::hist2d).
pub fn hist2d(x: &[f64], y: &[f64], options: Hist2dOptions<'_>) -> Result<Hist2d, ImageError> {
    let ax = gca();
    let histogram = ax.hist2d(x, y, options)?;
    make_current(&ax, &histogram.mesh);
    Ok(histogram)
}

/// Places an image in pixels on the current figure (matplotlib's
/// `figimage`).
///
/// # Errors
///
/// As [`Figure::figimage`](crate::figure::Figure::figimage).
pub fn figimage(
    data: impl Into<ImageData>,
    options: FigimageOptions,
) -> Result<FigureImage, ImageError> {
    gcf().figimage(data, options)
}

/// Reads a PNG, JPEG, TIFF or GIF file into an image array (matplotlib's
/// `imread`; see [`crate::image::imread`]).
///
/// # Errors
///
/// When the file cannot be read or is not a valid image of those formats.
pub fn imread(path: impl AsRef<Path>) -> Result<ImageData, ImageIoError> {
    crate::image::imread(path)
}

/// Saves an array as a PNG file, colour-mapping scalar data (matplotlib's
/// `imsave`; see [`crate::image::imsave`]).
///
/// # Errors
///
/// When the array cannot be encoded or the file written.
pub fn imsave(
    path: impl AsRef<Path>,
    data: impl Into<ImageData>,
    options: ImsaveOptions,
) -> Result<(), ImageIoError> {
    crate::image::imsave(path, data, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ColorizingArtist;
    use crate::pyplot::{close_all, gci, get_fignums};

    fn ramp(rows: usize, cols: usize) -> Grid<f64> {
        Grid::from_fn(rows, cols, |row, col| (row * cols + col) as f64)
    }

    fn current_collection() -> Collection {
        match gci() {
            Some(ColorizingArtist::Collection(cells)) => cells,
            other => panic!("expected a current collection, got {other:?}"),
        }
    }

    #[test]
    fn colour_mapped_plots_become_current() {
        close_all();
        let image = imshow(ramp(3, 3), ImshowOptions::default()).unwrap();
        assert!(matches!(gci(), Some(ColorizingArtist::Image(current)) if current == image));
        let cells = pcolor(Mesh::new(ramp(2, 3)), PcolorOptions::default()).unwrap();
        assert_eq!(current_collection(), cells);
        let mesh = pcolormesh(Mesh::new(ramp(2, 3)), PcolorOptions::default()).unwrap();
        assert_eq!(current_collection(), mesh);
        let (x, y) = ([0.0, 1.0, 2.0, 0.5], [1.0, 0.0, 2.0, 0.5]);
        let hexes = hexbin(&x, &y, HexbinOptions::default()).unwrap();
        assert_eq!(current_collection(), hexes);
        let histogram = hist2d(&x, &y, Hist2dOptions::default()).unwrap();
        assert_eq!(current_collection(), histogram.mesh);
        let Spy::Image(pattern) = spy(&ramp(3, 3), SpyOptions::default()).unwrap() else {
            panic!("spy draws an image by default");
        };
        assert!(matches!(gci(), Some(ColorizingArtist::Image(current)) if current == pattern));
        close_all();
    }

    #[test]
    fn matshow_sizes_new_figures_like_the_matrix() {
        close_all();
        matshow(ramp(2, 8), None, ImshowOptions::default()).unwrap();
        assert_eq!(gcf().size_inches(), (16.0, 4.0));
        matshow(ramp(8, 8), Some(5), ImshowOptions::default()).unwrap();
        assert_eq!(get_fignums(), [1, 5]);
        assert_eq!(gcf().size_inches(), (4.8, 4.8));
        matshow(ramp(1, 1), Some(0), ImshowOptions::default()).unwrap();
        assert_eq!(gcf().axes().len(), 1, "the current axes is reused");
        close_all();
    }

    #[test]
    fn figure_images_and_files() {
        close_all();
        figimage(ramp(4, 4), FigimageOptions::default()).unwrap();
        let path =
            std::env::temp_dir().join(format!("plt-pyplot-imsave-{}.png", std::process::id()));
        imsave(&path, ramp(2, 2), ImsaveOptions::default()).unwrap();
        assert_eq!(imread(&path).unwrap().shape(), (2, 2));
        std::fs::remove_file(&path).unwrap();
        close_all();
    }
}
