//! Probe operations for the `mpl_table` area: matplotlib's own non-image
//! tests of tables (see `spec/reference/probes/test_mpl_table.py`).

use plt::figure::Figure;
use plt::table::{CellEdges, TableLoc, TableOptions};
use plt_core::path::convert::{CleanupOptions, SimplifySettings};
use plt_core::{PathCode, Segment};

use super::mpl_tests::{in_test_style, numbers};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("CustomCell.get_path", customcell_path),
    Op::new("Table.fontsize", table_fontsize),
];

/// The segment codes of the outline of a unit cell drawing the edges
/// `visible_edges` describes (`test_customcell`).
fn customcell_path(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let edges: CellEdges = args
            .str("visible_edges")?
            .parse()
            .map_err(ProbeError::rejected)?;
        let table = Figure::new()
            .subplot()
            .table(TableOptions::with_text(vec![vec![String::new()]]))
            .map_err(ProbeError::rejected)?;
        let path = table
            .add_cell(1, 0, 1.0, 1.0)
            .set_visible_edges(edges)
            .path();
        let options = CleanupOptions::segment_defaults(&path, &SimplifySettings::default());
        let codes = path.iter_segments(&options).map(|segment| match segment {
            Segment::MoveTo(_) => PathCode::MoveTo,
            Segment::LineTo(_) => PathCode::LineTo,
            Segment::QuadTo(..) => PathCode::Curve3,
            Segment::CubicTo(..) => PathCode::Curve4,
            Segment::Close => PathCode::ClosePoly,
        });
        Ok(numbers(codes.map(|code| f64::from(code as u8))))
    })
}

/// The font size of two cells of a table made with `fontsize`
/// (`test_table_fontsize`).
fn table_fontsize(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let text = [["a", "1"], ["b", "2"]]
            .iter()
            .map(|row| row.iter().map(|&s| s.to_owned()).collect())
            .collect();
        let table = Figure::new()
            .subplot()
            .table(TableOptions {
                loc: TableLoc::Top,
                fontsize: Some(args.f64("fontsize")?),
                ..TableOptions::with_text(text)
            })
            .map_err(ProbeError::rejected)?;
        let size = |row, column| {
            table
                .cell(row, column)
                .map(|cell| cell.font().size)
                .ok_or_else(|| ProbeError::rejected("no such cell"))
        };
        Ok(numbers([size(0, 0)?, size(1, 1)?]))
    })
}
