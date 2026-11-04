//! Stacked area plots (matplotlib's `stackplot`).

use super::fill_between::FillBetweenOptions;
use super::{Axes, PlotError};
use crate::artist::ArtistHandle;
use crate::collections::{Collection, CollectionProps};
use crate::colors::Color;
use crate::lines::StepWhere;
use crate::units::AxisValues;

/// Where the bottom of the stack lies (matplotlib's `baseline`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Baseline {
    /// The stack rests on zero.
    #[default]
    Zero,
    /// The stack is centred on zero (a theme river).
    Sym,
    /// Minimises the sum of the squared slopes (a "streamgraph").
    Wiggle,
    /// Like `Wiggle`, weighting each layer by its size.
    WeightedWiggle,
}

/// Options of [`Axes::stackplot`]. The sequences are cycled over the
/// layers.
///
/// A layer is filled with its colour of `facecolors`, else with the face
/// colour of `props`, else with its colour of `colors`: matplotlib's
/// `facecolor` takes precedence over `colors`. Without `colors` the
/// layers take the next colours of the line property cycle, even when
/// another fill wins, as in matplotlib.
#[derive(Clone, Debug, Default)]
pub struct StackplotOptions {
    /// Where the bottom of the stack lies.
    pub baseline: Baseline,
    /// Colours of the layers (empty: the next colours of the line property
    /// cycle).
    pub colors: Vec<Color>,
    /// Fill colours, taking precedence over `colors` (matplotlib's
    /// `facecolor` or `facecolors`; empty: none).
    pub facecolors: Vec<Color>,
    /// Edge colours (empty: the collection default).
    pub edgecolors: Vec<Color>,
    /// Hatch patterns (empty: none).
    pub hatches: Vec<String>,
    /// Edge widths in points (empty: the collection default).
    pub linewidths: Vec<f64>,
    /// Style shared by every layer.
    pub props: CollectionProps,
    /// Where every layer is filled, one flag per point (matplotlib's
    /// `where`, passed on to each layer's
    /// [`fill_between`](Axes::fill_between); `None` fills everywhere).
    pub mask: Option<Vec<bool>>,
    /// Whether each layer's fill extends to where its curves cross.
    pub interpolate: bool,
    /// Fill every layer under a step function instead of straight
    /// segments.
    pub step: Option<StepWhere>,
}

impl Axes {
    /// Stacks the layers `ys` on top of each other along `x` (numbers,
    /// dates or categories) and fills each one (matplotlib's
    /// `stackplot`); returns one collection per layer, bottom first.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::StackplotOptions;
    /// let ax = Figure::new().subplot();
    /// let layers = ax.stackplot([0.0, 1.0], &[vec![1.0, 2.0], vec![1.0, 1.0]], StackplotOptions::default())?;
    /// assert_eq!(layers.len(), 2);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a layer's length differs from that of `x`.
    pub fn stackplot(
        &self,
        x: impl Into<AxisValues>,
        ys: &[Vec<f64>],
        options: StackplotOptions,
    ) -> Result<Vec<Collection>, PlotError> {
        let x = self.xaxis().convert_units(x);
        if let Some(bad) = ys.iter().find(|y| y.len() != x.len()) {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: bad.len(),
            });
        }
        let colors = if options.colors.is_empty() {
            ys.iter().map(|_| self.next_line_color()).collect()
        } else {
            options.colors.clone()
        };
        let (first_line, stack) = stack_layers(ys, x.len(), options.baseline);
        let x = x.as_slice();
        let mut layers = Vec::with_capacity(ys.len());
        for (i, top) in stack.iter().enumerate() {
            let bottom = if i == 0 { &first_line } else { &stack[i - 1] };
            let fill = FillBetweenOptions {
                mask: options.mask.clone(),
                interpolate: options.interpolate,
                step: options.step,
                props: layer_props(&options, &colors, i),
            };
            let layer = self.fill_between(x, bottom, top, fill)?;
            if i == 0 {
                layer.set_sticky_edges(Vec::new(), vec![0.0]);
            }
            layers.push(layer);
        }
        Ok(layers)
    }
}

/// The style of layer `i`: the shared props plus the cycled sequences.
fn layer_props(options: &StackplotOptions, colors: &[Color], i: usize) -> CollectionProps {
    let cycled = |len: usize| i % len;
    let mut props = options.props.clone();
    if !options.facecolors.is_empty() {
        props = props.facecolor(options.facecolors[cycled(options.facecolors.len())].clone());
    } else if props.facecolors.is_none() {
        props = props.facecolor(colors[cycled(colors.len())].clone());
    }
    if !options.edgecolors.is_empty() {
        props = props.edgecolor(options.edgecolors[cycled(options.edgecolors.len())].clone());
    }
    if !options.hatches.is_empty() {
        props = props.hatch(options.hatches[cycled(options.hatches.len())].clone());
    }
    if !options.linewidths.is_empty() {
        props = props.linewidth(options.linewidths[cycled(options.linewidths.len())]);
    }
    props
}

/// The bottom line and the top of each layer for `n` points (matplotlib's
/// cumulative sum plus the baseline shift).
fn stack_layers(ys: &[Vec<f64>], n: usize, baseline: Baseline) -> (Vec<f64>, Vec<Vec<f64>>) {
    let mut stack: Vec<Vec<f64>> = Vec::with_capacity(ys.len());
    for y in ys {
        let below = stack.last().cloned().unwrap_or_else(|| vec![0.0; n]);
        stack.push(below.iter().zip(y).map(|(b, v)| b + v).collect());
    }
    let total: Vec<f64> = (0..n).map(|j| ys.iter().map(|y| y[j]).sum()).collect();
    let first_line: Vec<f64> = match baseline {
        Baseline::Zero => vec![0.0; n],
        Baseline::Sym => total.iter().map(|t| -t * 0.5).collect(),
        Baseline::Wiggle => {
            let m = ys.len() as f64;
            (0..n)
                .map(|j| {
                    let weighted: f64 = ys
                        .iter()
                        .enumerate()
                        .map(|(i, y)| y[j] * (m - 0.5 - i as f64))
                        .sum();
                    weighted / -m
                })
                .collect()
        }
        Baseline::WeightedWiggle => weighted_wiggle(ys, &stack, &total),
    };
    if baseline != Baseline::Zero {
        for layer in &mut stack {
            for (v, shift) in layer.iter_mut().zip(&first_line) {
                *v += shift;
            }
        }
    }
    (first_line, stack)
}

/// matplotlib's `weighted_wiggle` baseline, from the unshifted `stack`.
fn weighted_wiggle(ys: &[Vec<f64>], stack: &[Vec<f64>], total: &[f64]) -> Vec<f64> {
    let n = total.len();
    let inv_total: Vec<f64> = total
        .iter()
        .map(|&t| if t > 0.0 { 1.0 / t } else { 0.0 })
        .collect();
    let mut center = vec![0.0; n];
    for (y, top) in ys.iter().zip(stack) {
        for j in 0..n {
            let increase = if j == 0 { y[0] } else { y[j] - y[j - 1] };
            let below = total[j] - top[j] + 0.5 * y[j];
            let move_up = if j == 0 { 0.5 } else { below * inv_total[j] };
            center[j] += (move_up - 0.5) * increase;
        }
    }
    let mut running = 0.0;
    center
        .iter()
        .zip(total)
        .map(|(c, t)| {
            running += c;
            running - 0.5 * t
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use pltrs_core::Rgba;

    use super::*;
    use crate::figure::Figure;

    fn layers() -> Vec<Vec<f64>> {
        vec![vec![1.0, 2.0, 3.0], vec![2.0, 2.0, 1.0]]
    }

    #[test]
    fn zero_and_sym_baselines() {
        let (first, stack) = stack_layers(&layers(), 3, Baseline::Zero);
        assert_eq!(first, [0.0; 3]);
        assert_eq!(stack, [vec![1.0, 2.0, 3.0], vec![3.0, 4.0, 4.0]]);
        let (first, stack) = stack_layers(&layers(), 3, Baseline::Sym);
        assert_eq!(first, [-1.5, -2.0, -2.0]);
        assert_eq!(stack[1], [1.5, 2.0, 2.0]);
    }

    #[test]
    fn wiggle_baselines_match_numpy() {
        // numpy: (y * (2 - 0.5 - [[0], [1]])).sum(0) / -2
        let (first, _) = stack_layers(&layers(), 3, Baseline::Wiggle);
        assert_eq!(first, [-1.25, -2.0, -2.5]);
        // matplotlib 3.11 stackplot(baseline="weighted_wiggle")
        let (first, _) = stack_layers(&layers(), 3, Baseline::WeightedWiggle);
        let expected = [-1.5, -1.75, -1.25];
        for (a, b) in first.iter().zip(expected) {
            assert!((a - b).abs() < 1e-12, "{first:?}");
        }
    }

    #[test]
    fn face_colours_take_precedence_over_colours() {
        let ax = Figure::new().subplot();
        let face = |options: StackplotOptions| -> Vec<Rgba> {
            ax.stackplot([0.0, 1.0, 2.0], &layers(), options)
                .unwrap()
                .iter()
                .map(|layer| layer.colors().unwrap().faces[0])
                .collect()
        };
        let rgba = |name: &str| Color::from(name).resolve().unwrap();
        let colors = vec![Color::from("c"), Color::from("m")];
        let given = StackplotOptions {
            facecolors: vec![Color::from("r"), Color::from("b")],
            colors: colors.clone(),
            ..StackplotOptions::default()
        };
        assert_eq!(face(given), [rgba("r"), rgba("b")]);
        let shared = StackplotOptions {
            props: CollectionProps::default().facecolor("k"),
            colors,
            ..StackplotOptions::default()
        };
        assert_eq!(face(shared), [rgba("k"), rgba("k")]);
        let cycled = StackplotOptions {
            facecolors: vec![Color::from("r")],
            ..StackplotOptions::default()
        };
        assert_eq!(face(cycled), [rgba("r"), rgba("r")]);
        let next = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap().rgba().unwrap();
        assert_eq!(next, rgba("C2"), "the layers took C0 and C1");
    }

    #[test]
    fn layers_take_line_cycle_colours_and_stick_at_zero() {
        let ax = Figure::new().subplot();
        let drawn = ax
            .stackplot([0.0, 1.0, 2.0], &layers(), StackplotOptions::default())
            .unwrap();
        assert_eq!(drawn[0].sticky_edges(), (Vec::new(), vec![0.0]));
        assert_eq!(ax.ylim(), (0.0, 4.2));
        assert!(
            ax.stackplot([0.0], &layers(), StackplotOptions::default())
                .is_err()
        );
    }
}
