//! Pie chart cases of `figures/cases_stats.py`.

use plt::axes::{
    PieDirection, PieLabelAlignment, PieLabelFormat, PieLabelOptions, PieOptions, PieShadow,
};
use plt::colors::Color;
use plt::fonts::FontSize;
use plt::lines::LineStyle;
use plt::patches::PatchProps;

use crate::figures::FigureCase;

/// `SHARES`.
const SHARES: [f64; 5] = [35.0, 25.0, 20.0, 12.0, 8.0];

/// `NAMES`.
fn names() -> Vec<String> {
    ["alpha", "beta", "gamma", "delta", "epsilon"]
        .map(String::from)
        .to_vec()
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("stats/pie_default", |fig| {
        fig.subplot().pie(&SHARES, &PieOptions::default())?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("stats/pie_labels_autopct", |fig| {
        let options = PieOptions {
            labels: names(),
            autopct: Some(PieLabelFormat::Printf("%1.1f%%".into())),
            ..PieOptions::default()
        };
        fig.subplot().pie(&SHARES, &options)?;
        Ok(())
    })
    .sized(5.0, 5.0, 100.0),
    FigureCase::new("stats/pie_explode_shadow_startangle", |fig| {
        let options = PieOptions {
            labels: names(),
            explode: vec![0.1, 0.0, 0.0, 0.2, 0.0],
            shadow: Some(PieShadow::default()),
            startangle: 90.0,
            ..PieOptions::default()
        };
        fig.subplot().pie(&SHARES, &options)?;
        Ok(())
    })
    .sized(5.0, 5.0, 100.0),
    FigureCase::new("stats/pie_clockwise_label_and_pct_distance", |fig| {
        let options = PieOptions {
            labels: names(),
            direction: PieDirection::Clockwise,
            labeldistance: Some(0.7),
            autopct: Some(PieLabelFormat::Printf("%d%%".into())),
            pctdistance: 1.15,
            ..PieOptions::default()
        };
        fig.subplot().pie(&SHARES, &options)?;
        Ok(())
    })
    .sized(5.0, 5.0, 100.0),
    FigureCase::new("stats/pie_donut_nested", |fig| {
        let ax = fig.subplot();
        let ring = |radius: f64, colors: Vec<Color>| PieOptions {
            radius,
            colors,
            wedge_width: Some(0.3),
            wedgeprops: PatchProps::default().edgecolor("w"),
            ..PieOptions::default()
        };
        ax.pie(&SHARES, &ring(1.0, Vec::new()))?;
        ax.pie(
            &[60.0, 40.0],
            &ring(0.7, vec![Color::from("0.5"), Color::from("0.8")]),
        )?;
        Ok(())
    })
    .sized(5.0, 5.0, 100.0),
    FigureCase::new("stats/pie_not_normalized", |fig| {
        let options = PieOptions {
            normalize: false,
            labels: ["a", "b", "c"].map(String::from).to_vec(),
            ..PieOptions::default()
        };
        fig.subplot().pie(&[0.2, 0.3, 0.1], &options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("stats/pie_wedgeprops_textprops_colors", |fig| {
        let options = PieOptions {
            labels: names(),
            colors: ["#66c2a5", "#fc8d62", "#8da0cb", "#e78ac3", "#a6d854"]
                .map(Color::from)
                .to_vec(),
            wedgeprops: PatchProps::default()
                .linewidth(2.0)
                .edgecolor("k")
                .linestyle(LineStyle::Dashed),
            text_size: Some(FontSize::Points(14.0)),
            text_color: Some(Color::from("darkblue")),
            hatches: ["", "/", "x", "", "."].map(String::from).to_vec(),
            ..PieOptions::default()
        };
        fig.subplot().pie(&SHARES, &options)?;
        Ok(())
    })
    .sized(5.0, 5.0, 100.0),
    FigureCase::new("stats/pie_rotatelabels_frame_center", |fig| {
        let options = PieOptions {
            labels: names(),
            rotatelabels: true,
            frame: true,
            center: (2.0, 1.0),
            radius: 1.5,
            ..PieOptions::default()
        };
        fig.subplot().pie(&SHARES, &options)?;
        Ok(())
    })
    .sized(5.0, 5.0, 100.0),
    FigureCase::new("stats/pie_label", |fig| {
        let axs = fig.subplots(1, 2)?;
        let (left, right) = (&axs[(0, 0)], &axs[(0, 1)]);
        let mut pie = left.pie(&[15.0, 30.0, 45.0, 10.0], &PieOptions::default())?;
        let inside = PieLabelOptions {
            distance: 0.7,
            ..PieLabelOptions::default()
        };
        left.pie_label(&mut pie, "{absval:d}", &inside)?;
        let outside = PieLabelOptions {
            distance: 1.15,
            ..PieLabelOptions::default()
        };
        let names: &[&str] = &["frogs", "hogs", "dogs", "logs"];
        left.pie_label(&mut pie, names, &outside)?;
        let unnormalized = PieOptions {
            normalize: false,
            ..PieOptions::default()
        };
        let mut partial = right.pie(&[0.2, 0.3, 0.1], &unnormalized)?;
        let rotated = PieLabelOptions {
            distance: 1.1,
            rotate: true,
            color: Some(Color::from("tab:red")),
            size: Some(FontSize::Points(8.0)),
            ..PieLabelOptions::default()
        };
        right.pie_label(&mut partial, "{frac:.1%}", &rotated)?;
        let outer = PieLabelOptions {
            distance: 0.5,
            alignment: PieLabelAlignment::Outer,
            ..PieLabelOptions::default()
        };
        let letters: &[&str] = &["a", "b", "c"];
        right.pie_label(&mut partial, letters, &outer)?;
        Ok(())
    })
    .sized(8.0, 4.0, 100.0),
];
