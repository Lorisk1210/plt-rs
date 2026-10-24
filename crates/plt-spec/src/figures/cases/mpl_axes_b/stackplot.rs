//! Stack plots: colours, baselines and hatches.

use plt::axes::{Baseline, FillBetweenOptions, StackplotOptions};
use plt::collections::CollectionProps;
use plt::colors::Color;
use plt::figure::Figure;
use plt::numeric::linspace;

use super::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `x = linspace(0, 10, 10)` and three straight layers over it.
fn straight_layers() -> (Vec<f64>, Vec<Vec<f64>>) {
    let x = linspace(0.0, 10.0, 10);
    let layers = vec![
        x.iter().map(|v| 1.0 * v).collect(),
        x.iter().map(|v| 2.0 * v + 1.0).collect(),
        x.iter().map(|v| 3.0 * v + 2.0).collect(),
    ];
    (x, layers)
}

/// The straight layers stacked in the view `[0, 10] × [0, 70]`.
fn stackplot(fig: &Figure, options: &StackplotOptions) -> CaseResult {
    let (x, layers) = straight_layers();
    let ax = fig.subplot();
    ax.stackplot(&x, &layers, options.clone())?;
    ax.set_xlim(0.0, 10.0)?;
    ax.set_ylim(0.0, 70.0)?;
    Ok(())
}

/// The reference of `test_stackplot_hatching`: each layer hatched by its
/// own `fill_between`.
fn stackplot_hatching_ref(fig: &Figure) -> CaseResult {
    let (x, layers) = straight_layers();
    let ax = fig.subplot();
    let mut bottom = vec![0.0; x.len()];
    for (layer, hatch) in layers.iter().zip(["x", "//", "\\\\"]) {
        let top: Vec<f64> = bottom.iter().zip(layer).map(|(b, y)| b + y).collect();
        let options = FillBetweenOptions {
            props: CollectionProps {
                hatch: Some(hatch.to_owned()),
                ..CollectionProps::default().facecolor("white")
            },
            ..FillBetweenOptions::default()
        };
        ax.fill_between(&x, &bottom, &top, options)?;
        bottom = top;
    }
    ax.set_xlim(0.0, 10.0)?;
    ax.set_ylim(0.0, 70.0)?;
    Ok(())
}

/// `layers(3, 100)` of `test_stackplot_baseline`: sums of five random
/// Gaussian bumps per layer, zeroed at index 50.
fn bumps() -> Vec<Vec<f64>> {
    let (n, m) = (3, 100);
    let mut stream = SpecRng::new(0);
    let mut layers = vec![vec![0.0; m]; n];
    for layer in &mut layers {
        for _ in 0..5 {
            let x = 1.0 / (0.1 + stream.next_f64());
            let y = 2.0 * stream.next_f64() - 0.5;
            let z = 10.0 / (0.1 + stream.next_f64());
            for (k, value) in layer.iter_mut().enumerate() {
                let t = (k as f64 / m as f64 - y) * z;
                *value += x * (-(t * t)).exp();
            }
        }
        layer[50] = 0.0;
    }
    layers
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_axes_b/stackplot_1", |fig| {
        stackplot(fig, &StackplotOptions::default())
    }),
    FigureCase::new("mpl_axes_b/stackplot_2", |fig| {
        let options = StackplotOptions {
            colors: ["C0", "C1", "C2"].map(Color::from).to_vec(),
            ..StackplotOptions::default()
        };
        stackplot(fig, &options)
    }),
    classic("mpl_axes_b/stackplot_hatching", |fig| {
        let options = StackplotOptions {
            hatches: ["x", "//", "\\\\"].map(String::from).to_vec(),
            colors: vec![Color::from("white")],
            ..StackplotOptions::default()
        };
        stackplot(fig, &options)
    }),
    classic("mpl_axes_b/stackplot_hatching_ref", stackplot_hatching_ref),
    classic("mpl_axes_b/stackplot_baseline", |fig| {
        let layers = bumps();
        let x: Vec<f64> = (0..100).map(f64::from).collect();
        let axs = fig.subplots(2, 2)?;
        let baselines = [
            Baseline::Zero,
            Baseline::Sym,
            Baseline::Wiggle,
            Baseline::WeightedWiggle,
        ];
        for (ax, baseline) in axs.as_slice().iter().zip(baselines) {
            let options = StackplotOptions {
                baseline,
                ..StackplotOptions::default()
            };
            ax.stackplot(&x, &layers, options)?;
        }
        remove_ticks_and_titles(fig);
        Ok(())
    }),
];
