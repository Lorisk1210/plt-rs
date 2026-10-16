//! Images on non-uniform rectangular grids (matplotlib's `PcolorImage`,
//! made by `pcolorfast`).

use std::cell::RefCell;

use plt_core::{Bbox, Point};
use plt_render::RgbaImage;

use super::artist::draw_rendered;
use super::make::RenderedImage;
use super::stages::normalize;
use super::{ImageData, ImageError};
use crate::artist::{Artist, ArtistBase, ArtistHandle, DrawContext, LayoutContext, RawHandle};
use crate::axes::AxesState;
use crate::colors::ScalarMappable;
use crate::numeric::{Grid, linspace};

/// State of an image whose cells have individual widths and heights.
#[derive(Debug)]
pub(crate) struct PcolorImageArtist {
    base: ArtistBase,
    data: ImageData,
    mappable: RefCell<ScalarMappable>,
    /// Increasing cell edges along x.
    x: Vec<f64>,
    /// Increasing cell edges along y.
    y: Vec<f64>,
}

impl PcolorImageArtist {
    /// An image with cell edges `x` and `y` (one more than the columns and
    /// rows of `data`); decreasing edges flip the data, as matplotlib does.
    pub(crate) fn new(
        data: ImageData,
        mappable: ScalarMappable,
        mut x: Vec<f64>,
        mut y: Vec<f64>,
    ) -> Self {
        let (mut flip_x, mut flip_y) = (false, false);
        if x.last() < x.first() {
            x.reverse();
            flip_x = true;
        }
        if y.last() < y.first() {
            y.reverse();
            flip_y = true;
        }
        Self {
            base: ArtistBase::with_zorder(0.0),
            data: flipped(data, flip_x, flip_y),
            mappable: RefCell::new(mappable),
            x,
            y,
        }
    }

    /// matplotlib's `PcolorImage.make_image`: every pixel of the axes box
    /// takes the colour of the cell containing its centre (transparent
    /// outside the grid).
    fn render(&self, ctx: &DrawContext<'_>) -> Result<Option<RenderedImage>, ImageError> {
        let layout = ctx.layout();
        let Some(axes_id) = self.base.parent else {
            return Ok(None);
        };
        let (Some(axes), Some(view)) = (
            layout.axes_extent(axes_id),
            layout
                .store()
                .get_as::<AxesState>(axes_id)
                .map(|axes| axes.view_lim),
        ) else {
            return Ok(None);
        };
        let colors = cell_colors(&self.data, &mut self.mappable.borrow_mut())?;
        let round = |v: f64| (v + 0.5) as i64;
        let (l, b, r, t) = (
            round(axes.x0()),
            round(axes.y0()),
            round(axes.x1()),
            round(axes.y1()),
        );
        let (Ok(width), Ok(height)) = (usize::try_from(r - l), usize::try_from(t - b)) else {
            return Ok(None);
        };
        if width == 0 || height == 0 {
            return Ok(None);
        }
        let x_cells = pixel_cells(&self.x, view.x0(), view.x1(), width);
        let y_cells = pixel_cells(&self.y, view.y0(), view.y1(), height);
        let (rows, cols) = colors.shape();
        // Index 0 and the last index fall outside the grid (the padding).
        let color = |row: usize, col: usize| {
            if row == 0 || col == 0 || row > rows || col > cols {
                [0; 4]
            } else {
                colors[(row - 1, col - 1)]
            }
        };
        let data = y_cells
            .iter()
            .rev()
            .flat_map(|&row| x_cells.iter().map(move |&col| (row, col)))
            .flat_map(|(row, col)| color(row, col))
            .collect();
        Ok(Some(RenderedImage {
            pixels: RgbaImage::new(width, height, data).expect("four bytes per pixel"),
            origin: Point::new(l as f64, b as f64),
        }))
    }
}

/// For each of `count` pixels spanning `low..high`, the index of the first
/// edge at or beyond its centre (numpy's `searchsorted`).
fn pixel_cells(edges: &[f64], low: f64, high: f64, count: usize) -> Vec<usize> {
    let pixel_edges = linspace(low, high, count + 1);
    pixel_edges
        .windows(2)
        .map(|w| {
            let centre = f64::midpoint(w[0], w[1]);
            edges.partition_point(|&edge| edge < centre)
        })
        .collect()
}

/// `data` with its columns and/or rows reversed.
fn flipped(data: ImageData, x: bool, y: bool) -> ImageData {
    fn flip<T: Clone>(grid: &Grid<T>, x: bool, y: bool) -> Grid<T> {
        let (rows, cols) = grid.shape();
        Grid::from_fn(rows, cols, |r, c| {
            let r = if y { rows - 1 - r } else { r };
            let c = if x { cols - 1 - c } else { c };
            grid[(r, c)].clone()
        })
    }
    if !x && !y {
        return data;
    }
    match data {
        ImageData::Scalar(field) => {
            let values = flip(field.values(), x, y);
            let mask = flip(field.mask(), x, y);
            ImageData::Scalar(super::ScalarField::masked(values, &mask).expect("flipped together"))
        }
        ImageData::Rgb(grid) => ImageData::Rgb(flip(&grid, x, y)),
        ImageData::Rgba(grid) => ImageData::Rgba(flip(&grid, x, y)),
        ImageData::RgbBytes(grid) => ImageData::RgbBytes(flip(&grid, x, y)),
        ImageData::RgbaBytes(grid) => ImageData::RgbaBytes(flip(&grid, x, y)),
    }
}

/// matplotlib's `to_rgba(A, bytes=True)`: mapped scalars, or colours
/// passed through (opaque when without alpha, transparent when NaN).
pub(crate) fn cell_colors(
    data: &ImageData,
    mappable: &mut ScalarMappable,
) -> Result<Grid<[u8; 4]>, ImageError> {
    let unit = |pixel: [f64; 4]| {
        if pixel.iter().any(|v| v.is_nan()) {
            [0; 4]
        } else {
            pixel.map(|v| (v * 255.0) as u8)
        }
    };
    Ok(match data {
        ImageData::Scalar(field) => {
            let (rows, cols) = field.values().shape();
            let normalized = normalize(mappable, &field.masked_values(), field.kind())?;
            let cmap = mappable.cmap();
            let colors = normalized
                .into_iter()
                .map(|n| cmap.lookup_bytes(n, None))
                .collect();
            Grid::from_vec(rows, cols, colors).expect("one colour per cell")
        }
        ImageData::Rgb(grid) => grid.map(|&[r, g, b]| unit([r, g, b, 1.0])),
        ImageData::Rgba(grid) => grid.map(|&pixel| unit(pixel)),
        ImageData::RgbBytes(grid) => grid.map(|&[r, g, b]| [r, g, b, u8::MAX]),
        ImageData::RgbaBytes(grid) => grid.clone(),
    })
}

impl Artist for PcolorImageArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let rendered = self.render(ctx);
        draw_rendered(ctx, &self.base, rendered);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        let resolved = ctx.resolve(self.base.transform.as_ref()?)?;
        let corners = [
            (self.x.first()?, self.y.first()?),
            (self.x.last()?, self.y.last()?),
        ];
        Some(Bbox::from_points(
            corners.map(|(&x, &y)| resolved.transform_point(Point::new(x, y))),
        ))
    }
}

/// A handle to an image with non-uniform cells (matplotlib's
/// `PcolorImage`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcolorImage {
    raw: RawHandle,
}

impl ArtistHandle for PcolorImage {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl PcolorImage {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// The increasing cell edges `(x, y)`.
    pub fn edges(&self) -> (Vec<f64>, Vec<f64>) {
        self.raw
            .with::<PcolorImageArtist, _>(|image| (image.x.clone(), image.y.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{Normalize, colormaps};

    #[test]
    fn pixels_find_their_cells() {
        let edges = [0.0, 1.0, 3.0];
        assert_eq!(pixel_cells(&edges, 0.0, 3.0, 3), vec![1, 2, 2]);
        assert_eq!(pixel_cells(&edges, -1.0, 4.0, 5), vec![0, 1, 2, 2, 3]);
    }

    #[test]
    fn decreasing_edges_flip_the_data() {
        let data = ImageData::from(Grid::from_vec(1, 2, vec![1.0, 2.0]).unwrap());
        let mappable = ScalarMappable::new(colormaps::get("gray").unwrap());
        let image = PcolorImageArtist::new(data, mappable, vec![2.0, 1.0, 0.0], vec![0.0, 1.0]);
        assert_eq!(image.x, vec![0.0, 1.0, 2.0]);
        let ImageData::Scalar(field) = &image.data else {
            unreachable!("still scalar")
        };
        assert_eq!(field.values().as_slice(), &[2.0, 1.0]);
    }

    #[test]
    fn colours_pass_through_or_map() {
        let mut gray = ScalarMappable::with_norm(
            colormaps::get("gray").unwrap(),
            Box::new(Normalize::new(0.0, 1.0)),
        );
        let scalars = ImageData::from(Grid::from_vec(1, 2, vec![0.0, 1.0]).unwrap());
        assert_eq!(
            cell_colors(&scalars, &mut gray).unwrap().as_slice(),
            &[[0, 0, 0, 255], [255, 255, 255, 255]]
        );
        let rgb = ImageData::from(Grid::from_vec(1, 1, vec![[1.0, 0.5, f64::NAN]]).unwrap());
        assert_eq!(cell_colors(&rgb, &mut gray).unwrap().as_slice(), &[[0; 4]]);
        let bytes = ImageData::from(Grid::from_vec(1, 1, vec![[1_u8, 2, 3]]).unwrap());
        assert_eq!(
            cell_colors(&bytes, &mut gray).unwrap().as_slice(),
            &[[1, 2, 3, 255]]
        );
    }
}
