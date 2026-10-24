//! Event, stem and stack cases of `figures/cases_stats.py`.

use plt::axes::{Baseline, EventplotOptions, StackplotOptions, StemOptions};
use plt::collections::CollectionProps;
use plt::colors::Color;
use plt::numeric::{arange, linspace};
use plt::orientation::Orientation;

use super::map;
use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// `STACK_X = np.arange(12)`.
fn stack_x() -> Vec<f64> {
    arange(0.0, 12.0, 1.0)
}

/// `STACK_Y`: three smooth layers over `STACK_X`.
fn stack_y() -> Vec<Vec<f64>> {
    let x = stack_x();
    vec![
        map(&x, |x| 2.0 + (x / 2.0).sin()),
        map(&x, |x| 1.5 + (x / 3.0).cos()),
        map(&x, |x| 1.0 + 0.1 * x),
    ]
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("stats/eventplot_default", |fig| {
        let events: Vec<Vec<f64>> = (0..4)
            .map(|i| SpecRng::new(210 + i).uniform(20, 0.0, 10.0))
            .collect();
        fig.subplot()
            .eventplot(&events, EventplotOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/eventplot_vertical_colors_offsets", |fig| {
        let events: Vec<Vec<f64>> = (0..3)
            .map(|i| SpecRng::new(220 + i).uniform(15, 0.0, 1.0))
            .collect();
        let options = EventplotOptions {
            orientation: Orientation::Vertical,
            lineoffsets: vec![1.0, 3.0, 4.0],
            linelengths: vec![0.5, 1.5, 0.8],
            colors: ["C0", "C1", "C2"].map(Color::from).to_vec(),
            linewidths: Some(vec![1.0, 2.0, 3.0]),
            ..EventplotOptions::default()
        };
        fig.subplot().eventplot(&events, options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/stem_default", |fig| {
        let x = linspace(0.1, 2.0 * std::f64::consts::PI, 25);
        fig.subplot()
            .stem(&x, map(&x, f64::cos), StemOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/stem_formats_bottom", |fig| {
        let x = arange(0.0, 12.0, 1.0);
        let options = StemOptions {
            linefmt: "g:".parse()?,
            markerfmt: "D".parse()?,
            basefmt: Some("k-".parse()?),
            bottom: 1.0,
            ..StemOptions::default()
        };
        fig.subplot()
            .stem(&x, map(&x, |x| (x / 2.0).sin() + 1.0), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/stem_horizontal", |fig| {
        let x = arange(0.0, 10.0, 1.0);
        let options = StemOptions {
            markerfmt: "ro".parse()?,
            orientation: Orientation::Horizontal,
            ..StemOptions::default()
        };
        fig.subplot().stem(&x, map(&x, f64::sqrt), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/stackplot_zero", |fig| {
        fig.subplot()
            .stackplot(stack_x(), &stack_y(), StackplotOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/stackplot_baselines_sym_wiggle_weighted", |fig| {
        let axs = fig.subplots(1, 3)?;
        let baselines = [
            (Baseline::Sym, "sym"),
            (Baseline::Wiggle, "wiggle"),
            (Baseline::WeightedWiggle, "weighted_wiggle"),
        ];
        for (ax, (baseline, name)) in axs.as_slice().iter().zip(baselines) {
            let options = StackplotOptions {
                baseline,
                ..StackplotOptions::default()
            };
            ax.stackplot(stack_x(), &stack_y(), options)?;
            ax.set_title(name);
        }
        Ok(())
    })
    .sized(9.0, 3.0, 100.0),
    FigureCase::new("stats/stackplot_colors_hatch_edges", |fig| {
        let options = StackplotOptions {
            colors: ["#8dd3c7", "#ffffb3", "#bebada"].map(Color::from).to_vec(),
            hatches: ["", "//", ".."].map(String::from).to_vec(),
            props: CollectionProps::default().edgecolor("k"),
            ..StackplotOptions::default()
        };
        fig.subplot().stackplot(stack_x(), &stack_y(), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
