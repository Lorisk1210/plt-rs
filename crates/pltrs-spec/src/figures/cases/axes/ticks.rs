//! Tick cases of `figures/cases_axes.py`.

use std::f64::consts::PI;

use pltrs::axes::{AxisName, LocatorParams, TickDirection, TickLabelProps, TickLevel, TickParams};
use pltrs::fonts::{FontSize, RelativeSize};
use pltrs::gridspec::SubplotOverrides;
use pltrs::prelude::*;
use pltrs::text::{HAlign, RotationMode, VAlign};
use pltrs::ticker::Nbins;

use super::line;
use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("axes/tick_direction_in_inout", |fig| {
        let axs = fig.subplots(1, 2)?;
        for (ax, direction) in axs
            .as_slice()
            .iter()
            .zip([TickDirection::In, TickDirection::InOut])
        {
            line(ax)?;
            ax.minorticks_on();
            ax.tick_params(TickParams {
                which: TickLevel::Both,
                direction: Some(direction),
                ..TickParams::default()
            });
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("axes/tick_length_width_colors", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.tick_params(TickParams {
            axis: Some(AxisName::X),
            length: Some(10.0),
            width: Some(2.0),
            colors: Some(Color::from("tab:red")),
            ..TickParams::default()
        });
        ax.tick_params(TickParams {
            axis: Some(AxisName::Y),
            length: Some(2.0),
            width: Some(0.5),
            color: Some(Color::from("tab:blue")),
            labelcolor: Some(Color::from("tab:green")),
            ..TickParams::default()
        });
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/tick_labelsize_rotation_pad", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.tick_params(TickParams {
            axis: Some(AxisName::X),
            labelsize: Some(FontSize::Points(14.0)),
            labelrotation: Some(45.0),
            pad: Some(8.0),
            ..TickParams::default()
        });
        ax.tick_params(TickParams {
            axis: Some(AxisName::Y),
            labelsize: Some(FontSize::Relative(RelativeSize::XSmall)),
            ..TickParams::default()
        });
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/tick_labels_top_right", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.tick_params(TickParams {
            top: Some(true),
            labeltop: Some(true),
            right: Some(true),
            labelright: Some(true),
            ..TickParams::default()
        });
        Ok(())
    }),
    FigureCase::new("axes/tick_labelbottom_false", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.tick_params(TickParams {
            labelbottom: Some(false),
            bottom: Some(false),
            labelleft: Some(false),
            ..TickParams::default()
        });
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/set_ticks_labels_minor", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.xaxis()
            .set_ticks([0.0, PI, 2.0 * PI, 3.0 * PI])
            .set_ticklabels(["0", r"$\pi$", r"$2\pi$", r"$3\pi$"])?;
        ax.xaxis().set_minor_ticks(arange(0.0, 10.0, 0.5));
        ax.set_yticks([-1.0, 0.0, 1.0]);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/ticklabel_rotation_alignment", |fig| {
        let ax = fig.subplot();
        ax.plot(arange(0.0, 5.0, 1.0), [3.0, 1.0, 4.0, 1.0, 5.0])?;
        ax.xaxis()
            .set_ticks(arange(0.0, 5.0, 1.0))
            .set_ticklabels(["January", "February", "March", "April", "May"])?
            .set_ticklabel_props(
                TickLevel::Major,
                TickLabelProps {
                    rotation: Some(45.0),
                    halign: Some(HAlign::Right),
                    rotation_mode: Some(RotationMode::Anchor),
                    ..TickLabelProps::default()
                },
            );
        fig.subplots_adjust(SubplotOverrides {
            bottom: Some(0.25),
            ..SubplotOverrides::default()
        })?;
        ax.yaxis()
            .set_ticks([1.0, 3.0, 5.0])
            .set_ticklabels(["low", "mid", "high"])?
            .set_ticklabel_props(
                TickLevel::Major,
                TickLabelProps {
                    rotation: Some(90.0),
                    valign: Some(VAlign::Center),
                    ..TickLabelProps::default()
                },
            );
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/locator_params_nbins", |fig| {
        let axs = fig.subplots(1, 2)?;
        line(&axs[(0, 0)])?;
        for (axis, nbins) in [(AxisName::X, 3), (AxisName::Y, 10)] {
            axs[(0, 0)].locator_params(LocatorParams {
                axis: Some(axis),
                nbins: Some(Nbins::Fixed(nbins)),
                ..LocatorParams::default()
            })?;
        }
        line(&axs[(0, 1)])?;
        axs[(0, 1)].locator_params(LocatorParams {
            nbins: Some(Nbins::Fixed(4)),
            steps: Some(vec![1.0, 2.5, 5.0]),
            ..LocatorParams::default()
        })?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
];
