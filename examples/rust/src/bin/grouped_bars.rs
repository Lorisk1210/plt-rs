//! Grouped bars with value labels: daily screen time by age group and
//! device.

use pltrs::axes::BarLabelOptions;
use pltrs::layout::LayoutEngine;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let groups = ["13–17", "18–24", "25–34", "35–49", "50–64", "65+"];
    let hours = [
        ("phone", [4.8, 4.4, 3.6, 2.9, 2.1, 1.3]),
        ("computer", [2.1, 3.0, 3.4, 3.1, 2.4, 1.6]),
        ("television", [1.2, 1.1, 1.5, 1.9, 2.8, 3.9]),
    ];

    let x = arange(0.0, groups.len() as f64, 1.0);
    let width = 0.27;
    let fig = Figure::with_size(7.2, 4.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    for (offset, (device, values)) in [-1.0, 0.0, 1.0].into_iter().zip(hours) {
        let options = BarOptions {
            thickness: width.into(),
            label: Some(device.into()),
            ..BarOptions::default()
        };
        let bars = ax.bar(x.iter().map(|x| x + offset * width), values, options)?;
        let labels = BarLabelOptions {
            fmt: "%.1f".into(),
            padding: vec![2.0],
            ..BarLabelOptions::default()
        };
        for label in ax.bar_label(&bars, labels)? {
            label.set_fontsize(7.5);
        }
    }
    ax.set_xticks(&x).set_xticklabels(groups)?;
    ax.set_title("Daily screen time by age group");
    ax.set_xlabel("age").set_ylabel("hours per day");
    ax.set_ylim(0.0, 5.6)?;
    ax.legend(LegendOptions::default().loc(AnchorLoc::UpperLeft).ncols(3))?;

    fig.savefig("grouped_bars.png")?;
    Ok(())
}
