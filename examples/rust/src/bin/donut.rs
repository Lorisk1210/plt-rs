//! Donut chart: how a household's monthly budget is spent, with the total
//! in the hole.

use plt::axes::{PieDirection, PieOptions};
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::text::{HAlign, VAlign};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spending = [
        ("housing", 1250),
        ("food", 540),
        ("transport", 310),
        ("health", 220),
        ("leisure", 260),
        ("savings", 400),
        ("other", 170),
    ];

    let fig = Figure::with_size(6.4, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let options = PieOptions {
        labels: spending
            .iter()
            .map(|(label, _)| label.to_string())
            .collect(),
        autopct: Some("%1.0f%%".into()),
        pctdistance: 0.8,
        startangle: 90.0,
        direction: PieDirection::Clockwise,
        wedge_width: Some(0.4),
        wedgeprops: PatchProps::default().edgecolor("white").linewidth(2.0),
        text_size: Some(9.0.into()),
        ..PieOptions::default()
    };
    let pie = ax.pie(spending.map(|(_, amount)| amount), options)?;
    for percent in &pie.pct_labels {
        percent.set_color("white");
    }
    let total: i32 = spending.iter().map(|(_, amount)| amount).sum();
    // The total with a thousands separator: "€3,150".
    let centre = ax.text(
        0.0,
        0.0,
        format!("€{},{:03}\nper month", total / 1000, total % 1000),
    );
    centre
        .set_halign(HAlign::Center)
        .set_valign(VAlign::Center)
        .set_fontsize(13.0);
    ax.set_title("Where the money goes");

    fig.savefig("donut.png")?;
    Ok(())
}
