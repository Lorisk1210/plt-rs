//! `image_annotated_heatmap`: labelled heatmaps with a colorbar and
//! values written into their cells.

use std::error::Error;
use std::rc::Rc;

use plt::axes::{Axes, GridOptions, ImshowOptions, TickLabelProps, TickLevel, TickParams};
use plt::colorbar::{ColorbarAxes, ColorbarFormat, ColorbarOptions, ColorbarTicks};
use plt::colors::{
    BoundaryNorm, Colorize, ColorizerOptions, Extend, Norm, Normalize, Normalized, colormaps,
};
use plt::figure::Figure;
use plt::fonts::FontWeight;
use plt::image::{ImageData, ScalarField};
use plt::layout::TightLayout;
use plt::lines::LineStyle;
use plt::numeric::{Grid, arange, linspace};
use plt::prelude::ArtistHandle;
use plt::text::{HAlign, TextRotationMode, VAlign};
use plt::ticker::pyfmt;
use plt::ticker::{Formatter, FuncFormatter, StrMethodFormatter, TickContext};

use super::data::{SEED, extremes};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

const VEGETABLES: [&str; 7] = [
    "cucumber",
    "tomato",
    "lettuce",
    "asparagus",
    "potato",
    "wheat",
    "barley",
];

const FARMERS: [&str; 7] = [
    "Farmer Joe",
    "Upland Bros.",
    "Smith Gardening",
    "Agrifun",
    "Organiculture",
    "BioGoods Ltd.",
    "Cornylee Corp.",
];

const HARVEST: [[f64; 7]; 7] = [
    [0.8, 2.4, 2.5, 3.9, 0.0, 4.0, 0.0],
    [2.4, 0.0, 4.0, 1.0, 2.7, 0.0, 0.0],
    [1.1, 2.4, 0.8, 4.3, 1.9, 4.4, 0.0],
    [0.6, 0.0, 0.3, 0.0, 3.1, 0.0, 0.0],
    [0.7, 1.7, 0.6, 2.6, 2.2, 6.2, 0.0],
    [1.3, 1.2, 0.0, 0.0, 0.0, 3.2, 5.1],
    [0.1, 2.0, 0.0, 1.4, 0.0, 1.9, 6.3],
];

fn harvest() -> Grid<f64> {
    Grid::from_fn(7, 7, |r, c| HARVEST[r][c])
}

/// Ticks at the cell centres labelled `labels`, rotated by `rotation`
/// degrees and aligned to their ticks (`set_xticks(..., labels=...,
/// rotation=..., rotation_mode="xtick")`).
fn label_columns(ax: &Axes, labels: &[String], rotation: f64) -> CaseResult {
    let xaxis = ax.xaxis();
    xaxis.set_ticks(arange(0.0, labels.len() as f64, 1.0));
    xaxis.set_ticklabels(labels.iter().cloned())?;
    xaxis.set_ticklabel_props(
        TickLevel::Major,
        TickLabelProps {
            rotation: Some(rotation),
            rotation_mode: Some(TextRotationMode::XTick),
            ..TickLabelProps::default()
        },
    );
    Ok(())
}

/// Ticks at the row centres labelled `labels`.
fn label_rows(ax: &Axes, labels: &[String]) -> CaseResult {
    let yaxis = ax.yaxis();
    yaxis.set_ticks(arange(0.0, labels.len() as f64, 1.0));
    yaxis.set_ticklabels(labels.iter().cloned())?;
    Ok(())
}

fn strings<S: ToString>(labels: &[S]) -> Vec<String> {
    labels.iter().map(ToString::to_string).collect()
}

/// A text centred on cell `(i, j)`.
fn cell_text(ax: &Axes, i: usize, j: usize, text: String) -> plt::text::Text {
    let text = ax.text(j as f64, i as f64, text);
    text.set_halign(HAlign::Center).set_valign(VAlign::Center);
    text
}

fn image_annotated_heatmap_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.imshow(harvest(), ImshowOptions::default())?;
    label_columns(&ax, &strings(&FARMERS), 45.0)?;
    label_rows(&ax, &strings(&VEGETABLES))?;
    for (i, row) in HARVEST.iter().enumerate() {
        for (j, &value) in row.iter().enumerate() {
            cell_text(&ax, i, j, pyfmt::repr(value)).set_color("w");
        }
    }
    ax.set_title("Harvest of local farmers (in tons/year)");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// `np.arange(n + 1) - .5`: the edges of `n` cells.
fn cell_edges(n: usize) -> Vec<f64> {
    arange(0.0, n as f64 + 1.0, 1.0)
        .into_iter()
        .map(|v| v - 0.5)
        .collect()
}

/// `heatmap(data, row_labels, col_labels, ax, cbar_kw, cbarlabel,
/// **kwargs)`: the image with a labelled colorbar, column labels on top,
/// no spines and white lines between the cells.
fn heatmap(
    ax: &Axes,
    data: impl Into<ImageData>,
    rows: &[String],
    cols: &[String],
    colorizer: ColorizerOptions,
    colorbar: ColorbarOptions,
    cbarlabel: &str,
) -> CaseResult {
    let im = ax.imshow(data, ImshowOptions::default().colorizer(colorizer))?;
    let cbar = ax.figure().colorbar(
        &im,
        ColorbarOptions {
            axes: ColorbarAxes::Parent(ax.clone()),
            ..colorbar
        },
    )?;
    cbar.ax().set_ylabel(cbarlabel);
    cbar.ax()
        .yaxis()
        .set_label_rotation(-90.0)
        .set_label_valign(VAlign::Bottom);
    label_columns(ax, cols, -30.0)?;
    label_rows(ax, rows)?;
    ax.tick_params(&TickParams {
        top: Some(true),
        bottom: Some(false),
        labeltop: Some(true),
        labelbottom: Some(false),
        ..TickParams::default()
    });
    for spine in ax.spines().iter() {
        spine.set_visible(false);
    }
    ax.xaxis().set_minor_ticks(cell_edges(cols.len()));
    ax.yaxis().set_minor_ticks(cell_edges(rows.len()));
    ax.grid_with(
        true,
        GridOptions {
            which: TickLevel::Minor,
            color: Some("w".into()),
            linestyle: Some(LineStyle::Solid),
            linewidth: Some(3.0),
            ..GridOptions::default()
        },
    );
    ax.tick_params(&TickParams {
        which: TickLevel::Minor,
        bottom: Some(false),
        left: Some(false),
        ..TickParams::default()
    });
    Ok(())
}

/// What `im.norm(x)` returns: a fraction, or a bin index for boundary
/// norms.
fn normalized(norm: &mut dyn Norm, x: f64) -> Result<f64, Box<dyn Error>> {
    Ok(match norm.normalize(&[x])?[0] {
        Normalized::Fraction(f) => f,
        Normalized::Index(i) => i as f64,
        Normalized::Masked => f64::NAN,
    })
}

/// `qrates[::-1][norm(x)]`: the quality letter of the bin of `x`; values
/// below the bins get Python's index -1, the last letter.
fn rating(bins: &BoundaryNorm, x: f64) -> String {
    const LETTERS: [char; 7] = ['G', 'F', 'E', 'D', 'C', 'B', 'A'];
    let index = normalized(&mut bins.clone(), x).unwrap_or(f64::NAN);
    let at = if index < 0.0 {
        LETTERS.len() - 1
    } else {
        index as usize
    };
    LETTERS[at].to_string()
}

/// What a cell format returns.
type Formatted = Result<String, Box<dyn Error>>;

/// How `annotate_heatmap` writes the cells.
struct Annotation<'a> {
    /// `valfmt(x, None)`.
    format: &'a dyn Fn(f64) -> Formatted,
    /// The colours below and above the threshold.
    textcolors: (&'a str, &'a str),
    /// The threshold in data units (`None` = half the normalised maximum).
    threshold: Option<f64>,
    /// Font size in points.
    size: Option<f64>,
    /// Bold text.
    bold: bool,
}

impl<'a> Annotation<'a> {
    /// `annotate_heatmap`'s defaults: black below half the normalised
    /// maximum, white above, in the default font.
    fn new(format: &'a dyn Fn(f64) -> Formatted) -> Self {
        Self {
            format,
            textcolors: ("black", "white"),
            threshold: None,
            size: None,
            bold: false,
        }
    }
}

/// `annotate_heatmap(im, ...)`: every value of `data` written into its
/// cell, coloured by whether `norm` puts it above the threshold.
fn annotate_heatmap(
    ax: &Axes,
    data: &Grid<f64>,
    norm: &mut dyn Norm,
    annotation: &Annotation<'_>,
) -> CaseResult {
    let threshold = if let Some(threshold) = annotation.threshold {
        normalized(norm, threshold)?
    } else {
        normalized(norm, extremes(data.as_slice()).1)? / 2.0
    };
    for i in 0..data.rows() {
        for j in 0..data.cols() {
            let value = data[(i, j)];
            let color = if normalized(norm, value)? > threshold {
                annotation.textcolors.1
            } else {
                annotation.textcolors.0
            };
            let text = cell_text(ax, i, j, (annotation.format)(value)?);
            text.set_color(color);
            if let Some(size) = annotation.size {
                text.set_fontsize(size);
            }
            if annotation.bold {
                text.set_weight(FontWeight::BOLD);
            }
        }
    }
    Ok(())
}

/// A `str.format` template applied as `StrMethodFormatter(fmt)(x, None)`.
fn str_method(fmt: &str) -> Result<impl Fn(f64) -> Formatted, Box<dyn Error>> {
    let formatter = StrMethodFormatter::new(fmt)?;
    Ok(move |x: f64| Ok(formatter.format_data(x, &TickContext::default())?))
}

/// The normalisation of an image of `data` whose limits come from it.
fn autoscaled(data: &Grid<f64>) -> Normalize {
    let (lo, hi) = extremes(data.as_slice());
    Normalize::new(lo, hi)
}

fn image_annotated_heatmap_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let data = harvest();
    heatmap(
        &ax,
        data.clone(),
        &strings(&VEGETABLES),
        &strings(&FARMERS),
        ColorizerOptions::default().cmap(colormaps::get("YlGn")?),
        ColorbarOptions::default(),
        "harvest [t/year]",
    )?;
    let format = str_method("{x:.1f} t")?;
    annotate_heatmap(
        &ax,
        &data,
        &mut autoscaled(&data),
        &Annotation::new(&format),
    )?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// numpy's `corrcoef` of the rows of `x`.
fn corrcoef(x: &Grid<f64>) -> Grid<f64> {
    let n = x.cols();
    let centred = Grid::from_fn(x.rows(), n, |r, c| {
        let mean = x.row(r).iter().sum::<f64>() / n as f64;
        x[(r, c)] - mean
    });
    let factor = 1.0 / (n - 1) as f64;
    let cov = Grid::from_fn(x.rows(), x.rows(), |a, b| {
        let dot: f64 = centred
            .row(a)
            .iter()
            .zip(centred.row(b))
            .map(|(u, v)| u * v)
            .sum();
        dot * factor
    });
    Grid::from_fn(x.rows(), x.rows(), |a, b| {
        (cov[(a, b)] / cov[(a, a)].sqrt() / cov[(b, b)].sqrt()).clamp(-1.0, 1.0)
    })
}

/// The harvest in `Wistia`, one decimal in small print.
fn harvest_panel(ax: &Axes) -> CaseResult {
    let data = harvest();
    heatmap(
        ax,
        data.clone(),
        &strings(&VEGETABLES),
        &strings(&FARMERS),
        ColorizerOptions::default().cmap(colormaps::get("Wistia")?),
        ColorbarOptions::default(),
        "harvest [t/year]",
    )?;
    let one_decimal = str_method("{x:.1f}")?;
    let annotation = Annotation {
        size: Some(7.0),
        ..Annotation::new(&one_decimal)
    };
    annotate_heatmap(ax, &data, &mut autoscaled(&data), &annotation)
}

/// Random weekly book sales from 0 up, red below 20.
fn sales_panel(ax: &Axes, rng: &mut SpecRng) -> CaseResult {
    let sold = Grid::from_vec(7, 7, rng.integers(49, 2, 100))?;
    let books: Vec<String> = (1..8).map(|i| format!("Book {i}")).collect();
    let stores: Vec<String> = "ABCDEFG".chars().map(|c| format!("Store {c}")).collect();
    heatmap(
        ax,
        ScalarField::integers(&sold),
        &books,
        &stores,
        ColorizerOptions::default()
            .cmap(colormaps::get("magma_r")?)
            .limits(Some(0.0), None),
        ColorbarOptions::default(),
        "weekly sold copies",
    )?;
    let sold = sold.map(|&v| v as f64);
    // `"{x:d}"` of the integers.
    let integer = |x: f64| -> Formatted { Ok((x as i64).to_string()) };
    let annotation = Annotation {
        textcolors: ("red", "white"),
        threshold: Some(20.0),
        size: Some(7.0),
        ..Annotation::new(&integer)
    };
    annotate_heatmap(
        ax,
        &sold,
        &mut Normalize::new(0.0, extremes(sold.as_slice()).1),
        &annotation,
    )
}

/// Random quality ratings binned into seven letters.
fn quality_panel(ax: &Axes, rng: &mut SpecRng) -> CaseResult {
    let quality = Grid::from_vec(6, 6, rng.normal(36, 0.0, 1.0))?;
    let products: Vec<String> = (1..7).map(|i| format!("Prod. {}", i * 10)).collect();
    let cycles: Vec<String> = (1..7).map(|i| format!("Cycle {i}")).collect();
    let bins = BoundaryNorm::new(linspace(-3.5, 3.5, 8), 7, Extend::Neither)?;
    let tick_rating = {
        let bins = bins.clone();
        FuncFormatter::new(move |x, _| rating(&bins, x))
    };
    heatmap(
        ax,
        quality.clone(),
        &products,
        &cycles,
        ColorizerOptions::default()
            .cmap(colormaps::get("PiYG")?.resampled(7)?)
            .norm(bins.clone()),
        ColorbarOptions {
            ticks: Some(ColorbarTicks::Values(arange(-3.0, 4.0, 1.0))),
            format: Some(ColorbarFormat::Formatter(Rc::new(tick_rating))),
            ..ColorbarOptions::default()
        },
        "Quality Rating",
    )?;
    let letter = |x: f64| -> Formatted { Ok(rating(&bins, x)) };
    let annotation = Annotation {
        textcolors: ("red", "black"),
        threshold: Some(-1.0),
        size: Some(9.0),
        bold: true,
        ..Annotation::new(&letter)
    };
    annotate_heatmap(ax, &quality, &mut bins.clone(), &annotation)
}

/// The correlations between the vegetables' harvests.
fn correlation_panel(ax: &Axes) -> CaseResult {
    let corr = corrcoef(&harvest());
    let vegetables = strings(&VEGETABLES);
    heatmap(
        ax,
        corr.clone(),
        &vegetables,
        &vegetables,
        ColorizerOptions::default()
            .cmap(colormaps::get("PuOr")?)
            .limits(Some(-1.0), Some(1.0)),
        ColorbarOptions::default(),
        "correlation coeff.",
    )?;
    let correlation =
        |x: f64| -> Formatted { Ok(pyfmt::fixed(x, 2).replace("0.", ".").replace("1.00", "")) };
    let annotation = Annotation {
        size: Some(7.0),
        ..Annotation::new(&correlation)
    };
    annotate_heatmap(ax, &corr, &mut Normalize::new(-1.0, 1.0), &annotation)
}

fn image_annotated_heatmap_3(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let axs = fig.subplots(2, 2)?;
    harvest_panel(&axs[(0, 0)])?;
    sales_panel(&axs[(0, 1)], &mut rng)?;
    quality_panel(&axs[(1, 0)], &mut rng)?;
    correlation_panel(&axs[(1, 1)])?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_images/image_annotated_heatmap_1",
        image_annotated_heatmap_1,
    ),
    FigureCase::new(
        "gallery_images/image_annotated_heatmap_2",
        image_annotated_heatmap_2,
    ),
    FigureCase::new(
        "gallery_images/image_annotated_heatmap_3",
        image_annotated_heatmap_3,
    )
    .sized(8.0, 6.0, 100.0),
];
