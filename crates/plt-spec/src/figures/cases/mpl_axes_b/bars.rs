//! Bars, their tick labels and grouped bars.

use plt::axes::{BarOptions, GroupedBarData, GroupedBarOptions};
use plt::colors::Color;
use plt::figure::Figure;
use plt::legend::LegendOptions;
use plt::numeric::Grid;

use super::classic;
use crate::figures::{CaseResult, FigureCase};

const DECIMAL_X: [f64; 4] = [1.5, 8.4, 5.3, 4.2];
const DECIMAL_Y: [f64; 4] = [1.1, 2.2, 3.3, 4.4];
const DECIMAL_SIZES: [f64; 4] = [0.7, 1.45, 1.0, 2.0];

/// Bars of the given thicknesses labelled `a` and `b`.
fn labelled(thickness: [f64; 2]) -> BarOptions {
    BarOptions {
        thickness: thickness.to_vec(),
        tick_labels: vec!["a".into(), "b".into()],
        ..BarOptions::default()
    }
}

/// `test_bar_tick_label_multiple` (and its old-alignment variant).
fn bar_tick_label_multiple(fig: &Figure) -> CaseResult {
    fig.subplot()
        .bar([1.0, 2.5], [1.0, 2.0], labelled([0.2, 0.5]))?;
    Ok(())
}

/// Bars of the given thicknesses.
fn thick(thickness: &[f64]) -> BarOptions {
    BarOptions {
        thickness: thickness.to_vec(),
        ..BarOptions::default()
    }
}

/// `GROUPED`: three datasets over two categories.
fn grouped() -> Vec<(String, Vec<f64>)> {
    vec![
        ("data1".into(), vec![1.0, 1.2]),
        ("data2".into(), vec![2.0, 2.4]),
        ("data3".into(), vec![3.0, 3.6]),
    ]
}

/// Grouped bars over categories `A` and `B`, with a legend.
fn grouped_with_legend(fig: &Figure, data: GroupedBarData, labels: Vec<String>) -> CaseResult {
    let ax = fig.subplot();
    let options = GroupedBarOptions {
        tick_labels: vec!["A".into(), "B".into()],
        labels,
        ..GroupedBarOptions::default()
    };
    ax.grouped_bar(data, options)?;
    ax.legend(LegendOptions::default())?;
    Ok(())
}

/// The names of [`grouped`].
fn grouped_names() -> Vec<String> {
    grouped().into_iter().map(|(name, _)| name).collect()
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "mpl_axes_b/bar_tick_label_multiple",
        bar_tick_label_multiple,
    ),
    FigureCase::new(
        "mpl_axes_b/bar_tick_label_multiple_old_alignment",
        bar_tick_label_multiple,
    )
    .with_rc(&[("ytick.alignment", "center")]),
    classic("mpl_axes_b/bar_decimal_center", |fig| {
        fig.subplot()
            .bar(DECIMAL_X, DECIMAL_Y, BarOptions::default())?;
        Ok(())
    }),
    classic("mpl_axes_b/barh_decimal_center", |fig| {
        fig.subplot()
            .barh(DECIMAL_X, DECIMAL_Y, thick(&[0.5, 0.5, 1.0, 1.0]))?;
        Ok(())
    }),
    classic("mpl_axes_b/bar_decimal_width", |fig| {
        fig.subplot()
            .bar(DECIMAL_X, DECIMAL_Y, thick(&DECIMAL_SIZES))?;
        Ok(())
    }),
    classic("mpl_axes_b/barh_decimal_height", |fig| {
        fig.subplot()
            .barh(DECIMAL_X, DECIMAL_Y, thick(&DECIMAL_SIZES))?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_b/barh_tick_label", |fig| {
        fig.subplot()
            .barh([1.0, 2.5], [1.0, 2.0], labelled([0.2, 0.5]))?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_b/grouped_bar", |fig| {
        let data = GroupedBarData::Named(vec![
            ("data1".into(), vec![1.0, 2.0, 3.0]),
            ("data2".into(), vec![1.2, 2.2, 3.2]),
            ("data3".into(), vec![1.4, 2.4, 3.4]),
        ]);
        let options = GroupedBarOptions {
            tick_labels: ["A", "B", "C"].map(String::from).to_vec(),
            group_spacing: 0.5,
            bar_spacing: 0.1,
            colors: ["#1f77b4", "#58a1cf", "#abd0e6"].map(Color::from).to_vec(),
            ..GroupedBarOptions::default()
        };
        let ax = fig.subplot();
        ax.grouped_bar(data, options)?;
        ax.set_yticks(Vec::<f64>::new());
        Ok(())
    }),
    classic("mpl_axes_b/grouped_bar_list_of_datasets", |fig| {
        let datasets = grouped().into_iter().map(|(_, values)| values).collect();
        grouped_with_legend(fig, GroupedBarData::Datasets(datasets), grouped_names())
    }),
    classic("mpl_axes_b/grouped_bar_list_of_datasets_ref", |fig| {
        let ax = fig.subplot();
        // Three bars and a group spacing of 1.5 bar widths per category.
        let bar_width = 1.0 / (3.0 + 1.5);
        let shifts = [0.0, bar_width, 2.0 * bar_width].map(|offset| -bar_width + offset);
        for (shift, (label, values)) in shifts.into_iter().zip(grouped()) {
            let options = BarOptions {
                thickness: vec![bar_width],
                label: Some(label),
                ..BarOptions::default()
            };
            ax.bar([shift, 1.0 + shift], values, options)?;
        }
        ax.set_xticks([0.0, 1.0]);
        ax.set_xticklabels(["A", "B"])?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    }),
    classic("mpl_axes_b/grouped_bar_dict_of_datasets", |fig| {
        grouped_with_legend(fig, GroupedBarData::Named(grouped()), Vec::new())
    }),
    classic("mpl_axes_b/grouped_bar_array", |fig| {
        let table = Grid::from_vec(2, 3, vec![1.0, 2.0, 3.0, 1.2, 2.4, 3.6])?;
        grouped_with_legend(fig, GroupedBarData::Table(table), grouped_names())
    }),
    FigureCase::new("mpl_axes_b/bar_hatches", |fig| {
        let options = BarOptions {
            hatch: vec!["x".into(), "o".into()],
            ..BarOptions::default()
        };
        fig.subplot().bar([1.0, 2.0], [2.0, 3.0], options)?;
        Ok(())
    }),
];
