//! `figures/cases_bars.py`.

use plt::axes::{BandAlign, BarAlign, BarOptions, ErrorSize, ErrorbarOptions};
use plt::colors::Color;
use plt::numeric::arange;

use crate::figures::FigureCase;

const HEIGHTS: [f64; 5] = [3.0, 7.0, 5.0, 2.0, 6.0];

/// `X = np.arange(len(HEIGHTS))`.
fn x() -> Vec<f64> {
    arange(0.0, 5.0, 1.0)
}

/// `[Color::from(name), ...]`.
fn colors(names: &[&str]) -> Vec<Color> {
    names.iter().map(|&name| Color::from(name)).collect()
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
        ax.set_ylim(5.0, 35.0).set_xlim(0.0, 200.0);
        ax.set_yticks([15.0, 25.0])
            .set_yticklabels(["Bill", "Jim"])?;
        ax.grid(true);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
];
