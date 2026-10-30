//! The raster canvas a figure keeps between draws (matplotlib's
//! `FigureCanvasAgg` and its `RendererAgg`): drawing, its pixels, and
//! blitting through saved regions.

use pltrs::artist::RawHandle;
use pltrs::axes::Axes;
use pltrs::figure::{BufferRegion, CanvasError, Figure, PixelBox};
use pltrs::transforms::{Bbox, Point};

use super::super::lines::markevery_error;
use crate::binding::prelude::*;

native_value!(BufferRegion);

/// A box `(x0, y0, x1, y1)` in display pixels.
type Extents = (f64, f64, f64, f64);

/// The box of `extents`.
fn bbox((x0, y0, x1, y1): Extents) -> Bbox {
    Bbox::from_extents(x0, y0, x1, y1)
}

/// The pixel box `(x0, y0, x1, y1)` from the canvas's top-left corner.
fn pixel_extents(pixels: PixelBox) -> (i64, i64, i64, i64) {
    (pixels.x0, pixels.y0, pixels.x1, pixels.y1)
}

/// RGBA rows from the top as a `height × width × 4` array.
fn rgba_array(width: usize, height: usize, data: Vec<u8>) -> PyResult<NdArray<u8>> {
    NdArray::with_shape(data, vec![height, width, 4])
}

/// Draws the whole figure onto its canvas, leaving out animated artists
/// (`FigureCanvasAgg.draw`); a line whose `markevery` does not fit it
/// raises matplotlib's `ValueError`.
fn draw(figure: Figure) -> PyResult<()> {
    match figure.canvas().draw() {
        Err(CanvasError::Draw(problems)) => Err(markevery_error(&figure, &problems)
            .unwrap_or_else(|| CanvasError::Draw(problems).into())),
        drawn => Ok(drawn?),
    }
}

/// The canvas's pixels: straight-alpha RGBA rows from the top, as a
/// `height × width × 4` array (`buffer_rgba`).
fn buffer_rgba(figure: Figure) -> PyResult<NdArray<u8>> {
    let image = figure.canvas().buffer_rgba()?;
    let (width, height) = (image.width(), image.height());
    rgba_array(width, height, image.into_data())
}

/// Saves the canvas's pixels inside the display box `extents`
/// (`copy_from_bbox`).
fn copy_from_bbox(figure: Figure, extents: Extents) -> PyResult<BufferRegion> {
    Ok(figure.canvas().copy_from_bbox(&bbox(extents))?)
}

/// Puts a saved region back, its part inside `extents` (canvas pixels
/// from the top-left corner) with its corner moved to `xy`
/// (`restore_region(region, bbox, xy)`).
fn restore_region(
    figure: Figure,
    region: BufferRegion,
    extents: Option<Extents>,
    xy: Option<(f64, f64)>,
) -> PyResult<()> {
    let xy = xy.map(|(x, y)| Point::new(x, y));
    Ok(figure
        .canvas()
        .restore_region_with(&region, extents.map(bbox), xy)?)
}

/// The rectangle a region saved, in canvas pixels from the top-left
/// corner (`BufferRegion.get_extents`).
fn region_extents(region: BufferRegion) -> PyResult<(i64, i64, i64, i64)> {
    Ok(pixel_extents(region.extents()))
}

/// Draws one artist of the figure onto its canvas, over what it holds
/// (`Figure.draw_artist`).
fn draw_artist(figure: Figure, artist: RawHandle) -> PyResult<()> {
    Ok(figure.draw_artist(&artist)?)
}

/// Draws the axes onto its figure's canvas without its x and y axes and
/// titles, its animated artists left out (`Axes.redraw_in_frame`).
fn redraw_in_frame(axes: Axes) -> PyResult<()> {
    Ok(axes.redraw_in_frame()?)
}

/// Marks the part of the canvas inside `extents` (all of it with
/// `None`) as changed for what shows the figure (`blit`).
fn blit(figure: Figure, extents: Option<Extents>) -> PyResult<()> {
    let bbox = extents.map(bbox);
    Ok(figure.canvas().blit(bbox.as_ref())?)
}

/// The canvas's physical pixels per logical pixel.
fn get_device_pixel_ratio(figure: Figure) -> PyResult<f64> {
    Ok(figure.canvas().device_pixel_ratio())
}

/// Sets the canvas's physical pixels per logical pixel, scaling the
/// figure's resolution; whether it changed.
fn set_device_pixel_ratio(figure: Figure, ratio: f64) -> PyResult<bool> {
    Ok(figure.canvas().set_device_pixel_ratio(ratio)?)
}

/// The canvas's size in whole pixels (`get_width_height(physical=True)`).
fn width_height(figure: Figure) -> PyResult<(u32, u32)> {
    Ok(figure.width_height())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("draw", draw)?;
    module.add("buffer_rgba", buffer_rgba)?;
    module.add("copy_from_bbox", copy_from_bbox)?;
    module.add("restore_region", restore_region)?;
    module.add("region_extents", region_extents)?;
    module.add("draw_artist", draw_artist)?;
    module.add("redraw_in_frame", redraw_in_frame)?;
    module.add("blit", blit)?;
    module.add("get_device_pixel_ratio", get_device_pixel_ratio)?;
    module.add("set_device_pixel_ratio", set_device_pixel_ratio)?;
    module.add("width_height", width_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::artist::ArtistHandle;

    #[test]
    fn the_canvas_draws_and_restores_regions() {
        let figure = Figure::with_size(2.0, 1.0, 50.0);
        assert!(buffer_rgba(figure.clone()).is_err(), "not drawn yet");
        draw(figure.clone()).unwrap();
        let pixels = buffer_rgba(figure.clone()).unwrap();
        assert_eq!(pixels.shape, [50, 100, 4]);
        let region = copy_from_bbox(figure.clone(), (0.0, 0.0, 10.0, 10.0)).unwrap();
        assert_eq!(region_extents(region.clone()).unwrap(), (0, 40, 10, 50));
        restore_region(figure.clone(), region.clone(), None, None).unwrap();
        let part = Some((0.0, 40.0, 5.0, 45.0));
        restore_region(figure.clone(), region, part, Some((20.0, 20.0))).unwrap();
        blit(figure.clone(), None).unwrap();
        assert_eq!(width_height(figure.clone()).unwrap(), (100, 50));
        assert!(set_device_pixel_ratio(figure.clone(), 2.0).unwrap());
        assert_eq!(get_device_pixel_ratio(figure.clone()).unwrap(), 2.0);
        assert!(set_device_pixel_ratio(figure, f64::NAN).is_err());
    }

    #[test]
    fn single_artists_draw_over_the_canvas() {
        let figure = Figure::with_size(2.0, 1.0, 50.0);
        let ax = figure.subplot();
        draw(figure.clone()).unwrap();
        let before = buffer_rgba(figure.clone()).unwrap().data;
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        draw_artist(figure.clone(), line.raw().clone()).unwrap();
        assert_ne!(buffer_rgba(figure.clone()).unwrap().data, before);
        redraw_in_frame(ax).unwrap();
        let other = Figure::new();
        assert!(
            draw_artist(other, line.raw().clone()).is_err(),
            "another figure's"
        );
    }
}
