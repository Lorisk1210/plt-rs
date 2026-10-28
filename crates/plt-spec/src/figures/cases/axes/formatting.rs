//! Tick formatting cases of `figures/cases_axes.py`.

use plt::axes::{AxisName, TicklabelFormat};
use plt::gridspec::SubplotOverrides;
use plt::ticker::OffsetMode;

use super::{of_t, t};
use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("axes/ticklabel_format_sci", |fig| {
        let ax = fig.subplot();
        ax.plot(t(), of_t(|t| t * 1234.5))?;
        ax.ticklabel_format(TicklabelFormat {
            axis: Some(AxisName::Y),
            scientific: Some(true),
            scilimits: Some((0, 0)),
            ..TicklabelFormat::default()
        })?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/ticklabel_format_mathtext", |fig| {
        let ax = fig.subplot();
        ax.plot(of_t(|t| t * 1e-5), of_t(|t| t * 3e6))?;
        ax.ticklabel_format(TicklabelFormat {
            scientific: Some(true),
            scilimits: Some((-2, 2)),
            use_mathtext: Some(true),
            ..TicklabelFormat::default()
        })?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/default_sci_large_values", |fig| {
        fig.subplot().plot(t(), of_t(|t| t * 1e9))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/large_offset_data", |fig| {
        fig.subplot().plot(t(), of_t(|t| 1e5 + t.sin() * 0.01))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/large_offset_use_offset_false", |fig| {
        let ax = fig.subplot();
        ax.plot(t(), of_t(|t| 1e5 + t.sin() * 0.01))?;
        ax.ticklabel_format(TicklabelFormat {
            use_offset: Some(OffsetMode::Off),
            ..TicklabelFormat::default()
        })?;
        fig.subplots_adjust(SubplotOverrides {
            left: Some(0.25),
            ..SubplotOverrides::default()
        })?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/tiny_range", |fig| {
        fig.subplot()
            .plot(of_t(|t| t * 1e-9), of_t(|t| t.sin() * 1e-9))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/epoch_seconds_values", |fig| {
        let hours: Vec<f64> = (0..24).map(f64::from).collect();
        let t: Vec<f64> = hours.iter().map(|h| 1.7e9 + h * 3600.0).collect();
        let y: Vec<f64> = hours.iter().map(|h| (h / 4.0).cos()).collect();
        fig.subplot().plot(t, y)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
