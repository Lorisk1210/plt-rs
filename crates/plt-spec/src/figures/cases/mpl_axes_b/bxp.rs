//! `bxp` from precomputed statistics on a log axis (`_bxp_test_helper`)
//! and a box plot with a wide median.

use plt::axes::{BoxProps, BoxplotOptions, BxpOptions};
use plt::figure::Figure;
use plt::lines::{LineProps, LineStyle};
use plt::markers::Marker;
use plt::orientation::Orientation;
use plt::patches::PatchProps;
use plt::rc::ColorOrAuto;
use plt::stats::{self, BoxStats, Whis};
use plt_render::CapStyle;

use super::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The helper sets `boxplot.boxprops.linewidth` to `lines.linewidth`
/// unless boxes are patches: the baselines predate `bxp` honouring it.
const LINE_BOXES: &[(&str, &str)] = &[("boxplot.boxprops.linewidth", "1.5")];

/// A `bxp` test saved at 40 dpi with outline boxes.
const fn lines_case(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build)
        .sized(6.4, 4.8, 40.0)
        .with_rc(LINE_BOXES)
}

/// Statistics of four log-normal samples of 37 values (the columns of a
/// 37 × 4 array).
fn log_normal_stats(whis: Whis) -> Result<Vec<BoxStats>, stats::StatsError> {
    let samples: Vec<f64> = SpecRng::new(937)
        .normal(37 * 4, 1.25, 1.0)
        .into_iter()
        .map(f64::exp)
        .collect();
    let columns: Vec<Vec<f64>> = (0..4)
        .map(|j| samples.iter().skip(j).step_by(4).copied().collect())
        .collect();
    let options = stats::BoxplotStatsOptions {
        whis,
        ..stats::BoxplotStatsOptions::default()
    };
    stats::boxplot_stats(&columns, options)
}

/// `_bxp_test_helper`: `bxp(stats, **options)` on a log value axis.
fn draw(fig: &Figure, stats: &[BoxStats], options: &BxpOptions) -> CaseResult {
    let ax = fig.subplot();
    match options.orientation {
        Orientation::Vertical => ax.set_yscale("log")?,
        Orientation::Horizontal => ax.set_xscale("log")?,
    };
    ax.bxp(stats, options.clone())?;
    Ok(())
}

/// The helper with default statistics, the text removed.
fn textless(fig: &Figure, options: &BxpOptions) -> CaseResult {
    draw(fig, &log_normal_stats(Whis::default())?, options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// Default statistics labelled `A` to `D`.
fn labelled(fig: &Figure, orientation: Orientation) -> CaseResult {
    let mut stats = log_normal_stats(Whis::default())?;
    for (s, label) in stats.iter_mut().zip(["A", "B", "C", "D"]) {
        s.label = Some(label.to_owned());
    }
    let options = BxpOptions {
        orientation,
        ..BxpOptions::default()
    };
    draw(fig, &stats, &options)
}

/// Line properties of the given style, colour and width.
fn styled(linestyle: LineStyle, color: &str, linewidth: f64) -> LineProps {
    LineProps::default()
        .linestyle(linestyle)
        .color(color)
        .linewidth(linewidth)
}

/// Diamond markers filled with `color` and no line.
fn diamonds(color: &str) -> Result<LineProps, plt::markers::MarkerError> {
    Ok(LineProps::default()
        .linestyle(LineStyle::None)
        .marker("d".parse::<Marker>()?)
        .markerfacecolor(ColorOrAuto::Color(color.into())))
}

/// `boxplot(np.arange(3), medianprops=...)`.
fn wide_median(fig: &Figure, medianprops: LineProps) -> CaseResult {
    let options = BoxplotOptions {
        draw: BxpOptions {
            medianprops,
            ..BxpOptions::default()
        },
        ..BoxplotOptions::default()
    };
    fig.subplot().boxplot(&[[0.0, 1.0, 2.0]], options)?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    lines_case("mpl_axes_b/bxp_baseline", |fig| {
        draw(
            fig,
            &log_normal_stats(Whis::default())?,
            &BxpOptions::default(),
        )
    }),
    lines_case("mpl_axes_b/bxp_rangewhis", |fig| {
        draw(
            fig,
            &log_normal_stats(Whis::Percentiles(0.0, 100.0))?,
            &BxpOptions::default(),
        )
    }),
    lines_case("mpl_axes_b/bxp_percentilewhis", |fig| {
        draw(
            fig,
            &log_normal_stats(Whis::Percentiles(5.0, 95.0))?,
            &BxpOptions::default(),
        )
    }),
    lines_case("mpl_axes_b/bxp_with_xlabels", |fig| {
        labelled(fig, Orientation::Vertical)
    }),
    lines_case("mpl_axes_b/bxp_horizontal", |fig| {
        let options = BxpOptions {
            orientation: Orientation::Horizontal,
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_with_ylabels", |fig| {
        labelled(fig, Orientation::Horizontal)
    }),
    FigureCase::new("mpl_axes_b/bxp_patchartist", |fig| {
        let options = BxpOptions {
            boxes: Some(BoxProps::Patch(PatchProps::default())),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    })
    .sized(6.4, 4.8, 40.0),
    FigureCase::new("mpl_axes_b/bxp_custompatchartist", |fig| {
        let boxes = PatchProps::default()
            .facecolor("yellow")
            .edgecolor("green")
            .linestyle(LineStyle::Dotted);
        let options = BxpOptions {
            boxes: Some(BoxProps::Patch(boxes)),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_customoutlier", |fig| {
        let options = BxpOptions {
            flierprops: diamonds("g")?,
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_showcustommean", |fig| {
        let options = BxpOptions {
            showmeans: Some(true),
            meanprops: diamonds("green")?,
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_custombox", |fig| {
        let options = BxpOptions {
            boxes: Some(BoxProps::Line(styled(LineStyle::Dashed, "b", 3.0))),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_custommedian", |fig| {
        let options = BxpOptions {
            medianprops: styled(LineStyle::Dashed, "b", 3.0),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_customcap", |fig| {
        let options = BxpOptions {
            capprops: styled(LineStyle::Dashed, "g", 3.0),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_customwhisker", |fig| {
        let options = BxpOptions {
            whiskerprops: styled(LineStyle::Solid, "m", 3.0),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_shownotches", |fig| {
        let options = BxpOptions {
            shownotches: Some(true),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_nocaps", |fig| {
        let options = BxpOptions {
            showcaps: Some(false),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_nobox", |fig| {
        let options = BxpOptions {
            showbox: Some(false),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_no_flier_stats", |fig| {
        let mut stats = log_normal_stats(Whis::default())?;
        for s in &mut stats {
            s.fliers.clear();
        }
        let options = BxpOptions {
            showfliers: Some(false),
            ..BxpOptions::default()
        };
        draw(fig, &stats, &options)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    lines_case("mpl_axes_b/bxp_showmean", |fig| {
        let options = BxpOptions {
            showmeans: Some(true),
            meanline: Some(false),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_showmeanasline", |fig| {
        let options = BxpOptions {
            showmeans: Some(true),
            meanline: Some(true),
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    lines_case("mpl_axes_b/bxp_scalarwidth", |fig| {
        let options = BxpOptions {
            widths: vec![0.25],
            ..BxpOptions::default()
        };
        textless(fig, &options)
    }),
    classic("mpl_axes_b/boxplot_median_bound_by_box", |fig| {
        wide_median(fig, LineProps::default().linewidth(12.0))
    }),
    classic("mpl_axes_b/boxplot_median_bound_by_box_ref", |fig| {
        let butt = LineProps::default()
            .linewidth(12.0)
            .solid_capstyle(CapStyle::Butt);
        wide_median(fig, butt)
    }),
];
