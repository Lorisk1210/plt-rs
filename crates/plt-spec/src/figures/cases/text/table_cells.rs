//! Table cases of `figures/cases_text.py` that style single cells, place
//! tables outside the axes and fit their fonts.

use plt::colors::colormaps;
use plt::fonts::{FontProperties, FontStyle, FontWeight};
use plt::gridspec::SubplotOverrides;
use plt::prelude::*;
use plt::table::{Table, TableCell, TableLoc, TableOptions};
use plt::text::HAlign;
use plt_core::Bbox;

use super::tables::{rows, strings};
use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("text/table_cell_styles_header", cell_styles).sized(5.0, 3.0, 100.0),
    FigureCase::new("text/table_visible_edges_per_cell", visible_edges).sized(5.0, 3.0, 100.0),
    FigureCase::new("text/table_outside_locations", outside).sized(6.0, 4.5, 100.0),
    FigureCase::new("text/table_auto_font_size_shrinks", auto_font).sized(6.0, 3.0, 100.0),
    FigureCase::new("text/table_bbox_cell_locs", bbox_locs).sized(5.0, 4.0, 100.0),
    FigureCase::new("text/table_colours_only_scaled", colours_only).sized(5.0, 3.0, 100.0),
    FigureCase::new("text/table_cell_size_text_edit", size_text_edit).sized(5.0, 3.0, 100.0),
];

/// `table[(row, col)]`.
fn cell(table: &Table, row: i32, col: i32) -> Result<TableCell, &'static str> {
    table.cell(row, col).ok_or("no such cell")
}

fn cell_styles(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_axis_off();
    let table = ax.table(TableOptions {
        col_labels: Some(strings(["day", "count", "mean"])),
        loc: TableLoc::Center,
        ..TableOptions::with_text(rows([
            ["Mon", "12", "3.5"],
            ["Tue", "15", "4.0"],
            ["Wed", "9", "2.25"],
            ["Thu", "21", "5.5"],
        ]))
    })?;
    for (row, col) in table.cell_keys() {
        let cell = cell(&table, row, col)?;
        cell.set_edgecolor("0.7");
        if row == 0 {
            let bold = cell.font().with_weight(FontWeight::BOLD);
            cell.set_facecolor("#40466e")
                .set_font(bold)
                .set_text_color("w")
                .set_linewidth(2.0);
        } else if row % 2 == 0 {
            cell.set_facecolor("#f1f1f2");
        }
    }
    cell(&table, 4, 1)?.set_text_color("tab:red");
    Ok(())
}

fn visible_edges(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_axis_off();
    let table = ax.table(TableOptions {
        loc: TableLoc::Center,
        cell_loc: Some(HAlign::Center),
        ..TableOptions::with_text(rows([["open", "B", "LR"], ["T", "closed", "BRT"]]))
    })?;
    for (row, col) in table.cell_keys() {
        let cell = cell(&table, row, col)?;
        cell.set_visible_edges(cell.text().parse()?);
    }
    table.scale(1.0, 2.0);
    Ok(())
}

fn outside(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot([0.0, 1.0], [0.0, 1.0])?;
    for loc in ["top", "left", "right", "bottom right", "top left"] {
        ax.table(TableOptions {
            loc: loc.parse()?,
            col_widths: Some(vec![0.2]),
            ..TableOptions::with_text(rows([[loc]]))
        })?;
    }
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.25),
        right: Some(0.75),
        top: Some(0.8),
        bottom: Some(0.2),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

fn auto_font(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let options = TableOptions {
        col_widths: Some(vec![0.4, 0.4]),
        loc: TableLoc::Center,
        ..TableOptions::with_text(rows([
            ["a fairly long entry", "x"],
            ["short", "another long entry"],
        ]))
    };
    for ax in axes.as_slice() {
        ax.set_axis_off();
    }
    axes[(0, 0)].table(options.clone())?;
    let fixed = axes[(0, 1)].table(options)?;
    fixed.auto_set_font_size(false);
    fixed.set_fontsize(7.0);
    Ok(())
}

fn bbox_locs(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xticks(Vec::<f64>::new());
    ax.set_yticks(Vec::<f64>::new());
    ax.table(TableOptions {
        row_labels: Some(strings(["first", "second"])),
        col_labels: Some(strings(["A", "B", "C"])),
        cell_loc: Some(HAlign::Left),
        row_loc: Some(HAlign::Center),
        col_loc: Some(HAlign::Right),
        bbox: Some(Bbox::from_bounds(0.25, 0.2, 0.7, 0.5)),
        ..TableOptions::with_text(rows([["1", "22", "333"], ["4444", "5", "66"]]))
    })?;
    Ok(())
}

fn colours_only(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_axis_off();
    let viridis = colormaps::get("viridis")?;
    let values = linspace(0.0, 1.0, 12);
    let colours = values
        .chunks(4)
        .map(|row| {
            row.iter()
                .map(|&v| Color::from(viridis.sample(v)))
                .collect()
        })
        .collect();
    let table = ax.table(TableOptions {
        cell_colours: Some(colours),
        row_labels: Some(strings(["r1", "r2", "r3"])),
        col_labels: Some(strings(["w", "x", "y", "z"])),
        loc: TableLoc::Center,
        ..TableOptions::default()
    })?;
    table.scale(1.0, 2.5);
    for col in 0..4 {
        cell(&table, 0, col)?.set_visible_edges("B".parse()?);
    }
    cell(&table, 1, 2)?.set_height(0.2);
    Ok(())
}

fn size_text_edit(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_axis_off();
    let table = ax.table(TableOptions {
        loc: TableLoc::Center,
        col_widths: Some(vec![0.2, 0.2]),
        ..TableOptions::with_text(rows([["a", "b"], ["c", "d"]]))
    })?;
    cell(&table, 0, 0)?.set_text("edited");
    cell(&table, 0, 1)?.set_width(0.35);
    let below_left = cell(&table, 1, 0)?;
    let italic = FontProperties {
        style: FontStyle::Italic,
        ..below_left.font()
    };
    below_left.set_font(italic).set_text_halign(HAlign::Left);
    cell(&table, 1, 1)?
        .set_facecolor("tab:olive")
        .set_alpha(Some(0.5));
    Ok(())
}
