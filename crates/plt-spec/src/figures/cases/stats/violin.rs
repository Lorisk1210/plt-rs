//! Violin plot cases of `figures/cases_stats.py`.

use plt::axes::{ViolinDrawOptions, ViolinSide, ViolinplotOptions};
use plt::colors::Color;
use plt::orientation::Orientation;
use plt::stats::{BandwidthMethod, ViolinOptions};

use super::samples;
use crate::figures::FigureCase;
use crate::figures::cases::common::drawn_violins;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("stats/violin_default", |fig| {
        fig.subplot()
            .violinplot(&samples(), ViolinplotOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/violin_means_medians_no_extrema", |fig| {
        let options = drawn_violins(ViolinDrawOptions {
            showmeans: true,
            showmedians: true,
            showextrema: false,
            ..ViolinDrawOptions::default()
        });
        fig.subplot().violinplot(&samples(), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/violin_quantiles_points_bw", |fig| {
        let options = ViolinplotOptions {
            stats: ViolinOptions {
                bw_method: BandwidthMethod::Factor(0.3),
                points: 50,
                quantiles: Some(vec![vec![0.25, 0.75], vec![0.1, 0.5, 0.9], vec![0.05]]),
            },
            ..ViolinplotOptions::default()
        };
        fig.subplot().violinplot(&samples(), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/violin_split_sides", |fig| {
        let ax = fig.subplot();
        let data = samples();
        for (datasets, side) in [
            (&data[..2], ViolinSide::Low),
            (&data[1..], ViolinSide::High),
        ] {
            let options = drawn_violins(ViolinDrawOptions {
                positions: Some(vec![1.0, 2.0]),
                side,
                showmedians: true,
                ..ViolinDrawOptions::default()
            });
            ax.violinplot(datasets, options)?;
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/violin_horizontal_widths_colors", |fig| {
        let options = drawn_violins(ViolinDrawOptions {
            orientation: Orientation::Horizontal,
            widths: vec![0.3, 0.6, 0.9],
            facecolors: vec![Color::from("tab:red")],
            linecolors: vec![Color::from("k")],
            ..ViolinDrawOptions::default()
        });
        fig.subplot().violinplot(&samples(), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
