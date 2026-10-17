//! `figures/cases_legend.py`.

use plt::axes::{
    Axes, BarOptions, ErrorbarOptions, FillBetweenOptions, HistPlotOptions, MappedColors,
    ScatterColors, ScatterOptions, StackplotOptions, StairsOptions, StemOptions,
};
use plt::collections::{CollectionProps, EdgeColors};
use plt::colors::{Color, colormaps};
use plt::fonts::RelativeSize;
use plt::gridspec::SubplotOverrides;
use plt::legend::{LegendAlignment, LegendMode, LegendOptions};
use plt::lines::StepWhere;
use plt::prelude::*;
use plt::stats::{Bins, HistOptions, HistType};

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `T = np.linspace(0, 10, 40)`.
fn t() -> Vec<f64> {
    linspace(0.0, 10.0, 40)
}

/// `values` mapped through `f`.
fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}

/// A line labelled `label`.
fn labelled(label: &str) -> LineProps {
    LineProps::default().label(label)
}

/// `_three_lines`: sine, cosine and their product.
fn three_lines(ax: &Axes) -> CaseResult {
    let t = t();
    ax.plot_with(&t, map(&t, f64::sin), labelled("sine"))?;
    ax.plot_with(&t, map(&t, f64::cos), labelled("cosine"))?;
    ax.plot_with(&t, map(&t, |v| v.sin() * v.cos()), labelled("product"))?;
    Ok(())
}

/// A 5 × 4 inch case.
const fn small(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build).sized(5.0, 4.0, 100.0)
}

/// A 5 × 4 inch case of the three lines with a legend made with `options`.
macro_rules! three_lines_case {
    ($name:literal, $options:expr) => {
        small($name, |fig| {
            let ax = fig.subplot();
            three_lines(&ax)?;
            ax.legend($options)?;
            Ok(())
        })
    };
}

const LOCATIONS: [&str; 10] = [
    "upper right",
    "upper left",
    "lower left",
    "lower right",
    "right",
    "center left",
    "center right",
    "lower center",
    "upper center",
    "center",
];

pub(crate) const CASES: &[FigureCase] = &[
    // --- placement ---------------------------------------------------------
    FigureCase::new("legend/default_best", |fig| {
        let ax = fig.subplot();
        three_lines(&ax)?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    }),
    small("legend/best_avoids_crowded_data", |fig| {
        let ax = fig.subplot();
        let x = linspace(0.0, 1.0, 30);
        ax.plot_with(
            &x,
            map(&x, |v| 1.0 - v * 0.2),
            LineProps::fmt("o")?.label("top band"),
        )?;
        ax.plot_with(
            &x,
            map(&x, |v| v * 0.1),
            LineProps::fmt("s")?.label("bottom band"),
        )?;
        ax.plot_with(vec![0.02; 30], &x, LineProps::fmt("^")?.label("left edge"))?;
        ax.legend(LegendOptions::default().loc("best"))?;
        Ok(())
    }),
    FigureCase::new("legend/all_location_strings", |fig| {
        let axes = fig.subplots(2, 5)?;
        for (ax, loc) in axes.as_slice().iter().zip(LOCATIONS) {
            ax.plot_with([0.0, 1.0], [0.0, 1.0], labelled(loc))?;
            ax.legend(LegendOptions::default().loc(loc).fontsize(7.0))?;
            ax.xaxis().set_ticks(Vec::<f64>::new());
            ax.yaxis().set_ticks(Vec::<f64>::new());
        }
        Ok(())
    })
    .sized(10.0, 4.5, 100.0),
    FigureCase::new("legend/numeric_location_codes", |fig| {
        let axes = fig.subplots(2, 2)?;
        for (code, ax) in [1, 3, 6, 10].into_iter().zip(axes.as_slice()) {
            ax.plot_with([0.0, 1.0], [1.0, 0.0], labelled(&format!("loc={code}")))?;
            ax.legend(LegendOptions::default().loc(code))?;
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("legend/bbox_to_anchor_outside_right", |fig| {
        let ax = fig.subplot();
        fig.subplots_adjust(&SubplotOverrides {
            right: Some(0.7),
            ..SubplotOverrides::default()
        })?;
        three_lines(&ax)?;
        ax.legend(
            LegendOptions::default()
                .loc("upper left")
                .bbox_to_anchor((1.02, 1.0))
                .borderaxespad(0.0),
        )?;
        Ok(())
    }),
    FigureCase::new("legend/bbox_to_anchor_below_mode_expand", |fig| {
        let ax = fig.subplot();
        fig.subplots_adjust(&SubplotOverrides {
            bottom: Some(0.25),
            ..SubplotOverrides::default()
        })?;
        three_lines(&ax)?;
        ax.legend(
            LegendOptions::default()
                .loc("upper left")
                .bbox_to_anchor((0.0, -0.22, 1.0, 0.1))
                .mode(LegendMode::Expand)
                .ncols(3),
        )?;
        Ok(())
    }),
    FigureCase::new("legend/figure_legend", |fig| {
        let axes = fig.subplots(1, 2)?;
        let t = t();
        axes[(0, 0)].plot_with(&t, map(&t, f64::sin), labelled("left sine"))?;
        axes[(0, 1)].plot_with(
            &t,
            map(&t, f64::cos),
            LineProps::fmt("C1")?.label("right cosine"),
        )?;
        fig.legend(LegendOptions::default().loc("upper center").ncols(2))?;
        Ok(())
    }),
    small("legend/figure_legend_explicit_handles", |fig| {
        let ax = fig.subplot();
        let t = t();
        let a = ax.plot(&t, map(&t, f64::sin))?;
        let b = ax.plot(&t, map(&t, f64::cos))?;
        fig.legend(
            LegendOptions::default()
                .handles([&a, &b])
                .labels(["first", "second"])
                .loc("lower right"),
        )?;
        Ok(())
    }),
    // --- layout of entries -------------------------------------------------
    small("legend/ncols", |fig| {
        let ax = fig.subplot();
        let t = t();
        for i in 0..6 {
            let phase = f64::from(i);
            ax.plot_with(
                &t,
                map(&t, |v| (v + phase).sin()),
                labelled(&format!("phase {i}")),
            )?;
        }
        ax.legend(LegendOptions::default().ncols(3).loc("lower center"))?;
        Ok(())
    }),
    three_lines_case!(
        "legend/title_and_fontsizes",
        LegendOptions::default()
            .title("Functions")
            .title_fontsize(13.0)
            .fontsize(RelativeSize::Small)
    ),
    three_lines_case!(
        "legend/alignment_left_title",
        LegendOptions::default()
            .title("A rather long legend title")
            .alignment(LegendAlignment::Left)
    ),
    three_lines_case!(
        "legend/handlelength_spacing_pads",
        LegendOptions::default()
            .handlelength(4.0)
            .handleheight(1.5)
            .handletextpad(1.5)
            .labelspacing(1.2)
            .borderpad(1.0)
            .borderaxespad(2.0)
            .columnspacing(3.0)
    ),
    small("legend/markerscale_numpoints", |fig| {
        let ax = fig.subplot();
        let t = t();
        let dots = LineProps::fmt("o-")?.markersize(4.0).label("dots");
        ax.plot_with(&t, map(&t, f64::sin), dots)?;
        let squares = LineProps::fmt("s--")?.markersize(4.0).label("squares");
        ax.plot_with(&t, map(&t, f64::cos), squares)?;
        ax.legend(LegendOptions::default().markerscale(2.5).numpoints(3_usize))?;
        Ok(())
    }),
    small("legend/scatterpoints_and_yoffsets", |fig| {
        let ax = fig.subplot();
        let x = SpecRng::new(300).uniform(30, 0.0, 1.0);
        let y = SpecRng::new(301).uniform(30, 0.0, 1.0);
        let sized = ScatterOptions {
            sizes: Some(linspace(10.0, 200.0, 30)),
            ..ScatterOptions::default()
        };
        ax.scatter_with(&x, &y, sized)?.set_label("sized");
        let triangles = ScatterOptions {
            marker: Some("^".parse()?),
            ..ScatterOptions::default()
        };
        ax.scatter_with(&y, &x, triangles)?.set_label("triangles");
        ax.legend(
            LegendOptions::default()
                .scatterpoints(3_usize)
                .scatteryoffsets(vec![0.2, 0.5, 0.8]),
        )?;
        Ok(())
    }),
    three_lines_case!(
        "legend/reverse_order",
        LegendOptions::default().reverse(true)
    ),
    small("legend/nolegend_and_underscore_labels", |fig| {
        let ax = fig.subplot();
        let t = t();
        ax.plot_with(&t, map(&t, f64::sin), labelled("shown"))?;
        ax.plot_with(&t, map(&t, f64::cos), labelled("_nolegend_"))?;
        ax.plot_with(&t, map(&t, |v| -v.sin()), labelled("_hidden"))?;
        ax.plot(&t, map(&t, |v| -v.cos()))?;
        ax.plot_with(&t, map(&t, |v| 0.5 * v.sin()), labelled("also shown"))?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    }),
    three_lines_case!(
        "legend/labelcolor_linecolor",
        LegendOptions::default().labelcolor("linecolor")
    ),
    // --- frame styling -----------------------------------------------------
    three_lines_case!(
        "legend/frameon_false",
        LegendOptions::default().frameon(false)
    ),
    three_lines_case!(
        "legend/square_frame_with_shadow",
        LegendOptions::default().fancybox(false).shadow(true)
    ),
    small("legend/framealpha_facecolor_edgecolor", |fig| {
        let ax = fig.subplot();
        let t = t();
        let band = FillBetweenOptions {
            props: CollectionProps::default().color("tab:green"),
            ..FillBetweenOptions::default()
        };
        ax.fill_between(&t, [-1.0], [1.0], band)?;
        ax.plot_with(&t, map(&t, f64::sin), LineProps::fmt("k")?.label("sine"))?;
        ax.legend(
            LegendOptions::default()
                .framealpha(0.3)
                .facecolor("yellow")
                .edgecolor("red")
                .loc("center"),
        )?;
        Ok(())
    }),
    // --- handles for artist types ------------------------------------------
    small("legend/handles_lines_and_markers", |fig| {
        let ax = fig.subplot();
        let t = t();
        let dashed = LineProps::fmt("--")?.linewidth(3.0).label("dashed thick");
        ax.plot_with(&t, map(&t, f64::sin), dashed)?;
        ax.plot_with(
            &t,
            map(&t, f64::cos),
            LineProps::fmt("o")?.label("markers only"),
        )?;
        let every = LineProps::fmt("s-")?
            .markevery(Some(MarkEvery::Every { start: 0, step: 5 }))
            .label("line + markers");
        ax.plot_with(&t, map(&t, |v| -v.sin()), every)?;
        let half = LineProps::fmt("D")?
            .fillstyle(FillStyle::Left)
            .markersize(8.0)
            .label("half filled");
        ax.plot_with(&t, map(&t, |v| -v.cos()), half)?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    }),
    small("legend/handles_patches_and_bars", |fig| {
        let ax = fig.subplot();
        let bars = BarOptions {
            label: Some("bars".into()),
            ..BarOptions::default()
        };
        ax.bar([0.0, 1.0, 2.0], [3.0, 2.0, 4.0], bars)?;
        let hatched = BarOptions {
            hatch: vec!["//".into()],
            edgecolor: vec![Color::from("k")],
            color: vec![Color::from("C1")],
            label: Some("hatched".into()),
            ..BarOptions::default()
        };
        ax.bar([3.0, 4.0], [1.0, 2.0], hatched)?;
        ax.axvspan(4.5, 5.5).set_color("0.8").set_label("span");
        ax.legend(LegendOptions::default())?;
        Ok(())
    }),
    FigureCase::new("legend/handles_fill_between_and_stackplot", |fig| {
        let axes = fig.subplots(1, 2)?;
        let (ax1, ax2) = (&axes[(0, 0)], &axes[(0, 1)]);
        let t = t();
        let band = FillBetweenOptions {
            props: CollectionProps::default().alpha(0.4).label("band"),
            ..FillBetweenOptions::default()
        };
        ax1.fill_between(
            &t,
            map(&t, |v| v.sin() - 0.3),
            map(&t, |v| v.sin() + 0.3),
            band,
        )?;
        ax1.plot_with(&t, map(&t, f64::sin), labelled("center"))?;
        ax1.legend(LegendOptions::default())?;
        let layers = [map(&t, |v| v.sin().abs()), map(&t, |v| v.cos().abs())];
        let stacked = ax2.stackplot(&t, &layers, &StackplotOptions::default())?;
        for (layer, label) in stacked.iter().zip(["first", "second"]) {
            layer.set_label(label);
        }
        ax2.legend(LegendOptions::default().loc("upper left"))?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    small("legend/handles_errorbar_containers", |fig| {
        let ax = fig.subplot();
        let x = arange(0.0, 6.0, 1.0);
        let y_errors = ErrorbarOptions {
            yerr: Some(0.5.into()),
            fmt: "o-".into(),
            capsize: Some(3.0),
            line: labelled("y errors"),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, &x, y_errors)?;
        let both = ErrorbarOptions {
            xerr: Some(0.3.into()),
            yerr: Some(0.4.into()),
            fmt: "s".into(),
            line: labelled("x and y errors"),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, map(&x, |v| v + 3.0), both)?;
        let bars_only = ErrorbarOptions {
            yerr: Some(0.4.into()),
            fmt: "none".into(),
            line: labelled("bars only"),
            ..ErrorbarOptions::default()
        };
        ax.errorbar(&x, map(&x, |v| v + 6.0), bars_only)?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    }),
    small("legend/handles_scatter_collections", |fig| {
        let ax = fig.subplot();
        let x = SpecRng::new(302).uniform(20, 0.0, 1.0);
        let y = SpecRng::new(303).uniform(20, 0.0, 1.0);
        let mapped = ScatterOptions {
            colors: MappedColors::new(y.clone())
                .with_cmap(colormaps::get("viridis")?)
                .into(),
            ..ScatterOptions::default()
        };
        ax.scatter_with(&x, &y, mapped)?.set_label("colormapped");
        let crosses = ScatterOptions {
            marker: Some("x".parse()?),
            colors: ScatterColors::Single(Color::from("k")),
            ..ScatterOptions::default()
        };
        ax.scatter_with(&y, &x, crosses)?.set_label("crosses");
        let hollow = ScatterOptions {
            sizes: Some(vec![150.0]),
            colors: ScatterColors::Single(Color::none()),
            edgecolors: Some(EdgeColors::Colors(vec![Color::from("r")])),
            ..ScatterOptions::default()
        };
        ax.scatter_with(&x, &x, hollow)?.set_label("hollow");
        ax.legend(LegendOptions::default())?;
        Ok(())
    }),
    small("legend/handles_stem_and_step", |fig| {
        let ax = fig.subplot();
        let x = arange(0.0, 10.0, 1.0);
        let stems = StemOptions {
            label: Some("stem".into()),
            ..StemOptions::default()
        };
        ax.stem(&x, map(&x, |v| (v / 2.0).sin()), &stems)?;
        ax.step(
            &x,
            map(&x, |v| (v / 2.0).cos() + 1.0),
            StepWhere::Pre,
            labelled("step"),
        )?;
        let values = map(&arange(1.0, 10.0, 1.0), |v| v / 5.0 - 1.5);
        ax.stairs(values, StairsOptions::default())?
            .set_label("stairs");
        ax.legend(LegendOptions::default())?;
        Ok(())
    }),
    small("legend/handles_hist_types", |fig| {
        let ax = fig.subplot();
        let data = SpecRng::new(304).normal(300, 0.0, 1.0);
        let histogram = |histtype: HistType, label: &str, props: PatchProps| HistPlotOptions {
            layout: HistOptions {
                bins: Some(Bins::Count(15)),
                histtype,
                ..HistOptions::default()
            },
            label: vec![label.to_owned()],
            props,
            ..HistPlotOptions::default()
        };
        ax.hist(
            &[&data],
            &histogram(HistType::Bar, "bar", PatchProps::default()),
        )?;
        let shifted = map(&data, |v| v + 3.0);
        ax.hist(
            &[shifted],
            &histogram(HistType::Step, "step", PatchProps::default()),
        )?;
        let lowered = map(&data, |v| v - 3.0);
        let translucent = PatchProps::default().alpha(0.5);
        ax.hist(
            &[lowered],
            &histogram(HistType::StepFilled, "stepfilled", translucent),
        )?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    }),
];
