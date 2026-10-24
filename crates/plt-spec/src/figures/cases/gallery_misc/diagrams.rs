//! Diagrams: a fishbone diagram, packed bubbles, a Hinton diagram and
//! Anscombe's quartet (`ishikawa_diagram`, `packed_bubbles`, `hinton_demo`
//! and `anscombe`).

use std::f64::consts::PI;

use plt::axes::TickParams;
use plt::figure::Share;
use plt::fonts::{FontSize, FontWeight};
use plt::layout::LayoutEngine;
use plt::lines::AxLineThrough;
use plt::patches::{Circle, Polygon, Rectangle, Wedge};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{AnnotateOptions, HAlign, VAlign};
use plt::ticker::NullLocator;
use plt::transforms::Point;

use super::spaced_grid;
use crate::figures::cases::common::{GALLERY_SEED, arrow, styled_box};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The arrow of the fishbone's annotations (`arrowstyle="->"`, black
/// face).
fn fishbone_arrow() -> Result<plt::text::ArrowProps, Box<dyn std::error::Error>> {
    let mut props = arrow("->", "arc3")?;
    props.patch = props.patch.facecolor("black");
    Ok(plt::text::ArrowProps::Fancy(props))
}

/// `problems(...)`: a category in a blue box with an arrow to the spine.
fn fishbone_problem(ax: &Axes, data: &str, x: f64, angle_y: f64) -> CaseResult {
    let note = ax.annotate(
        data.to_uppercase(),
        (x, 0.0),
        AnnotateOptions {
            xytext: Some(Point::new(-12.0, angle_y)),
            textcoords: Some("offset fontsize".parse()?),
            arrowprops: Some(fishbone_arrow()?),
            ..AnnotateOptions::default()
        },
    )?;
    note.set_fontsize(10.0)
        .set_color("white")
        .set_weight(FontWeight::BOLD)
        .set_valign(VAlign::Center)
        .set_halign(HAlign::Center)
        .set_bbox(styled_box(
            "square,pad=0.8",
            PatchProps::default().facecolor("tab:blue"),
        )?);
    Ok(())
}

/// `causes(...)`: the causes of a category, stacked above or below its
/// arrow.
fn fishbone_causes(ax: &Axes, data: &[&str], x: f64, y: f64, top: bool) -> CaseResult {
    const COORDS: [(f64, f64); 6] = [
        (0.02, 0.0),
        (0.23, 0.5),
        (-0.46, -1.0),
        (0.69, 1.5),
        (-0.92, -2.0),
        (1.15, 2.5),
    ];
    let (mut x, mut y) = (x, y);
    for (cause, (dx, dy)) in data.iter().zip(COORDS) {
        x -= dx;
        y += if top { dy } else { -dy };
        ax.annotate(
            *cause,
            (x, y),
            AnnotateOptions {
                xytext: Some(Point::new(-9.0, -0.3)),
                textcoords: Some("offset fontsize".parse()?),
                arrowprops: Some(fishbone_arrow()?),
                ..AnnotateOptions::default()
            },
        )?
        .set_halign(HAlign::Center)
        .set_fontsize(9.0);
    }
    Ok(())
}

/// `draw_spine(xmin, xmax)`: the spine, the head and the tail.
fn fishbone_spine(ax: &Axes, xmin: f64, xmax: f64) -> CaseResult {
    ax.plot_with(
        [xmin - 0.1, xmax],
        [0.0, 0.0],
        LineProps::default().color("tab:blue").linewidth(2.0),
    )?;
    ax.text(xmax + 0.1, -0.05, "PROBLEM")
        .set_fontsize(10.0)
        .set_weight(FontWeight::BOLD)
        .set_color("white");
    let blue = || PatchProps::default().facecolor("tab:blue");
    ax.add_patch_with(Wedge::new((xmax, 0.0), 1.0, 270.0, 90.0), blue());
    let tail = [
        Point::new(xmin - 0.8, 0.8),
        Point::new(xmin - 0.8, -0.8),
        Point::new(xmin, -0.01),
    ];
    ax.add_patch_with(Polygon::new(tail, true), blue());
    Ok(())
}

fn ishikawa_diagram(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    ax.set_xlim(-5.0, 5.0).set_ylim(-5.0, 5.0);
    ax.set_axis_off();
    let categories: [(&str, &[&str]); 6] = [
        (
            "Method",
            &[
                "Time consumption",
                "Cost",
                "Procedures",
                "Inefficient process",
                "Sampling",
            ],
        ),
        ("Machine", &["Faulty equipment", "Compatibility"]),
        (
            "Material",
            &[
                "Poor-quality input",
                "Raw materials",
                "Supplier",
                "Shortage",
            ],
        ),
        (
            "Measurement",
            &["Calibration", "Performance", "Wrong measurements"],
        ),
        ("Environment", &["Bad conditions"]),
        (
            "People",
            &[
                "Lack of training",
                "Managers",
                "Labor shortage",
                "Procedures",
                "Sales strategy",
            ],
        ),
    ];
    let length = (categories.len().div_ceil(2) - 1) as f64;
    fishbone_spine(&ax, -2.0 - length, 2.0 + length)?;
    let mut offset = 0.0;
    for (index, (name, causes)) in categories.iter().enumerate() {
        let above = index % 2 == 0;
        let problem_x = 1.55 + length + offset;
        let cause_x = 0.8 + length + offset;
        if !above {
            offset -= 2.5;
        }
        fishbone_problem(&ax, name, problem_x, if above { 16.0 } else { -16.0 })?;
        fishbone_causes(&ax, causes, cause_x, if above { 1.7 } else { -1.7 }, above)?;
    }
    Ok(())
}

/// `BubbleChart`: bubbles of the given areas packed around their centre
/// of mass, as `[x, y, radius, area]`.
struct BubbleChart {
    bubbles: Vec<[f64; 4]>,
    spacing: f64,
    step_dist: f64,
    com: [f64; 2],
}

/// `np.dot(v, v)` as `OpenBLAS` computes it here: a fused multiply-add.
fn norm_squared(v: [f64; 2]) -> f64 {
    v[1].mul_add(v[1], v[0] * v[0])
}

/// `v` scaled to length 1.
fn unit(v: [f64; 2]) -> [f64; 2] {
    let length = norm_squared(v).sqrt();
    [v[0] / length, v[1] / length]
}

impl BubbleChart {
    fn new(area: &[f64], spacing: f64) -> Self {
        let radii: Vec<f64> = area.iter().map(|a| (a / PI).sqrt()).collect();
        let maxstep = 2.0 * radii.iter().copied().fold(f64::MIN, f64::max) + spacing;
        let length = (area.len() as f64).sqrt().ceil() as usize;
        let bubbles = radii
            .iter()
            .zip(area)
            .enumerate()
            .map(|(i, (&r, &a))| {
                let (row, col) = (i / length, i % length);
                [col as f64 * maxstep, row as f64 * maxstep, r, a]
            })
            .collect();
        let mut chart = Self {
            bubbles,
            spacing,
            step_dist: maxstep / 2.0,
            com: [0.0; 2],
        };
        chart.com = chart.center_of_mass();
        chart
    }

    /// The area-weighted mean position (`np.average`).
    fn center_of_mass(&self) -> [f64; 2] {
        let total: f64 = self.bubbles.iter().map(|b| b[3]).sum();
        let weighted = |k: usize| self.bubbles.iter().map(|b| b[k] * b[3]).sum::<f64>();
        [weighted(0) / total, weighted(1) / total]
    }

    /// The gaps between the outline of `bubble` and those of the other
    /// bubbles than `skip`.
    fn outline_distances(&self, bubble: &[f64; 4], skip: usize) -> Vec<(usize, f64)> {
        self.bubbles
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != skip)
            .map(|(j, other)| {
                let centres = (bubble[0] - other[0]).hypot(bubble[1] - other[1]);
                (j, centres - bubble[2] - other[2] - self.spacing)
            })
            .collect()
    }

    fn collides(&self, bubble: &[f64; 4], skip: usize) -> bool {
        self.outline_distances(bubble, skip)
            .iter()
            .any(|&(_, d)| d < 0.0)
    }

    /// The first of the other bubbles closest to `bubble` (`argmin`).
    fn closest(&self, bubble: &[f64; 4], skip: usize) -> usize {
        self.outline_distances(bubble, skip)
            .into_iter()
            .fold(None, |best: Option<(usize, f64)>, (j, d)| match best {
                Some((_, low)) if low <= d => best,
                _ => Some((j, d)),
            })
            .map_or(0, |(j, _)| j)
    }

    /// Moves `bubble` `step` along `dir`.
    fn moved(bubble: &[f64; 4], dir: [f64; 2], step: f64) -> [f64; 4] {
        [
            bubble[0] + dir[0] * step,
            bubble[1] + dir[1] * step,
            bubble[2],
            bubble[3],
        ]
    }

    /// `collapse(n_iterations)`: moves the bubbles towards the centre of
    /// mass, around the ones in the way.
    fn collapse(&mut self, iterations: usize) {
        for _ in 0..iterations {
            let mut moves = 0_u32;
            for i in 0..self.bubbles.len() {
                let current = self.bubbles[i];
                let dir = unit([self.com[0] - current[0], self.com[1] - current[1]]);
                let candidate = Self::moved(&current, dir, self.step_dist);
                if !self.collides(&candidate, i) {
                    self.bubbles[i] = candidate;
                    self.com = self.center_of_mass();
                    moves += 1;
                    continue;
                }
                let other = self.bubbles[self.closest(&candidate, i)];
                let dir = unit([other[0] - current[0], other[1] - current[1]]);
                let orth = [dir[1], -dir[0]];
                let one = Self::moved(&current, orth, self.step_dist);
                let two = Self::moved(&current, [-orth[0], -orth[1]], self.step_dist);
                let distance = |b: &[f64; 4]| (self.com[0] - b[0]).hypot(self.com[1] - b[1]);
                let candidate = if distance(&one) < distance(&two) {
                    one
                } else {
                    two
                };
                if !self.collides(&candidate, i) {
                    self.bubbles[i] = candidate;
                    self.com = self.center_of_mass();
                }
            }
            if f64::from(moves) / (self.bubbles.len() as f64) < 0.1 {
                self.step_dist /= 2.0;
            }
        }
    }
}

fn packed_bubbles(fig: &Figure) -> CaseResult {
    let mut chart = BubbleChart::new(&[8.61, 69.55, 8.36, 4.12, 2.76, 2.43], 0.1);
    chart.collapse(50);
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Equal)?;
    let browsers = ["firefox", "chrome", "safari", "edge", "ie", "opera"];
    let colors = [
        "#5A69AF", "#579E65", "#F9C784", "#FC944A", "#F24C00", "#00B825",
    ];
    for ((bubble, label), color) in chart.bubbles.iter().zip(browsers).zip(colors) {
        ax.add_patch_with(
            Circle::new((bubble[0], bubble[1]), bubble[2]),
            PatchProps::default().color(color),
        );
        ax.text(bubble[0], bubble[1], label)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center);
    }
    ax.set_axis_off();
    ax.relim(false);
    ax.autoscale_view(None);
    ax.set_title("Browser market share");
    Ok(())
}

fn hinton_demo(fig: &Figure) -> CaseResult {
    let values = SpecRng::new(GALLERY_SEED).uniform(400, 0.0, 1.0);
    let matrix: Vec<f64> = values.iter().map(|v| v - 0.5).collect();
    let ax = fig.subplot();
    let largest = matrix.iter().fold(0.0_f64, |m, w| m.max(w.abs()));
    let max_weight = 2.0_f64.powf(largest.log2().ceil());
    ax.set_facecolor("gray");
    ax.set_shared_aspect(Aspect::Equal, plt::axes::Adjustable::Box)?;
    ax.xaxis().set_major_locator(NullLocator);
    ax.yaxis().set_major_locator(NullLocator);
    for (index, &w) in matrix.iter().enumerate() {
        let (x, y) = ((index / 20) as f64, (index % 20) as f64);
        let color = if w > 0.0 { "white" } else { "black" };
        let size = (w.abs() / max_weight).sqrt();
        ax.add_patch_with(
            Rectangle::new((x - size / 2.0, y - size / 2.0), size, size),
            PatchProps::default().facecolor(color).edgecolor(color),
        );
    }
    ax.autoscale_view(None);
    ax.yaxis().set_inverted(true);
    Ok(())
}

/// The mean of `values`.
fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Slope and intercept of the least-squares line (`np.polyfit(x, y, 1)`),
/// the standard deviation of `y` and the correlation of `x` and `y`.
fn line_statistics(x: &[f64], y: &[f64]) -> (f64, f64, f64, f64) {
    let (mx, my) = (mean(x), mean(y));
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sxx: f64 = x.iter().map(|a| (a - mx) * (a - mx)).sum();
    let syy: f64 = y.iter().map(|b| (b - my) * (b - my)).sum();
    let slope = sxy / sxx;
    let std = (syy / y.len() as f64).sqrt();
    (slope, my - slope * mx, std, sxy / (sxx * syy).sqrt())
}

fn anscombe(fig: &Figure) -> CaseResult {
    let x = [10.0, 8.0, 13.0, 9.0, 11.0, 14.0, 6.0, 4.0, 12.0, 7.0, 5.0];
    let x4 = [8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 19.0, 8.0, 8.0, 8.0];
    let datasets: [(&str, &[f64; 11], [f64; 11]); 4] = [
        (
            "I",
            &x,
            [
                8.04, 6.95, 7.58, 8.81, 8.33, 9.96, 7.24, 4.26, 10.84, 4.82, 5.68,
            ],
        ),
        (
            "II",
            &x,
            [
                9.14, 8.14, 8.74, 8.77, 9.26, 8.10, 6.13, 3.10, 9.13, 7.26, 4.74,
            ],
        ),
        (
            "III",
            &x,
            [
                7.46, 6.77, 12.74, 7.11, 7.81, 8.84, 6.08, 5.39, 8.15, 6.42, 5.73,
            ],
        ),
        (
            "IV",
            &x4,
            [
                6.58, 5.76, 7.71, 8.84, 8.47, 7.04, 5.25, 12.50, 5.56, 7.91, 6.89,
            ],
        ),
    ];
    let axs = fig.subplots_on(&spaced_grid(2, 2, 0.08, 0.08)?, Share::All, Share::All)?;
    let first = &axs[(0, 0)];
    first.set_xlim(0.0, 20.0).set_ylim(2.0, 14.0);
    first
        .set_xticks([0.0, 10.0, 20.0])
        .set_yticks([4.0, 8.0, 12.0]);
    for (ax, (label, x, y)) in axs.as_slice().iter().zip(&datasets) {
        ax.text(0.1, 0.9, *label)
            .set_fontsize(20.0)
            .set_transform(ax.trans_axes())
            .set_valign(VAlign::Top);
        ax.tick_params(&TickParams {
            direction: Some("in".parse()?),
            top: Some(true),
            right: Some(true),
            ..TickParams::default()
        });
        ax.plot_fmt(x.as_slice(), y.as_slice(), "o")?;
        let (slope, intercept, std, r) = line_statistics(x.as_slice(), y);
        ax.axline(
            Point::new(0.0, intercept),
            AxLineThrough::Slope(slope),
            LineProps::default().color("r").linewidth(2.0),
        )?;
        let stats = format!(
            "$\\mu$ = {:.2}\n$\\sigma$ = {std:.2}\n$r$ = {r:.2}",
            mean(y)
        );
        ax.text(0.95, 0.07, stats)
            .set_fontsize(FontSize::Points(9.0))
            .set_bbox(styled_box(
                "round",
                PatchProps::default()
                    .facecolor("blanchedalmond")
                    .edgecolor("orange")
                    .alpha(0.5),
            )?)
            .set_transform(ax.trans_axes())
            .set_halign(HAlign::Right);
    }
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_misc/ishikawa_diagram", ishikawa_diagram).sized(10.0, 6.0, 100.0),
    FigureCase::new("gallery_misc/packed_bubbles", packed_bubbles),
    FigureCase::new("gallery_misc/hinton_demo", hinton_demo),
    FigureCase::new("gallery_misc/anscombe", anscombe).sized(6.0, 6.0, 100.0),
];
