//! `image_antialiasing`: down- and up-sampled images with every
//! interpolation stage, one case per figure.

use plt::axes::{Axes, ImshowOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{Colorize, ColorizerOptions, Colormap, Extend, colormaps};
use plt::figure::Figure;
use plt::fonts::{FontSize, RelativeSize};
use plt::image::{AxesImage, Interpolation, InterpolationStage};
use plt::layout::LayoutEngine;
use plt::numeric::Grid;

use super::data::{SEED, chirp, uniform_grid};
use crate::figures::{CaseResult, FigureCase};

/// Side of the large test pattern.
const N: usize = 450;

/// `_alarge()`: alternating rows on the left third, a chirp with two flat
/// discs in its upper half elsewhere.
fn alarge() -> Grid<f64> {
    let (r, a) = chirp(N, 0.5);
    Grid::from_fn(N, N, |row, col| {
        if col < N / 3 {
            if row % 2 == 0 { -1.0 } else { 1.0 }
        } else if row < N / 2 && r[(row, col)] < 0.3 {
            1.0
        } else if row < N / 2 && r[(row, col)] < 0.4 {
            -1.0
        } else {
            a[(row, col)]
        }
    })
}

/// `_asmall()`: 4 × 4 uniform samples.
fn asmall() -> Grid<f64> {
    uniform_grid(SEED + 9, 4, 4)
}

/// `imshow(data, cmap=cmap, interpolation=..., interpolation_stage=...)`.
fn show(
    ax: &Axes,
    data: Grid<f64>,
    cmap: Colormap,
    interpolation: Option<Interpolation>,
    stage: Option<InterpolationStage>,
) -> Result<AxesImage, Box<dyn std::error::Error>> {
    let options = ImshowOptions {
        interpolation,
        interpolation_stage: stage,
        ..ImshowOptions::default()
    }
    .cmap(cmap);
    Ok(ax.imshow(data, options)?)
}

/// A title in the medium font size.
fn medium_title(ax: &Axes, title: &str) {
    ax.set_title(title)
        .set_fontsize(FontSize::Relative(RelativeSize::Medium));
}

/// A colorbar for `im` beside all of `axs`, extended both ways.
fn shared_colorbar(fig: &Figure, im: &AxesImage, axs: &[Axes], shrink: f64) -> CaseResult {
    fig.colorbar(
        im,
        ColorbarOptions {
            axes: ColorbarAxes::Parents(axs.to_vec()),
            extend: Some(Extend::Both),
            shrink,
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

fn image_antialiasing_1(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    show(
        &axs[(0, 0)],
        alarge(),
        colormaps::get("RdBu_r")?,
        None,
        None,
    )?;
    medium_title(&axs[(0, 0)], "(450, 450) Down-sampled");
    show(
        &axs[(0, 1)],
        asmall(),
        colormaps::get("viridis")?,
        None,
        None,
    )?;
    medium_title(&axs[(0, 1)], "(4, 4) Up-sampled");
    Ok(())
}

fn image_antialiasing_2(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::compressed()))?;
    let ax = fig.subplot();
    show(
        &ax,
        alarge(),
        colormaps::get("RdBu_r")?,
        Some(Interpolation::Nearest),
        None,
    )?;
    ax.set_xlim(100.0, 200.0);
    ax.set_ylim(275.0, 175.0);
    ax.set_title("Zoom");
    Ok(())
}

/// Two panels resampling the large pattern with `interpolation` in data
/// and in RGBA space.
fn stages(fig: &Figure, interpolation: Interpolation) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::compressed()))?;
    let axs = fig.subplots(1, 2)?;
    for (ax, (stage, space)) in axs.as_slice().iter().zip([
        (InterpolationStage::Data, "data"),
        (InterpolationStage::Rgba, "rgba"),
    ]) {
        show(
            ax,
            alarge(),
            colormaps::get("RdBu_r")?,
            Some(interpolation),
            Some(stage),
        )?;
        ax.set_title(format!(
            "interpolation='{}'\nstage='{space}'",
            interpolation.name()
        ));
    }
    Ok(())
}

fn image_antialiasing_3(fig: &Figure) -> CaseResult {
    stages(fig, Interpolation::Nearest)
}

fn image_antialiasing_4(fig: &Figure) -> CaseResult {
    stages(fig, Interpolation::Hanning)
}

/// The large pattern slightly magnified with `interpolation`.
fn upsampled(fig: &Figure, interpolation: Interpolation) -> CaseResult {
    let ax = fig.subplot();
    show(
        &ax,
        alarge(),
        colormaps::get("grey")?,
        Some(interpolation),
        None,
    )?;
    ax.set_title(format!(
        "up-sampled by factor a 1.17, interpolation='{}'",
        interpolation.name()
    ));
    Ok(())
}

fn image_antialiasing_5(fig: &Figure) -> CaseResult {
    upsampled(fig, Interpolation::Nearest)
}

fn image_antialiasing_6(fig: &Figure) -> CaseResult {
    upsampled(fig, Interpolation::Auto)
}

fn image_antialiasing_7(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let axs = fig.subplots(1, 2)?;
    for (ax, interpolation) in axs
        .as_slice()
        .iter()
        .zip([Interpolation::Hanning, Interpolation::Lanczos])
    {
        show(
            ax,
            alarge(),
            colormaps::get("gray")?,
            Some(interpolation),
            None,
        )?;
        ax.set_title(format!("interpolation='{}'", interpolation.name()));
    }
    Ok(())
}

fn image_antialiasing_8(fig: &Figure) -> CaseResult {
    let a = alarge().map(|v| v + 1.0);
    let cmap = colormaps::get("RdBu_r")?
        .with_under("yellow")?
        .with_over("limegreen")?;
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let axs = fig.subplots(1, 3)?;
    let mut last = None;
    for (i, (ax, (interpolation, (stage, space)))) in axs
        .as_slice()
        .iter()
        .zip([
            (Interpolation::Hanning, (InterpolationStage::Data, "data")),
            (Interpolation::Nearest, (InterpolationStage::Data, "data")),
            (Interpolation::Hanning, (InterpolationStage::Rgba, "rgba")),
        ])
        .enumerate()
    {
        let options = ImshowOptions {
            colorizer: ColorizerOptions::default()
                .cmap(cmap.clone())
                .limits(Some(0.0), Some(2.0)),
            interpolation_stage: Some(stage),
            ..ImshowOptions::default()
        }
        .interpolation(interpolation);
        last = Some(ax.imshow(a.clone(), options)?);
        let mut title = format!("interpolation='{}'\nstage='{space}'", interpolation.name());
        if i == 2 {
            title.push_str("\nDefault");
        }
        medium_title(ax, &title);
    }
    let im = last.ok_or("three images were drawn")?;
    shared_colorbar(fig, &im, axs.as_slice(), 0.8)
}

fn image_antialiasing_9(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::compressed()))?;
    let axs = fig.subplots(1, 2)?;
    show(
        &axs[(0, 0)],
        asmall(),
        colormaps::get("viridis")?,
        None,
        None,
    )?;
    axs[(0, 0)].set_title("interpolation='auto'\nstage='auto'");
    show(
        &axs[(0, 1)],
        asmall(),
        colormaps::get("viridis")?,
        Some(Interpolation::Nearest),
        Some(InterpolationStage::Data),
    )?;
    axs[(0, 1)].set_title("interpolation='nearest'\nstage='data'");
    Ok(())
}

fn image_antialiasing_10(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::compressed()))?;
    let axs = fig.subplots(1, 2)?;
    let im = show(
        &axs[(0, 0)],
        asmall(),
        colormaps::get("viridis")?,
        Some(Interpolation::Sinc),
        Some(InterpolationStage::Data),
    )?;
    axs[(0, 0)].set_title("interpolation='sinc'\nstage='data'\n(default for upsampling)");
    show(
        &axs[(0, 1)],
        asmall(),
        colormaps::get("viridis")?,
        Some(Interpolation::Sinc),
        Some(InterpolationStage::Rgba),
    )?;
    axs[(0, 1)].set_title("interpolation='sinc'\nstage='rgba'");
    shared_colorbar(fig, &im, axs.as_slice(), 0.7)
}

fn image_antialiasing_11(fig: &Figure) -> CaseResult {
    let corner = alarge();
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    show(
        &ax,
        Grid::from_fn(400, 400, |r, c| corner[(r, c)]),
        colormaps::get("RdBu_r")?,
        Some(Interpolation::Nearest),
        None,
    )?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_images/image_antialiasing_1", image_antialiasing_1)
        .sized(4.0, 2.0, 100.0),
    FigureCase::new("gallery_images/image_antialiasing_2", image_antialiasing_2)
        .sized(4.0, 4.0, 100.0),
    FigureCase::new("gallery_images/image_antialiasing_3", image_antialiasing_3)
        .sized(5.0, 2.7, 100.0),
    FigureCase::new("gallery_images/image_antialiasing_4", image_antialiasing_4)
        .sized(5.0, 2.7, 100.0),
    FigureCase::new("gallery_images/image_antialiasing_5", image_antialiasing_5)
        .sized(6.8, 6.8, 100.0),
    FigureCase::new("gallery_images/image_antialiasing_6", image_antialiasing_6)
        .sized(6.8, 6.8, 100.0),
    FigureCase::new("gallery_images/image_antialiasing_7", image_antialiasing_7)
        .sized(7.0, 4.0, 100.0),
    FigureCase::new("gallery_images/image_antialiasing_8", image_antialiasing_8)
        .sized(7.0, 3.0, 100.0),
    FigureCase::new("gallery_images/image_antialiasing_9", image_antialiasing_9)
        .sized(6.5, 4.0, 100.0),
    FigureCase::new(
        "gallery_images/image_antialiasing_10",
        image_antialiasing_10,
    )
    .sized(6.5, 4.0, 100.0),
    FigureCase::new(
        "gallery_images/image_antialiasing_11",
        image_antialiasing_11,
    )
    .sized(2.0, 2.0, 100.0),
];
