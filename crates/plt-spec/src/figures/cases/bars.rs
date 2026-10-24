//! `figures/cases_bars.py`.

use plt::axes::{
    BandAlign, BarAlign, BarLabelOptions, BarLabelType, BarOptions, ErrorSize, ErrorbarOptions,
    GroupedBarData, GroupedBarOptions,
};
use plt::colors::Color;
use plt::legend::LegendOptions;
use plt::lines::LineProps;
use plt::numeric::arange;
use plt::orientation::Orientation;

use crate::figures::FigureCase;
use crate::figures::cases::common::{colors, padded};
use plt::ticker::LabelFormat;

const HEIGHTS: [f64; 5] = [3.0, 7.0, 5.0, 2.0, 6.0];

/// `X = np.arange(len(HEIGHTS))`.
fn x() -> Vec<f64> {
    arange(0.0, 5.0, 1.0)
}

/// `HEIGHTS` scaled by `factor`.
fn heights(factor: f64) -> Vec<f64> {
    HEIGHTS.iter().map(|h| h * factor).collect()
}

/// `bar_label` options formatting values with `template`.
fn formatted(template: &str) -> BarLabelOptions {
    BarLabelOptions {
        fmt: LabelFormat::Template(template.to_owned()),
        ..BarLabelOptions::default()
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("bars/default", |fig| {
        fig.subplot().bar(x(), HEIGHTS, BarOptions::default())?;
        Ok(())
    }),
    FigureCase::new("bars/barh_default", |fig| {
        fig.subplot().barh(x(), HEIGHTS, BarOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/width_scalar_and_array", |fig| {
        let axes = fig.subplots(1, 2)?;
        let [ax1, ax2] = axes.as_slice() else {
            unreachable!("one row of two axes")
        };
        let narrow = BarOptions {
            thickness: vec![0.4],
            ..BarOptions::default()
        };
        ax1.bar(x(), HEIGHTS, narrow)?;
        let varying = BarOptions {
            thickness: vec![0.2, 0.4, 0.6, 0.8, 1.0],
            ..BarOptions::default()
        };
        ax2.bar(x(), HEIGHTS, varying)?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("bars/align_edge", |fig| {
        let axes = fig.subplots(1, 2)?;
        let [ax1, ax2] = axes.as_slice() else {
            unreachable!("one row of two axes")
        };
        let forward = BarOptions {
            thickness: vec![0.5],
            align: BarAlign::Edge,
            ..BarOptions::default()
        };
        ax1.bar(x(), HEIGHTS, forward)?;
        let backward = BarOptions {
            thickness: vec![-0.5],
            align: BarAlign::Edge,
            color: colors(&["C1"]),
            ..BarOptions::default()
        };
        ax2.bar(x(), HEIGHTS, backward)?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("bars/stacked_bottom", |fig| {
        let ax = fig.subplot();
        let a = [2.0, 3.0, 4.0, 1.0];
        let b = [1.0, 2.0, 1.0, 3.0];
        let c = [3.0, 1.0, 2.0, 2.0];
        let x = arange(0.0, 4.0, 1.0);
        let ab: Vec<f64> = a.iter().zip(&b).map(|(a, b)| a + b).collect();
        ax.bar(&x, a, BarOptions::default())?;
        let on_a = BarOptions {
            base: a.to_vec(),
            ..BarOptions::default()
        };
        ax.bar(&x, b, on_a)?;
        let on_ab = BarOptions {
            base: ab,
            ..BarOptions::default()
        };
        ax.bar(&x, c, on_ab)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/barh_stacked_left", |fig| {
        let ax = fig.subplot();
        let a = [2.0, 3.0, 4.0];
        let b = [4.0, 1.0, 2.0];
        let y = arange(0.0, 3.0, 1.0);
        let thin = BarOptions {
            thickness: vec![0.5],
            ..BarOptions::default()
        };
        ax.barh(&y, a, thin.clone())?;
        let on_a = BarOptions {
            base: a.to_vec(),
            ..thin
        };
        ax.barh(&y, b, on_a)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/colors_edgecolors_linewidth", |fig| {
        let options = BarOptions {
            color: colors(&["tab:red", "tab:green", "tab:blue", "gold", "0.6"]),
            edgecolor: colors(&["k"]),
            linewidth: vec![0.0, 1.0, 2.0, 3.0, 4.0],
            ..BarOptions::default()
        };
        fig.subplot().bar(x(), HEIGHTS, options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/hatches", |fig| {
        let hatches = [
            "/", "\\", "|", "-", "+", "x", "o", "O", ".", "*", "//", "xx", "/o", "\\|",
        ];
        let options = BarOptions {
            hatch: hatches.iter().map(|&h| h.to_owned()).collect(),
            color: colors(&["white"]),
            edgecolor: colors(&["k"]),
            ..BarOptions::default()
        };
        let x = arange(0.0, hatches.len() as f64, 1.0);
        fig.subplot().bar(x, [1.0], options)?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("bars/hatch_colors_and_linewidth", |fig| {
        let ax = fig.subplot();
        let blue = BarOptions {
            hatch: vec!["//".to_owned()],
            color: colors(&["lightblue"]),
            edgecolor: colors(&["navy"]),
            ..BarOptions::default()
        };
        ax.bar([0.0, 1.0, 2.0], [3.0, 2.0, 4.0], blue)?;
        let hollow = BarOptions {
            hatch: vec!["x".to_owned()],
            color: vec![Color::none()],
            edgecolor: colors(&["tab:red"]),
            props: plt::patches::PatchProps::default().hatchcolor("tab:green"),
            ..BarOptions::default()
        };
        ax.bar([3.0, 4.0], [2.0, 3.0], hollow)?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0)
    .with_rc(&[("hatch.linewidth", "2.0")]),
    FigureCase::new("bars/yerr_capsize", |fig| {
        let options = BarOptions {
            color: colors(&["C2"]),
            errors: ErrorbarOptions {
                yerr: Some(vec![0.5, 1.0, 0.3, 0.8, 0.6].into()),
                capsize: Some(5.0),
                ecolor: Some(Color::from("k")),
                ..ErrorbarOptions::default()
            },
            ..BarOptions::default()
        };
        fig.subplot().bar(x(), HEIGHTS, options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/asymmetric_yerr_error_kw", |fig| {
        let options = BarOptions {
            errors: ErrorbarOptions {
                yerr: Some(ErrorSize::Asymmetric {
                    lower: vec![0.5, 1.0, 0.2, 0.3, 1.0],
                    upper: vec![1.0, 0.5, 0.8, 1.5, 0.4],
                }),
                elinewidth: Some(2.0),
                capsize: Some(8.0),
                capthick: Some(2.0),
                ecolor: Some(Color::from("tab:red")),
                ..ErrorbarOptions::default()
            },
            ..BarOptions::default()
        };
        fig.subplot().bar(x(), HEIGHTS, options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/barh_xerr", |fig| {
        let options = BarOptions {
            errors: ErrorbarOptions {
                xerr: Some(0.7.into()),
                capsize: Some(3.0),
                ..ErrorbarOptions::default()
            },
            ..BarOptions::default()
        };
        fig.subplot().barh(x(), HEIGHTS, options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/bar_label_edge_default", |fig| {
        let ax = fig.subplot();
        let bars = ax.bar(x(), heights(1.5), BarOptions::default())?;
        ax.bar_label(&bars, BarLabelOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/bar_label_center_stacked", |fig| {
        let ax = fig.subplot();
        let a = [20.0, 35.0, 30.0];
        let b = [25.0, 32.0, 34.0];
        let x = arange(0.0, 3.0, 1.0);
        let lower = ax.bar(&x, a, BarOptions::default())?;
        let on_a = BarOptions {
            base: a.to_vec(),
            ..BarOptions::default()
        };
        let upper = ax.bar(&x, b, on_a)?;
        let center = BarLabelOptions {
            label_type: BarLabelType::Center,
            ..BarLabelOptions::default()
        };
        ax.bar_label(&lower, center.clone())?;
        for label in ax.bar_label(&upper, center)? {
            label.set_color("white");
        }
        ax.bar_label(&upper, BarLabelOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/bar_label_fmt_padding", |fig| {
        let axes = fig.subplots(1, 2)?;
        let [ax1, ax2] = axes.as_slice() else {
            unreachable!("one row of two axes")
        };
        let bars = ax1.bar(x(), heights(1.0 / 7.0), BarOptions::default())?;
        let two_places = BarLabelOptions {
            padding: vec![3.0],
            ..formatted("%.2f")
        };
        for label in ax1.bar_label(&bars, two_places)? {
            label.set_fontsize(8.0);
        }
        let bars = ax2.barh(x(), heights(1.0 / 7.0), BarOptions::default())?;
        let percent = BarLabelOptions {
            padding: vec![-25.0],
            ..formatted("{:.0%}")
        };
        for label in ax2.bar_label(&bars, percent)? {
            label.set_color("white");
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("bars/bar_label_custom_labels", |fig| {
        let ax = fig.subplot();
        let bars = ax.bar(x(), HEIGHTS, BarOptions::default())?;
        let options = BarLabelOptions {
            labels: ["a", "bb", "ccc", "", "e"].map(String::from).to_vec(),
            ..padded(2.0)
        };
        for label in ax.bar_label(&bars, options)? {
            label.set_rotation(90.0);
        }
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/negative_values_labeled", |fig| {
        let ax = fig.subplot();
        let values = [3.0, -2.0, 4.5, -1.0, 0.5];
        let options = BarOptions {
            color: values
                .iter()
                .map(|&v| Color::from(if v < 0.0 { "tab:red" } else { "tab:green" }))
                .collect(),
            ..BarOptions::default()
        };
        let bars = ax.bar(x(), values, options)?;
        ax.axhline(0.0, LineProps::default().color("k").linewidth(0.8));
        ax.bar_label(&bars, formatted("%+.1f"))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/negative_barh_labeled", |fig| {
        let ax = fig.subplot();
        let bars = ax.barh(x(), [-3.0, 2.0, -1.5, 4.0, 1.0], BarOptions::default())?;
        ax.bar_label(&bars, padded(4.0))?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/log_scale", |fig| {
        let options = BarOptions {
            log: true,
            ..BarOptions::default()
        };
        fig.subplot()
            .bar(x(), [3.0, 40.0, 700.0, 5000.0, 25.0], options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/categorical_strings", |fig| {
        fig.subplot().bar(
            ["apple", "banana", "cherry", "date"],
            [4.0, 6.0, 3.0, 5.0],
            BarOptions::default(),
        )?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/categorical_repeated_calls", |fig| {
        let ax = fig.subplot();
        ax.barh(
            ["north", "south", "east"],
            [10.0, 20.0, 15.0],
            BarOptions::default(),
        )?;
        let orange = BarOptions {
            color: colors(&["C1"]),
            ..BarOptions::default()
        };
        ax.barh(["west", "north"], [5.0, 8.0], orange)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/tick_label_param", |fig| {
        let options = BarOptions {
            tick_labels: vec!["one".into(), "three".into(), "five".into()],
            thickness: vec![1.5],
            ..BarOptions::default()
        };
        fig.subplot()
            .bar([1.0, 3.0, 5.0], [2.0, 5.0, 3.0], options)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("bars/broken_barh", |fig| {
        let ax = fig.subplot();
        let ranges = [(10.0, 50.0), (100.0, 20.0), (130.0, 10.0)];
        ax.broken_barh(&ranges, (20.0, 9.0), BandAlign::Bottom)
            .set_facecolor("tab:blue");
        ax.broken_barh(&ranges, (10.0, 9.0), BandAlign::Bottom)
            .set_facecolors(colors(&["tab:orange", "tab:green", "tab:red"]));
        ax.set_ylim(5.0, 35.0)?.set_xlim(0.0, 200.0)?;
        ax.set_yticks([15.0, 25.0])
            .set_yticklabels(["Bill", "Jim"])?;
        ax.grid(true);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("bars/grouped_via_offsets", |fig| {
        let ax = fig.subplot();
        let groups = ["G1", "G2", "G3", "G4"];
        let series = [
            ("men", [20.0, 34.0, 30.0, 35.0]),
            ("women", [25.0, 32.0, 34.0, 20.0]),
            ("kids", [10.0, 15.0, 12.0, 18.0]),
        ];
        let x = arange(0.0, 4.0, 1.0);
        let width = 0.25;
        for (i, (name, values)) in (0_i32..).zip(series) {
            let offset = f64::from(i - 1) * width;
            let options = BarOptions {
                thickness: vec![width],
                label: Some(name.to_owned()),
                ..BarOptions::default()
            };
            let bars = ax.bar(
                x.iter().map(|x| x + offset).collect::<Vec<_>>(),
                values,
                options,
            )?;
            for label in ax.bar_label(&bars, padded(2.0))? {
                label.set_fontsize(7.0);
            }
        }
        ax.set_xticks(x);
        ax.set_xticklabels(groups)?;
        ax.set_ylim(0.0, 40.0)?;
        ax.legend(LegendOptions::default().loc("upper left").ncols(3))?;
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("bars/grouped_bar_lists", |fig| {
        let ax = fig.subplot();
        let heights = GroupedBarData::Datasets(vec![
            vec![1.0, 2.0, 3.0],
            vec![1.5, 2.5, 2.0],
            vec![0.5, 1.0, 4.0],
        ]);
        let options = GroupedBarOptions {
            tick_labels: ["A", "B", "C"].map(String::from).to_vec(),
            labels: ["x", "y", "z"].map(String::from).to_vec(),
            ..GroupedBarOptions::default()
        };
        ax.grouped_bar(heights, options)?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("bars/grouped_bar_dict_horizontal", |fig| {
        let data = GroupedBarData::Named(vec![
            ("2023".into(), vec![3.0, 5.0]),
            ("2024".into(), vec![4.0, 2.0]),
            ("2025".into(), vec![6.0, 1.0]),
        ]);
        let options = GroupedBarOptions {
            tick_labels: ["east", "west"].map(String::from).to_vec(),
            orientation: Orientation::Horizontal,
            group_spacing: 0.5,
            bar_spacing: 0.1,
            ..GroupedBarOptions::default()
        };
        fig.subplot().grouped_bar(data, options)?;
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
];
