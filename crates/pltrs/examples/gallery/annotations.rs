//! Annotation styles: every connection style, the fancy box styles, the
//! arrow styles, and a table under a bar chart.

use pltrs::patches::{BoxStyle, ConnectionStyle};
use pltrs::prelude::*;
use pltrs::table::{TableLoc, TableOptions};
use pltrs::text::{AnnotateOptions, FancyArrowProps, HAlign, TextBoxProps, VAlign};
use pltrs::transforms::Point;

use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure(
        "connection_styles",
        "Annotation arrows in every connection style",
        connection_styles,
    ),
    Entry::figure("box_styles", "Text boxes in every box style", box_styles),
    Entry::figure("arrow_styles", "Annotation arrow styles", arrow_styles),
    Entry::figure(
        "table",
        "A table under a stacked bar chart",
        table_under_bars,
    ),
];

/// One panel per connection style: an arrow from a text box at the
/// lower left to a point at the upper right, its route set by the style.
fn connection_styles() -> Built {
    let fig = Figure::with_size(8.0, 6.0, 100.0);
    let axs = fig.subplots(3, 3)?;
    let styles = [
        "arc3,rad=0",
        "arc3,rad=0.3",
        "angle3,angleA=90,angleB=0",
        "angle,angleA=-90,angleB=180,rad=0",
        "angle,angleA=-90,angleB=180,rad=5",
        "arc,angleA=-90,angleB=0,armA=30,armB=30,rad=0",
        "arc,angleA=-90,angleB=0,armA=30,armB=30,rad=5",
        "bar,fraction=0.3",
        "bar,angle=180,fraction=-0.2",
    ];
    for (ax, style) in axs.as_slice().iter().zip(styles) {
        let (from, to) = (Point::new(0.15, 0.2), Point::new(0.85, 0.75));
        ax.plot_with(
            [from.x, to.x],
            [from.y, to.y],
            LineProps::fmt("o")?.color("0.6"),
        )?;
        let mut arrow: FancyArrowProps = "->".parse()?;
        arrow.connectionstyle = style.parse::<ConnectionStyle>()?;
        arrow.shrink_a = 5.0;
        arrow.shrink_b = 5.0;
        let options = AnnotateOptions {
            xytext: Some(from.into()),
            arrowprops: Some(arrow.into()),
            ..AnnotateOptions::default()
        };
        ax.annotate("", to, options)?;
        let label = ax.text(0.04, 0.96, style.replace(',', ",\n"));
        label.set_transform(ax.trans_axes());
        label
            .set_valign(VAlign::Top)
            .set_fontsize(7.5)
            .set_family(["DejaVu Sans Mono"]);
        ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
        ax.set_xticks(Vec::<f64>::new())
            .set_yticks(Vec::<f64>::new());
    }
    Ok(fig)
}

/// The same label framed by each box style.
fn box_styles() -> Built {
    let fig = Figure::with_size(6.4, 4.5, 100.0);
    let ax = fig.subplot();
    let styles = [
        "square",
        "circle",
        "ellipse",
        "round",
        "round4",
        "sawtooth",
        "roundtooth",
        "larrow",
        "rarrow",
        "darrow",
    ];
    for (i, style) in styles.iter().enumerate() {
        let (column, row) = ((i % 2) as f64, (i / 2) as f64);
        let text = ax.text(0.25 + 0.5 * column, 0.9 - 0.2 * row, *style);
        text.set_halign(HAlign::Center)
            .set_valign(VAlign::Center)
            .set_fontsize(11.0);
        let frame = TextBoxProps::new(format!("{style},pad=0.3").parse::<BoxStyle>()?).with_patch(
            PatchProps::default()
                .facecolor(format!("C{i}"))
                .alpha(0.3)
                .edgecolor("0.2"),
        );
        text.set_bbox(Some(frame));
    }
    ax.set_axis_off();
    ax.set_title("BoxStyle");
    Ok(fig)
}

/// The arrow styles of annotations, from plain lines to filled wedges.
fn arrow_styles() -> Built {
    let fig = Figure::with_size(6.4, 4.5, 100.0);
    let ax = fig.subplot();
    let styles = [
        "-", "->", "-[", "|-|", "-|>", "<-", "<->", "<|-", "<|-|>", "]-[", "fancy", "simple",
        "wedge",
    ];
    for (i, style) in styles.iter().enumerate() {
        let (column, row) = ((i % 2) as f64, (i / 2) as f64);
        let y = 0.92 - 0.15 * row;
        let x = 0.05 + 0.5 * column;
        let mut arrow: FancyArrowProps = style.parse()?;
        arrow.patch = PatchProps::default().facecolor("C0").edgecolor("C0");
        // Heads scale with the mutation scale, by default the font size.
        arrow.mutation_scale = Some(18.0);
        let options = AnnotateOptions {
            xytext: Some(Point::new(x, y).into()),
            arrowprops: Some(arrow.into()),
            ..AnnotateOptions::default()
        };
        let label = ax.annotate(*style, Point::new(x + 0.38, y), options)?;
        label
            .set_valign(VAlign::Center)
            .set_halign(HAlign::Right)
            .set_family(["DejaVu Sans Mono"]);
    }
    ax.set_xlim(-0.12, 1.0)?.set_ylim(0.0, 1.0)?;
    ax.set_axis_off();
    ax.set_title("ArrowStyle");
    Ok(fig)
}

/// Yearly losses stacked by cause, with the running totals in a table
/// placed under the bars, its columns lined up with them.
fn table_under_bars() -> Built {
    let fig = Figure::with_size(6.4, 4.8, 100.0);
    let ax = fig.subplot();
    let causes = ["Freeze", "Wind", "Flood", "Quake", "Hail"];
    let losses = [
        [66.0, 174.0, 75.0, 578.0, 32.0],
        [58.0, 381.0, 78.0, 99.0, 160.0],
        [89.0, 81.0, 153.0, 498.0, 604.0],
        [78.0, 82.0, 151.0, 193.0, 70.0],
    ];
    let periods = ["50 year", "20 year", "10 year", "5 year"];
    let blues = colormaps::get("Blues")?;
    let colors: Vec<Color> = (0..losses.len())
        .map(|i| blues.sample(0.15 + 0.15 * i as f64).into())
        .collect();
    let mut totals = vec![0.0; causes.len()];
    let mut rows = Vec::new();
    for (row, color) in losses.iter().zip(&colors) {
        let bars = BarOptions {
            thickness: 0.5.into(),
            base: totals.clone().into(),
            color: vec![color.clone()],
            ..BarOptions::default()
        };
        ax.bar(&[0.0, 1.0, 2.0, 3.0, 4.0][..], row, bars)?;
        totals
            .iter_mut()
            .zip(row)
            .for_each(|(total, loss)| *total += loss);
        rows.push(totals.iter().map(|t| format!("{t:.0}")).collect());
    }
    // The table lists the top layer first, as the stack reads.
    rows.reverse();
    let table = ax.table(TableOptions {
        row_labels: Some(periods.iter().rev().map(|p| (*p).to_owned()).collect()),
        row_colours: Some(colors.into_iter().rev().collect()),
        col_labels: Some(causes.map(String::from).to_vec()),
        loc: TableLoc::Bottom,
        ..TableOptions::with_text(rows)
    })?;
    // Row labels sit in column -1, below the column labels in row 0; on
    // their light swatches they keep a dark ink whatever the look.
    for row in 1..=4 {
        if let Some(cell) = table.cell(row, -1) {
            cell.set_text_color("0.1");
        }
    }
    ax.set_xticks(Vec::<f64>::new());
    ax.set_xlim(-0.5, 4.5)?;
    ax.set_ylabel("loss (k$)");
    ax.set_title("Loss by disaster");
    Ok(fig)
}
