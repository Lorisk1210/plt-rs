//! The benchmark workloads. Each mirrors a function of the same name in
//! `spec/reference/bench/bench.py`; the two must build the same figure
//! from the same data.

mod fields;
mod plots;
mod text;

use plt::figure::{Figure, SaveFormat};
use plt::numeric::{Grid, linspace};
use plt_spec::figures::CaseResult;
use plt_spec::rng::SpecRng;

/// The resolution every workload is drawn and saved at.
pub(super) const DPI: f64 = 100.0;

/// Populates a figure from data made before the clock started.
pub(super) type Build = Box<dyn Fn(&Figure) -> CaseResult>;

/// What a workload does with the figure it built.
#[derive(Clone, Copy, Debug)]
pub(super) enum Output {
    /// Saves the figure in `format`, cropped to the tight bounding box of
    /// what it draws when `tight` (`bbox_inches="tight"`).
    Save {
        /// The format the figure is saved in.
        format: SaveFormat,
        /// Whether the saved area is the tight bounding box.
        tight: bool,
    },
    /// Lays the figure out without drawing it (matplotlib's
    /// `draw_without_rendering`).
    LayoutOnly,
}

/// A named figure to build and save.
#[derive(Clone, Copy, Debug)]
pub(super) struct Workload {
    /// `group/name`, as `bench.py` names it.
    pub(super) name: &'static str,
    /// Figure size in inches.
    pub(super) figsize: (f64, f64),
    /// What becomes of the figure once built.
    pub(super) output: Output,
    /// rcParams set while the figure is built and saved.
    pub(super) rc: &'static [(&'static str, &'static str)],
    /// Makes the data (untimed) and returns what builds the figure.
    pub(super) setup: fn() -> Build,
}

impl Workload {
    /// A workload saved as PNG on the default 6.4 × 4.8 in canvas.
    const fn new(name: &'static str, setup: fn() -> Build) -> Self {
        Self {
            name,
            figsize: (6.4, 4.8),
            output: Output::Save {
                format: SaveFormat::Png,
                tight: false,
            },
            rc: &[],
            setup,
        }
    }

    /// The same workload saved in another format.
    const fn saved_as(self, format: SaveFormat) -> Self {
        Self {
            output: Output::Save {
                format,
                tight: false,
            },
            ..self
        }
    }

    /// The same workload saved cropped to its tight bounding box.
    const fn tight(self) -> Self {
        let Output::Save { format, .. } = self.output else {
            return self;
        };
        Self {
            output: Output::Save {
                format,
                tight: true,
            },
            ..self
        }
    }

    /// The same workload laid out but neither drawn nor saved.
    const fn layout_only(self) -> Self {
        Self {
            output: Output::LayoutOnly,
            ..self
        }
    }

    /// The same workload under further rcParams.
    const fn with_rc(self, rc: &'static [(&'static str, &'static str)]) -> Self {
        Self { rc, ..self }
    }

    /// The same workload on another canvas.
    const fn sized(self, width: f64, height: f64) -> Self {
        Self {
            figsize: (width, height),
            ..self
        }
    }
}

/// Every workload, in table order.
pub(super) fn all() -> impl Iterator<Item = &'static Workload> {
    plots::WORKLOADS
        .iter()
        .chain(fields::WORKLOADS)
        .chain(text::WORKLOADS)
}

/// `n` points of a Gaussian random walk against `0..n`.
fn random_walk(n: usize, seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut level = 0.0;
    let walk = SpecRng::new(seed)
        .normal(n, 0.0, 1.0)
        .into_iter()
        .map(|step| {
            level += step;
            level
        })
        .collect();
    ((0..n).map(|i| i as f64).collect(), walk)
}

/// `sin(x) cos(y) + x / 10` on an `n × n` grid over `[-10, 10]²`.
fn smooth_field(n: usize) -> Grid<f64> {
    let t = linspace(-10.0, 10.0, n);
    Grid::from_fn(n, n, |i, j| t[j].sin() * t[i].cos() + t[j] / 10.0)
}
