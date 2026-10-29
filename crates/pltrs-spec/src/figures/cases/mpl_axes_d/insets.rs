//! Inset and secondary axes, annotations across axes, `axis` options and
//! polar interpolation.

use std::f64::consts::{FRAC_PI_2, TAU};

use pltrs::axes::{AxisMode, SecondaryLocation};
use pltrs::coords::Coords;
use pltrs::figure::Figure;
use pltrs::layout::LayoutEngine;
use pltrs::patches::{ArrowStyle, Circle, PatchProps};
use pltrs::prelude::{ArtistHandle, LineProps, arange, linspace};
use pltrs::rc::Aspect;
use pltrs::scale::FuncTransform;
use pltrs::spines::SpineSide;
use pltrs::text::{AnnotateOptions, ArrowProps, FancyArrowProps};
use pltrs::transforms::Point;

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// `test_secondary_xy`: secondary axes of every kind on both axes.
fn secondary_xy(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, 2)?.into_vec();
    for (nn, ax) in axs.iter().enumerate() {
        let n = arange(2.0, 11.0, 1.0);
        ax.plot(&n, &n)?;
        let secax = |at, functions| {
            if nn == 0 {
                ax.secondary_xaxis(at, functions)
            } else {
                ax.secondary_yaxis(at, functions)
            }
        };
        let top = if nn == 0 {
            SpineSide::Top
        } else {
            SpineSide::Right
        };
        let invert = FuncTransform::new(f64::recip, f64::recip);
        secax(SecondaryLocation::Fraction(0.2), Some(invert))?;
        let double = FuncTransform::new(|x| 2.0 * x, |x| x / 2.0);
        secax(SecondaryLocation::Fraction(0.4), Some(double))?;
        let square = FuncTransform::new(|x| x * x, |x: f64| x.powf(0.5));
        secax(SecondaryLocation::Fraction(0.6), Some(square))?;
        secax(SecondaryLocation::Fraction(0.8), None)?;
        let translation = FuncTransform::new(|x| x + 2.0, |x| x - 2.0);
        secax(SecondaryLocation::Side(top), Some(translation))?;
        secax(SecondaryLocation::Data(6.25), None)?;
    }
    Ok(())
}

/// `test_axis_options`: a line and a circle fitting in it, high/narrow
/// and low/wide, under each `axis` option.
fn axis_options(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(2, 3)?;
    for (i, option) in [AxisMode::Scaled, AxisMode::Tight, AxisMode::Image]
        .into_iter()
        .enumerate()
    {
        let rows = [
            ((1.0, 2.0), (1.0, 3.2), (1.5, 1.5), 0.5),
            ((1.0, 2.25), (1.0, 1.75), (1.5, 1.25), 0.25),
        ];
        for (row, (x, y, center, radius)) in rows.into_iter().enumerate() {
            let ax = &axes[(row, i)];
            ax.plot([x.0, x.1], [y.0, y.1])?;
            ax.axis_mode(option)?;
            ax.add_artist(Circle::new(center, radius))
                .update_props(PatchProps::default().facecolor("none").edgecolor("k"));
        }
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_axes_d/inset_polar", |fig| {
        let ax = fig.subplot();
        let axins = ax.polar_inset_axes([0.5, 0.1, 0.45, 0.45]);
        let r = arange(0.0, 2.0, 0.01);
        let theta: Vec<f64> = r.iter().map(|r| TAU * r).collect();
        ax.plot(&theta, &r)?;
        axins.plot(theta, r)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/annotate_across_transforms", |fig| {
        let x = linspace(0.0, 10.0, 200);
        let y: Vec<f64> = x.iter().map(|x| (-x).exp() * x.sin()).collect();
        let ax = fig.subplot();
        ax.plot(&x, &y)?;
        let axins = ax.inset_axes([0.4, 0.5, 0.3, 0.3]);
        axins.set_aspect(Aspect::Ratio(0.2))?;
        axins.xaxis().set_visible(false);
        axins.yaxis().set_visible(false);
        let arrow = FancyArrowProps::new("->".parse::<ArrowStyle>()?);
        ax.annotate(
            "",
            Point::new(x[150], y[150]),
            AnnotateOptions {
                xytext: Some(Point::new(1.0, 0.0).into()),
                xycoords: Coords::Transform(ax.trans_data()),
                textcoords: Some(Coords::Transform(axins.trans_axes())),
                arrowprops: Some(ArrowProps::Fancy(arrow)),
                ..AnnotateOptions::default()
            },
        )?;
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .sized(3.39, 3.0, 100.0),
    FigureCase::new("mpl_axes_d/secondary_xy", secondary_xy).sized(10.0, 5.0, 100.0),
    FigureCase::new("mpl_axes_d/axis_options", axis_options),
    classic("mpl_axes_d/axis_bool_arguments", |fig| {
        fig.add_subplot(2, 1, 1)?.axis_mode(AxisMode::Off)?;
        let ax = fig.add_subplot(2, 1, 2)?;
        ax.axis_mode(AxisMode::Off)?.axis_mode(AxisMode::On)?;
        Ok(())
    }),
    classic("mpl_axes_d/polar_interpolation_steps_variable_r", |fig| {
        fig.polar_subplot()
            .plot([0.0, FRAC_PI_2], [1.0, 2.0])?
            .set_interpolation_steps(100_usize);
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/axhvlinespan_interpolation", |fig| {
        let ax = fig.polar_subplot();
        ax.set_axis_off();
        ax.axvline(0.1, LineProps::default().color("C0"));
        ax.axvspan(0.2, 0.3).set_facecolor("C1");
        ax.axvspan_between(0.4, 0.5, 0.1, 0.2).set_facecolor("C2");
        ax.axhline(1.0, LineProps::default().color("C0"))
            .set_alpha(Some(0.5));
        ax.axhspan(0.8, 0.9)
            .set_facecolor("C1")
            .set_alpha(Some(0.5));
        ax.axhspan_between(0.6, 0.7, 0.8, 0.9)
            .set_facecolor("C2")
            .set_alpha(Some(0.5));
        Ok(())
    }),
];
