//! The image tutorial: a photograph (here a synthetic grey picture of its
//! size) shown as it is, its red channel as a pseudocolor image under
//! other colormaps, with a colorbar, as a histogram and with clipped
//! limits, and a thumbnail of it under three interpolations.

use plt::axes::{HistPlotOptions, ImshowOptions};
use plt::colorbar::ColorbarOptions;
use plt::colors::colormaps;
use plt::image::{ColorizerOptions, Interpolation, ScalarField};
use plt::numeric::Grid;
use plt::prelude::*;
use plt::stats::{Bins, HistOptions};

use crate::figures::{CaseResult, FigureCase};

/// The size of the photograph, and of its thumbnail.
const PHOTO: (usize, usize) = (500, 375);
const THUMBNAIL: (usize, usize) = (64, 48);

/// A `width × height` grey picture in place of `stinkbug.png`
/// (`_stinkbug`): a dark oval bug with rings on its back on a ground lit
/// from the left, with a fine diagonal grain, the red channel a little
/// brighter and the blue one a little darker. Smaller sizes draw the same
/// picture (`img.thumbnail`).
fn stinkbug((width, height): (usize, usize)) -> Grid<[u8; 3]> {
    Grid::from_fn(height, width, |row, col| {
        let u = (col as f64 + 0.5) / width as f64;
        let v = (row as f64 + 0.5) / height as f64;
        let grain = 12.0 * ((37.0 * u + 23.0 * v) % 1.0);
        let ground = 170.0 - 60.0 * u + 30.0 * v + grain;
        let (across, along) = ((u - 0.5) / 0.3, (v - 0.55) / 0.22);
        let body = across * across + along * along;
        let bug = 25.0 + 60.0 * body + 30.0 * ((6.0 * body) % 1.0) + grain;
        let grey = if body < 1.0 { bug } else { ground };
        [grey + 6.0, grey, grey - 8.0].map(|channel| channel.clamp(0.0, 255.0) as u8)
    })
}

/// The red channel of the photograph (`lum_img`).
fn luminosity() -> Grid<u8> {
    stinkbug(PHOTO).map(|&[red, _, _]| red)
}

/// The red channel as a pseudocolor image.
fn pseudocolor() -> ScalarField {
    ScalarField::integers(&luminosity().map(|&v| i64::from(v)))
}

fn images_1(fig: &Figure) -> CaseResult {
    fig.subplot()
        .imshow(stinkbug(PHOTO), ImshowOptions::default())?;
    Ok(())
}

fn images_2(fig: &Figure) -> CaseResult {
    fig.subplot()
        .imshow(pseudocolor(), ImshowOptions::default())?;
    Ok(())
}

fn images_3(fig: &Figure) -> CaseResult {
    let hot = ImshowOptions::default().cmap(colormaps::get("hot")?);
    fig.subplot().imshow(pseudocolor(), hot)?;
    Ok(())
}

fn images_4(fig: &Figure) -> CaseResult {
    let imgplot = fig
        .subplot()
        .imshow(pseudocolor(), ImshowOptions::default())?;
    imgplot.set_cmap(colormaps::get("nipy_spectral")?);
    Ok(())
}

fn images_5(fig: &Figure) -> CaseResult {
    let imgplot = fig
        .subplot()
        .imshow(pseudocolor(), ImshowOptions::default())?;
    fig.colorbar(&imgplot, ColorbarOptions::default())?;
    Ok(())
}

fn images_6(fig: &Figure) -> CaseResult {
    let black = HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Edges((0..256).map(f64::from).collect())),
            ..HistOptions::default()
        },
        props: PatchProps::default().facecolor("k").edgecolor("k"),
        ..HistPlotOptions::default()
    };
    fig.subplot().hist(&[luminosity().into_vec()], black)?;
    Ok(())
}

fn images_7(fig: &Figure) -> CaseResult {
    let clipped =
        ImshowOptions::default().colors(ColorizerOptions::default().limits(Some(0.0), Some(175.0)));
    fig.subplot().imshow(pseudocolor(), clipped)?;
    Ok(())
}

fn images_8(fig: &Figure) -> CaseResult {
    let imgplot = fig
        .subplot()
        .imshow(pseudocolor(), ImshowOptions::default())?;
    imgplot.set_clim(Some(0.0), Some(175.0));
    Ok(())
}

fn images_9(fig: &Figure) -> CaseResult {
    fig.subplot()
        .imshow(stinkbug(THUMBNAIL), ImshowOptions::default())?;
    Ok(())
}

fn images_10(fig: &Figure) -> CaseResult {
    let bilinear = ImshowOptions::default().interpolation(Interpolation::Bilinear);
    fig.subplot().imshow(stinkbug(THUMBNAIL), bilinear)?;
    Ok(())
}

fn images_11(fig: &Figure) -> CaseResult {
    let bicubic = ImshowOptions::default().interpolation(Interpolation::Bicubic);
    fig.subplot().imshow(stinkbug(THUMBNAIL), bicubic)?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_tutorials/images_1", images_1),
    FigureCase::new("gallery_tutorials/images_2", images_2),
    FigureCase::new("gallery_tutorials/images_3", images_3),
    FigureCase::new("gallery_tutorials/images_4", images_4),
    FigureCase::new("gallery_tutorials/images_5", images_5),
    FigureCase::new("gallery_tutorials/images_6", images_6),
    FigureCase::new("gallery_tutorials/images_7", images_7),
    FigureCase::new("gallery_tutorials/images_8", images_8),
    FigureCase::new("gallery_tutorials/images_9", images_9),
    FigureCase::new("gallery_tutorials/images_10", images_10),
    FigureCase::new("gallery_tutorials/images_11", images_11),
];
