//! `figures/cases_styles.py`: individual rcParams (the style sheets need
//! legends).

use plt::axes::{ErrorbarOptions, HistPlotOptions, MappedColors, ScatterOptions};
use plt::patches::Rectangle;
use plt::prelude::*;

use crate::figures::FigureCase;
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

pub(crate) const CASES: &[FigureCase] = &[
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
        ax.hist(&[SpecRng::new(400).normal(500, 0.0, 1.0)], &options)?;
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
    FigureCase::new("styles/rc_scatter_errorbar_cmap", |fig| {
        let ax = fig.subplot();
        let scatter = ScatterOptions {
            colors: MappedColors::new(x()).into(),
            ..ScatterOptions::default()
        };
        ax.scatter_with(x(), of_x(f64::sin), scatter)?;
        let errors = ErrorbarOptions {
            yerr: Some(0.2.into()),
            fmt: "o".into(),
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
