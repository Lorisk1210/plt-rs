//! Subplot mosaic dashboard: four views of an online shop's year laid out
//! by name, the revenue trend across the top.

use pltrs::axes::{GridOptions, PieOptions};
use pltrs::collections::CollectionProps;
use pltrs::figure::MosaicOptions;
use pltrs::layout::LayoutEngine;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;
use pltrs::ticker::AxisName;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let revenue = [
        82.0, 79.0, 91.0, 95.0, 102.0, 99.0, 94.0, 97.0, 108.0, 117.0, 139.0, 162.0,
    ]; // k€
    let target = linspace(80.0, 140.0, 12);
    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let orders = [610.0, 580.0, 640.0, 700.0, 820.0, 960.0, 720.0];
    let products = [
        "headphones",
        "keyboards",
        "monitors",
        "webcams",
        "chairs",
        "lamps",
    ];
    let sold = [1840.0, 1420.0, 960.0, 870.0, 610.0, 430.0];
    let channels = ["search", "social", "e-mail", "direct"];
    let share = [42.0, 23.0, 15.0, 20.0];

    let fig = Figure::with_size(9.0, 5.5, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let options = MosaicOptions {
        width_ratios: Some(vec![1.2, 1.0, 1.1]),
        ..MosaicOptions::default()
    };
    let axd = fig.subplot_mosaic("RRP;WCP", options)?;
    fig.suptitle("Online shop: 2024 at a glance");

    let ax = &axd["R"];
    let line = LineProps::default()
        .marker(MarkerSymbol::Circle)
        .linewidth(2.0);
    ax.plot_with(months, revenue, line.label("revenue"))?;
    let dashed = LineProps::default()
        .linestyle(LineStyle::Dashed)
        .color("0.5")
        .linewidth(1.0);
    ax.plot_with(months, &target, dashed.label("target"))?;
    let above_target = FillBetweenOptions {
        mask: Some(revenue.iter().zip(&target).map(|(r, t)| r >= t).collect()),
        interpolate: true,
        props: CollectionProps::default().alpha(0.2),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(months, revenue, &target, above_target)?;
    ax.set_title("Monthly revenue (k€)");
    ax.legend(LegendOptions::default().loc(AnchorLoc::UpperLeft))?;

    let ax = &axd["P"];
    let green = BarOptions {
        color: vec!["C2".into()],
        ..BarOptions::default()
    };
    // Reversed, to list the best seller at the top.
    let names: Vec<&str> = products.iter().rev().copied().collect();
    let counts: Vec<f64> = sold.iter().rev().copied().collect();
    ax.barh(names, counts, green)?;
    ax.set_title("Units sold by product");
    let no_y_grid = GridOptions {
        axis: Some(AxisName::Y),
        ..GridOptions::default()
    };
    ax.grid_with(false, no_y_grid);

    let ax = &axd["W"];
    let orange = BarOptions {
        color: vec!["C1".into()],
        ..BarOptions::default()
    };
    ax.bar(weekdays, orders, orange)?;
    ax.set_title("Orders by weekday");

    let ax = &axd["C"];
    let pie = PieOptions {
        labels: channels.map(String::from).to_vec(),
        autopct: Some("%1.0f%%".into()),
        startangle: 90.0,
        wedge_width: Some(0.45),
        ..PieOptions::default()
    };
    ax.pie(share, pie)?;
    ax.set_title("Traffic by channel");

    fig.savefig("dashboard.png")?;
    Ok(())
}
