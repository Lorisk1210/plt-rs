//! Box plot cases of `figures/cases_stats.py`.

use plt::axes::{BoxArtist, BoxProps, BoxplotOptions, BxpOptions};
use plt::figure::Share;
use plt::lines::{LineProps, LineStyle};
use plt::markers::{Marker, MarkerSymbol};
use plt::orientation::Orientation;
use plt::patches::PatchProps;
use plt::rc::ColorOrAuto;
use plt::stats::Whis;

use super::samples;
use crate::figures::FigureCase;
use crate::figures::cases::common::drawn_boxes;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("stats/boxplot_default", |fig| {
        fig.subplot()
            .boxplot(&samples(), BoxplotOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/boxplot_notch", |fig| {
        let options = drawn_boxes(BxpOptions {
            shownotches: Some(true),
            ..BxpOptions::default()
        });
        fig.subplot().boxplot(&samples(), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/boxplot_horizontal", |fig| {
        let options = drawn_boxes(BxpOptions {
            orientation: Orientation::Horizontal,
            ..BxpOptions::default()
        });
        fig.subplot().boxplot(&samples(), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/boxplot_whis_percentiles_and_range", |fig| {
        let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
        for (ax, (low, high)) in axs.as_slice().iter().zip([(5.0, 95.0), (0.0, 100.0)]) {
            let options = BoxplotOptions {
                whis: Some(Whis::Percentiles(low, high)),
                ..BoxplotOptions::default()
            };
            ax.boxplot(&samples(), options)?;
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("stats/boxplot_no_fliers_custom_sym", |fig| {
        let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
        let no_fliers = drawn_boxes(BxpOptions {
            showfliers: Some(false),
            ..BxpOptions::default()
        });
        axs[(0, 0)].boxplot(&samples(), no_fliers)?;
        let custom = BoxplotOptions {
            sym: Some("r+".into()),
            ..BoxplotOptions::default()
        };
        axs[(0, 1)].boxplot(&samples(), custom)?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("stats/boxplot_showmeans_meanline", |fig| {
        let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
        for (ax, meanline) in axs.as_slice().iter().zip([false, true]) {
            let options = drawn_boxes(BxpOptions {
                showmeans: Some(true),
                meanline: Some(meanline),
                ..BxpOptions::default()
            });
            ax.boxplot(&samples(), options)?;
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("stats/boxplot_patch_artist_colored", |fig| {
        let options = drawn_boxes(BxpOptions {
            boxes: Some(BoxProps::Patch(PatchProps::default())),
            medianprops: LineProps::default().color("k").linewidth(2.0),
            ..BxpOptions::default()
        });
        let parts = fig.subplot().boxplot(&samples(), options)?;
        for (part, color) in parts.boxes.iter().zip(["pink", "lightblue", "lightgreen"]) {
            if let BoxArtist::Patch(patch) = part {
                patch.set_facecolor(color);
            }
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/boxplot_labels_positions_widths", |fig| {
        let options = BoxplotOptions {
            tick_labels: Some(["low", "mid", "wide"].map(String::from).to_vec()),
            draw: BxpOptions {
                positions: Some(vec![1.0, 2.5, 6.0]),
                widths: vec![0.3, 0.8, 1.5],
                ..BxpOptions::default()
            },
            ..BoxplotOptions::default()
        };
        fig.subplot().boxplot(&samples(), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("stats/boxplot_props_and_caps", |fig| {
        let options = drawn_boxes(BxpOptions {
            boxes: Some(BoxProps::Line(
                LineProps::default().color("tab:blue").linewidth(2.0),
            )),
            whiskerprops: LineProps::default().linestyle(LineStyle::Dashed),
            capprops: LineProps::default().color("tab:red"),
            flierprops: LineProps::default()
                .marker(Marker::Symbol(MarkerSymbol::Diamond))
                .markerfacecolor(ColorOrAuto::Color("tab:orange".into()))
                .markersize(5.0),
            capwidths: vec![0.4],
            ..BxpOptions::default()
        });
        fig.subplot().boxplot(&samples(), options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
