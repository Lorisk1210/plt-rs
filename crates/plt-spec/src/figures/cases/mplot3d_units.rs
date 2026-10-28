//! `figures/cases_mplot3d_units.py`: dates, category names and `jpl_units`
//! distances given to the 3-D plotting methods, converted on the axis each
//! lies along (matplotlib needs `Axes3D._process_unit_info` first; plt-rs'
//! 3-D plotting methods convert them).

use std::f64::consts::PI;

use plt::axes::{BarOptions, MappedColors, ScatterColors, ScatterOptions};
use plt::colors::{Colorize, colormaps};
use plt::dates::{DateError, DateTime};
use plt::figure::Figure;
use plt::mplot3d::{Axis3, Bar3DOptions, Boxes, Scatter3DOptions};
use plt::numeric::linspace;
use plt::patches::PatchProps;

use crate::figures::{CaseResult, FigureCase};
use crate::user_units::jpl::{UnitDbl, UnitName};

/// Every case of the group, on 4 × 4 in canvases.
pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mplot3d_units/dates_on_z", dates_on_z).sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d_units/names_bar3d", names_bar3d).sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d_units/names_scatter_text", names_scatter_text).sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d_units/names_bar_zdir", names_bar_zdir).sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d_units/jpl_metres_on_z", jpl_metres_on_z).sized(4.0, 4.0, 100.0),
];

/// `_quarter_days(count)`: `count` instants six hours apart from
/// 2021-03-01.
fn quarter_days(count: u32) -> Result<Vec<DateTime>, DateError> {
    (0..count)
        .map(|i| DateTime::new(2021, 3, 1 + i / 4, 6 * (i % 4), 0, 0, 0))
        .collect()
}

/// A helix and a scatter rising over four days on z.
fn dates_on_z(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let t = linspace(0.0, 2.0 * PI, 17);
    let days = quarter_days(17)?;
    ax.plot(t.iter().map(|t| t.cos()), t.iter().map(|t| t.sin()), &days)?;
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            colors: ScatterColors::Single("tab:orange".into()),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(
        t.iter().map(|t| 0.5 * t.cos()),
        t.iter().map(|t| 0.5 * t.sin()),
        &days,
        options,
    )?;
    Ok(())
}

/// Boxes at names on x and y: a grid of regions by year.
fn names_bar3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let regions = ["north", "south", "east", "west"];
    let years = ["2019", "2020", "2021"];
    let x: Vec<&str> = years.iter().flat_map(|_| regions).collect();
    let y: Vec<&str> = years.iter().flat_map(|&year| [year; 4]).collect();
    let heights: Vec<f64> = (0..x.len())
        .map(|i| 1.0 + ((3 * i) % 7) as f64 / 2.0)
        .collect();
    let boxes = Boxes::new(&x, &y, [0.0], [0.5], [0.5], &heights);
    let options = Bar3DOptions {
        color: vec!["tab:cyan".into()],
        ..Bar3DOptions::default()
    };
    ax.bar3d(boxes, options)?;
    Ok(())
}

/// A scatter with names on y and a text placed at one of them.
fn names_scatter_text(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let x = linspace(0.0, 1.0, 9);
    let names: Vec<&str> = (0..3).flat_map(|_| ["low", "mid", "high"]).collect();
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            colors: MappedColors::new(x.clone())
                .cmap(colormaps::get("viridis")?)
                .into(),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(&x, &names, x.iter().map(|x| x * x), options)?;
    ax.text(0.5, "mid", 0.8, "mid point", None);
    Ok(())
}

/// Rows of flat bars at names, placed in planes normal to y.
fn names_bar_zdir(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let months = ["jan", "feb", "mar", "apr", "may"];
    for (row, color) in ["tab:red", "tab:green", "tab:blue"].into_iter().enumerate() {
        let heights: Vec<f64> = (0..months.len())
            .map(|i| 1.0 + ((row + 2 * i) % 5) as f64)
            .collect();
        let options = BarOptions {
            color: vec![color.into()],
            props: PatchProps::default().alpha(0.8),
            ..BarOptions::default()
        };
        ax.bar(months, &heights, [row as f64], Axis3::Y, options)?;
    }
    Ok(())
}

/// A line in kilometres on a z axis in metres, the unit set before the
/// line.
fn jpl_metres_on_z(fig: &Figure) -> CaseResult {
    let km = UnitDbl::new(1.0, UnitName::Kilometres);
    let ax = fig.subplot_3d();
    ax.zaxis().set_units(UnitName::Metres);
    let t = linspace(0.0, 1.0, 20);
    let z: Vec<UnitDbl> = t.iter().map(|v| (1.0 + v) * km).collect();
    ax.plot(
        t.iter().map(|t| (4.0 * t).cos()),
        t.iter().map(|t| (4.0 * t).sin()),
        z,
    )?;
    Ok(())
}
