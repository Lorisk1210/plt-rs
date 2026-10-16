//! Violin plots (matplotlib's `violinplot` and `violin`): kernel density
//! estimates drawn as filled bodies with lines at the extrema, means,
//! medians and quantiles.

use plt_render::CapStyle;

use super::fill_between::FillBetweenOptions;
use super::{Axes, PlotError};
use crate::collections::{Collection, CollectionProps, EdgeColors};
use crate::colors::Color;
use crate::orientation::Orientation;
use crate::stats::{ViolinOptions, ViolinStats, violin_stats};

/// Which half of each violin is drawn (matplotlib's `side`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ViolinSide {
    /// Both halves.
    #[default]
    Both,
    /// The half towards lower positions.
    Low,
    /// The half towards higher positions.
    High,
}

impl ViolinSide {
    /// The offsets of the line ends from the position, in widths.
    fn line_ends(self) -> (f64, f64) {
        match self {
            Self::Both => (-0.25, 0.25),
            Self::Low => (-0.25, 0.0),
            Self::High => (0.0, 0.25),
        }
    }
}

/// Options of [`Axes::violin`].
#[derive(Clone, Debug, PartialEq)]
pub struct ViolinDrawOptions {
    /// Violin positions (default `1, 2, ..., n`).
    pub positions: Option<Vec<f64>>,
    /// Vertical violins stand on the x axis; horizontal ones on the y axis.
    pub orientation: Orientation,
    /// Maximal widths: one for all or one per violin.
    pub widths: Vec<f64>,
    /// Draw a line at each mean.
    pub showmeans: bool,
    /// Draw lines at the extrema and a bar between them.
    pub showextrema: bool,
    /// Draw a line at each median.
    pub showmedians: bool,
    /// Which half of each violin is drawn.
    pub side: ViolinSide,
    /// Body colours, cycled (empty: the next line-cycle colour at 0.3
    /// opacity).
    pub facecolors: Vec<Color>,
    /// Line colours, cycled (empty: the next line-cycle colour).
    pub linecolors: Vec<Color>,
}

impl Default for ViolinDrawOptions {
    fn default() -> Self {
        Self {
            positions: None,
            orientation: Orientation::Vertical,
            widths: vec![0.5],
            showmeans: false,
            showextrema: true,
            showmedians: false,
            side: ViolinSide::Both,
            facecolors: Vec::new(),
            linecolors: Vec::new(),
        }
    }
}

/// Options of [`Axes::violinplot`]: the density estimate and how
/// [`Axes::violin`] draws it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViolinplotOptions {
    /// Kernel bandwidth, evaluation points and quantiles.
    pub stats: ViolinOptions,
    /// How the statistics are drawn.
    pub draw: ViolinDrawOptions,
}

/// The artists of a violin plot (matplotlib's `violinplot` result).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ViolinParts {
    /// One filled body per violin.
    pub bodies: Vec<Collection>,
    /// Lines at the means.
    pub cmeans: Option<Collection>,
    /// Lines at the maxima.
    pub cmaxes: Option<Collection>,
    /// Lines at the minima.
    pub cmins: Option<Collection>,
    /// Bars from minimum to maximum.
    pub cbars: Option<Collection>,
    /// Lines at the medians.
    pub cmedians: Option<Collection>,
    /// Lines at the quantiles.
    pub cquantiles: Option<Collection>,
}

impl Axes {
    /// Draws a violin per dataset from Gaussian kernel density estimates
    /// (matplotlib's `violinplot`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::axes::ViolinplotOptions;
    /// let ax = Figure::new().subplot();
    /// let parts = ax.violinplot(&[vec![1.0, 2.0, 2.5, 4.0]], &ViolinplotOptions::default())?;
    /// assert_eq!(parts.bodies.len(), 1);
    /// assert!(parts.cbars.is_some());
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the density cannot be estimated or the per-violin options do
    /// not match the datasets.
    pub fn violinplot<D: AsRef<[f64]>>(
        &self,
        datasets: &[D],
        options: &ViolinplotOptions,
    ) -> Result<ViolinParts, PlotError> {
        let stats = violin_stats(datasets, &options.stats).map_err(PlotError::Stats)?;
        self.violin(&stats, &options.draw)
    }

    /// Draws violins from precomputed statistics (matplotlib's `violin`).
    ///
    /// # Errors
    ///
    /// When `positions` or `widths` hold neither one value (widths) nor
    /// one per violin.
    pub fn violin(
        &self,
        stats: &[ViolinStats],
        options: &ViolinDrawOptions,
    ) -> Result<ViolinParts, PlotError> {
        let n = stats.len();
        let positions = match &options.positions {
            Some(p) if p.len() != n => {
                return Err(PlotError::DatasetCount {
                    argument: "positions",
                    len: p.len(),
                    datasets: n,
                });
            }
            Some(p) => p.clone(),
            None => (1..=n).map(|i| i as f64).collect(),
        };
        let widths = match options.widths.as_slice() {
            [width] => vec![*width; n],
            w if w.len() == n => w.to_vec(),
            w => {
                return Err(PlotError::DatasetCount {
                    argument: "widths",
                    len: w.len(),
                    datasets: n,
                });
            }
        };
        let next = (options.facecolors.is_empty() || options.linecolors.is_empty())
            .then(|| self.next_line_color());
        let faces = cycled(&options.facecolors, next.as_ref(), n);
        let lines = cycled(&options.linecolors, next.as_ref(), n);
        let body_alpha = options.facecolors.is_empty().then_some(0.3);

        let mut parts = ViolinParts::default();
        for (((stats, &pos), &width), face) in stats.iter().zip(&positions).zip(&widths).zip(faces)
        {
            parts
                .bodies
                .push(self.violin_body(stats, pos, width, face, body_alpha, options)?);
        }
        let (low, high) = options.side.line_ends();
        let starts: Vec<f64> = positions
            .iter()
            .zip(&widths)
            .map(|(p, w)| low * w + p)
            .collect();
        let ends: Vec<f64> = positions
            .iter()
            .zip(&widths)
            .map(|(p, w)| high * w + p)
            .collect();
        let painter = LinePainter {
            axes: self,
            orientation: options.orientation,
            colors: &lines,
            projecting: options.side != ViolinSide::Both,
        };
        let field = |f: fn(&ViolinStats) -> f64| -> Vec<f64> { stats.iter().map(f).collect() };
        if options.showmeans {
            parts.cmeans = Some(painter.across(&field(|s| s.mean), &starts, &ends)?);
        }
        if options.showextrema {
            parts.cmaxes = Some(painter.across(&field(|s| s.max), &starts, &ends)?);
            parts.cmins = Some(painter.across(&field(|s| s.min), &starts, &ends)?);
            parts.cbars = Some(painter.along(&positions, &field(|s| s.min), &field(|s| s.max))?);
        }
        if options.showmedians {
            parts.cmedians = Some(painter.across(&field(|s| s.median), &starts, &ends)?);
        }
        let quantiles: Vec<f64> = stats
            .iter()
            .flat_map(|s| s.quantiles.iter().copied())
            .collect();
        if !quantiles.is_empty() {
            let repeat = |ends: &[f64]| -> Vec<f64> {
                stats
                    .iter()
                    .zip(ends)
                    .flat_map(|(s, &e)| std::iter::repeat_n(e, s.quantiles.len()))
                    .collect()
            };
            parts.cquantiles =
                Some(painter.across(&quantiles, &repeat(&starts), &repeat(&ends))?);
        }
        Ok(parts)
    }

    /// The filled density of one violin, scaled to `width`.
    fn violin_body(
        &self,
        stats: &ViolinStats,
        pos: f64,
        width: f64,
        face: Color,
        alpha: Option<f64>,
        options: &ViolinDrawOptions,
    ) -> Result<Collection, PlotError> {
        let peak = stats.vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let vals: Vec<f64> = stats.vals.iter().map(|v| 0.5 * width * v / peak).collect();
        let side = |v: f64, outward: bool| if outward { pos + v } else { pos };
        let low: Vec<f64> = vals
            .iter()
            .map(|&v| side(-v, options.side != ViolinSide::High))
            .collect();
        let high: Vec<f64> = vals
            .iter()
            .map(|&v| side(v, options.side != ViolinSide::Low))
            .collect();
        let mut props = CollectionProps::default().facecolor(face);
        if let Some(alpha) = alpha {
            props = props.alpha(alpha);
        }
        let fill = FillBetweenOptions {
            props,
            ..FillBetweenOptions::default()
        };
        match options.orientation {
            Orientation::Vertical => self.fill_betweenx(&stats.coords, low, high, fill),
            Orientation::Horizontal => self.fill_between(&stats.coords, low, high, fill),
        }
    }
}

/// `colors` cycled over `n` violins, or `fallback` for all when empty.
fn cycled(colors: &[Color], fallback: Option<&Color>, n: usize) -> Vec<Color> {
    match (colors, fallback) {
        ([], Some(color)) => vec![color.clone(); n],
        _ => (0..n)
            .map(|i| {
                colors
                    .get(i % colors.len().max(1))
                    .cloned()
                    .unwrap_or_else(Color::none)
            })
            .collect(),
    }
}

/// Draws the statistic lines of a violin plot in its colours.
struct LinePainter<'a> {
    axes: &'a Axes,
    orientation: Orientation,
    colors: &'a [Color],
    /// Half violins use projecting caps.
    projecting: bool,
}

impl LinePainter<'_> {
    /// Lines across the violins at `values`, from `starts` to `ends`.
    fn across(
        &self,
        values: &[f64],
        starts: &[f64],
        ends: &[f64],
    ) -> Result<Collection, PlotError> {
        let lines = match self.orientation {
            Orientation::Vertical => self.axes.hlines(values, starts, ends)?,
            Orientation::Horizontal => self.axes.vlines(values, starts, ends)?,
        };
        Ok(self.style(lines))
    }

    /// Bars along the violins at `positions`, from `lows` to `highs`.
    fn along(
        &self,
        positions: &[f64],
        lows: &[f64],
        highs: &[f64],
    ) -> Result<Collection, PlotError> {
        let bars = match self.orientation {
            Orientation::Vertical => self.axes.vlines(positions, lows, highs)?,
            Orientation::Horizontal => self.axes.hlines(positions, lows, highs)?,
        };
        Ok(self.style(bars))
    }

    fn style(&self, lines: Collection) -> Collection {
        lines.set_edgecolors(EdgeColors::Colors(self.colors.to_vec()));
        if self.projecting {
            lines.set_capstyle(CapStyle::Projecting);
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn data() -> Vec<Vec<f64>> {
        vec![vec![1.0, 2.0, 2.0, 3.0, 5.0], vec![0.0, 1.0, 4.0]]
    }

    #[test]
    fn default_parts_and_colours() {
        let ax = Figure::new().subplot();
        let parts = ax
            .violinplot(&data(), &ViolinplotOptions::default())
            .unwrap();
        assert_eq!(parts.bodies.len(), 2);
        assert!(parts.cmeans.is_none() && parts.cmedians.is_none());
        assert!(parts.cbars.is_some() && parts.cquantiles.is_none());
        // One line-cycle colour for faces and lines.
        let next = ax.plot([0.0], [0.0]).unwrap().rgba().unwrap();
        assert_eq!(next, Color::from("C1").resolve().unwrap());
    }

    #[test]
    fn line_ends_follow_the_side() {
        assert_eq!(ViolinSide::Both.line_ends(), (-0.25, 0.25));
        assert_eq!(ViolinSide::Low.line_ends(), (-0.25, 0.0));
        assert_eq!(ViolinSide::High.line_ends(), (0.0, 0.25));
    }

    #[test]
    fn quantile_lines_repeat_the_ends() {
        let ax = Figure::new().subplot();
        let options = ViolinplotOptions {
            stats: ViolinOptions {
                quantiles: Some(vec![vec![0.25, 0.75], vec![0.5]]),
                ..ViolinOptions::default()
            },
            ..ViolinplotOptions::default()
        };
        let parts = ax.violinplot(&data(), &options).unwrap();
        let quantiles = parts.cquantiles.unwrap();
        assert_eq!(quantiles.paths().len(), 3);
    }

    #[test]
    fn mismatched_widths_are_rejected() {
        let ax = Figure::new().subplot();
        let options = ViolinplotOptions {
            draw: ViolinDrawOptions {
                widths: vec![0.1, 0.2, 0.3],
                ..ViolinDrawOptions::default()
            },
            ..ViolinplotOptions::default()
        };
        assert!(ax.violinplot(&data(), &options).is_err());
    }
}
