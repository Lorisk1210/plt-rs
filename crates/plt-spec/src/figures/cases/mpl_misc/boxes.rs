//! The cases from `test_offsetbox.py` and `test_table.py`.

use std::f64::consts::TAU;
use std::fmt;

use plt::colors::{Color, colormaps};
use plt::layout::LayoutEngine;
use plt::numeric::linspace;
use plt::offsetbox::{
    AnchorLoc, AnchoredBox, AnchoredOptions, BoxItem, DrawingArea, PaddedBox, TextArea,
};
use plt::patches::Rectangle;
use plt::prelude::*;
use plt::table::{TableLoc, TableOptions};
use plt::text::{HAlign, TextProps};
use plt::units::{ConversionError, ConversionInterface, Convertible, UnitAxis};
use plt_core::Bbox;

use super::{classic, remove_text};
use crate::figures::cases::common::strings;
use crate::figures::{CaseResult, FigureCase};

/// Rows of owned strings.
fn rows<const R: usize, const C: usize>(cells: [[&str; C]; R]) -> Vec<Vec<String>> {
    cells
        .iter()
        .map(|row| row.iter().map(|&cell| cell.to_owned()).collect())
        .collect()
}

// --- test_offsetbox.py ---------------------------------------------------

/// `test_offsetbox_clipping`: a thick line across a clipping drawing area
/// stops at its edges.
fn offsetbox_clipping(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let size = 100.0;
    let background = BoxItem::patch(
        Rectangle::new((0.0, 0.0), size, size),
        PatchProps::default()
            .facecolor("#CCCCCC")
            .edgecolor("None")
            .linewidth(0.0),
    );
    let line = BoxItem::line(
        vec![-size * 0.5, size * 1.5],
        vec![size / 2.0, size / 2.0],
        LineProps::default().color("black").linewidth(10.0),
    )?;
    let mut area = DrawingArea::new(size, size, 0.0, 0.0)
        .with_artist(background)
        .with_artist(line);
    area.clip = true;
    ax.add_anchored_box(AnchoredBox::new(
        area,
        AnchoredOptions {
            pad: 0.0,
            frameon: false,
            bbox_to_anchor: Some((0.5, 0.5).into()),
            bbox_transform: Some(ax.trans_axes()),
            borderpad: (0.0, 0.0),
            ..AnchoredOptions::new(AnchorLoc::Center)
        },
    ));
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    remove_text(fig);
    Ok(())
}

/// `test_anchoredtext_horizontal_alignment`: two-line anchored texts
/// aligned left, centred and right.
fn anchoredtext_horizontal_alignment(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    for (loc, halign) in [
        (AnchorLoc::CenterLeft, HAlign::Left),
        (AnchorLoc::Center, HAlign::Center),
        (AnchorLoc::CenterRight, HAlign::Right),
    ] {
        ax.add_anchored_box(AnchoredBox::text(
            "test\ntest long text",
            TextProps::default().halign(halign),
            AnchoredOptions {
                pad: 0.2,
                ..AnchoredOptions::new(loc)
            },
        )?);
    }
    remove_text(fig);
    Ok(())
}

/// `test_textarea_set_text`: an anchored text changed after it was added.
fn textarea_set_text(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    let anchored = ax.add_anchored_box(AnchoredBox::text(
        "Bar",
        TextProps::default(),
        AnchoredOptions::new(AnchorLoc::UpperLeft),
    )?);
    for text in anchored.texts() {
        text.set_text("Foo");
    }
    Ok(())
}

/// `test_paddedbox`: texts in padded boxes, framed or not.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn paddedbox(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let boxes = [
        ("foo", 5.0, true, Some("r"), AnchorLoc::UpperLeft),
        ("bar", 10.0, false, Some("b"), AnchorLoc::UpperRight),
        ("foobar", 15.0, true, None, AnchorLoc::LowerRight),
    ];
    for (text, pad, draw_frame, face, loc) in boxes {
        let frame = PatchProps {
            facecolor: face.map(Color::from),
            ..PatchProps::default()
        };
        let padded = PaddedBox::new(
            TextArea::from_text(text, TextProps::default()),
            pad,
            draw_frame,
            frame,
        );
        ax.add_anchored_box(AnchoredBox::new(padded, AnchoredOptions::new(loc)));
    }
    remove_text(fig);
    Ok(())
}

// --- test_table.py -------------------------------------------------------

/// `test_zorder`: tables below and above a thick line.
fn zorder(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 1, 1)?;
    let t = linspace(0.0, TAU, 100);
    let cosine: Vec<f64> = t.iter().map(|t| t.cos()).collect();
    ax.plot_with(&t, cosine, LineProps::default().linewidth(4.0).zorder(2.0))?;
    // The cumulative sums of the data rows, bottom row first, in
    // thousands.
    let data = [[66386.0, 174_296.0], [58230.0, 381_139.0]];
    let mut yoff = [0.0; 2];
    let cell_text: Vec<Vec<String>> = data
        .iter()
        .rev()
        .map(|row| {
            yoff.iter_mut().zip(row).for_each(|(sum, x)| *sum += x);
            yoff.iter()
                .map(|sum| format!("{:.1}", sum / 1000.0))
                .collect()
        })
        .collect();
    for (loc, zorder) in [(TableLoc::Center, -2.0), (TableLoc::UpperCenter, 4.0)] {
        ax.table(TableOptions {
            row_labels: Some(strings(&["100 year", "50 year"])),
            col_labels: Some(strings(&["Freeze", "Wind"])),
            loc,
            ..TableOptions::with_text(cell_text.clone())
        })?
        .set_zorder(zorder);
    }
    ax.set_yticks(Vec::<f64>::new());
    remove_text(fig);
    Ok(())
}

/// `test_label_colours`: row and column colours, with and without
/// headers.
fn label_colours(fig: &Figure) -> CaseResult {
    let cmap = colormaps::get("RdYlGn")?;
    let colours: Vec<Color> = linspace(0.0, 1.0, 3)
        .into_iter()
        .map(|x| cmap.sample(x).into())
        .collect();
    let headers = || Some(strings(&["Header"; 3]));
    let tables = [
        TableOptions {
            row_colours: Some(colours.clone()),
            ..TableOptions::default()
        },
        TableOptions {
            row_colours: Some(colours.clone()),
            row_labels: headers(),
            ..TableOptions::default()
        },
        TableOptions {
            col_colours: Some(colours.clone()),
            ..TableOptions::default()
        },
        TableOptions {
            col_colours: Some(colours),
            col_labels: headers(),
            ..TableOptions::default()
        },
    ];
    for (i, options) in (1..).zip(tables) {
        let ax = fig.add_subplot(4, 1, i)?;
        ax.set_axis_off();
        ax.table(TableOptions {
            cell_text: rows([["1"; 3]; 3]),
            loc: TableLoc::Best,
            ..options
        })?;
    }
    Ok(())
}

/// `test_diff_cell_table`: every kind of visible cell edges, under the
/// tight layout.
fn diff_cell_table(fig: &Figure) -> CaseResult {
    let cells = [
        "horizontal",
        "vertical",
        "open",
        "closed",
        "T",
        "R",
        "B",
        "L",
    ];
    fig.set_layout_engine(LayoutEngine::tight())?;
    let axes = fig.subplots(cells.len(), 1)?;
    for (ax, edges) in axes.as_slice().iter().zip(cells) {
        ax.table(TableOptions {
            col_widths: Some(vec![0.1; cells.len()]),
            loc: TableLoc::Center,
            edges: edges.parse()?,
            ..TableOptions::with_text(vec![vec!["1".to_owned(); cells.len()]; 2])
        })?;
        ax.set_axis_off();
    }
    Ok(())
}

/// `test_auto_column`: columns fitted to their text, given as a list, a
/// tuple and one by one, and not fitted.
fn auto_column(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(4, 1)?;
    let fitted: [&[&[i32]]; 4] = [&[&[-1, 0, 1]], &[&[-1, 0, 1]], &[&[-1], &[0], &[1]], &[]];
    for (ax, columns) in axes.as_slice().iter().zip(fitted) {
        ax.set_axis_off();
        let table = ax.table(TableOptions {
            row_labels: Some(strings(&["A", "B"])),
            col_labels: Some(strings(&["Col1", "Col2"])),
            loc: TableLoc::Center,
            ..TableOptions::with_text(rows([
                ["Fit Text", "2"],
                ["very long long text, Longer text than default", "1"],
            ]))
        })?;
        table.auto_set_font_size(false).set_fontsize(12.0);
        for &column in columns {
            table.auto_set_column_width(column);
        }
    }
    Ok(())
}

/// `test_table_bbox`: a table stretched into a box of the axes.
fn table_bbox(fig: &Figure) -> CaseResult {
    fig.subplot().table(TableOptions {
        row_labels: Some(strings(&["Ada", "Bob"])),
        col_labels: Some(strings(&["Foo", "Bar"])),
        loc: TableLoc::Center,
        bbox: Some(Bbox::from_bounds(0.1, 0.2, 0.8, 0.6)),
        ..TableOptions::with_text(rows([["2", "3"], ["4", "5"]]))
    })?;
    Ok(())
}

/// `test_table_unit`'s `FakeUnit`: shown as `Hello`, whatever it holds.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FakeUnit;

impl fmt::Display for FakeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Hello")
    }
}

/// The test's mocked converter of `FakeUnit`s, which tables must not
/// call: it fails, which the figure would report when it draws.
struct UncalledConverter;

impl ConversionInterface for UncalledConverter {
    type Value = FakeUnit;
    type Unit = ();

    fn convert(
        &self,
        _: &[FakeUnit],
        _: Option<&()>,
        _: &UnitAxis,
    ) -> Result<Vec<f64>, ConversionError> {
        Err(ConversionError::Failed(
            "tables do not convert units".to_owned(),
        ))
    }
}

impl Convertible for FakeUnit {
    type Converter = UncalledConverter;

    fn converter() -> UncalledConverter {
        UncalledConverter
    }
}

/// `test_table_unit`: the cells of values with a unit converter show
/// their text (Python's `str`), and the converter is not called.
fn table_unit(fig: &Figure) -> CaseResult {
    let data = [[FakeUnit; 2]; 2];
    let cell_text = data
        .iter()
        .map(|row| row.iter().map(ToString::to_string).collect())
        .collect();
    fig.subplot().table(TableOptions::with_text(cell_text))?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_misc/offsetbox_clipping", offsetbox_clipping),
    FigureCase::new(
        "mpl_misc/anchoredtext_horizontal_alignment",
        anchoredtext_horizontal_alignment,
    ),
    classic("mpl_misc/textarea_set_text", textarea_set_text),
    FigureCase::new("mpl_misc/paddedbox", paddedbox),
    FigureCase::new("mpl_misc/zorder", zorder),
    FigureCase::new("mpl_misc/label_colours", label_colours),
    FigureCase::new("mpl_misc/diff_cell_table", diff_cell_table).sized(4.0, 9.0, 100.0),
    FigureCase::new("mpl_misc/auto_column", auto_column),
    classic("mpl_misc/table_bbox", table_bbox),
    classic("mpl_misc/table_bbox_ref", table_bbox),
    classic("mpl_misc/table_unit", table_unit),
    classic("mpl_misc/table_unit_ref", |fig| {
        fig.subplot()
            .table(TableOptions::with_text(rows([["Hello"; 2]; 2])))?;
        Ok(())
    }),
];
