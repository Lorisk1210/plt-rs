//! `figures/cases_mpl_tests_backend_pgf.py`: the figures of matplotlib's
//! PGF backend tests, whose `.pgf` text the wide vector comparison checks.

use plt::artist::{ArtistHandle, Sketch};
use plt::axes::{FillBetweenOptions, HistPlotOptions, Mesh, PcolorOptions};
use plt::collections::CollectionProps;
use plt::figure::Figure;
use plt::lines::LineProps;
use plt::numeric::{Grid, linspace};
use plt::patches::PatchProps;
use plt::stats::Bins;
use plt::text::HAlign;

use super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The rcParams of `test_xelatex`, `test_pathclip` and `test_mixedmode`.
const SERIF: &[(&str, &str)] = &[("font.family", "serif"), ("pgf.rcfonts", "False")];

/// The first rcParam set of `test_rcupdate`.
const RCUPDATE_1: &[(&str, &str)] = &[
    ("font.family", "sans-serif"),
    ("font.size", "30"),
    ("figure.subplot.left", "0.2"),
    ("lines.markersize", "10"),
    ("pgf.rcfonts", "False"),
    ("pgf.texsystem", "xelatex"),
];

/// The second rcParam set of `test_rcupdate`.
const RCUPDATE_2: &[(&str, &str)] = &[
    ("font.family", "monospace"),
    ("font.size", "10"),
    ("figure.subplot.left", "0.1"),
    ("hatch.color", "orange"),
    ("hatch.linewidth", "2"),
    ("lines.markersize", "20"),
    ("pgf.rcfonts", "False"),
    ("pgf.texsystem", "pdflatex"),
    (
        "pgf.preamble",
        r"\usepackage[utf8x]{inputenc}\usepackage[T1]{fontenc}\usepackage{sfmath}",
    ),
];

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        FigureCase::new("mpl_tests_backend_pgf/xelatex", create_figure).with_rc(SERIF),
        FigureCase::new("mpl_tests_backend_pgf/rcupdate_1", create_figure).with_rc(RCUPDATE_1),
        FigureCase::new("mpl_tests_backend_pgf/rcupdate_2", create_figure).with_rc(RCUPDATE_2),
        FigureCase::new("mpl_tests_backend_pgf/pathclip", pathclip).with_rc(SERIF),
        FigureCase::new("mpl_tests_backend_pgf/mixedmode", mixedmode).with_rc(SERIF),
        // `test_sketch_params`: a sketched diagonal without ticks or frame.
        classic("mpl_tests_backend_pgf/sketch_params", |fig| {
            let ax = fig.subplot();
            ax.set_xticks(Vec::<f64>::new())
                .set_yticks(Vec::<f64>::new())
                .set_frame_on(false);
            ax.plot_y(vec![0.0, 1.0]).set_sketch_params(Some(Sketch {
                scale: 5.0,
                length: 30.0,
                randomness: 42.0,
            }));
            Ok(())
        })
        .sized(3.0, 3.0, 100.0),
    ]
}

/// The tests' `create_figure`: lines, markers, a hatched fill, a filled
/// polygon, unicode and math text, a sans-serif blue math label and a text
/// clipped by the axes.
fn create_figure(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 1.0, 15);
    let squares: Vec<f64> = x.iter().map(|v| v * v).collect();
    let falling: Vec<f64> = squares.iter().map(|v| 1.0 - v).collect();
    ax.plot_with(x.clone(), squares, LineProps::fmt("b-")?)?;
    ax.plot_with(x, falling, LineProps::fmt("g>")?)?;
    let hatched = FillBetweenOptions {
        props: CollectionProps::default()
            .hatch("//")
            .facecolor("lightgray")
            .edgecolor("red"),
        ..FillBetweenOptions::default()
    };
    ax.fill_between([0.0, 0.4], [0.4, 0.0], [0.0], hatched)?;
    ax.fill(
        [3.0, 3.0, 0.8, 0.8, 3.0],
        [2.0, -2.0, -2.0, 0.0, 2.0],
        PatchProps::default().facecolor("b"),
    )?;
    ax.plot_with([0.9], [0.5], LineProps::fmt("ro")?.markersize(3.0))?;
    ax.text(
        0.9,
        0.5,
        "unicode (ü, °, \u{a7}) and math ($\\mu_i = x_i^2$)",
    )
    .set_halign(HAlign::Right)
    .set_fontsize(20.0);
    ax.yaxis()
        .set_label_text("sans-serif, blue, $\\frac{\\sqrt{x}}{y^2}$..")
        .set_label_family(["sans-serif"])
        .set_label_color("blue");
    ax.text(
        1.0,
        1.0,
        "should be clipped as default clip_box is Axes bbox",
    )
    .set_fontsize(20.0)
    .set_clip_on(true);
    ax.set_xlim(0.0, 1.0)?;
    ax.set_ylim(0.0, 1.0)?;
    Ok(())
}

/// `test_pathclip`: a line to `1e100` clipped by its axes, and a log-scaled
/// histogram next to two points.
fn pathclip(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (left, right) = (&axs[(0, 0)], &axs[(0, 1)]);
    left.plot([0.0, 1e100], [0.0, 1e100])?;
    left.set_xlim(0.0, 1.0)?;
    left.set_ylim(0.0, 1.0)?;
    right.scatter(vec![0.0, 1.0], vec![1.0, 1.0])?;
    let options = HistPlotOptions {
        bins: Some(Bins::Count(20)),
        range: Some((-10.0, 10.0)),
        ..HistPlotOptions::default()
    };
    right.hist(&[SpecRng::new(19_680_801).normal(1000, 0.0, 1.0)], options)?;
    right.set_xscale("log")?;
    Ok(())
}

/// `test_mixedmode`: a rasterized `pcolor` of `x² + y²`.
fn mixedmode(fig: &Figure) -> CaseResult {
    let t = linspace(-1.0, 1.0, 40);
    let z = Grid::from_fn(40, 40, |r, c| t[c] * t[c] + t[r] * t[r]);
    fig.subplot()
        .pcolor(Mesh::new(z), PcolorOptions::default())?
        .set_rasterized(true);
    Ok(())
}
