//! Bar rectangles and step outlines of `Axes.hist`.

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

/// One bar: lower-left corner and extent in data coordinates (negative
/// heights extend downwards, like `Rectangle`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarRect {
    /// Left edge.
    pub x: f64,
    /// Bottom edge.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// The outline polygon of one dataset for the step histogram types.
#[derive(Clone, Debug, PartialEq)]
pub struct StepOutline {
    /// Vertices as `Polygon.get_xy()` returns them (a closed outline
    /// repeats its first vertex at the end).
    pub vertices: Vec<[f64; 2]>,
    /// Whether the polygon is closed.
    pub closed: bool,
    /// Whether the polygon is filled.
    pub filled: bool,
}

/// The patches of every dataset.
#[derive(Clone, Debug, PartialEq)]
pub enum HistPatches {
    /// For bar types: one rectangle per bin, per dataset.
    Bars(Vec<Vec<BarRect>>),
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
fn bars(tops: &[Vec<f64>], edges: &[f64], opts: &HistOptions<'_>) -> Vec<Vec<BarRect>> {
    let stacked = opts.is_stacked();
    let nx = tops.len() as f64;
    let totwidth: Vec<f64> = edges.windows(2).map(|w| w[1] - w[0]).collect();
    let dr = match opts.rwidth {
        Some(r) => r.clamp(0.0, 1.0),
        None if tops.len() > 1 && !stacked => 0.8,
        None => 1.0,
    };
    let (width, mut boffset, dw): (Vec<f64>, Vec<f64>, Vec<f64>) = if stacked {
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
    let mut bottom: Vec<f64> = (0..totwidth.len()).map(|i| opts.bottom.at(i)).collect();
    let mut groups = Vec::with_capacity(tops.len());
    for top in tops {
        let centres: Vec<f64> = edges.iter().zip(&boffset).map(|(e, b)| e + b).collect();
        let heights: Vec<f64> = top
            .iter()
            .zip(&bottom)
            .map(|(&t, &b)| if stacked { t - b } else { t })
            .collect();
        let thickness = relative_extents(&centres, &width);
        let lengths = relative_extents(&bottom, &heights);
        let group = (0..top.len())
            .map(|i| {
                let across = centres[i] - thickness[i] / 2.0;
                match opts.orientation {
                    HistOrientation::Vertical => BarRect {
                        x: across,
                        y: bottom[i],
                        width: thickness[i],
                        height: lengths[i],
                    },
                    HistOrientation::Horizontal => BarRect {
                        x: bottom[i],
                        y: across,
                        width: lengths[i],
                        height: thickness[i],
                    },
                }
            })
            .collect();
        groups.push(group);
        if stacked {
            bottom.clone_from(top);
        }
        for (b, d) in boffset.iter_mut().zip(&dw) {
            *b += d;
        }
    }
    groups
}

/// `Axes.bar` converts bar extents through the axis units relative to the
/// first finite position, i.e. `(origin + extent) - origin`, which rounds.
fn relative_extents(positions: &[f64], extents: &[f64]) -> Vec<f64> {
    let Some(&first) = positions.first() else {
        return extents.to_vec();
    };
    let origin = positions
        .iter()
        .copied()
        .find(|p| p.is_finite())
        .unwrap_or(first);
    extents.iter().map(|&d| (origin + d) - origin).collect()
}

/// matplotlib's step outline: `4·len(edges) - 3` points running up the
/// left edge, along the tops and back along the baseline (or, stacked,
/// along the previous dataset's tops).
fn steps(tops: &[Vec<f64>], edges: &[f64], opts: &HistOptions<'_>) -> Vec<StepOutline> {
    let filled = opts.histtype == HistType::StepFilled;
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
    let keep = if filled { len - 1 } else { 2 * n };
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
            let vertices = xs[..keep].iter().zip(&ys[..keep]).map(|(&a, &b)| [a, b]);
            polygon(vertices.collect(), filled)
        })
        .collect()
}

/// Sets `values[half..]` to `values[1..half]` reversed.
fn mirror(values: &mut [f64], half: usize) {
    for j in 0..half - 1 {
        values[half + j] = values[half - 1 - j];
    }
}

/// `Polygon.set_xy`: closed polygons end on their first vertex, open ones
/// drop a duplicated closing vertex.
fn polygon(mut vertices: Vec<[f64; 2]>, closed: bool) -> StepOutline {
    match (vertices.first().copied(), vertices.last().copied()) {
        (Some(first), Some(last)) if closed && first != last => vertices.push(first),
        (Some(first), Some(last)) if !closed && vertices.len() > 2 && first == last => {
            vertices.pop();
        }
        _ => {}
    }
    StepOutline {
        vertices,
        closed,
        filled: closed,
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
        let rects = bars(&tops, &[0.0, 1.0, 3.0], &layout(HistType::Bar));
        assert_eq!(
            rects[0],
            vec![
                BarRect {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 2.0
                },
                BarRect {
                    x: 1.0,
                    y: 0.0,
                    width: 2.0,
                    height: 1.0
                },
            ]
        );
    }

    #[test]
    fn side_by_side_and_horizontal_bars() {
        let tops = [vec![1.0], vec![3.0]];
        let side = bars(&tops, &[0.0, 1.0], &layout(HistType::Bar));
        assert!((side[0][0].x - 0.1).abs() < 1e-15 && (side[0][0].width - 0.4).abs() < 1e-15);
        assert!((side[1][0].x - 0.5).abs() < 1e-15);
        let opts = HistOptions {
            orientation: HistOrientation::Horizontal,
            histtype: HistType::BarStacked,
            bottom: HistBottom::Uniform(1.0),
            ..HistOptions::default()
        };
        let stacked = bars(&tops, &[0.0, 1.0], &opts);
        assert_eq!(
            stacked[0][0],
            BarRect {
                x: 1.0,
                y: 0.0,
                width: 0.0,
                height: 1.0
            }
        );
        assert_eq!(
            stacked[1][0],
            BarRect {
                x: 1.0,
                y: 0.0,
                width: 2.0,
                height: 1.0
            }
        );
    }

    #[test]
    fn step_outlines() {
        let tops = [vec![2.0, 1.0]];
        let open = steps(&tops, &[0.0, 1.0, 2.0], &layout(HistType::Step));
        assert_eq!(
            open[0].vertices,
            vec![
                [0.0, 0.0],
                [0.0, 2.0],
                [1.0, 2.0],
                [1.0, 1.0],
                [2.0, 1.0],
                [2.0, 0.0]
            ]
        );
        assert!(!open[0].closed && !open[0].filled);
        let filled = steps(&tops, &[0.0, 1.0, 2.0], &layout(HistType::StepFilled));
        assert_eq!(filled[0].vertices.len(), 9);
        assert_eq!(filled[0].vertices[8], [0.0, 0.0]);
        assert!(filled[0].closed && filled[0].filled);
    }

    #[test]
    fn open_polygons_drop_duplicate_closing_vertex() {
        let outline = polygon(vec![[0.0, 0.0], [1.0, 1.0], [0.0, 0.0]], false);
        assert_eq!(outline.vertices.len(), 2);
    }
}
