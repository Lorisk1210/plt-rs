//! `figures/cases_hist.py`.

use plt::axes::HistPlotOptions;
use plt::colors::Color;
use plt::numeric::{arange, linspace};
use plt::patches::PatchProps;
use plt::stats::{
    BinEstimator, Bins, Cumulative, HistAlign, HistBottom, HistOptions, HistOrientation, HistType,
};

use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// `NORMAL = rng(100).normal(1000)`.
fn normal() -> Vec<f64> {
    SpecRng::new(100).normal(1000, 0.0, 1.0)
}

/// `SKEWED = rng(101).normal(800, loc=1.0, scale=0.6) ** 2`.
fn skewed() -> Vec<f64> {
    let samples = SpecRng::new(101).normal(800, 1.0, 0.6);
    samples.iter().map(|x| x * x).collect()
}

/// `THREE`: three normal samples of different sizes and spreads.
fn three() -> Vec<Vec<f64>> {
    vec![
        SpecRng::new(102).normal(500, -1.0, 1.0),
        SpecRng::new(103).normal(400, 0.5, 0.7),
        SpecRng::new(104).normal(300, 2.0, 1.2),
    ]
}

/// Histogram options with `layout` and default styling.
fn laid_out(layout: HistOptions<'_>) -> HistPlotOptions<'_> {
    HistPlotOptions {
        layout,
        ..HistPlotOptions::default()
    }
}

/// `bins=n`.
fn bins(n: usize) -> HistOptions<'static> {
    HistOptions {
        bins: Some(Bins::Count(n)),
        ..HistOptions::default()
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("hist/default", |fig| {
        fig.subplot()
            .hist(&[normal()], &HistPlotOptions::default())?;
        Ok(())
    }),
    FigureCase::new("hist/bins_explicit_edges", |fig| {
        let edges = vec![-4.0, -2.0, -1.0, -0.5, 0.0, 0.25, 0.5, 1.0, 2.0, 4.0];
        let options = HistPlotOptions {
            layout: HistOptions {
                bins: Some(Bins::Edges(edges)),
                ..HistOptions::default()
            },
            props: PatchProps::default().edgecolor("k"),
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[normal()], &options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/bins_auto_and_sqrt", |fig| {
        let axes = fig.subplots(1, 2)?;
        for (ax, estimator) in axes
            .as_slice()
            .iter()
            .zip([BinEstimator::Auto, BinEstimator::Sqrt])
        {
            let layout = HistOptions {
                bins: Some(Bins::Estimator(estimator)),
                ..HistOptions::default()
            };
            ax.hist(&[skewed()], &laid_out(layout))?;
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("hist/range_clips_data", |fig| {
        let layout = HistOptions {
            range: Some((-1.0, 2.0)),
            ..bins(20)
        };
        fig.subplot().hist(&[normal()], &laid_out(layout))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/density_with_pdf", |fig| {
        let ax = fig.subplot();
        let options = HistPlotOptions {
            layout: HistOptions {
                density: true,
                ..bins(30)
            },
            props: PatchProps::default().alpha(0.6),
            ..HistPlotOptions::default()
        };
        ax.hist(&[normal()], &options)?;
        let t = linspace(-4.0, 4.0, 200);
        let pdf: Vec<f64> = t
            .iter()
            .map(|t| (-t * t / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt())
            .collect();
        ax.plot_fmt(t, pdf, "k")?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/cumulative", |fig| {
        let axes = fig.subplots(1, 2)?;
        let [ax1, ax2] = axes.as_slice() else {
            unreachable!("one row of two axes")
        };
        let forward = HistOptions {
            cumulative: Cumulative::Forward,
            ..bins(25)
        };
        ax1.hist(&[normal()], &laid_out(forward))?;
        let reverse = HistOptions {
            cumulative: Cumulative::Reverse,
            density: true,
            histtype: HistType::Step,
            ..bins(25)
        };
        ax2.hist(&[normal()], &laid_out(reverse))?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("hist/histtype_step", |fig| {
        let options = HistPlotOptions {
            layout: HistOptions {
                histtype: HistType::Step,
                ..bins(20)
            },
            props: PatchProps::default().linewidth(1.5),
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&three(), &options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/histtype_stepfilled", |fig| {
        let ax = fig.subplot();
        for data in three() {
            let options = HistPlotOptions {
                layout: HistOptions {
                    histtype: HistType::StepFilled,
                    ..bins(20)
                },
                props: PatchProps::default().alpha(0.5),
                ..HistPlotOptions::default()
            };
            ax.hist(&[data], &options)?;
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/multiple_side_by_side", |fig| {
        fig.subplot().hist(&three(), &laid_out(bins(10)))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/barstacked", |fig| {
        let layout = HistOptions {
            histtype: HistType::BarStacked,
            ..bins(15)
        };
        fig.subplot().hist(&three(), &laid_out(layout))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/stacked_step", |fig| {
        let options = HistPlotOptions {
            layout: HistOptions {
                histtype: HistType::Step,
                stacked: true,
                ..bins(15)
            },
            props: PatchProps::default().fill(false),
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&three(), &options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/horizontal", |fig| {
        let layout = HistOptions {
            orientation: HistOrientation::Horizontal,
            ..bins(20)
        };
        fig.subplot().hist(&[normal()], &laid_out(layout))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/horizontal_step_multi", |fig| {
        let layout = HistOptions {
            orientation: HistOrientation::Horizontal,
            histtype: HistType::Step,
            ..bins(15)
        };
        fig.subplot().hist(&three(), &laid_out(layout))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/weights", |fig| {
        let values: Vec<i64> = (0..10).collect();
        let weights = [values.iter().map(|&v| (v * v) as f64).collect::<Vec<f64>>()];
        let options = HistPlotOptions {
            layout: HistOptions {
                bins: Some(Bins::Edges(
                    arange(0.0, 11.0, 1.0).iter().map(|e| e - 0.5).collect(),
                )),
                weights: Some(&weights),
                ..HistOptions::default()
            },
            props: PatchProps::default().edgecolor("w"),
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[values], &options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/log", |fig| {
        let options = HistPlotOptions {
            layout: bins(30),
            log: true,
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[skewed()], &options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/rwidth_align", |fig| {
        let axes = fig.subplots(1, 2)?;
        let data = SpecRng::new(105).integers(200, 0, 6);
        let sides = [(HistAlign::Left, None), (HistAlign::Right, Some("C1"))];
        for (ax, (align, color)) in axes.as_slice().iter().zip(sides) {
            let options = HistPlotOptions {
                layout: HistOptions {
                    bins: Some(Bins::Edges(arange(0.0, 7.0, 1.0))),
                    rwidth: Some(0.6),
                    align,
                    ..HistOptions::default()
                },
                color: color.into_iter().map(Color::from).collect(),
                ..HistPlotOptions::default()
            };
            ax.hist(std::slice::from_ref(&data), &options)?;
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("hist/colors_edgecolor_linewidth", |fig| {
        let options = HistPlotOptions {
            layout: bins(15),
            color: vec![Color::from("lightgreen")],
            props: PatchProps::default()
                .edgecolor("darkgreen")
                .linewidth(1.5)
                .hatch("/"),
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[normal()], &options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("hist/bottom_offset", |fig| {
        let layout = HistOptions {
            bottom: HistBottom::Uniform(50.0),
            ..bins(12)
        };
        fig.subplot().hist(&[normal()], &laid_out(layout))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
