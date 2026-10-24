//! Images read back from PNG files with `imread`: single-precision RGBA
//! colours and greyscale scalars.

use plt::axes::ImshowOptions;
use plt::colors::{Colorize, ColorizerOptions, colormaps};
use plt::figure::Figure;
use plt::image::{ImsaveOptions, Interpolation, imread_png, imsave_png};
use plt::lines::LineProps;
use plt::numeric::Grid;
use plt_codec::png::{BitDepth, ColorType, Encoder, ImageRef};

use crate::figures::CaseResult;

pub(super) fn imread_rgba_interpolation(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let rgba = Grid::from_fn(12, 16, |r, c| {
        [
            (37 * r + 11 * c) % 256,
            255 - 15 * c,
            20 * r,
            (40 + 13 * r + 9 * c) % 256,
        ]
        .map(|v| v as u8)
    });
    let image = imread_png(&imsave_png(rgba, ImsaveOptions::default())?)?;
    for (ax, interpolation) in axes
        .as_slice()
        .iter()
        .zip([Interpolation::Nearest, Interpolation::Bilinear])
    {
        ax.plot_with(
            [0.0, 15.0],
            [0.0, 11.0],
            LineProps::default().color("k").linewidth(5.0),
        )?;
        ax.imshow(
            image.clone(),
            ImshowOptions::default().interpolation(interpolation),
        )?;
    }
    Ok(())
}

pub(super) fn imread_gray_float32(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let levels: Vec<u8> = (0..10)
        .flat_map(|r| (0..12).map(move |c| ((19 * r + 23 * c) % 256) as u8))
        .collect();
    let png = Encoder::new().encode(ImageRef::new(
        12,
        10,
        ColorType::Gray,
        BitDepth::Eight,
        &levels,
    )?)?;
    let gray = imread_png(&png)?;
    let limited = ImshowOptions {
        colorizer: ColorizerOptions::default()
            .cmap(colormaps::get("gray")?)
            .limits(Some(0.1), Some(0.9)),
        ..ImshowOptions::default().interpolation(Interpolation::Bilinear)
    };
    axes[(0, 0)].imshow(gray.clone(), limited)?;
    axes[(0, 1)].imshow(
        gray,
        ImshowOptions::default().interpolation(Interpolation::Nearest),
    )?;
    Ok(())
}
