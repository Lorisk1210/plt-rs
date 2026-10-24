//! Marker, edge, colour-mapping and degenerate-data cases of
//! `figures/cases_scatter.py`.

use plt::axes::{MappedColors, ScatterColors, ScatterOptions};
use plt::collections::EdgeColors;
use plt::colorbar::{ColorbarAxes, ColorbarOptions, ColorbarTicks, Spacing};
use plt::colors::{Alpha, BoundaryNorm, Color, Colorize, Colormap, Extend, colormaps};
use plt::figure::Figure;
use plt::lines::LineStyle;
use plt::markers::{FillStyle, Marker, MarkerStyle, PolygonStyle};
use plt::numeric::{arange, linspace};
use plt_core::{Affine2D, Point};

use super::points;
use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("scatter/marker_variants", marker_variants).sized(5.0, 4.0, 100.0),
    FigureCase::new("scatter/markerstyle_fillstyle_transform", marker_styles)
        .sized(5.0, 3.0, 100.0),
    FigureCase::new("scatter/edgecolors_linewidths_per_point", edges_per_point)
        .sized(5.0, 3.0, 100.0),
    FigureCase::new("scatter/rgba_array_colors", rgba_colors).sized(4.0, 3.0, 100.0),
    FigureCase::new("scatter/norm_extremes_extend_colorbar", extremes).sized(5.0, 4.0, 100.0),
    FigureCase::new("scatter/boundarynorm_discrete_colorbar", boundary_norm).sized(5.0, 4.0, 100.0),
    FigureCase::new("scatter/nonfinite_coordinates_sizes", nonfinite).sized(6.0, 3.0, 100.0),
    FigureCase::new("scatter/empty_single_zero_size", degenerate).sized(6.0, 2.5, 100.0),
    FigureCase::new("scatter/alpha_array", alpha_array).sized(6.0, 3.0, 100.0),
];

/// One colour for every marker, given as matplotlib's `color=`.
fn single(color: &str) -> ScatterColors {
    ScatterColors::Single(Color::from(color))
}

fn marker_variants(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let triangle = vec![
        Point::new(-1.0, -1.0),
        Point::new(1.0, -1.0),
        Point::new(0.0, 1.5),
    ];
    let markers = [
        Marker::polygon(5, PolygonStyle::Regular, 0.0),
        Marker::polygon(5, PolygonStyle::Star, 0.0),
        Marker::polygon(5, PolygonStyle::Asterisk, 0.0),
        Marker::polygon(4, PolygonStyle::Regular, 45.0),
        Marker::Vertices(triangle),
        r"$\clubsuit$".parse()?,
        "$f$".parse()?,
        "8".parse()?,
        "p".parse()?,
    ];
    for (i, marker) in markers.into_iter().enumerate() {
        let options = ScatterOptions {
            marker: Some(MarkerStyle::new(marker, FillStyle::Full)?),
            sizes: Some(vec![40.0, 80.0, 160.0, 320.0]),
            colors: single(&format!("C{i}")),
            ..ScatterOptions::default()
        };
        ax.scatter_with(arange(0.0, 4.0, 1.0), vec![i as f64; 4], options)?;
    }
    ax.set_xlim(-0.5, 3.5)?;
    Ok(())
}

fn marker_styles(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let angles = [0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0];
    for (i, angle) in angles.into_iter().enumerate() {
        let x = vec![i as f64];
        let rotated = "^"
            .parse::<MarkerStyle>()?
            .transformed(Affine2D::identity().rotate_deg(angle));
        let triangles = ScatterOptions {
            marker: Some(rotated),
            sizes: Some(vec![300.0]),
            ..ScatterOptions::default()
        };
        ax.scatter_with(x.clone(), vec![0.0], triangles)?;
        let squares = ScatterOptions {
            marker: Some("s".parse::<MarkerStyle>()?.rotated_deg(angle)),
            sizes: Some(vec![200.0]),
            colors: single("tab:green"),
            ..ScatterOptions::default()
        };
        ax.scatter_with(x, vec![1.0], squares)?;
    }
    let circles = ScatterOptions {
        marker: Some(MarkerStyle::new("o".parse::<Marker>()?, FillStyle::None)?),
        sizes: Some(vec![250.0]),
        colors: ScatterColors::PerPoint(["C0", "C1", "C2", "C3"].map(Color::from).to_vec()),
        linewidths: Some(vec![2.0]),
        ..ScatterOptions::default()
    };
    ax.scatter_with(arange(0.0, 4.0, 1.0), vec![2.0; 4], circles)?;
    let diamonds = ScatterOptions {
        marker: Some(MarkerStyle::new("D".parse::<Marker>()?, FillStyle::None)?),
        sizes: Some(vec![250.0]),
        colors: single("k"),
        ..ScatterOptions::default()
    };
    ax.scatter_with(arange(4.0, 7.0, 1.0), vec![2.0; 3], diamonds)?;
    ax.set_ylim(-0.7, 2.7)?;
    Ok(())
}

fn edges_per_point(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = arange(0.0, 8.0, 1.0);
    let rings = ScatterOptions {
        sizes: Some(vec![300.0]),
        colors: single("w"),
        edgecolors: Some(EdgeColors::Colors(
            ["C0", "C1", "C2", "C3"].map(Color::from).to_vec(),
        )),
        linewidths: Some(linspace(0.5, 4.0, 8)),
        ..ScatterOptions::default()
    };
    ax.scatter_with(&x, vec![0.0; 8], rings)?;
    let dashed = ScatterOptions {
        sizes: Some(vec![200.0]),
        colors: MappedColors::new(x.clone())
            .cmap(colormaps::get("viridis")?)
            .into(),
        edgecolors: Some(EdgeColors::Colors(vec![Color::from("k")])),
        linewidths: Some(vec![0.5, 1.0, 2.0, 3.0]),
        ..ScatterOptions::default()
    };
    let points = ax.scatter_with(&x, vec![1.0; 8], dashed)?;
    points.set_linestyles(vec![LineStyle::Dashed]);
    ax.set_ylim(-1.0, 2.0)?;
    Ok(())
}

fn rgba_colors(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = linspace(0.0, 1.0, 20);
    let rgba = t
        .iter()
        .map(|&t| Color::from((t, 0.2, 1.0 - t, 0.3 + 0.7 * t)))
        .collect();
    let options = ScatterOptions {
        colors: ScatterColors::PerPoint(rgba),
        sizes: Some(vec![150.0]),
        ..ScatterOptions::default()
    };
    ax.scatter_with(&t, t.iter().map(|t| t * t).collect::<Vec<_>>(), options)?;
    let options = ScatterOptions {
        colors: ScatterColors::Single(Color::from((0.9, 0.5, 0.1))),
        sizes: Some(vec![80.0]),
        ..ScatterOptions::default()
    };
    ax.scatter_with(
        &t,
        t.iter().map(|t| 1.0 - t * t).collect::<Vec<_>>(),
        options,
    )?;
    Ok(())
}

fn extremes(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 1.0, 40);
    let cmap = colormaps::get("viridis")?
        .with_under("tab:red")?
        .with_over("tab:orange")?;
    let options = ScatterOptions {
        colors: MappedColors::new(linspace(-0.5, 1.5, 40))
            .cmap(cmap)
            .limits(Some(0.0), Some(1.0))
            .into(),
        sizes: Some(vec![60.0]),
        edgecolors: Some(EdgeColors::Colors(vec![Color::from("k")])),
        linewidths: Some(vec![0.5]),
        ..ScatterOptions::default()
    };
    let y: Vec<f64> = x.iter().map(|x| (5.0 * x).cos()).collect();
    let points = ax.scatter_with(x, y, options)?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax),
        extend: Some(Extend::Both),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&points, options)?;
    Ok(())
}

fn boundary_norm(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let (x, y) = points(12, 60);
    let bounds = vec![0.0, 0.2, 0.5, 0.8, 1.6, 2.0];
    let cmap = Colormap::listed(
        "from_list",
        vec!["tab:blue", "tab:green", "gold", "tab:orange", "tab:red"],
    )?;
    let norm = BoundaryNorm::new(bounds.clone(), cmap.n(), Extend::Neither)?;
    let c: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x + y).collect();
    let options = ScatterOptions {
        colors: MappedColors::new(c).cmap(cmap).norm(norm).into(),
        sizes: Some(vec![70.0]),
        marker: Some("s".parse()?),
        ..ScatterOptions::default()
    };
    let points = ax.scatter_with(x, y, options)?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax),
        ticks: Some(ColorbarTicks::Values(bounds)),
        spacing: Spacing::Proportional,
        ..ColorbarOptions::default()
    };
    fig.colorbar(&points, options)?;
    Ok(())
}

fn nonfinite(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let (nan, inf) = (f64::NAN, f64::INFINITY);
    let x = [0.0, 1.0, 2.0, nan, 4.0, 5.0, 6.0, inf, 8.0, 9.0];
    let y = [1.0, nan, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let s = vec![
        50.0, 100.0, nan, 50.0, 100.0, 50.0, 100.0, 50.0, 100.0, 50.0,
    ];
    let c = [0.0, 1.0, 2.0, 3.0, nan, 5.0, 6.0, 7.0, -inf, 9.0];
    let options = ScatterOptions {
        sizes: Some(s.clone()),
        colors: MappedColors::new(c).into(),
        ..ScatterOptions::default()
    };
    axes[(0, 0)].scatter_with(x, y, options)?;
    let cmap = colormaps::get("cividis")?.with_bad("magenta")?;
    let options = ScatterOptions {
        sizes: Some(s),
        colors: MappedColors::new(c).cmap(cmap).into(),
        plot_nonfinite: true,
        edgecolors: Some(EdgeColors::Colors(vec![Color::from("k")])),
        ..ScatterOptions::default()
    };
    axes[(0, 1)].scatter_with(x, y, options)?;
    Ok(())
}

fn degenerate(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 3)?;
    axes[(0, 0)].scatter(Vec::<f64>::new(), Vec::<f64>::new())?;
    let star = ScatterOptions {
        sizes: Some(vec![200.0]),
        marker: Some("*".parse()?),
        ..ScatterOptions::default()
    };
    axes[(0, 1)].scatter_with([2.5], [-1.0], star)?;
    let zero = ScatterOptions {
        sizes: Some(vec![0.0, 20.0, 0.0, 200.0]),
        colors: single("tab:red"),
        ..ScatterOptions::default()
    };
    axes[(0, 2)].scatter_with([0.0, 1.0, 2.0, 3.0], [0.0, 1.0, 0.0, 1.0], zero)?;
    Ok(())
}

fn alpha_array(fig: &Figure) -> CaseResult {
    let axes = fig.subplots(1, 2)?;
    let t = linspace(0.0, 1.0, 20);
    let fading = ScatterOptions {
        sizes: Some(vec![150.0]),
        colors: single("tab:blue"),
        alpha: Alpha::Each(linspace(0.1, 1.0, 20)),
        edgecolors: Some(EdgeColors::Colors(vec![Color::from("k")])),
        ..ScatterOptions::default()
    };
    let wave: Vec<f64> = t.iter().map(|t| (6.0 * t).sin()).collect();
    axes[(0, 0)].scatter_with(&t, wave, fading)?;
    let mapped = ScatterOptions {
        sizes: Some(vec![150.0]),
        colors: MappedColors::new(t.clone())
            .cmap(colormaps::get("plasma")?)
            .into(),
        alpha: Alpha::Each(t.iter().map(|t| 1.0 - 0.9 * t).collect()),
        ..ScatterOptions::default()
    };
    let wave: Vec<f64> = t.iter().map(|t| (6.0 * t).cos()).collect();
    axes[(0, 1)].scatter_with(&t, wave, mapped)?;
    Ok(())
}
