//! Table cases of `figures/cases_text.py`.

use pltrs::gridspec::SubplotOverrides;
use pltrs::prelude::*;
use pltrs::table::{TableLoc, TableOptions};
use pltrs::text::HAlign;

use crate::figures::FigureCase;
use crate::figures::cases::common::strings;

/// Rows of owned strings.
pub(super) fn rows<const R: usize, const C: usize>(cells: [[&str; C]; R]) -> Vec<Vec<String>> {
    cells
        .iter()
        .map(|row| row.iter().map(|&s| s.to_owned()).collect())
        .collect()
}

/// `_table_cells()`.
fn table_cells() -> Vec<Vec<String>> {
    rows([
        ["1.0", "2.5", "3.25"],
        ["10", "20", "30"],
        ["a", "bb", "ccc"],
    ])
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("text/table_bottom", |fig| {
        let ax = fig.subplot();
        ax.plot([0.0, 1.0, 2.0], [1.0, 3.0, 2.0])?;
        ax.table(TableOptions {
            row_labels: Some(strings(&["r1", "r2", "r3"])),
            col_labels: Some(strings(&["A", "B", "C"])),
            loc: TableLoc::Bottom,
            ..TableOptions::with_text(table_cells())
        })?;
        fig.subplots_adjust(SubplotOverrides {
            bottom: Some(0.3),
            ..SubplotOverrides::default()
        })?;
        ax.set_xticks(Vec::<f64>::new());
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("text/table_loc_variants", |fig| {
        let ax = fig.subplot();
        for loc in [
            "upper left",
            "upper right",
            "center",
            "lower left",
            "lower right",
        ] {
            ax.table(TableOptions {
                col_widths: Some(vec![0.25]),
                loc: loc.parse()?,
                ..TableOptions::with_text(rows([[loc], ["x"]]))
            })?;
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("text/table_colours_widths_fontsize", |fig| {
        let ax = fig.subplot();
        ax.set_axis_off();
        let pastel = || ["#fdd", "#dfd", "#ddf"].map(Color::from).to_vec();
        let table = ax.table(TableOptions {
            cell_colours: Some(vec![pastel(), pastel(), pastel()]),
            row_labels: Some(strings(&["r1", "r2", "r3"])),
            row_colours: Some(vec![Color::from("0.8"); 3]),
            col_labels: Some(strings(&["A", "B", "C"])),
            col_colours: Some(vec![Color::from("gold"); 3]),
            col_widths: Some(vec![0.15, 0.3, 0.2]),
            cell_loc: Some(HAlign::Right),
            loc: TableLoc::Center,
            ..TableOptions::with_text(table_cells())
        })?;
        table.auto_set_font_size(false);
        table.set_fontsize(12.0);
        table.scale(1.0, 1.6);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("text/table_edges_and_auto_width", |fig| {
        let ax = fig.subplot();
        ax.set_axis_off();
        let table = ax.table(TableOptions {
            col_labels: Some(strings(&["one", "two"])),
            edges: "horizontal".parse()?,
            loc: TableLoc::UpperCenter,
            ..TableOptions::with_text(rows([["short", "a much longer cell"], ["x", "y"]]))
        })?;
        table.auto_set_column_width(&[0, 1]);
        ax.table(TableOptions {
            edges: "BT".parse()?,
            loc: TableLoc::LowerCenter,
            ..TableOptions::with_text(rows([["open", "closed"]]))
        })?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
];
