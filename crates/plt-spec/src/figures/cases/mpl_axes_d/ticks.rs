//! Tick labels, rc-driven spines and grids, dash offsets, log axes and
//! the drawing order of axis and artists, plus small line tests.

use plt::axes::{AxisName, GridOptions, StairsOptions, TickLabelProps, TickLevel, TickParams};
use plt::figure::{Figure, Share};
use plt::fonts::FontWeight;
use plt::lines::{LineProps, LineStyle};
use plt::patches::{Circle, PatchProps};
use plt::prelude::{ArtistHandle, arange, linspace};
use plt::rc::{AxisBelow, ColorOrAuto};
use plt::scale::{NonPositive, ScaleOptions};

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// `test_set_get_ticklabels`: labels set on one axes and copied from it
/// to another.
fn set_get_ticklabels(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?.into_vec();
    for (ax, title) in axs.iter().zip(["normal", "set_x/yticklabels"]) {
        ax.plot_y(arange(0.0, 10.0, 1.0));
        ax.set_title(title);
    }
    let ticks = arange(0.0, 10.0, 1.0);
    let pad = |labels: [&str; 4]| -> Vec<String> {
        labels
            .into_iter()
            .chain(std::iter::repeat_n("", 6))
            .map(String::from)
            .collect()
    };
    axs[0].set_xticks(ticks.clone()).set_yticks(ticks);
    axs[0].set_xticklabels(pad(["a", "b", "c", "d"]))?;
    axs[0].set_yticklabels(pad(["11", "12", "13", "14"]))?;
    for (source, target) in [
        (axs[0].xaxis(), axs[1].xaxis()),
        (axs[0].yaxis(), axs[1].yaxis()),
    ] {
        target.set_ticks(source.ticklocs(false)?);
        target.set_ticklabels(source.ticklabels(false)?)?;
    }
    Ok(())
}

/// `test_set_ticks_with_labels`: bold major labels and labelled minor
/// ticks.
fn set_ticks_with_labels(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xticks([1.0, 2.0, 4.0, 6.0]);
    ax.set_xticklabels(["a", "b", "c", "d"])?;
    ax.xaxis().set_ticklabel_props(
        TickLevel::Major,
        &TickLabelProps {
            fontweight: Some(FontWeight::BOLD),
            ..TickLabelProps::default()
        },
    );
    ax.set_yticks([1.0, 3.0, 5.0]);
    ax.yaxis()
        .set_minor_ticks([2.0, 4.0])
        .set_minor_ticklabels(["A", "B"])?;
    Ok(())
}

/// `test_loglog_nonpos`: every combination of masking and clipping the
/// non-positive values on either axis.
fn loglog_nonpos(fig: &Figure) -> CaseResult {
    let mut x: Vec<f64> = arange(1.0, 11.0, 1.0);
    let mut y: Vec<f64> = x.iter().map(|x| x.powi(3)).collect();
    y[7] = -3.0;
    x[4] = -10.0;
    let y: Vec<f64> = y.iter().map(|y| y.powi(3)).collect();
    let modes = [Some(NonPositive::Mask), Some(NonPositive::Clip), None];
    let axs = fig.subplots(3, 3)?;
    let combinations = modes
        .iter()
        .flat_map(|mcy| modes.iter().map(move |mcx| (*mcy, *mcx)));
    for ((mcy, mcx), ax) in combinations.zip(axs.as_slice()) {
        let log = |nonpositive| ScaleOptions {
            nonpositive,
            ..ScaleOptions::default()
        };
        // `loglog(..., nonpositive=...)` only when both agree.
        let (x_mode, y_mode) = if mcx == mcy { (mcx, mcx) } else { (None, None) };
        ax.set_scale_with(AxisName::X, "log", &log(x_mode))?
            .set_scale_with(AxisName::Y, "log", &log(y_mode))?;
        ax.plot_with(&x, &y, LineProps::default().linewidth(2.0))?;
        if mcx != mcy {
            if mcx.is_some() {
                ax.set_scale_with(AxisName::X, "log", &log(mcx))?;
            }
            if mcy.is_some() {
                ax.set_scale_with(AxisName::Y, "log", &log(mcy))?;
            }
        }
        ax.set_yticks([1e3, 1e7]);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_axisbelow`: gridlines above, between and below the artists.
fn axisbelow(fig: &Figure) -> CaseResult {
    let axs = fig.subplots_shared(1, 3, Share::All, Share::All)?;
    for (ax, setting) in
        axs.as_slice()
            .iter()
            .zip([AxisBelow::Above, AxisBelow::Line, AxisBelow::Below])
    {
        ax.plot_with(
            [0.0, 10.0],
            [0.0, 10.0],
            LineProps::default().linewidth(10.0).color("m"),
        )?;
        ax.add_patch(Circle::new((3.0, 3.0), 5.0))
            .update_props(PatchProps::default().color("r"));
        ax.grid_with(
            true,
            &GridOptions {
                color: Some("c".into()),
                linestyle: Some(LineStyle::Solid),
                linewidth: Some(3.0),
                ..GridOptions::default()
            },
        );
        ax.tick_params(&TickParams {
            top: Some(false),
            bottom: Some(false),
            left: Some(false),
            right: Some(false),
            ..TickParams::default()
        });
        for spine in ax.spines().iter() {
            spine.set_visible(false);
        }
        ax.set_axisbelow(setting);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_axes_d/set_get_ticklabels", set_get_ticklabels),
    classic("mpl_axes_d/set_ticks_with_labels", set_ticks_with_labels),
    FigureCase::new("mpl_axes_d/retain_tick_visibility", |fig| {
        let ax = fig.subplot();
        ax.plot([0.0, 1.0, 2.0], [0.0, -1.0, 4.0])?;
        ax.tick_params(&TickParams {
            axis: Some(AxisName::Y),
            labelleft: Some(false),
            ..TickParams::default()
        });
        ax.tick_params(&TickParams {
            axis: Some(AxisName::Y),
            which: TickLevel::Both,
            length: Some(0.0),
            ..TickParams::default()
        });
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/o_marker_path_snap", |fig| {
        let ax = fig.subplot();
        ax.set_margins(0.1, 0.1)?;
        let markers = |x: [f64; 2], ms: f64| {
            ax.plot_with(x, [1.0 + ms; 2], LineProps::fmt("o")?.markersize(ms))
                .map_err(Box::<dyn std::error::Error>::from)
        };
        for ms in 1..15 {
            markers([1.0, 2.0], f64::from(ms))?;
        }
        for ms in linspace(1.0, 10.0, 25) {
            markers([3.0, 4.0], ms)?;
        }
        Ok(())
    })
    .sized(6.4, 4.8, 72.0),
    FigureCase::new("mpl_axes_d/rc_spines", |fig| {
        fig.subplot();
        Ok(())
    })
    .sized(6.4, 4.8, 40.0)
    .with_rc(&[
        ("axes.spines.left", "False"),
        ("axes.spines.right", "False"),
        ("axes.spines.top", "False"),
        ("axes.spines.bottom", "False"),
    ]),
    FigureCase::new("mpl_axes_d/rc_grid", |fig| {
        for (i, axis) in ["both", "x", "y"].into_iter().enumerate() {
            plt::rc::context(|| -> CaseResult {
                plt::rc::set("axes.grid", "True")?;
                plt::rc::set("axes.grid.axis", axis)?;
                fig.add_subplot(3, 1, i + 1)?;
                Ok(())
            })?;
        }
        Ok(())
    })
    .sized(6.4, 4.8, 40.0),
    classic("mpl_axes_d/dash_offset", |fig| {
        let ax = fig.subplot();
        let x = linspace(0.0, 10.0, 50);
        for j in (0..100).step_by(2) {
            let offset = f64::from(j);
            let dashes = LineStyle::Dashes {
                offset,
                pattern: vec![10.0, 10.0],
            };
            let props = LineProps::default()
                .linestyle(dashes)
                .linewidth(5.0)
                .color("k");
            ax.plot_with(&x, vec![offset; x.len()], props)?;
        }
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_d/loglog", |fig| {
        let ax = fig.subplot();
        let x = arange(1.0, 11.0, 1.0);
        let cubes: Vec<f64> = x.iter().map(|x| x.powi(3)).collect();
        ax.loglog(&x, cubes)?.set_linewidth(5.0);
        for (which, length) in [(TickLevel::Major, 25.0), (TickLevel::Minor, 15.0)] {
            ax.tick_params(&TickParams {
                which,
                length: Some(length),
                width: Some(2.0),
                ..TickParams::default()
            });
        }
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/loglog_nonpos", loglog_nonpos),
    classic("mpl_axes_d/axisbelow", axisbelow),
    FigureCase::new("mpl_axes_d/stairs_fill_zero_linewidth", |fig| {
        let options = StairsOptions {
            edges: Some(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
            fill: true,
            ..StairsOptions::default()
        };
        fig.subplot()
            .stairs(vec![1.0, 2.0, 3.0, 4.0], options)?
            .set_linestyle(LineStyle::Dashed);
        Ok(())
    }),
    classic("mpl_axes_d/plot_decimal", |fig| {
        let x0 = arange(-10.0, 10.0, 0.3);
        let y0: Vec<f64> = x0
            .iter()
            .map(|x| 5.2 * x.powi(3) - 2.1 * x.powi(2) + 7.34 * x + 4.5)
            .collect();
        fig.subplot().plot(&x0, y0)?;
        Ok(())
    }),
    classic("mpl_axes_d/markerfacecolor_none_alpha", |fig| {
        let props = LineProps::fmt("o")?.markerfacecolor(ColorOrAuto::Color("none".into()));
        fig.subplot()
            .plot_with([0.0], [0.0], props)?
            .set_alpha(Some(0.5));
        Ok(())
    }),
];
