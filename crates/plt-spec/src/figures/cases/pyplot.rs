//! `figures/cases_pyplot.py`: figures built only through `plt::pyplot`,
//! acting on the harness's figure made pyplot's current figure.

use std::f64::consts::PI;

use plt::artist::ArtistHandle;
use plt::axes::{
    BarOptions, HistPlotOptions, ImshowOptions, MappedColors, ScatterOptions, StemOptions,
};
use plt::colorbar::ColorbarOptions;
use plt::colors::Color;
use plt::contour::{ClabelOptions, ContourOptions, Levels};
use plt::figure::Figure;
use plt::gridspec::SubplotOverrides;
use plt::legend::LegendOptions;
use plt::lines::{LineProps, StepWhere};
use plt::numeric::linspace;
use plt::pyplot;
use plt::stats::{Bins, HistOptions};

use super::colorbar::data::{field, xyz};
use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("pyplot/line_decorations", line_decorations),
    FigureCase::new("pyplot/subplot_grid", subplot_grid),
    FigureCase::new("pyplot/image_cmap_colorbar", image_cmap_colorbar),
    FigureCase::new("pyplot/contour_colorbar_labels", contour_colorbar_labels),
    FigureCase::new("pyplot/subplot2grid_twin", subplot2grid_twin),
    FigureCase::new("pyplot/polar_grids", polar_grids),
    FigureCase::new(
        "pyplot/contour_clim_cmap_after_colorbar",
        contour_clim_cmap_after_colorbar,
    ),
];

/// Runs `build` with `fig` as pyplot's current figure, then lets pyplot
/// forget the figure again (the harness's `plt.close(fig)`).
fn with_current(fig: &Figure, build: impl FnOnce() -> CaseResult) -> CaseResult {
    pyplot::figure_activate(fig);
    let result = build();
    pyplot::close_figure(fig);
    result
}

/// `f` applied to every value.
fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().map(|&v| f(v)).collect()
}

fn line_decorations(fig: &Figure) -> CaseResult {
    with_current(fig, || {
        let x = linspace(0.0, 2.0 * PI, 60);
        pyplot::plot_with(&x, map(&x, f64::sin), LineProps::default().label("sin"))?;
        pyplot::plot_fmt(&x, map(&x, f64::cos), "r--")?.set_label("cos");
        pyplot::axhline(0.0, LineProps::default().color("0.5").linewidth(0.8));
        pyplot::axvspan(1.0, 2.0).set_alpha(Some(0.2));
        pyplot::title("Waves");
        pyplot::xlabel("x");
        pyplot::ylabel("y");
        pyplot::xlim(0.0, 2.0 * PI);
        pyplot::ylim(-1.5, 1.5);
        pyplot::grid(true);
        pyplot::legend(LegendOptions::default())?;
        pyplot::text(4.0, 1.2, "note");
        Ok(())
    })
}

fn subplot_grid(fig: &Figure) -> CaseResult {
    with_current(fig, || {
        pyplot::subplot(2, 2, 1)?;
        pyplot::bar(vec![1.0, 2.0, 3.0], [3.0, 1.0, 2.0], BarOptions::default())?;
        pyplot::title("bar");
        pyplot::subplot(2, 2, 2)?;
        pyplot::barh(vec![1.0, 2.0, 3.0], [2.0, 3.0, 1.0], BarOptions::default())?;
        pyplot::subplot(2, 2, 3)?;
        let ramp = linspace(0.0, 1.0, 400);
        let data: Vec<f64> = linspace(0.0, 20.0, 400)
            .iter()
            .zip(&ramp)
            .map(|(t, scale)| t.sin() * scale)
            .collect();
        let options = HistPlotOptions {
            layout: HistOptions {
                bins: Some(Bins::Count(12)),
                ..HistOptions::default()
            },
            ..HistPlotOptions::default()
        };
        pyplot::hist(&[data], &options)?;
        pyplot::subplot(2, 2, 4)?;
        pyplot::stem(
            [1.0, 2.0, 3.0, 4.0],
            [1.0, 3.0, 2.0, 4.0],
            &StemOptions::default(),
        )?;
        pyplot::suptitle("pyplot subplots");
        pyplot::subplots_adjust(&SubplotOverrides {
            hspace: Some(0.4),
            ..SubplotOverrides::default()
        })?;
        Ok(())
    })
}

fn image_cmap_colorbar(fig: &Figure) -> CaseResult {
    with_current(fig, || {
        pyplot::imshow(field(30), ImshowOptions::default())?;
        pyplot::hot()?;
        pyplot::clim(Some(-0.3), Some(0.3))?;
        pyplot::colorbar(None, ColorbarOptions::default())?;
        pyplot::title("hot, clim");
        Ok(())
    })
}

fn contour_colorbar_labels(fig: &Figure) -> CaseResult {
    with_current(fig, || {
        let field = xyz();
        let fills = ContourOptions {
            levels: Levels::Count(8),
            ..ContourOptions::default()
        };
        pyplot::contourf(&field, fills)?;
        pyplot::colorbar(None, ColorbarOptions::default())?;
        let lines = ContourOptions {
            levels: Levels::Count(8),
            colors: Some(vec![Color::from("k")]),
            linewidths: Some(vec![0.6]),
            ..ContourOptions::default()
        };
        let contours = pyplot::contour(&field, lines)?;
        pyplot::clabel(&contours, &ClabelOptions::default())?;
        Ok(())
    })
}

fn contour_clim_cmap_after_colorbar(fig: &Figure) -> CaseResult {
    with_current(fig, || {
        let fills = ContourOptions {
            levels: Levels::Count(8),
            ..ContourOptions::default()
        };
        pyplot::contourf(&xyz(), fills)?;
        pyplot::colorbar(None, ColorbarOptions::default())?;
        pyplot::clim(Some(-0.2), Some(0.3))?;
        pyplot::magma()?;
        Ok(())
    })
}

fn subplot2grid_twin(fig: &Figure) -> CaseResult {
    with_current(fig, || {
        let x = linspace(0.0, 3.0, 31);
        pyplot::subplot2grid((3, 3), (0, 0), 1, 3)?;
        pyplot::plot(&x, map(&x, f64::exp))?;
        pyplot::yscale("log")?;
        pyplot::subplot2grid((3, 3), (1, 0), 2, 2)?;
        let colors = ScatterOptions {
            colors: MappedColors::new(x.clone()).into(),
            ..ScatterOptions::default()
        };
        pyplot::scatter_with(&x, map(&x, |v| (3.0 * v).sin()), colors)?;
        pyplot::xticks(vec![0.0, 1.5, 3.0], &["start", "middle", "end"])?;
        pyplot::margins(0.2, 0.1)?;
        pyplot::subplot2grid((3, 3), (1, 2), 2, 1)?;
        let head = &x[..8];
        pyplot::step(
            head,
            map(head, |v| v * v),
            StepWhere::Mid,
            LineProps::default(),
        )?;
        pyplot::r#box(Some(false));
        pyplot::twinx(None);
        pyplot::plot_with(head, map(head, |v| -v), LineProps::default().color("C3"))?;
        Ok(())
    })
}

fn polar_grids(fig: &Figure) -> CaseResult {
    with_current(fig, || {
        let theta = linspace(0.0, 2.0 * PI, 100);
        pyplot::polar(&theta, map(&theta, |t| 1.0 + 0.5 * (3.0 * t).sin()))?;
        pyplot::rgrids(&[0.5, 1.0, 1.5], &["a", "b", "c"], Some(45.0))?;
        pyplot::thetagrids(&[0.0, 90.0, 180.0, 270.0], &[])?;
        pyplot::title("polar");
        Ok(())
    })
}
