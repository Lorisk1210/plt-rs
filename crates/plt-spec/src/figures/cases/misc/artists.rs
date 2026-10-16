//! Figure-level artist and property cycle cases of
//! `figures/cases_axes_misc.py`.

use plt::patches::{Circle, Rectangle};
use plt::prelude::*;
use plt::rc::{CycleValue, Cycler};
use plt::transforms::Transform;

use super::wave;
use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("misc/fig_add_artist", |fig| {
        fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
        fig.add_line(
            &[0.05, 0.95],
            &[0.5, 0.5],
            LineProps::default().color("tab:red").linewidth(3.0),
        )?;
        fig.add_patch(Circle::new((0.5, 0.5), 0.1)).update_props(
            PatchProps::default()
                .facecolor("none")
                .edgecolor("k")
                .linewidth(2.0),
        );
        fig.add_patch(Rectangle::new((10.0, 10.0), 50.0, 30.0))
            .set_facecolor("gold")
            .set_transform(Transform::Display);
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("misc/set_prop_cycle", |fig| {
        let ax = fig.subplot();
        let colors = ["tab:red", "tab:green", "k"].map(|c| CycleValue::Color(Color::from(c)));
        let styles =
            [LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted].map(CycleValue::LineStyle);
        let widths = [1.0, 2.0, 3.0].map(CycleValue::Number);
        let cycle = Cycler::new("color", colors.to_vec())
            .zip(Cycler::new("linestyle", styles.to_vec()))?
            .zip(Cycler::new("linewidth", widths.to_vec()))?;
        ax.set_prop_cycle(Some(cycle));
        for i in 0..5 {
            let shift = f64::from(i);
            ax.plot_y(wave(20).into_iter().map(|y| y + shift).collect::<Vec<_>>());
        }
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
];
