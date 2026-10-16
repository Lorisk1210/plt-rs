//! Grid and minor tick cases of `figures/cases_axes.py`.

use plt::axes::{AxisName, GridOptions, TickLevel};
use plt::lines::LineStyle;
use plt::prelude::*;
use plt::rc::AxisBelow;

use super::line;
use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("axes/grid_default", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.grid(true);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/grid_major_minor_styles", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.minorticks_on();
        ax.grid_with(
            true,
            &GridOptions {
                which: TickLevel::Major,
                color: Some(Color::from("0.4")),
                linewidth: Some(1.0),
                ..GridOptions::default()
            },
        );
        ax.grid_with(
            true,
            &GridOptions {
                which: TickLevel::Minor,
                color: Some(Color::from("0.8")),
                linestyle: Some(LineStyle::Dotted),
                linewidth: Some(0.8),
                ..GridOptions::default()
            },
        );
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/grid_y_only_styled", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.grid_with(
            true,
            &GridOptions {
                axis: Some(AxisName::Y),
                color: Some(Color::from("tab:red")),
                linestyle: Some(LineStyle::Dashed),
                linewidth: Some(0.5),
                alpha: Some(0.7),
                ..GridOptions::default()
            },
        );
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/grid_below_bars", |fig| {
        let ax = fig.subplot();
        ax.bar(
            arange(0.0, 5.0, 1.0),
            [3.0, 5.0, 2.0, 4.0, 6.0],
            BarOptions::default(),
        )?;
        ax.set_axisbelow(AxisBelow::Below);
        ax.grid_with(
            true,
            &GridOptions {
                linewidth: Some(2.0),
                ..GridOptions::default()
            },
        );
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/minorticks_on", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.minorticks_on();
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/negative_tick_labels_unicode_minus", |fig| {
        fig.subplot().plot([-5.0, 5.0], [-0.003, 0.002])?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
