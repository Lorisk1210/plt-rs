//! Images on non-uniform rectangular grids (matplotlib's `PcolorImage`,
//! made by `pcolorfast`).

use std::cell::RefCell;

use pltrs_core::{Affine2D, Bbox, Point};
use pltrs_render::RgbaImage;

use super::any::{ImageSettings, default_settings};
use super::artist::fit_axes_to;
use super::make::{RenderedImage, autoscale_unset, mapping_copy};
use super::normalize::normalize_in;
use super::resample::Resampling;
use super::shared::{
    axes_image_contains, draw_rendered_at_alpha, grid_cell, grid_extent, grid_limits, split_alpha,
};
use super::{Extent, ImageAlpha, ImageData, ImageError, InterpolationStage, MadeImage};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, LayoutContext, LimitPoints, RawHandle,
    TypedHandle,
};
use crate::axes::AxesState;
use crate::colors::{ColorizerOptions, ScalarMappable};
use crate::numeric::{Grid, linspace};
use crate::transforms::TransformContext;

/// State of an image whose cells have individual widths and heights.
#[derive(Clone, Debug)]
pub(crate) struct PcolorImageArtist {
    base: ArtistBase,
    data: ImageData,
    mappable: RefCell<ScalarMappable>,
    /// Increasing cell edges along x.
    x: Vec<f64>,
    /// Increasing cell edges along y.
    y: Vec<f64>,
    /// The resampling settings every image keeps, which drawing the cells
    /// does not read (matplotlib's `_ImageBase` attributes).
    resampling: Resampling,
    /// Whether data or colours would be resampled.
    stage: InterpolationStage,
    /// The extent set (matplotlib's `AxesImage._extent`), which drawing
    /// the cells does not read.
    extent: Option<Extent>,
}

pltrs_core::archive_struct!(PcolorImageArtist {
    base,
    data,
    mappable,
    x,
    y,
    resampling,
    stage,
    extent
});

impl PcolorImageArtist {
    /// An image with cell edges `x` and `y` (one more than the columns and
    /// rows of `data`); decreasing edges flip the data, as matplotlib does.
    pub(crate) fn new(data: ImageData, mappable: ScalarMappable, x: Vec<f64>, y: Vec<f64>) -> Self {
        let (data, x, y) = increasing(data, x, y);
        let mut base = ArtistBase::with_zorder(0.0);
        base.mouseover = true;
        let (resampling, stage) = default_settings(&crate::rc::params());
        Self {
            base,
            data,
            mappable: RefCell::new(mappable),
            x,
            y,
            resampling,
            stage,
            extent: None,
        }
    }

    /// matplotlib's `PcolorImage.make_image`: every pixel of the axes box,
    /// at `magnification` pixels per display unit, takes the colour of the
    /// cell containing its centre (transparent outside the grid).
    pub(super) fn render(
        &self,
        layout: LayoutContext<'_>,
        magnification: f64,
    ) -> Result<Option<RenderedImage>, ImageError> {
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
        let mut mappable = mapping_copy(&self.mappable);
        let colors = cell_colors(&self.data, &mut mappable);
        self.mappable.replace(mappable);
        let colors = colors?;
        let round = |v: f64| (v * magnification + 0.5) as i64;
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
            origin: Point::new(l as f64 / magnification, b as f64 / magnification),
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
            ImageData::Scalar(
                super::ScalarField::masked(values, &mask)
                    .expect("values and mask flip to one shape"),
            )
        }
        ImageData::Rgb(grid) => ImageData::Rgb(flip(&grid, x, y)),
        ImageData::Rgba(grid) => ImageData::Rgba(flip(&grid, x, y)),
        ImageData::RgbF32(grid) => ImageData::RgbF32(flip(&grid, x, y)),
        ImageData::RgbaF32(grid) => ImageData::RgbaF32(flip(&grid, x, y)),
        ImageData::RgbBytes(grid) => ImageData::RgbBytes(flip(&grid, x, y)),
        ImageData::RgbaBytes(grid) => ImageData::RgbaBytes(flip(&grid, x, y)),
    }
}

/// matplotlib's `to_rgba(A, bytes=True)`: mapped scalars, or colours
/// passed through (opaque when without alpha, transparent when NaN) and
/// scaled to bytes in their own precision.
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
    let unit_f32 = |pixel: [f32; 4]| {
        if pixel.iter().any(|v| v.is_nan()) {
            [0; 4]
        } else {
            pixel.map(|v| (v * 255.0) as u8)
        }
    };
    Ok(match data {
        ImageData::Scalar(field) => {
            let (rows, cols) = field.values().shape();
            let values = field.masked_values();
            let shape = (rows, cols);
            let normalized =
                normalize_in(mappable, &values, field.kind(), field.precision(), shape)?;
            let cmap = mappable.cmap();
            let colors = normalized
                .into_iter()
                .map(|n| cmap.lookup_bytes(n, None))
                .collect();
            Grid::from_vec(rows, cols, colors).expect("one colour per cell")
        }
        ImageData::Rgb(grid) => grid.map(|&[r, g, b]| unit([r, g, b, 1.0])),
        ImageData::Rgba(grid) => grid.map(|&pixel| unit(pixel)),
        ImageData::RgbF32(grid) => grid.map(|&[r, g, b]| unit_f32([r, g, b, 1.0])),
        ImageData::RgbaF32(grid) => grid.map(|&pixel| unit_f32(pixel)),
        ImageData::RgbBytes(grid) => grid.map(|&[r, g, b]| [r, g, b, u8::MAX]),
        ImageData::RgbaBytes(grid) => grid.clone(),
    })
}

impl ImageSettings for PcolorImageArtist {
    fn resampling(&self) -> &Resampling {
        &self.resampling
    }

    fn resampling_mut(&mut self) -> &mut Resampling {
        &mut self.resampling
    }

    fn stage(&self) -> InterpolationStage {
        self.stage
    }

    fn set_stage(&mut self, stage: InterpolationStage) {
        self.stage = stage;
    }

    /// A uniform opacity is drawn; one per pixel is not, as matplotlib's
    /// renderers draw only the scalar alpha of pcolor images.
    fn set_alpha(&mut self, alpha: Option<ImageAlpha>) {
        self.base.alpha = split_alpha(alpha).0;
    }
}

impl Artist for PcolorImageArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let magnification = ctx.renderer().image_magnification();
        let rendered = self.render(ctx.layout(), magnification);
        draw_rendered_at_alpha(ctx, &self.base, rendered);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        grid_extent(&self.base, ctx, &self.x, &self.y)
    }

    /// The corners of the grid (matplotlib's `_update_image_limits`).
    fn limit_points(&self, _axes: ArtistId, _ctx: &dyn TransformContext) -> Option<LimitPoints> {
        grid_limits(&self.x, &self.y)
    }

    /// matplotlib's `AxesImage.contains` with the grid as extent.
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        axes_image_contains(&self.base, ctx, point, || self.window_extent(ctx))
    }

    /// matplotlib's `PcolorImage.get_cursor_data`: the value of the cell
    /// under `at`.
    fn cursor_text(&self, ctx: &LayoutContext<'_>, at: Point) -> Option<String> {
        let element = grid_cell(&self.base, ctx, at, &self.x, &self.y)?;
        Some(self.data.cursor_text(element, &self.mappable.borrow()))
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

impl TypedHandle for PcolorImage {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<PcolorImageArtist>()
            .then(|| Self::from_raw(raw.clone()))
    }
}

super::impl_mapping_changes!(PcolorImage, PcolorImageArtist);
super::impl_color_mapped!(PcolorImageArtist, data, mappable);

impl PcolorImage {
    /// The image drawn into the axes box at `magnification` pixels per
    /// display unit, as the figure is now (matplotlib's `make_image`): its
    /// pixels, top row first, and the display position of their
    /// bottom-left corner, mapped to the display as they are (the
    /// identity); `None` outside an axes.
    ///
    /// # Errors
    ///
    /// When the image's colormap or norm cannot map its data.
    pub fn make_image(&self, magnification: f64) -> Result<Option<MadeImage>, ImageError> {
        let state = self.raw.figure_state().borrow();
        let layout = LayoutContext::new(&state);
        let rendered = self
            .raw
            .with::<PcolorImageArtist, _>(|image| image.render(layout, magnification))?;
        Ok(rendered.map(|image| MadeImage {
            pixels: image.pixels,
            origin: image.origin,
            transform: Some(Affine2D::IDENTITY),
        }))
    }

    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// An image without cells, colour-mapped as `colorizer` says, outside
    /// any figure until an axes adopts it (matplotlib's `PcolorImage(ax)`
    /// before `set_data` and `ax.add_image`; see
    /// [`Axes::adopt_artist`](crate::axes::Axes::adopt_artist)).
    ///
    /// ```
    /// use pltrs::colors::ColorizerOptions;
    /// use pltrs::image::PcolorImage;
    /// use pltrs::numeric::Grid;
    /// let image = PcolorImage::loose(ColorizerOptions::default())?;
    /// image.set_data(vec![0.0, 1.0, 3.0], vec![0.0, 2.0], Grid::filled(1, 2, 1.0))?;
    /// assert_eq!(image.edges(), (vec![0.0, 1.0, 3.0], vec![0.0, 2.0]));
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the colormap options are invalid.
    pub fn loose(colorizer: ColorizerOptions) -> Result<Self, ImageError> {
        let mappable = colorizer.into_mappable(&crate::rc::params())?;
        let artist = PcolorImageArtist::new(ImageData::empty(), mappable, vec![0.0], vec![0.0]);
        Ok(Self::from_raw(RawHandle::loose(Box::new(artist))))
    }

    /// The extent set, else the array's cells centred on their indices
    /// (matplotlib's `AxesImage.get_extent`, which `PcolorImage` keeps:
    /// it is not where the cells are drawn).
    ///
    /// ```
    /// use pltrs::colors::ColorizerOptions;
    /// use pltrs::image::{Extent, PcolorImage};
    /// use pltrs::numeric::Grid;
    /// let image = PcolorImage::loose(ColorizerOptions::default())?;
    /// image.set_data(vec![0.0, 1.0, 3.0], vec![0.0, 2.0], Grid::filled(1, 2, 1.0))?;
    /// assert_eq!(image.extent(), Extent::new(-0.5, 1.5, 0.5, -0.5));
    /// image.set_extent(Extent::new(0.0, 4.0, 0.0, 3.0));
    /// assert_eq!(image.extent(), Extent::new(0.0, 4.0, 0.0, 3.0));
    /// # Ok::<(), pltrs::image::ImageError>(())
    /// ```
    pub fn extent(&self) -> Extent {
        self.raw.with::<PcolorImageArtist, _>(|image| {
            image.extent.unwrap_or_else(|| {
                let (rows, cols) = image.data.shape();
                Extent::default_for(rows, cols, image.resampling.origin)
            })
        })
    }

    /// Sets the extent the image reports, fitting its axes to it as
    /// [`AxesImage::set_extent`](super::AxesImage::set_extent) does
    /// (matplotlib's `PcolorImage` inherits `set_extent`); the cells stay
    /// where their edges are.
    pub fn set_extent(&self, extent: Extent) -> &Self {
        self.raw
            .with_mut::<PcolorImageArtist, _>(|image| image.extent = Some(extent));
        fit_axes_to(&self.raw, extent);
        self
    }

    /// The increasing cell edges `(x, y)`.
    pub fn edges(&self) -> (Vec<f64>, Vec<f64>) {
        self.raw
            .with::<PcolorImageArtist, _>(|image| (image.x.clone(), image.y.clone()))
    }

    /// Replaces the cell edges and values: one edge more than the columns
    /// along `x` and than the rows along `y`; decreasing edges flip the
    /// data. Unset limits of the norm come from new scalar data
    /// (matplotlib's `PcolorImage.set_data`).
    ///
    /// ```
    /// use pltrs::axes::{MeshCoords, PcolorFast, PcolorfastOptions};
    /// use pltrs::numeric::Grid;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let coords = (MeshCoords::Vector(vec![0.0, 1.0, 3.0]), MeshCoords::Vector(vec![0.0, 2.0]));
    /// let values = Grid::filled(1, 2, 1.0);
    /// let PcolorFast::PcolorImage(image) = ax.pcolorfast(Some(coords), values, PcolorfastOptions::default())? else {
    ///     unreachable!("uneven edges make a pcolor image");
    /// };
    /// image.set_data(vec![3.0, 2.0, 0.0], vec![0.0, 1.0, 2.0], Grid::filled(2, 2, 5.0))?;
    /// assert_eq!(image.edges(), (vec![0.0, 2.0, 3.0], vec![0.0, 1.0, 2.0]));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the edges do not fit the data; then the image is unchanged.
    pub fn set_data(
        &self,
        x: Vec<f64>,
        y: Vec<f64>,
        data: impl Into<ImageData>,
    ) -> Result<&Self, ImageError> {
        let data = data.into().normalized();
        let (rows, cols) = data.shape();
        let edges = (y.len(), x.len());
        if edges != (rows + 1, cols + 1) {
            return Err(ImageError::ShapeMismatch {
                what: "cell edges",
                expected: (rows + 1, cols + 1),
                actual: edges,
            });
        }
        let (data, x, y) = increasing(data, x, y);
        self.raw.with_mut::<PcolorImageArtist, _>(|image| {
            autoscale_unset(image.mappable.get_mut(), &data);
            (image.data, image.x, image.y) = (data, x, y);
        });
        Ok(self)
    }
}

/// The data and edges with the edges increasing: decreasing edges are
/// reversed and the data flipped along them, as matplotlib does.
fn increasing(
    data: ImageData,
    mut x: Vec<f64>,
    mut y: Vec<f64>,
) -> (ImageData, Vec<f64>, Vec<f64>) {
    let flip_x = x.last() < x.first();
    let flip_y = y.last() < y.first();
    if flip_x {
        x.reverse();
    }
    if flip_y {
        y.reverse();
    }
    (flipped(data, flip_x, flip_y), x, y)
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
