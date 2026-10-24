//! `figures/cases_api_gaps_a.py`: cells added to a table one by one and a
//! 3-D z axis of dates.

use std::f64::consts::PI;

use plt::dates::TimeZone;
use plt::prelude::*;
use plt::table::{TableLoc, TableOptions};
use plt::text::HAlign;

use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("api_gaps_a/table_add_cell", table_add_cell).sized(5.0, 3.0, 100.0),
    FigureCase::new("api_gaps_a/axes3d_zaxis_date", zaxis_date),
    FigureCase::new("api_gaps_a/axes3d_zaxis_date_concise", zaxis_date_concise)
        .with_rc(&[("date.converter", "concise")]),
];

fn table_add_cell(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_axis_off();
    let text = |cells: &[&str]| cells.iter().map(|&s| s.to_owned()).collect::<Vec<_>>();
    let table = ax.table(TableOptions {
        col_labels: Some(text(&["A", "B"])),
        loc: TableLoc::Center,
        col_widths: Some(vec![0.2, 0.2]),
        ..TableOptions::with_text(vec![text(&["1", "2"]), text(&["3", "4"])])
    })?;
    table
        .add_cell(3, 0, 0.2, 0.1)
        .set_text("sum")
        .set_text_halign(HAlign::Center)
        .set_facecolor("#ddeeff");
    table.add_cell(3, 1, 0.2, 0.1).set_text("10");
    table
        .add_cell(1, -1, 0.15, 0.05)
        .set_text("row")
        .set_text_halign(HAlign::Left);
    table
        .add_cell(0, 1, 0.3, 0.05)
        .set_text("wide B")
        .set_facecolor("gold");
    Ok(())
}

fn zaxis_date(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let t = linspace(0.0, 4.0 * PI, 60);
    let x: Vec<f64> = t.iter().map(|t| t.cos()).collect();
    let y: Vec<f64> = t.iter().map(|t| t.sin()).collect();
    let z: Vec<f64> = t.iter().map(|t| 19_000.0 + t).collect();
    ax.plot(&x, &y, &z)?;
    ax.zaxis().axis_date(&TimeZone::Utc);
    Ok(())
}

fn zaxis_date_concise(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    ax.set_zlim(19_000.0, 19_000.5)?;
    ax.zaxis().axis_date(&TimeZone::Utc);
    Ok(())
}
