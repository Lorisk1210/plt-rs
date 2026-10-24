//! Plotting: random figures — tiny or huge, at any resolution, with a
//! layout engine or none — holding rectilinear, logarithmic, symlog,
//! logit, inverted, shared, twin, inset, polar, geographic and 3-D axes,
//! filled by random plotting calls with random options and data, then
//! decorated and saved in a random format.

mod axis_decor;
mod calls;
mod data;
mod decor;
mod fields;
mod stats;
mod three_d;

use plt::axes::{Axes, BarContainer, ProjectionKind};
use plt::collections::Collection;
use plt::contour::ContourSet;
use plt::figure::{BboxInches, Figure, SaveFormat, SaveOptions};
use plt::image::AxesImage;
use plt::layout::{ConstrainedLayout, LayoutEngine, TightLayout};
use plt::mplot3d::{Axes3D, Axes3DOptions};

use axis_decor::AxisDecor;
use calls::Call;
use decor::Decor;
use fields::FieldCall;
use stats::StatCall;
use three_d::Call3D;

use crate::harness::{Gen, fuzz};
use crate::support::format;

/// What colorbars and bar labels can be made for: the mappables and bar
/// containers made so far.
#[derive(Default)]
pub(crate) struct Mappables {
    images: Vec<AxesImage>,
    collections: Vec<Collection>,
    contours: Vec<ContourSet>,
    bars: Vec<BarContainer>,
}

/// The kinds of axes.
#[derive(Debug)]
enum Kind {
    Rect,
    Projected(ProjectionKind),
    ThreeD,
    Inset([f64; 4]),
    TwinX,
    TwinY,
    SharedX,
}

/// One axes: its kind and how its axes are set up.
#[derive(Debug)]
struct AxesSpec {
    kind: Kind,
    scales: (&'static str, &'static str),
    inverted: (bool, bool),
    aspect: Option<f64>,
    margins: Option<(f64, f64)>,
}

/// A step of the plan: a call or decoration on axes `index` (modulo the
/// axes made).
#[derive(Debug)]
enum Step {
    Plot(usize, Call),
    Stat(usize, StatCall),
    Field(usize, FieldCall),
    ThreeD(usize, Call3D),
    Decorate(usize, Decor),
    DecorateAxis(usize, AxisDecor),
}

/// A whole figure.
#[derive(Debug)]
struct Plan {
    size: (f64, f64),
    dpi: f64,
    grid: (usize, usize),
    layout: usize,
    axes: Vec<AxesSpec>,
    steps: Vec<Step>,
    save_dpi: Option<f64>,
    tight: bool,
    format: SaveFormat,
}

/// Scale names, with the linear scale the likeliest.
const SCALES: [&str; 7] = [
    "linear", "linear", "linear", "log", "symlog", "logit", "asinh",
];

/// The projections of 2-D axes.
const PROJECTIONS: [ProjectionKind; 5] = [
    ProjectionKind::Polar,
    ProjectionKind::Aitoff,
    ProjectionKind::Hammer,
    ProjectionKind::Lambert,
    ProjectionKind::Mollweide,
];

impl AxesSpec {
    fn generate(g: &mut Gen) -> Self {
        let kind = match g.below(14) {
            0 | 1 => Kind::Projected(g.pick(&PROJECTIONS)),
            2 | 3 => Kind::ThreeD,
            4 => Kind::Inset([
                g.number_in(0.0, 1.0),
                g.number_in(0.0, 1.0),
                g.number_in(0.0, 1.0),
                g.number_in(0.0, 1.0),
            ]),
            5 => Kind::TwinX,
            6 => Kind::TwinY,
            7 => Kind::SharedX,
            _ => Kind::Rect,
        };
        Self {
            kind,
            scales: (g.pick(&SCALES), g.pick(&SCALES)),
            inverted: (g.chance(0.15), g.chance(0.15)),
            aspect: g.chance(0.1).then(|| g.number()),
            margins: g
                .chance(0.1)
                .then(|| (g.number_in(-0.6, 1.0), g.number_in(-0.6, 1.0))),
        }
    }
}

impl Step {
    fn generate(g: &mut Gen) -> Self {
        let at = g.below(4);
        match g.below(10) {
            0..3 => Self::Plot(at, Call::generate(g)),
            3 => Self::Stat(at, StatCall::generate(g)),
            4 | 5 => Self::Field(at, FieldCall::generate(g)),
            6 => Self::ThreeD(at, Call3D::generate(g)),
            7 | 8 => Self::Decorate(at, Decor::generate(g)),
            _ => Self::DecorateAxis(at, AxisDecor::generate(g)),
        }
    }
}

impl Plan {
    fn generate(g: &mut Gen) -> Self {
        let extreme = g.chance(0.1);
        let length = |g: &mut Gen| {
            if extreme {
                g.number()
            } else {
                g.between(1.0, 4.0)
            }
        };
        Self {
            size: (length(g), length(g)),
            dpi: if extreme {
                g.number()
            } else {
                g.between(20.0, 60.0)
            },
            grid: (1 + g.below(3), 1 + g.below(3)),
            layout: g.below(4),
            axes: (0..=g.below(3)).map(|_| AxesSpec::generate(g)).collect(),
            steps: (0..=g.len(8)).map(|_| Step::generate(g)).collect(),
            save_dpi: g.chance(0.1).then(|| g.number_in(10.0, 80.0)),
            tight: g.chance(0.15),
            format: format(g),
        }
    }
}

/// The axes made for a plan, 2-D or 3-D.
enum Made {
    Flat(Axes),
    Deep(Axes3D),
}

/// Makes the axes of `spec` in cell `index` of the plan's grid.
fn make_axes(
    fig: &Figure,
    plan: &Plan,
    index: usize,
    spec: &AxesSpec,
    made: &[Made],
) -> Option<Made> {
    let (rows, cols) = plan.grid;
    let cell = index % (rows * cols) + 1;
    let previous = made.iter().rev().find_map(|m| match m {
        Made::Flat(ax) => Some(ax),
        Made::Deep(_) => None,
    });
    let ax = match (&spec.kind, previous) {
        (Kind::ThreeD, _) => {
            return fig
                .add_subplot_3d(rows, cols, cell, Axes3DOptions::default())
                .ok()
                .map(Made::Deep);
        }
        (Kind::Projected(projection), _) => fig
            .add_subplot_with_projection(rows, cols, cell, *projection)
            .ok()?,
        (Kind::Inset(bounds), Some(parent)) => parent.inset_axes(*bounds),
        (Kind::TwinX, Some(parent)) => parent.twinx().ok()?,
        (Kind::TwinY, Some(parent)) => parent.twiny().ok()?,
        (Kind::SharedX, Some(parent)) => {
            let ax = fig.add_subplot(rows, cols, cell).ok()?;
            let _ = ax.share_x(parent);
            ax
        }
        _ => fig.add_subplot(rows, cols, cell).ok()?,
    };
    let _ = ax.set_xscale(spec.scales.0);
    let _ = ax.set_yscale(spec.scales.1);
    ax.xaxis().set_inverted(spec.inverted.0);
    ax.yaxis().set_inverted(spec.inverted.1);
    if let Some(ratio) = spec.aspect {
        let _ = ax.set_aspect(plt::rc::Aspect::Ratio(ratio));
    }
    if let Some((x, y)) = spec.margins {
        let _ = ax.set_margins(x, y);
    }
    Some(Made::Flat(ax))
}

impl Step {
    /// Runs the step on its axes (modulo the axes `made`), if they are of
    /// the kind it needs.
    fn run(&self, fig: &Figure, made: &[Made], mappables: &mut Mappables) {
        let pick = |at: usize| &made[at % made.len()];
        match self {
            Self::Plot(at, call) => {
                if let Made::Flat(ax) = pick(*at) {
                    call.run(ax, mappables);
                }
            }
            Self::Stat(at, call) => {
                if let Made::Flat(ax) = pick(*at) {
                    call.run(ax);
                }
            }
            Self::Field(at, call) => {
                if let Made::Flat(ax) = pick(*at) {
                    call.run(ax, mappables);
                }
            }
            Self::ThreeD(at, call) => {
                if let Made::Deep(ax) = pick(*at) {
                    call.run(ax);
                }
            }
            Self::Decorate(at, decor) => {
                if let Made::Flat(ax) = pick(*at) {
                    decor.run(fig, ax, mappables);
                }
            }
            Self::DecorateAxis(at, decor) => {
                if let Made::Flat(ax) = pick(*at) {
                    decor.run(ax);
                }
            }
        }
    }
}

/// Draws the plan and saves it.
fn draw(plan: &Plan) {
    let fig = Figure::with_size(plan.size.0, plan.size.1, plan.dpi);
    match plan.layout {
        1 => drop(fig.set_layout_engine(LayoutEngine::Constrained(ConstrainedLayout::default()))),
        2 => drop(fig.set_layout_engine(LayoutEngine::Tight(TightLayout::default()))),
        _ => {}
    }
    let mut made = Vec::new();
    for (index, spec) in plan.axes.iter().enumerate() {
        if let Some(axes) = make_axes(&fig, plan, index, spec, &made) {
            made.push(axes);
        }
    }
    if made.is_empty() {
        return;
    }
    let mut mappables = Mappables::default();
    for step in &plan.steps {
        step.run(&fig, &made, &mut mappables);
    }
    if plan.layout == 3 {
        let _ = fig.tight_layout(&TightLayout::default());
    }
    let options = SaveOptions {
        dpi: plan.save_dpi,
        bbox_inches: plan.tight.then_some(BboxInches::Tight),
        ..SaveOptions::default()
    };
    let _ = fig.to_format(plan.format, options);
}

#[test]
fn random_plots() {
    fuzz("plotting", 250, Plan::generate, draw);
}
