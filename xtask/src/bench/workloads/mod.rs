//! The benchmark workloads. Each draws the figure of a script in
//! `benchmarks/python` (matplotlib's syntax) from the same data, and saves
//! it in one format: the script and the workload must draw the same
//! figure, which the report's pixel comparison checks.

mod calls;
mod everyday;
mod fields;
mod plots;
mod text;

use pltrs::figure::{Figure, SaveFormat};
use pltrs::numeric::{Grid, linspace};
use pltrs_spec::figures::CaseResult;
use pltrs_spec::rng::SpecRng;

/// The resolution every workload is drawn and saved at (matplotlib's
/// default `figure.dpi`, which `savefig` keeps).
pub(super) const DPI: f64 = 100.0;

/// Populates a figure from data made before the clock started.
pub(super) type Build = Box<dyn Fn(&Figure) -> CaseResult>;

/// What a workload does with the figure it built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Which table of the report a workload belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Tier {
    /// Everyday figures of a few artists, where start-up and the cost of
    /// each call weigh most.
    Small,
    /// Figures of thousands to tens of thousands of points.
    Medium,
    /// Many small calls, one artist each.
    Calls,
    /// Millions of points, large images, heavy text and layout.
    Large,
}

impl Tier {
    /// Every tier, in report order.
    pub(super) const ALL: [Self; 4] = [Self::Small, Self::Medium, Self::Calls, Self::Large];

    /// The tier's heading in the report.
    pub(super) const fn title(self) -> &'static str {
        match self {
            Self::Small => "Small, everyday figures",
            Self::Medium => "Medium figures",
            Self::Calls => "Many small calls",
            Self::Large => "Large workloads",
        }
    }
}

/// The matplotlib-syntax script of a workload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Script {
    /// `benchmarks/python/<name>.py` draws the same figure.
    Python(&'static str),
    /// matplotlib cannot be timed doing the same work, for this reason.
    NotComparable(&'static str),
}

/// A named figure to build and save.
#[derive(Clone, Copy, Debug)]
pub(super) struct Workload {
    /// `group/name`.
    pub(super) name: &'static str,
    /// The script drawing the same figure in Python.
    pub(super) script: Script,
    /// The report table it belongs to.
    pub(super) tier: Tier,
    /// Figure size in inches.
    pub(super) figsize: (f64, f64),
    /// What becomes of the figure once built.
    pub(super) output: Output,
    /// rcParams set while the figure is built and saved.
    pub(super) rc: &'static [(&'static str, &'static str)],
    /// Makes the data (untimed) and returns what builds the figure.
    pub(super) setup: fn() -> Build,
    /// How many plotting and setter calls build the figure, for a
    /// workload of many small calls.
    pub(super) calls: Option<u32>,
}

impl Workload {
    /// A large workload drawing the figure of `benchmarks/python/<script>.py`,
    /// saved as PNG on the default 6.4 × 4.8 in canvas.
    const fn new(name: &'static str, script: &'static str, setup: fn() -> Build) -> Self {
        Self {
            name,
            script: Script::Python(script),
            tier: Tier::Large,
            figsize: (6.4, 4.8),
            output: Output::Save {
                format: SaveFormat::Png,
                tight: false,
            },
            rc: &[],
            setup,
            calls: None,
        }
    }

    /// The same workload, built by `count` plotting and setter calls.
    const fn calls(self, count: u32) -> Self {
        Self {
            calls: Some(count),
            ..self
        }
    }

    /// The same workload in another table of the report.
    const fn in_tier(self, tier: Tier) -> Self {
        Self { tier, ..self }
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

    /// The same workload without a matplotlib counterpart, for `reason`.
    const fn not_comparable(self, reason: &'static str) -> Self {
        Self {
            script: Script::NotComparable(reason),
            ..self
        }
    }

    /// What the pixel comparison saves: the figure as PNG, cropped as the
    /// workload crops it.
    pub(super) const fn png_output(&self) -> Output {
        let tight = matches!(self.output, Output::Save { tight: true, .. });
        Output::Save {
            format: SaveFormat::Png,
            tight,
        }
    }

    /// The file extension of what the workload saves (`None` when it only
    /// lays the figure out).
    pub(super) fn extension(&self) -> Option<&'static str> {
        match self.output {
            Output::Save { format, .. } => Some(format.extension()),
            Output::LayoutOnly => None,
        }
    }
}

/// Every workload, in table order.
pub(super) fn all() -> impl Iterator<Item = &'static Workload> {
    everyday::WORKLOADS
        .iter()
        .chain(calls::WORKLOADS)
        .chain(plots::WORKLOADS)
        .chain(fields::WORKLOADS)
        .chain(text::WORKLOADS)
}

/// The workload called `name`.
pub(super) fn named(name: &str) -> Option<&'static Workload> {
    all().find(|workload| workload.name == name)
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

/// `sin(x) cos(y) + x / 10` on an `n × n` grid over `[-10, 10]²`, with
/// one sine and one cosine per row and column, as numpy broadcasts it.
fn smooth_field(n: usize) -> Grid<f64> {
    let t = linspace(-10.0, 10.0, n);
    let sines: Vec<f64> = t.iter().map(|t| t.sin()).collect();
    let cosines: Vec<f64> = t.iter().map(|t| t.cos()).collect();
    Grid::from_fn(n, n, |i, j| sines[j] * cosines[i] + t[j] / 10.0)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;

    use super::*;

    #[test]
    fn names_are_unique_and_grouped() {
        let mut seen = HashSet::new();
        for workload in all() {
            assert!(seen.insert(workload.name), "{} twice", workload.name);
            assert!(
                workload.name.contains('/'),
                "{} has no group",
                workload.name
            );
        }
    }

    #[test]
    fn every_script_exists() {
        let scripts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../benchmarks/python");
        for workload in all() {
            if let Script::Python(script) = workload.script {
                let path = scripts.join(format!("{script}.py"));
                assert!(path.exists(), "{}: no {}", workload.name, path.display());
            }
        }
    }

    #[test]
    fn the_pixel_comparison_keeps_the_crop() {
        let tight = named("output/mid_png_tight").expect("a tight workload");
        assert_eq!(
            tight.png_output(),
            Output::Save {
                format: SaveFormat::Png,
                tight: true
            }
        );
        let layout = named("layout/constrained_8x8_no_draw").expect("a layout workload");
        assert_eq!(layout.extension(), None);
        assert!(matches!(
            layout.png_output(),
            Output::Save { tight: false, .. }
        ));
    }
}
