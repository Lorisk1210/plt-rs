//! The bars and step outlines `Axes.hist` draws for each dataset.

use plt_core::Point;

use super::HistOptions;

/// How `Axes.hist` draws each dataset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HistType {
    /// Side-by-side bars (stacked when [`HistOptions::stacked`]).
    #[default]
    Bar,
    /// Bars stacked on top of each other.
    BarStacked,
    /// An unfilled outline.
    Step,
    /// A filled outline.
    StepFilled,
}

/// Where bars sit relative to their bin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HistAlign {
    /// Centred on the left edge.
    Left,
    /// Centred in the bin.
    #[default]
    Mid,
    /// Centred on the right edge.
    Right,
}

/// The direction the bars grow in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HistOrientation {
    /// Bins along x, counts along y.
    #[default]
    Vertical,
    /// Bins along y, counts along x.
    Horizontal,
}

/// One dataset's bars: the centred bars `Axes.hist` asks `bar` (or `barh`)
/// to draw, one per bin.
#[derive(Clone, Debug, PartialEq)]
pub struct HistBars {
    /// Bar centres along the bin axis.
    pub positions: Vec<f64>,
    /// Bar lengths along the count axis.
    pub lengths: Vec<f64>,
    /// Bar thickness along the bin axis.
    pub thickness: Vec<f64>,
    /// Where the bars start along the count axis.
    pub bases: Vec<f64>,
}

/// One dataset's outline for the step types, as passed to `Polygon`
/// (which adds or drops the closing vertex).
#[derive(Clone, Debug, PartialEq)]
pub struct StepOutline {
    /// Outline vertices in data coordinates.
    pub vertices: Vec<Point>,
    /// Whether the outline is closed and filled (`stepfilled`).
    pub closed: bool,
}

/// What `Axes.hist` draws for every dataset.
#[derive(Clone, Debug, PartialEq)]
pub enum HistPatches {
    /// For bar types: the bars of each dataset.
    Bars(Vec<HistBars>),
    /// For step types: one outline per dataset.
    Steps(Vec<StepOutline>),
}

pub(super) fn patches(tops: &[Vec<f64>], edges: &[f64], opts: &HistOptions<'_>) -> HistPatches {
    match opts.histtype {
        HistType::Bar | HistType::BarStacked => HistPatches::Bars(bars(tops, edges, opts)),
        HistType::Step | HistType::StepFilled => HistPatches::Steps(steps(tops, edges, opts)),
    }
}

/// matplotlib's bar layout: side-by-side bars share `rwidth` of each bin
/// (0.8 by default for several unstacked datasets), stacked bars use it
/// whole and start where the previous dataset ended.
fn bars(tops: &[Vec<f64>], edges: &[f64], opts: &HistOptions<'_>) -> Vec<HistBars> {
    let stacked = opts.is_stacked();
    let nx = tops.len() as f64;
    let totwidth: Vec<f64> = edges.windows(2).map(|w| w[1] - w[0]).collect();
    let dr = match opts.rwidth {
        Some(r) => r.clamp(0.0, 1.0),
        None if tops.len() > 1 && !stacked => 0.8,
        None => 1.0,
    };
    let (thickness, mut boffset, dw): (Vec<f64>, Vec<f64>, Vec<f64>) = if stacked {
        let width = totwidth.iter().map(|t| dr * t).collect();
        (width, vec![0.0; totwidth.len()], vec![0.0; totwidth.len()])
    } else {
        let width: Vec<f64> = totwidth.iter().map(|t| dr * t / nx).collect();
        let boffset = totwidth
            .iter()
            .map(|t| -0.5 * dr * t * (1.0 - 1.0 / nx))
            .collect();
        (width.clone(), boffset, width)
    };
    for (b, t) in boffset.iter_mut().zip(&totwidth) {
        match opts.align {
            HistAlign::Left => {}
            HistAlign::Mid => *b += 0.5 * t,
            HistAlign::Right => *b += t,
        }
    }
    let mut bases: Vec<f64> = (0..totwidth.len()).map(|i| opts.bottom.at(i)).collect();
    let mut groups = Vec::with_capacity(tops.len());
    for top in tops {
        let lengths = top
            .iter()
            .zip(&bases)
            .map(|(&t, &b)| if stacked { t - b } else { t })
            .collect();
        groups.push(HistBars {
            positions: edges.iter().zip(&boffset).map(|(e, b)| e + b).collect(),
            lengths,
            thickness: thickness.clone(),
            bases: bases.clone(),
        });
        if stacked {
            bases.clone_from(top);
        }
        for (b, d) in boffset.iter_mut().zip(&dw) {
            *b += d;
        }
    }
    groups
}

/// matplotlib's step outline: `4·len(edges) - 3` points running up the
/// left edge, along the tops and back along the baseline (or, stacked,
/// along the previous dataset's tops).
fn steps(tops: &[Vec<f64>], edges: &[f64], opts: &HistOptions<'_>) -> Vec<StepOutline> {
    let closed = opts.histtype == HistType::StepFilled;
    let n = edges.len();
    if n == 0 {
        return Vec::new();
    }
    let len = 4 * n - 3;
    let half = 2 * n - 1;
    let mut x = vec![0.0; len];
    for (i, &e) in edges.iter().enumerate() {
        x[2 * i] = e;
        if i + 1 < n {
            x[2 * i + 1] = e;
        }
    }
    mirror(&mut x, half);
    if n >= 2 {
        let shift = 0.5 * (edges[1] - edges[0]);
        match opts.align {
            HistAlign::Left => x.iter_mut().for_each(|v| *v -= shift),
            HistAlign::Right => x.iter_mut().for_each(|v| *v += shift),
            HistAlign::Mid => {}
        }
    }
    let bottom = |bin: usize| opts.bottom.at(bin);
    let mut y = vec![0.0; len];
    for bin in 0..n - 1 {
        y[2 * bin + 1] = bottom(bin);
        y[2 * bin + 2] = bottom(bin);
    }
    mirror(&mut y, half);
    let keep = if closed { len - 1 } else { 2 * n };
    tops.iter()
        .map(|top| {
            if opts.is_stacked() {
                mirror(&mut y, half);
            }
            for (bin, &t) in top.iter().enumerate() {
                let level = t + bottom(bin);
                y[2 * bin + 1] = level;
                y[2 * bin + 2] = level;
            }
            y[0] = y[len - 1];
            let (xs, ys) = match opts.orientation {
                HistOrientation::Vertical => (&x, &y),
                HistOrientation::Horizontal => (&y, &x),
            };
            let vertices = xs[..keep]
                .iter()
                .zip(&ys[..keep])
                .map(|(&a, &b)| Point::new(a, b))
                .collect();
            StepOutline { vertices, closed }
        })
        .collect()
}

/// Sets `values[half..]` to `values[1..half]` reversed.
fn mirror(values: &mut [f64], half: usize) {
    for j in 0..half - 1 {
        values[half + j] = values[half - 1 - j];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::HistBottom;

    fn layout(histtype: HistType) -> HistOptions<'static> {
        HistOptions {
            histtype,
            ..HistOptions::default()
        }
    }

    #[test]
    fn single_dataset_bars_fill_their_bins() {
        let tops = [vec![2.0, 1.0]];
        let groups = bars(&tops, &[0.0, 1.0, 3.0], &layout(HistType::Bar));
        assert_eq!(
            groups,
            [HistBars {
                positions: vec![0.5, 2.0],
                lengths: vec![2.0, 1.0],
                thickness: vec![1.0, 2.0],
                bases: vec![0.0, 0.0],
            }]
        );
    }

    #[test]
    fn side_by_side_and_stacked_bars() {
        let tops = [vec![1.0], vec![3.0]];
        let side = bars(&tops, &[0.0, 1.0], &layout(HistType::Bar));
        assert!((side[0].positions[0] - 0.3).abs() < 1e-15);
        assert!((side[1].positions[0] - 0.7).abs() < 1e-15);
        assert!((side[0].thickness[0] - 0.4).abs() < 1e-15);
        let opts = HistOptions {
            histtype: HistType::BarStacked,
            bottom: HistBottom::Uniform(1.0),
            ..HistOptions::default()
        };
        let stacked = bars(&tops, &[0.0, 1.0], &opts);
        assert_eq!((stacked[0].bases[0], stacked[0].lengths[0]), (1.0, 0.0));
        assert_eq!((stacked[1].bases[0], stacked[1].lengths[0]), (1.0, 2.0));
    }

    #[test]
    fn step_outlines() {
        let tops = [vec![2.0, 1.0]];
        let open = steps(&tops, &[0.0, 1.0, 2.0], &layout(HistType::Step));
        let xy: Vec<(f64, f64)> = open[0].vertices.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(
            xy,
            [
                (0.0, 0.0),
                (0.0, 2.0),
                (1.0, 2.0),
                (1.0, 1.0),
                (2.0, 1.0),
                (2.0, 0.0)
            ]
        );
        assert!(!open[0].closed);
        let filled = steps(&tops, &[0.0, 1.0, 2.0], &layout(HistType::StepFilled));
        assert_eq!(filled[0].vertices.len(), 8);
        assert!(filled[0].closed);
    }
}
