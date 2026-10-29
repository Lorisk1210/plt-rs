//! Box-and-whisker plots from raw data (matplotlib's `boxplot`).

use super::bxp::{BoxProps, BoxplotArtists, BxpOptions};
use super::{Axes, PlotError};
use crate::colors::Color;
use crate::lines::{FormatString, LineStyle};
use crate::markers::Marker;
use crate::rc::{ColorOrAuto, RcParams};
use crate::stats::{Bootstrap, BoxStats, Whis, boxplot_stats};

/// Options of [`Axes::boxplot`]: the statistics, plus how [`Axes::bxp`]
/// draws them. Unset switches and whiskers come from the `boxplot.*` rc
/// parameters.
#[derive(Clone, Debug, Default)]
pub struct BoxplotOptions {
    /// Whisker reach (default `boxplot.whiskers`).
    pub whis: Option<Whis>,
    /// Extend zero-IQR whiskers to the data range.
    pub autorange: bool,
    /// Notches from a bootstrap of the median, drawn from a seeded
    /// generator rather than numpy's (see [`Bootstrap`]); by default
    /// `boxplot.bootstrap` resamples drawn with seed 0, or the Gaussian
    /// approximation when that is `None`.
    pub bootstrap: Option<Bootstrap>,
    /// One tick label per dataset (default: the positions).
    pub tick_labels: Option<Vec<String>>,
    /// Outlier format string (matplotlib's `sym`): its marker and colour
    /// restyle the outliers; an empty string hides them.
    pub sym: Option<String>,
    /// Medians replacing the computed ones (`None` keeps a computed one).
    pub usermedians: Option<Vec<Option<f64>>>,
    /// Notch intervals replacing the computed ones, per end.
    pub conf_intervals: Option<Vec<(Option<f64>, Option<f64>)>>,
    /// How the statistics are drawn.
    pub draw: BxpOptions,
}

impl Axes {
    /// Draws one box per dataset: the quartiles, whiskers, median and
    /// outliers (matplotlib's `boxplot`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::BoxplotOptions;
    /// let ax = Figure::new().subplot();
    /// let parts = ax.boxplot(&[vec![1.0, 2.0, 3.0, 4.0, 20.0]], BoxplotOptions::default())?;
    /// assert_eq!(parts.fliers[0].ydata(), [20.0]);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When per-dataset options do not match the number of datasets,
    /// `sym` is not a valid format string, or a bootstrap has no resamples
    /// or too many ([`MAX_BOOTSTRAP_DRAWS`](crate::stats::MAX_BOOTSTRAP_DRAWS)).
    pub fn boxplot<D: AsRef<[f64]>>(
        &self,
        datasets: &[D],
        options: BoxplotOptions,
    ) -> Result<BoxplotArtists, PlotError> {
        let rc = crate::rc::params();
        let stats_options = crate::stats::BoxplotStatsOptions {
            whis: options.whis.unwrap_or_else(|| rc.boxplot_whiskers.into()),
            labels: options.tick_labels.clone(),
            autorange: options.autorange,
            bootstrap: options.bootstrap.or_else(|| {
                rc.boxplot_bootstrap.map(|resamples| Bootstrap {
                    // A negative count has no resamples.
                    resamples: usize::try_from(resamples).unwrap_or(0),
                    seed: 0,
                })
            }),
        };
        let mut stats = boxplot_stats(datasets, stats_options).map_err(PlotError::Stats)?;
        override_stats(&mut stats, &options)?;
        let draw = resolved_draw_options(&options, &rc)?;
        self.bxp(&stats, draw)
    }
}

/// Applies `usermedians` and `conf_intervals`.
fn override_stats(stats: &mut [BoxStats], options: &BoxplotOptions) -> Result<(), PlotError> {
    let n = stats.len();
    let check = |argument: &'static str, len: usize| {
        if len == n {
            Ok(())
        } else {
            Err(PlotError::DatasetCount {
                argument,
                len,
                datasets: n,
            })
        }
    };
    if let Some(medians) = &options.usermedians {
        check("usermedians", medians.len())?;
        for (box_stats, median) in stats.iter_mut().zip(medians) {
            if let Some(median) = median {
                box_stats.med = *median;
            }
        }
    }
    if let Some(intervals) = &options.conf_intervals {
        check("conf_intervals", intervals.len())?;
        for (box_stats, (low, high)) in stats.iter_mut().zip(intervals) {
            if let Some(low) = low {
                box_stats.cilo = *low;
            }
            if let Some(high) = high {
                box_stats.cihi = *high;
            }
        }
    }
    Ok(())
}

/// The drawing options with matplotlib's `boxplot` rules applied: rc
/// defaults for unset switches, solid patch outlines whose `color` is the
/// edge colour, and the outlier style from `sym`.
fn resolved_draw_options(options: &BoxplotOptions, rc: &RcParams) -> Result<BxpOptions, PlotError> {
    let mut draw = options.draw.clone();
    draw.shownotches = Some(draw.shownotches.unwrap_or(rc.boxplot_notch));
    draw.meanline = Some(draw.meanline.unwrap_or(rc.boxplot_meanline));
    draw.showmeans = Some(draw.showmeans.unwrap_or(rc.boxplot_showmeans));
    draw.showcaps = Some(draw.showcaps.unwrap_or(rc.boxplot_showcaps));
    draw.showbox = Some(draw.showbox.unwrap_or(rc.boxplot_showbox));
    draw.showfliers = Some(draw.showfliers.unwrap_or(rc.boxplot_showfliers));
    draw.boxes = Some(match draw.boxes.take() {
        Some(BoxProps::Patch(mut props)) => {
            if let Some(color) = props.color.take() {
                props.edgecolor = Some(color);
            }
            BoxProps::Patch(props.linestyle(LineStyle::Solid))
        }
        Some(lines @ BoxProps::Line(_)) => lines,
        None if rc.boxplot_patchartist => {
            BoxProps::Patch(crate::patches::PatchProps::default().linestyle(LineStyle::Solid))
        }
        None => BoxProps::Line(crate::lines::LineProps::default()),
    });
    match options.sym.as_deref() {
        None => {}
        Some("") => {
            draw.flierprops = draw
                .flierprops
                .linestyle(LineStyle::None)
                .marker(Marker::default())
                .color(Color::none());
            draw.showfliers = Some(false);
        }
        Some(sym) => {
            let format = FormatString::parse_with(sym, rc)?;
            if let Some(marker) = format.marker {
                draw.flierprops = draw.flierprops.marker(marker);
            }
            if let Some(color) = format.color {
                draw.flierprops = draw
                    .flierprops
                    .color(color.clone())
                    .markerfacecolor(ColorOrAuto::Color(color.clone()))
                    .markeredgecolor(ColorOrAuto::Color(color));
            }
        }
    }
    Ok(draw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::{AxisName, BoxArtist};
    use crate::figure::Figure;
    use crate::markers::MarkerSymbol;

    fn data() -> Vec<Vec<f64>> {
        vec![
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 20.0],
            vec![2.0, 3.0, 3.5, 4.0, 6.0],
        ]
    }

    #[test]
    fn default_boxes_and_ticks() {
        let ax = Figure::new().subplot();
        let parts = ax.boxplot(&data(), BoxplotOptions::default()).unwrap();
        assert_eq!(parts.whiskers.len(), 4);
        assert_eq!(parts.caps.len(), 4);
        assert_eq!(parts.medians.len(), 2);
        assert_eq!(parts.means.len(), 0);
        assert!(matches!(parts.boxes[0], BoxArtist::Line(_)));
        assert_eq!(parts.fliers[0].ydata(), [20.0]);
        assert_eq!(parts.medians[0].color(), Color::from("C1"));
        assert_eq!(parts.medians[0].zorder(), 2.1);
        ax.set_margins(0.0, 0.05).unwrap();
        assert_eq!(ax.xlim(), (0.5, 2.5));
        let ticks = ax.ticks(AxisName::X).unwrap();
        let labels: Vec<&str> = ticks.major.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels, ["1", "2"]);
    }

    #[test]
    fn positions_label_their_ticks_as_given() {
        let labels = |integer_positions: bool| {
            let ax = Figure::new().subplot();
            let options = BoxplotOptions {
                draw: BxpOptions {
                    positions: Some(vec![2.0, 4.0]),
                    integer_positions,
                    ..BxpOptions::default()
                },
                ..BoxplotOptions::default()
            };
            ax.boxplot(&data(), options).unwrap();
            let ticks = ax.ticks(AxisName::X).unwrap();
            ticks
                .major
                .iter()
                .map(|t| t.label.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(labels(false), ["2.0", "4.0"], "floats");
        assert_eq!(labels(true), ["2", "4"], "integers");
    }

    #[test]
    fn patch_boxes_take_colour_as_edge() {
        let ax = Figure::new().subplot();
        let options = BoxplotOptions {
            draw: BxpOptions {
                boxes: Some(BoxProps::Patch(
                    crate::patches::PatchProps::default().color("r"),
                )),
                ..BxpOptions::default()
            },
            ..BoxplotOptions::default()
        };
        let parts = ax.boxplot(&data(), options).unwrap();
        let BoxArtist::Patch(patch) = &parts.boxes[0] else {
            panic!("patch boxes");
        };
        let style = patch.style();
        assert_eq!(style.edgecolor, Some(Color::from("r")));
        assert_eq!(style.facecolor, Some(Color::from("C0")));
    }

    #[test]
    fn sym_restyles_or_hides_outliers() {
        let ax = Figure::new().subplot();
        let options = BoxplotOptions {
            sym: Some("r+".into()),
            ..BoxplotOptions::default()
        };
        let parts = ax.boxplot(&data(), options).unwrap();
        assert_eq!(parts.fliers[0].marker(), Marker::Symbol(MarkerSymbol::Plus));
        assert_eq!(parts.fliers[0].color(), Color::from("r"));
        let hidden = BoxplotOptions {
            sym: Some(String::new()),
            ..BoxplotOptions::default()
        };
        assert_eq!(ax.boxplot(&data(), hidden).unwrap().fliers.len(), 0);
    }

    #[test]
    fn bootstrapped_notches_come_from_the_option_or_rc() {
        let notches = |bootstrap| {
            let ax = Figure::new().subplot();
            let options = BoxplotOptions {
                bootstrap,
                draw: BxpOptions {
                    shownotches: Some(true),
                    ..BxpOptions::default()
                },
                ..BoxplotOptions::default()
            };
            let parts = ax.boxplot(&data(), options)?;
            let BoxArtist::Line(outline) = &parts.boxes[0] else {
                panic!("line boxes");
            };
            // The notch ends are the third and fifth y of the outline.
            let y = outline.ydata();
            Ok::<_, PlotError>((y[2], y[4]))
        };
        let gaussian = notches(None).unwrap();
        let seeded = Some(Bootstrap {
            resamples: 300,
            seed: 5,
        });
        let bootstrapped = notches(seeded).unwrap();
        assert_ne!(bootstrapped, gaussian);
        assert!(bootstrapped.0 >= 1.0 && bootstrapped.1 <= 20.0);
        assert_eq!(notches(seeded).unwrap(), bootstrapped, "seeded");
        let from_rc = crate::rc::context(|| {
            crate::rc::update(|rc| rc.boxplot_bootstrap = Some(300));
            notches(None).unwrap()
        });
        let seed_zero = Some(Bootstrap {
            resamples: 300,
            seed: 0,
        });
        assert_eq!(from_rc, notches(seed_zero).unwrap());
        let negative = crate::rc::context(|| {
            crate::rc::update(|rc| rc.boxplot_bootstrap = Some(-1));
            notches(None)
        });
        assert!(matches!(
            negative,
            Err(PlotError::Stats(crate::stats::StatsError::NoResamples))
        ));
    }

    #[test]
    fn overrides_must_match_the_datasets() {
        let ax = Figure::new().subplot();
        let options = BoxplotOptions {
            usermedians: Some(vec![Some(3.0)]),
            ..BoxplotOptions::default()
        };
        assert!(ax.boxplot(&data(), options).is_err());
        let options = BoxplotOptions {
            usermedians: Some(vec![Some(10.0), None]),
            ..BoxplotOptions::default()
        };
        let parts = ax.boxplot(&data(), options).unwrap();
        assert_eq!(parts.medians[0].ydata(), [10.0, 10.0]);
    }
}
