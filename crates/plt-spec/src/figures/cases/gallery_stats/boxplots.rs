//! The box plots of the section: custom fill colours, the many options of
//! `boxplot_demo` (but its comparison of distributions) and `boxplot`, box
//! plots beside violins and `bxp` on precomputed statistics.

use plt::axes::{
    Axes, BoxArtist, BoxStyle, BoxplotOptions, BxpOptions, GridOptions, ViolinDrawOptions,
    ViolinplotOptions,
};
use plt::figure::Share;
use plt::gridspec::SubplotOverrides;
use plt::numeric::Grid;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::rc::ColorOrAuto;
use plt::stats::{BoxStats, BoxplotOptions as StatsOptions, Whis, boxplot_stats};

use super::{SEED, columns, strings};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The font size of the grids' titles (`fs`).
const FS: f64 = 10.0;

/// Draws every box as a filled patch (`patch_artist=True`), with `patch`
/// overriding the box style (`boxprops`).
fn filled(patch: PatchProps) -> BxpOptions {
    BxpOptions {
        boxes: Some(BoxStyle::Patch(patch)),
        ..BxpOptions::default()
    }
}

/// Box plot options that only change how the boxes are drawn.
fn drawn(draw: BxpOptions) -> BoxplotOptions {
    BoxplotOptions {
        draw,
        ..BoxplotOptions::default()
    }
}

/// Box plot options with outliers styled by the format string `sym`.
pub(super) fn with_sym(sym: &str) -> BoxplotOptions {
    BoxplotOptions {
        sym: Some(sym.into()),
        ..BoxplotOptions::default()
    }
}

fn boxplot_color(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let fruit_weights = [
        g.normal(100, 130.0, 10.0),
        g.normal(100, 125.0, 20.0),
        g.normal(100, 120.0, 30.0),
    ];
    let ax = fig.subplot();
    ax.set_ylabel("fruit weight (g)");
    let options = BoxplotOptions {
        tick_labels: Some(strings(&["peaches", "oranges", "tomatoes"])),
        ..drawn(filled(PatchProps::default()))
    };
    let bplot = ax.boxplot(&fruit_weights, &options)?;
    for (patch, color) in bplot.boxes.iter().zip(["peachpuff", "orange", "tomato"]) {
        if let BoxArtist::Patch(patch) = patch {
            patch.set_facecolor(color);
        }
    }
    Ok(())
}

/// `boxplot_demo`'s data: a uniform spread, a constant centre and fliers
/// on both sides (`fake_data`).
fn fake_data(g: &mut SpecRng, center: f64) -> Vec<f64> {
    let mut data = g.uniform(50, 0.0, 100.0);
    data.extend([center; 25]);
    data.extend(g.uniform(10, 100.0, 200.0));
    data.extend(g.uniform(10, 0.0, -100.0));
    data
}

fn boxplot_demo_1(fig: &Figure) -> CaseResult {
    let data = [fake_data(&mut SpecRng::new(SEED), 50.0)];
    let axs = fig.subplots(2, 3)?;
    let horizontal = |whis: Option<Whis>| BoxplotOptions {
        whis,
        draw: BxpOptions {
            orientation: Orientation::Horizontal,
            ..BxpOptions::default()
        },
        ..with_sym("rs")
    };
    let notched = drawn(BxpOptions {
        shownotches: Some(true),
        ..BxpOptions::default()
    });
    let panels = vec![
        (BoxplotOptions::default(), "basic plot"),
        (notched, "notched plot"),
        (with_sym("gD"), "change outlier\npoint symbols"),
        (with_sym(""), "don't show\noutlier points"),
        (horizontal(None), "horizontal boxes"),
        (
            horizontal(Some(Whis::Factor(0.75))),
            "change whisker length",
        ),
    ];
    for (ax, (options, title)) in axs.as_slice().iter().zip(panels) {
        ax.boxplot(&data, &options)?;
        ax.set_title(title);
    }
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.08),
        right: Some(0.98),
        bottom: Some(0.05),
        top: Some(0.9),
        hspace: Some(0.4),
        wspace: Some(0.3),
    })?;
    Ok(())
}

fn boxplot_demo_2(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let data = fake_data(&mut g, 50.0);
    let d2 = fake_data(&mut g, 40.0);
    let every_other: Vec<f64> = d2.iter().copied().step_by(2).collect();
    fig.subplot()
        .boxplot(&[data, d2, every_other], &BoxplotOptions::default())?;
    Ok(())
}

fn boxplot_demo_4(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let inc = 0.1;
    let treatments = [
        g.normal(500, 0.0, 1.0),
        g.normal(500, 0.0, 1.0),
        g.normal(500, 0.0, 1.0 + inc),
        g.normal(500, 0.0, 1.0 + 2.0 * inc),
    ];
    let ax = fig.subplot();
    let options = BoxplotOptions {
        // `positions=np.arange(4) + 1` are integers, which label the ticks.
        tick_labels: Some(strings(&["1", "2", "3", "4"])),
        usermedians: Some(vec![None, None, Some(0.1), Some(0.2)]),
        conf_intervals: Some(vec![
            (None, None),
            (None, None),
            (Some(-0.25), Some(0.25)),
            (Some(-0.35), Some(0.50)),
        ]),
        draw: BxpOptions {
            positions: Some(vec![1.0, 2.0, 3.0, 4.0]),
            shownotches: Some(true),
            ..BxpOptions::default()
        },
        ..with_sym("k+")
    };
    let bp = ax.boxplot(&treatments, &options)?;
    ax.set_xlabel("treatment");
    ax.set_ylabel("response");
    for whisker in &bp.whiskers {
        whisker.set_color("k").set_linestyle(LineStyle::Solid);
    }
    for flier in &bp.fliers {
        flier.set_markersize(3.0);
    }
    Ok(())
}

fn boxplot_demo_5(fig: &Figure) -> CaseResult {
    let mut x = vec![-25.0];
    x.extend(linspace(-7.0, 7.0, 140));
    x.push(25.0);
    let options = drawn(BxpOptions {
        shownotches: Some(true),
        capwidths: vec![0.01, 0.2],
        ..BxpOptions::default()
    });
    fig.subplot().boxplot(&[x.clone(), x], &options)?;
    Ok(())
}

/// `boxplot`'s data: 37 lognormal draws in each of four columns.
fn lognormal_columns() -> Vec<Vec<f64>> {
    let draws: Vec<f64> = SpecRng::new(SEED)
        .normal(37 * 4, 1.5, 1.75)
        .into_iter()
        .map(f64::exp)
        .collect();
    columns(&draws, 4)
}

/// The 2×3 grid of `boxplot`, sharing a log y-axis without labels
/// (`log_grid`).
fn log_grid(fig: &Figure) -> Result<Grid<Axes>, Box<dyn std::error::Error>> {
    let axs = fig.subplots_shared(2, 3, Share::None, Share::All)?;
    for ax in axs.as_slice() {
        ax.set_yscale("log")?;
        ax.set_yticklabels(Vec::<String>::new())?;
    }
    fig.subplots_adjust(&SubplotOverrides {
        hspace: Some(0.4),
        ..SubplotOverrides::default()
    })?;
    Ok(axs)
}

/// Draws one box plot per panel of `axs` with its title.
fn panels(axs: &Grid<Axes>, data: &[Vec<f64>], panels: Vec<(BoxplotOptions, &str)>) -> CaseResult {
    for (ax, (options, title)) in axs.as_slice().iter().zip(panels) {
        ax.boxplot(data, &options)?;
        ax.set_title(title).set_fontsize(FS);
    }
    Ok(())
}

fn boxplot_1(fig: &Figure) -> CaseResult {
    let labelled = |draw: BxpOptions| BoxplotOptions {
        tick_labels: Some(strings(&["A", "B", "C", "D"])),
        ..drawn(draw)
    };
    let defaults = BxpOptions::default;
    let axs = log_grid(fig)?;
    panels(
        &axs,
        &lognormal_columns(),
        vec![
            (labelled(defaults()), "Default"),
            (
                labelled(BxpOptions {
                    showmeans: Some(true),
                    ..defaults()
                }),
                "showmeans=True",
            ),
            (
                labelled(BxpOptions {
                    showmeans: Some(true),
                    meanline: Some(true),
                    ..defaults()
                }),
                "showmeans=True,\nmeanline=True",
            ),
            (
                labelled(BxpOptions {
                    showbox: Some(false),
                    showcaps: Some(false),
                    ..defaults()
                }),
                "Tufte Style \n(showbox=False,\nshowcaps=False)",
            ),
            (
                labelled(BxpOptions {
                    shownotches: Some(true),
                    ..defaults()
                }),
                "notch=True,\nbootstrap=10000",
            ),
            (
                labelled(BxpOptions {
                    showfliers: Some(false),
                    ..defaults()
                }),
                "showfliers=False",
            ),
        ],
    )
}

fn boxplot_2(fig: &Figure) -> CaseResult {
    let boxprops = LineProps::default()
        .linestyle(LineStyle::Dashed)
        .linewidth(3.0)
        .color("darkgoldenrod");
    let flierprops = LineProps::default()
        .marker(Marker::Symbol(MarkerSymbol::Circle))
        .markerfacecolor(ColorOrAuto::Color("green".into()))
        .markersize(12.0)
        .markeredgecolor(ColorOrAuto::Color("none".into()));
    let medianprops = LineProps::default()
        .linestyle(LineStyle::DashDot)
        .linewidth(2.5)
        .color("firebrick");
    let meanpointprops = LineProps::default()
        .marker(Marker::Symbol(MarkerSymbol::Diamond))
        .markeredgecolor(ColorOrAuto::Color("black".into()))
        .markerfacecolor(ColorOrAuto::Color("firebrick".into()));
    let meanlineprops = LineProps::default()
        .linestyle(LineStyle::Dashed)
        .linewidth(2.5)
        .color("purple");
    let whis = |whis: Whis| BoxplotOptions {
        whis: Some(whis),
        ..BoxplotOptions::default()
    };
    let mean = |meanprops: LineProps, meanline: bool| {
        drawn(BxpOptions {
            meanprops,
            meanline: Some(meanline),
            showmeans: Some(true),
            ..BxpOptions::default()
        })
    };
    let axs = log_grid(fig)?;
    panels(
        &axs,
        &lognormal_columns(),
        vec![
            (
                drawn(BxpOptions {
                    boxes: Some(BoxStyle::Line(boxprops)),
                    ..BxpOptions::default()
                }),
                "Custom boxprops",
            ),
            (
                drawn(BxpOptions {
                    flierprops,
                    medianprops,
                    ..BxpOptions::default()
                }),
                "Custom medianprops\nand flierprops",
            ),
            (whis(Whis::Percentiles(0.0, 100.0)), "whis=(0, 100)"),
            (mean(meanpointprops, false), "Custom mean\nas point"),
            (mean(meanlineprops, true), "Custom mean\nas line"),
            (
                whis(Whis::Percentiles(15.0, 85.0)),
                "whis=[15, 85]\n#percentiles",
            ),
        ],
    )?;
    fig.suptitle("I never said they'd be pretty");
    Ok(())
}

fn boxplot_vs_violin(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let all_data: Vec<Vec<f64>> = (6..10)
        .map(|std| g.normal(100, 0.0, f64::from(std)))
        .collect();
    let axs = fig.subplots(1, 2)?;
    let violins = ViolinplotOptions {
        draw: ViolinDrawOptions {
            showmeans: false,
            showmedians: true,
            ..ViolinDrawOptions::default()
        },
        ..ViolinplotOptions::default()
    };
    axs[(0, 0)].violinplot(&all_data, &violins)?;
    axs[(0, 0)].set_title("Violin plot");
    axs[(0, 1)].boxplot(&all_data, &BoxplotOptions::default())?;
    axs[(0, 1)].set_title("Box plot");
    for ax in axs.as_slice() {
        ax.yaxis().grid(true, &GridOptions::default());
        ax.set_xticks([1.0, 2.0, 3.0, 4.0]);
        ax.set_xticklabels(["x1", "x2", "x3", "x4"])?;
        ax.set_xlabel("Four separate samples");
        ax.set_ylabel("Observed values");
    }
    Ok(())
}

/// Bisque boxes (`patch_artist=True, boxprops=BISQUE`).
fn bisque() -> BxpOptions {
    filled(PatchProps::default().facecolor("bisque"))
}

fn bxp_1(fig: &Figure) -> CaseResult {
    let data = columns(&SpecRng::new(SEED).normal(60, 0.0, 1.0), 3);
    let labels = strings(&["A", "B", "C"]);
    let axs = fig.subplots(1, 2)?;
    let options = BoxplotOptions {
        tick_labels: Some(labels.clone()),
        ..drawn(bisque())
    };
    axs[(0, 0)].boxplot(&data, &options)?;
    let stats = boxplot_stats(
        &data,
        &StatsOptions {
            labels: Some(labels),
            ..StatsOptions::default()
        },
    )?;
    axs[(0, 1)].bxp(&stats, &bisque())?;
    Ok(())
}

/// Hand-written statistics of a box centred on zero (`bxp_2`).
fn centred_box(label: &str, half_box: f64, fliers: Vec<f64>) -> BoxStats {
    BoxStats {
        label: Some(label.into()),
        med: 0.0,
        q1: -half_box,
        q3: half_box,
        whislo: -half_box - 1.0,
        whishi: half_box + 1.0,
        fliers,
        ..BoxStats::default()
    }
}

fn bxp_2(fig: &Figure) -> CaseResult {
    let stats = [
        centred_box("A", 1.0, vec![-4.0, -3.0, 3.0, 4.0]),
        centred_box("B", 2.0, Vec::new()),
        centred_box("C", 3.0, Vec::new()),
    ];
    fig.subplot().bxp(&stats, &bisque())?;
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_stats/boxplot_color", boxplot_color),
    FigureCase::new("gallery_stats/boxplot_demo_1", boxplot_demo_1),
    FigureCase::new("gallery_stats/boxplot_demo_2", boxplot_demo_2),
    FigureCase::new("gallery_stats/boxplot_demo_4", boxplot_demo_4),
    FigureCase::new("gallery_stats/boxplot_demo_5", boxplot_demo_5),
    FigureCase::new("gallery_stats/boxplot_1", boxplot_1).sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_stats/boxplot_2", boxplot_2).sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_stats/boxplot_vs_violin", boxplot_vs_violin).sized(9.0, 4.0, 100.0),
    FigureCase::new("gallery_stats/bxp_1", bxp_1),
    FigureCase::new("gallery_stats/bxp_2", bxp_2),
];
