//! `figures/cases_modern.py`: plt-rs' modern look (the `plt-modern` and
//! `plt-modern-dark` style sheets, with Inter) on typical plots, against
//! matplotlib drawing them with the same sheets.

use std::f64::consts::PI;

use plt::axes::{
    ErrorbarOptions, FillBetweenOptions, HistPlotOptions, ImshowOptions, MappedColors,
    ScatterOptions,
};
use plt::collections::CollectionProps;
use plt::colorbar::ColorbarOptions;
use plt::contour::{ClabelOptions, ContourOptions, Levels, QuadCoords, QuadField};
use plt::figure::Share;
use plt::fonts::FontSize;
use plt::image::{Extent, Origin};
use plt::legend::LegendOptions;
use plt::lines::StepWhere;
use plt::mplot3d::SurfaceOptions;
use plt::numeric::{Grid, meshgrid};
use plt::offsetbox::AnchorLoc;
use plt::prelude::*;
use plt::stats::{Bins, HistOptions};
use plt::table::{TableLoc, TableOptions};
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps};
use plt_core::Point;

use super::common::{face_cmap, map, padded, strings};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// A case at the sheets' 7.2 × 4.5 in figure size, in `style`.
const fn case(
    name: &'static str,
    style: &'static str,
    build: fn(&Figure) -> CaseResult,
) -> FigureCase {
    FigureCase::new(name, build)
        .sized(7.2, 4.5, 100.0)
        .with_style(style)
}

/// The cases of the modern look.
pub(crate) const CASES: &[FigureCase] = &[
    case("modern/lines", "plt-modern", damped),
    case("modern/dark_lines", "plt-modern-dark", damped),
    case("modern/bars", "plt-modern", bars),
    case("modern/scatter_colorbar", "plt-modern", scatter_colorbar),
    case("modern/histogram", "plt-modern", histogram),
    case("modern/subplots", "plt-modern", subplots),
    case("modern/image", "plt-modern", image),
    case("modern/polar", "plt-modern", polar),
    case("modern/annotations", "plt-modern", annotations),
    case("modern/log_math", "plt-modern", log_math),
    case("modern/surface3d", "plt-modern", surface3d),
    case("modern/dark_surface3d", "plt-modern-dark", surface3d),
    case("modern/dark_polar", "plt-modern-dark", dark_polar),
    case("modern/dark_contour", "plt-modern-dark", dark_contour),
    case("modern/dark_bars", "plt-modern-dark", dark_bars),
    case("modern/table", "plt-modern", table),
];

/// `X = np.linspace(0, 10, 200)`.
fn x() -> Vec<f64> {
    linspace(0.0, 10.0, 200)
}

/// `_damped`: four phase-shifted waves with a legend.
fn damped(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = x();
    for i in 0..4 {
        let phase = 0.7 * f64::from(i);
        let y = map(&x, |x| (x + phase).sin() / (1.0 + x / 5.0));
        ax.plot_with(&x, y, LineProps::default().label(format!("phase {i}")))?;
    }
    ax.set_title("Damped oscillations");
    ax.set_xlabel("time (s)").set_ylabel("amplitude");
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn bars(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = [0.0, 1.0, 2.0, 3.0];
    let years = [
        ("2023", [18.0, 24.0, 31.0, 12.0]),
        ("2024", [21.0, 22.0, 35.0, 17.0]),
        ("2025", [26.0, 25.0, 33.0, 23.0]),
    ];
    for (i, (year, values)) in years.into_iter().enumerate() {
        let offset = (i as f64 - 1.0) * 0.25;
        let options = BarOptions {
            thickness: 0.25.into(),
            label: Some(year.to_owned()),
            ..BarOptions::default()
        };
        let bars = ax.bar(map(&x, |x| x + offset), values, options)?;
        ax.bar_label(&bars, padded(2.0))?;
    }
    ax.set_xticks(x.to_vec());
    ax.set_xticklabels(["north", "south", "east", "west"])?;
    ax.set_ylabel("units (thousands)");
    ax.set_title("Sales by region");
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn scatter_colorbar(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let mut rng = SpecRng::new(11);
    let x = rng.normal(150, 0.0, 1.0);
    let noise = rng.normal(150, 0.0, 0.5);
    let y: Vec<f64> = x.iter().zip(&noise).map(|(x, n)| 0.6 * x + n).collect();
    let sums: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x + y).collect();
    let options = ScatterOptions {
        sizes: Some(vec![30.0]),
        colors: MappedColors::new(sums).into(),
        ..ScatterOptions::default()
    };
    let points = ax.scatter_with(x, y, options)?;
    let colorbar = ColorbarOptions {
        label: "x + y".to_owned(),
        ..ColorbarOptions::beside(&ax)
    };
    fig.colorbar(&points, colorbar)?;
    ax.set_title("Correlated samples");
    ax.set_xlabel("x").set_ylabel("y");
    Ok(())
}

fn histogram(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let mut rng = SpecRng::new(12);
    let samples = [
        (rng.normal(800, 0.0, 1.0), "N(0, 1)"),
        (rng.normal(800, 1.5, 0.7), "N(1.5, 0.7)"),
    ];
    for (sample, label) in samples {
        let options = HistPlotOptions {
            layout: HistOptions {
                bins: Some(Bins::Count(30)),
                ..HistOptions::default()
            },
            props: PatchProps::default().alpha(0.7).label(label),
            ..HistPlotOptions::default()
        };
        ax.hist(&[sample], options)?;
    }
    ax.set_title("Two samples");
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn subplots(fig: &Figure) -> CaseResult {
    let axs = fig.subplots_shared(2, 2, Share::All, Share::None)?;
    let mut rng = SpecRng::new(13);
    let x = x();
    let sine = map(&x, f64::sin);
    axs[(0, 0)].plot(&x, &sine)?;
    axs[(0, 0)].set_title("Line");
    let sparse: Vec<f64> = x.iter().step_by(8).copied().collect();
    let noise = rng.normal(sparse.len(), 0.0, 0.1);
    let wobble: Vec<f64> = sparse
        .iter()
        .zip(&noise)
        .map(|(x, n)| x.cos() + n)
        .collect();
    axs[(0, 1)].scatter(&sparse, wobble)?;
    axs[(0, 1)].set_title("Scatter");
    let steps: Vec<f64> = x.iter().step_by(10).copied().collect();
    let squares = map(&steps, |x| x.sin().powi(2));
    axs[(1, 0)].step(&steps, squares, StepWhere::Mid, LineProps::default())?;
    axs[(1, 0)].set_title("Steps");
    let band = FillBetweenOptions {
        props: CollectionProps::default().alpha(0.3),
        ..FillBetweenOptions::default()
    };
    let (low, high) = (map(&sine, |s| s - 0.3), map(&sine, |s| s + 0.3));
    axs[(1, 1)].fill_between(&x, low, high, band)?;
    axs[(1, 1)].plot(&x, &sine)?;
    axs[(1, 1)].set_title("Band");
    fig.suptitle("Four views of a wave");
    Ok(())
}

/// `np.mgrid[-3:3:n*1j]` along one axis: `i · 6 / (n − 1) − 3`.
fn mgrid(i: usize, n: usize) -> f64 {
    i as f64 * (6.0 / (n - 1) as f64) + -3.0
}

fn image(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let field = Grid::from_fn(60, 80, |i, j| {
        let (y, x) = (mgrid(i, 60), mgrid(j, 80));
        x * y / (1.0 + x * x + y * y)
    });
    let options = ImshowOptions {
        extent: Some(Extent::new(-3.0, 3.0, -3.0, 3.0)),
        origin: Some(Origin::Lower),
        ..ImshowOptions::default()
    };
    // `plt.image.hide_grid` turns the grid off, as `ax.grid(False)` does.
    let image = ax.imshow(field, options)?;
    fig.colorbar(&image, ColorbarOptions::beside(&ax))?;
    ax.set_title("A field");
    Ok(())
}

fn polar(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta = linspace(0.0, 2.0 * PI, 400);
    let r = map(&theta, |t| (3.0 * t).cos().abs());
    ax.plot(theta, r)?;
    ax.set_title("r = |cos 3θ|");
    Ok(())
}

fn annotations(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 6.0, 13);
    let y = map(&x, |x| (2.0 * x).cos() / (1.0 + x / 2.0));
    let band = FillBetweenOptions {
        props: CollectionProps::default().alpha(0.25),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(&x, map(&y, |y| y - 0.15), map(&y, |y| y + 0.15), band)?;
    let errors = ErrorbarOptions {
        yerr: Some(vec![0.15].into()),
        fmt: "o".parse()?,
        capsize: Some(3.0),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(&x, &y, errors)?;
    let arrow = AnnotateOptions {
        xytext: Some(Point::new(2.5, -0.8).into()),
        arrowprops: Some(ArrowProps::Fancy("->".parse::<FancyArrowProps>()?)),
        ..AnnotateOptions::default()
    };
    ax.annotate("first trough", Point::new(x[3], y[3]), arrow)?;
    ax.set_title("Measurements with a band");
    Ok(())
}

fn log_math(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 20.0, 100);
    // `semilogy` sets the scale, then plots.
    ax.set_yscale("log")?;
    let cubic = map(&x, |x| 1.0 / (1.0 + x * x * x));
    ax.plot_with(
        &x,
        cubic,
        LineProps::default().label(r"$\frac{1}{1 + x^3}$"),
    )?;
    let square = map(&x, |x| 1.0 / (1.0 + x * x));
    ax.plot_with(
        &x,
        square,
        LineProps::default().label(r"$\frac{1}{1 + x^2}$"),
    )?;
    ax.set_title(r"Two decays as $x \to \infty$");
    ax.set_xlabel(r"$x$");
    ax.legend(LegendOptions::default())?;
    Ok(())
}

/// `_saddle`: `x y / (1 + x² + y²)` on an `n` × `n` grid over [-3, 3]²
/// (`np.meshgrid` of two `np.linspace(-3, 3, n)`).
fn saddle(n: usize) -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let (xx, yy) = meshgrid(&linspace(-3.0, 3.0, n), &linspace(-3.0, 3.0, n));
    let zz = Grid::from_fn(n, n, |i, j| {
        let (x, y) = (xx[(i, j)], yy[(i, j)]);
        x * y / (1.0 + x * x + y * y)
    });
    (xx, yy, zz)
}

/// `_surface`: the saddle as a surface in 3-D, with a colorbar.
fn surface3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let (x, y, z) = saddle(25);
    let options = SurfaceOptions {
        colors: face_cmap("viridis")?,
        ..SurfaceOptions::default()
    };
    let surface = ax.plot_surface(&x, &y, &z, options)?;
    let colorbar = ColorbarOptions {
        shrink: 0.6,
        label: "height".to_owned(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&surface, colorbar)?;
    ax.set_title("A saddle");
    ax.set_xlabel("x").set_ylabel("y").set_zlabel("z");
    Ok(())
}

fn dark_polar(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let theta = linspace(0.0, 2.0 * PI, 400);
    let roses = [
        (map(&theta, |t| (3.0 * t).cos().abs()), "|cos 3θ|"),
        (map(&theta, |t| (2.0 * t).sin().abs()), "|sin 2θ|"),
    ];
    for (r, label) in roses {
        ax.plot_with(&theta, r, LineProps::default().label(label))?;
    }
    ax.set_title("Two roses");
    ax.legend(
        LegendOptions::default()
            .loc(AnchorLoc::UpperLeft)
            .bbox_to_anchor((1.05, 1.0)),
    )?;
    Ok(())
}

fn dark_contour(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let (x, y, z) = saddle(80);
    let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x, y });
    let filled = ax.contourf(
        &field,
        ContourOptions {
            levels: Levels::Count(10),
            ..ContourOptions::default()
        },
    )?;
    // Every other level as a line, in the sheet's ink.
    let lines = ax.contour(
        &field,
        ContourOptions {
            levels: Levels::Values(filled.levels().into_iter().step_by(2).collect()),
            colors: Some(vec![plt::rc::params().plt_ink.clone()]),
            linewidths: Some(vec![0.8]),
            ..ContourOptions::default()
        },
    )?;
    lines.clabel(ClabelOptions {
        fontsize: Some(FontSize::Points(8.0)),
        ..ClabelOptions::default()
    })?;
    let colorbar = ColorbarOptions {
        label: "height".to_owned(),
        ..ColorbarOptions::beside(&ax)
    };
    fig.colorbar(&filled, colorbar)?;
    ax.set_title("Contours of a saddle");
    ax.set_xlabel("x").set_ylabel("y");
    Ok(())
}

/// Bars with error bars, which `plt.ink` draws in ink.
fn dark_bars(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = [0.0, 1.0, 2.0, 3.0];
    let options = BarOptions {
        thickness: 0.6.into(),
        errors: ErrorbarOptions {
            yerr: Some(vec![2.0, 3.0, 4.0, 2.0].into()),
            capsize: Some(4.0),
            ..ErrorbarOptions::default()
        },
        ..BarOptions::default()
    };
    ax.bar(x, [18.0, 24.0, 31.0, 12.0], options)?;
    ax.set_xticks(x.to_vec());
    ax.set_xticklabels(["north", "south", "east", "west"])?;
    ax.set_ylabel("units (thousands)");
    ax.set_title("Sales with their spread");
    Ok(())
}

/// Bars over a table, whose cells `plt.table.*` draw in hairlines.
fn table(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = [0.0, 1.0, 2.0, 3.0];
    let years = [
        ("2023", [18, 24, 31, 12]),
        ("2024", [21, 22, 35, 17]),
        ("2025", [26, 25, 33, 23]),
    ];
    for (i, (year, values)) in years.into_iter().enumerate() {
        let offset = (i as f64 - 1.0) * 0.25;
        let options = BarOptions {
            thickness: 0.25.into(),
            label: Some(year.to_owned()),
            ..BarOptions::default()
        };
        ax.bar(map(&x, |x| x + offset), values.map(f64::from), options)?;
    }
    let cells = years
        .iter()
        .map(|(_, values)| values.iter().map(ToString::to_string).collect())
        .collect();
    ax.table(TableOptions {
        row_labels: Some(years.iter().map(|&(year, _)| year.to_owned()).collect()),
        col_labels: Some(strings(&["north", "south", "east", "west"])),
        loc: TableLoc::Bottom,
        ..TableOptions::with_text(cells)
    })?;
    ax.set_xticks(Vec::<f64>::new());
    ax.set_ylabel("units (thousands)");
    ax.set_title("Sales by region");
    ax.legend(LegendOptions::default())?;
    Ok(())
}
