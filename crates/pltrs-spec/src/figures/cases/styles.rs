//! `figures/cases_styles.py`: every bundled style sheet applied to the
//! same standard plot, and individual rcParams.

use pltrs::axes::{
    BoxplotOptions, ErrorbarOptions, FillBetweenOptions, HistPlotOptions, MappedColors,
    ScatterOptions,
};
use pltrs::collections::CollectionProps;
use pltrs::legend::LegendOptions;
use pltrs::offsetbox::AnchorLoc;
use pltrs::patches::Rectangle;
use pltrs::prelude::*;

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `X = np.linspace(0, 10, 30)`.
fn x() -> Vec<f64> {
    linspace(0.0, 10.0, 30)
}

/// `X` mapped through `f`.
fn of_x(f: impl Fn(f64) -> f64) -> Vec<f64> {
    x().into_iter().map(f).collect()
}

/// Every third value of `values` (numpy's `[::3]`).
fn every_third(values: &[f64]) -> Vec<f64> {
    values.iter().step_by(3).copied().collect()
}

/// `_standard_plot`: lines from the colour cycle, a scatter, a legend,
/// titles and labels, and bars with error bars.
fn standard_plot(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axes[(0, 0)], &axes[(0, 1)]);
    for i in 0..4 {
        let shift = f64::from(i);
        let line = LineProps::default().label(format!("line {i}"));
        ax1.plot_with(x(), of_x(|x| (x + 0.8 * shift).sin() + 0.5 * shift), line)?;
    }
    let every = every_third(&x());
    let below = every.iter().map(|x| x.cos() - 1.5).collect::<Vec<_>>();
    ax1.scatter(every, below)?.set_label("points");
    ax1.set_title("Lines and scatter");
    ax1.set_xlabel("x").set_ylabel("y");
    ax1.legend(
        LegendOptions::default()
            .fontsize(pltrs::fonts::RelativeSize::Small)
            .loc(AnchorLoc::UpperRight),
    )?;
    let bars = BarOptions {
        errors: ErrorbarOptions {
            yerr: Some(vec![0.3, 0.5, 0.2, 0.4].into()),
            ..ErrorbarOptions::default()
        },
        ..BarOptions::default()
    };
    ax2.bar(["a", "b", "c", "d"], [3.0, 5.0, 2.0, 4.0], bars)?;
    ax2.set_title("Bars");
    Ok(())
}

/// Parameters pinned on top of every sheet so that goldens do not depend
/// on the host (`PINNED`).
const PINNED: &[(&str, &str)] = &[("font.family", "DejaVu Sans"), ("text.hinting", "none")];

/// One `styles/sheet_<name>` case per bundled sheet.
macro_rules! sheet_cases {
    ($($name:literal),* $(,)?) => {
        &[$(
            FigureCase::new(concat!("styles/sheet_", $name), standard_plot)
                .sized(7.0, 3.5, 100.0)
                .with_style($name)
                .with_rc(PINNED),
        )*]
    };
}

/// `STYLE_SHEETS`: the sheets applied as `{**library[name], **PINNED}`.
pub(crate) const SHEETS: &[FigureCase] = sheet_cases![
    "Solarize_Light2",
    "bmh",
    "classic",
    "dark_background",
    "fast",
    "fivethirtyeight",
    "ggplot",
    "grayscale",
    "petroff6",
    "petroff8",
    "petroff10",
    "seaborn-v0_8",
    "seaborn-v0_8-bright",
    "seaborn-v0_8-colorblind",
    "seaborn-v0_8-dark",
    "seaborn-v0_8-dark-palette",
    "seaborn-v0_8-darkgrid",
    "seaborn-v0_8-deep",
    "seaborn-v0_8-muted",
    "seaborn-v0_8-notebook",
    "seaborn-v0_8-paper",
    "seaborn-v0_8-pastel",
    "seaborn-v0_8-poster",
    "seaborn-v0_8-talk",
    "seaborn-v0_8-ticks",
    "seaborn-v0_8-white",
    "seaborn-v0_8-whitegrid",
    "tableau-colorblind10",
];

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("styles/rc_boxplot_horizontal", |fig| {
        let data: Vec<Vec<f64>> = [1.0, 2.0, 3.0]
            .into_iter()
            .map(|k| of_x(|x| (x * k).sin() * k))
            .collect();
        fig.subplot().boxplot(&data, BoxplotOptions::default())?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0)
    .with_rc(&[("boxplot.vertical", "False")]),
    FigureCase::new("styles/rc_path_snap_off", |fig| {
        let ax = fig.subplot();
        ax.plot_with(
            vec![0.13, 0.13, 0.87, 0.87, 0.13],
            vec![0.21, 0.79, 0.79, 0.21, 0.21],
            LineProps::default().linewidth(1.0),
        )?;
        ax.bar(
            vec![0.3, 0.55],
            [0.4, 0.6],
            BarOptions {
                thickness: 0.17.into(),
                ..BarOptions::default()
            },
        )?;
        ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0)
    .with_rc(&[("path.snap", "False"), ("axes.grid", "True")]),
    FigureCase::new("styles/rc_agg_path_chunksize", |fig| {
        let ax = fig.subplot();
        let x = linspace(0.0, 10.0, 2000);
        let wave: Vec<f64> = x.iter().map(|v| v.sin() + 0.3 * (37.0 * v).sin()).collect();
        ax.plot_with(x.clone(), wave, LineProps::default().linewidth(2.0))?;
        let head = &x[..400];
        let lower: Vec<f64> = head.iter().map(|v| v.sin() - 1.0).collect();
        let options = FillBetweenOptions {
            props: CollectionProps::default().alpha(0.3),
            ..FillBetweenOptions::default()
        };
        ax.fill_between(head, &lower, [-1.5], options)?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0)
    .with_rc(&[("agg.path.chunksize", "200")]),
    FigureCase::new("styles/rc_prop_cycle_color_and_linestyle", |fig| {
        let ax = fig.subplot();
        for i in 0..5 {
            let gain = f64::from(i + 1);
            ax.plot(x(), of_x(|x| x * gain))?;
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0)
    .with_rc(&[(
        "axes.prop_cycle",
        "cycler(color=['#e41a1c', '#377eb8', '#4daf4a']) + cycler(linestyle=['-', '--', ':'])",
    )]),
    FigureCase::new("styles/rc_font_sizes_and_title", |fig| {
        let ax = fig.subplot();
        ax.plot(x(), of_x(f64::sin))?;
        ax.set_title("Title");
        ax.set_xlabel("x label").set_ylabel("y label");
        Ok(())
    })
    .with_rc(&[
        ("font.size", "13"),
        ("axes.titlesize", "x-large"),
        ("axes.titleweight", "bold"),
        ("axes.titlelocation", "left"),
        ("axes.titlepad", "12"),
        ("axes.labelsize", "small"),
        ("xtick.labelsize", "8"),
        ("ytick.labelsize", "15"),
    ]),
    FigureCase::new("styles/rc_lines_defaults", |fig| {
        let ax = fig.subplot();
        ax.plot(x(), of_x(f64::sin))?;
        ax.plot(x(), of_x(f64::cos))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0)
    .with_rc(&[
        ("lines.linewidth", "3"),
        ("lines.linestyle", "--"),
        ("lines.marker", "o"),
        ("lines.markersize", "4"),
        ("lines.dashed_pattern", "2, 1"),
    ]),
    FigureCase::new("styles/rc_axes_grid", |fig| {
        fig.subplot().bar(
            arange(0.0, 5.0, 1.0),
            [3.0, 5.0, 2.0, 4.0, 6.0],
            BarOptions::default(),
        )?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0)
    .with_rc(&[
        ("axes.grid", "True"),
        ("axes.grid.axis", "y"),
        ("grid.color", "tab:purple"),
        ("grid.linestyle", ":"),
        ("grid.linewidth", "1.5"),
        ("axes.axisbelow", "True"),
    ]),
    FigureCase::new("styles/rc_ticks_inward_mirrored_minor", |fig| {
        fig.subplot().plot(x(), of_x(f64::sin))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0)
    .with_rc(&[
        ("xtick.direction", "in"),
        ("ytick.direction", "in"),
        ("xtick.top", "True"),
        ("ytick.right", "True"),
        ("xtick.minor.visible", "True"),
        ("ytick.minor.visible", "True"),
        ("xtick.major.size", "7"),
        ("ytick.major.size", "7"),
        ("xtick.major.pad", "6"),
    ]),
    FigureCase::new("styles/rc_spines_and_colors", |fig| {
        let ax = fig.subplot();
        ax.plot(x(), of_x(f64::sin))?;
        ax.set_title("colored title");
        ax.set_xlabel("x").set_ylabel("y");
        Ok(())
    })
    .with_rc(&[
        ("axes.spines.top", "False"),
        ("axes.spines.right", "False"),
        ("axes.facecolor", "#eeeeee"),
        ("axes.edgecolor", "gray"),
        ("axes.linewidth", "2"),
        ("axes.labelcolor", "darkred"),
        ("xtick.color", "tab:blue"),
        ("ytick.labelcolor", "tab:green"),
        ("text.color", "navy"),
    ]),
    FigureCase::new("styles/rc_figure_facecolor_subplot_params", |fig| {
        for (i, ax) in fig.subplots(1, 2)?.as_slice().iter().enumerate() {
            let shift = i as f64;
            ax.plot(x(), of_x(|x| (x + shift).sin()))?;
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0)
    .with_rc(&[
        ("figure.facecolor", "lightgray"),
        ("figure.subplot.left", "0.2"),
        ("figure.subplot.right", "0.95"),
        ("figure.subplot.top", "0.95"),
        ("figure.subplot.wspace", "0.5"),
    ]),
    FigureCase::new("styles/rc_margins_and_formatter", |fig| {
        fig.subplot()
            .plot(of_x(|x| x * 100.0), of_x(|x| x.sin() * 1e-3))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0)
    .with_rc(&[
        ("axes.xmargin", "0"),
        ("axes.ymargin", "0.2"),
        ("axes.formatter.limits", "-2, 2"),
        ("axes.formatter.use_mathtext", "True"),
        ("axes.unicode_minus", "False"),
    ]),
    FigureCase::new("styles/rc_patch_and_hist", |fig| {
        let ax = fig.subplot();
        let options = HistPlotOptions {
            color: vec![Color::from("C2")],
            ..HistPlotOptions::default()
        };
        ax.hist(&[SpecRng::new(400).normal(500, 0.0, 1.0)], options)?;
        ax.add_patch(Rectangle::new((-3.0, 40.0), 1.5, 15.0))
            .update_props(PatchProps::default().hatch("//"));
        Ok(())
    })
    .sized(5.0, 4.0, 100.0)
    .with_rc(&[
        ("patch.force_edgecolor", "True"),
        ("patch.edgecolor", "k"),
        ("patch.linewidth", "1.5"),
        ("patch.facecolor", "salmon"),
        ("hist.bins", "20"),
        ("hatch.color", "white"),
    ]),
    FigureCase::new("styles/rc_legend", |fig| {
        let ax = fig.subplot();
        ax.plot_with(x(), of_x(f64::sin), LineProps::default().label("sine"))?;
        ax.plot_with(x(), of_x(f64::cos), LineProps::default().label("cosine"))?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0)
    .with_rc(&[
        ("legend.frameon", "True"),
        ("legend.loc", "upper left"),
        ("legend.fontsize", "large"),
        ("legend.handlelength", "3"),
        ("legend.facecolor", "lightyellow"),
        ("legend.edgecolor", "k"),
        ("legend.fancybox", "False"),
        ("legend.framealpha", "1"),
    ]),
    FigureCase::new("styles/rc_scatter_errorbar_cmap", |fig| {
        let ax = fig.subplot();
        let scatter = ScatterOptions {
            colors: MappedColors::new(x()).into(),
            ..ScatterOptions::default()
        };
        ax.scatter_with(x(), of_x(f64::sin), scatter)?;
        let errors = ErrorbarOptions {
            yerr: Some(0.2.into()),
            fmt: "o".parse()?,
            ..ErrorbarOptions::default()
        };
        ax.errorbar(every_third(&x()), every_third(&of_x(f64::cos)), errors)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0)
    .with_rc(&[
        ("scatter.marker", "s"),
        ("scatter.edgecolors", "k"),
        ("errorbar.capsize", "4"),
        ("image.cmap", "plasma"),
        ("lines.markeredgewidth", "2"),
    ]),
];
