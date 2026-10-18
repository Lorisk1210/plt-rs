//! The tight-layout section of `figures/cases_layout.py`, plus label
//! alignment.

use plt::axes::{AxisName, TickParams};
use plt::axes::{Mesh, PcolorOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::colormaps;
use plt::gridspec::SubplotOverrides;
use plt::image::ColorizerOptions;
use plt::layout::{LayoutEngine, TightLayout};

use super::{fill, grid, t};
use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("layout/tight_layout_long_labels", |fig| {
        for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
            let shift = i as f64;
            let y: Vec<f64> = t().into_iter().map(|t| 1e4 * (t + shift).sin()).collect();
            ax.plot(t(), y)?;
            ax.set_xlabel("a rather long x axis label");
            ax.set_ylabel("long y label\nwith two lines");
            ax.set_title(format!("panel title {i}"));
        }
        fig.tight_layout(&TightLayout::default())?;
        Ok(())
    }),
    FigureCase::new("layout/tight_layout_pads", |fig| {
        for (i, ax) in fig.subplots(1, 2)?.as_slice().iter().enumerate() {
            fill(ax, i)?;
            ax.set_title("title");
            ax.set_xlabel("x");
        }
        fig.suptitle("Suptitle");
        fig.tight_layout(&TightLayout {
            pad: 2.0,
            w_pad: Some(4.0),
            ..TightLayout::default()
        })?;
        Ok(())
    }),
    FigureCase::new("layout/autolayout_rc", |fig| {
        for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
            let shift = i as f64;
            let y: Vec<f64> = t().into_iter().map(|t| 1e4 * (t + shift).sin()).collect();
            ax.plot(t(), y)?;
            ax.set_xlabel("a rather long x axis label");
            ax.set_ylabel("long y label\nwith two lines");
            ax.set_title(format!("panel title {i}"));
        }
        Ok(())
    })
    .with_rc(&[("figure.autolayout", "True")]),
    FigureCase::new("layout/tight_engine_colorbar", |fig| {
        fig.set_layout_engine(Some(LayoutEngine::Tight(TightLayout::default())));
        for (i, ax) in fig.subplots(1, 2)?.as_slice().iter().enumerate() {
            let shift = i as f64;
            let mesh = ax.pcolormesh(
                Mesh::new(grid().map(|v| v + shift)),
                PcolorOptions {
                    colors: ColorizerOptions::default().cmap(colormaps::get("viridis")?),
                    ..PcolorOptions::default()
                },
            )?;
            fig.colorbar(
                &mesh,
                ColorbarOptions {
                    axes: ColorbarAxes::Parent(ax.clone()),
                    label: "value".into(),
                    ..ColorbarOptions::default()
                },
            )?;
            ax.set_title(format!("panel {i}"));
            ax.set_ylabel("rows");
        }
        Ok(())
    }),
    FigureCase::new("layout/align_labels", |fig| {
        fig.subplots_adjust(&SubplotOverrides {
            left: Some(0.2),
            wspace: Some(0.6),
            ..SubplotOverrides::default()
        })?;
        let axs = fig.subplots(2, 2)?;
        for (i, ax) in axs.as_slice().iter().enumerate() {
            let gain = 10f64.powi(2 * i32::try_from(i)?);
            let y: Vec<f64> = t().into_iter().map(|t| t.sin() * gain).collect();
            ax.plot(t(), y)?;
            ax.set_ylabel(format!("y label {i}"));
            ax.set_xlabel(format!("x label {i}"));
        }
        axs[(0, 0)].tick_params(&TickParams {
            axis: Some(AxisName::X),
            labelrotation: Some(55.0),
            ..TickParams::default()
        });
        fig.align_labels(None);
        Ok(())
    }),
];
