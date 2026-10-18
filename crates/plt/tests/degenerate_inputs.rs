//! Plotting degenerate data never panics: every plotting function gets
//! empty, single-valued, constant, NaN, infinite, huge and tiny data, and
//! the figure is rendered to every output format. Functions may reject
//! the data with an error; they must not panic.

use std::panic::{AssertUnwindSafe, catch_unwind};

use plt::axes::{
    BoxplotOptions, EcdfOptions, EventplotOptions, FillBetweenOptions, HexbinOptions,
    Hist2dOptions, HistPlotOptions, ImshowOptions, Mesh, PcolorOptions, PieOptions,
    SpecgramOptions, SpectralPlotOptions, StackplotOptions, StairsOptions, StemOptions, TriValues,
    TripcolorOptions, ViolinplotOptions,
};
use plt::colorbar::ColorbarOptions;
use plt::contour::{ContourOptions, QuadField};
use plt::figure::SaveFormat;
use plt::lines::StepWhere;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::tri::Triangulation;

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// The data sets every case is run with.
fn data_sets() -> Vec<(&'static str, Vec<f64>)> {
    vec![
        ("empty", vec![]),
        ("single", vec![1.0]),
        ("constant", vec![2.0; 5]),
        ("nan", vec![f64::NAN; 5]),
        (
            "mixed",
            vec![1.0, f64::NAN, 3.0, f64::INFINITY, f64::NEG_INFINITY, 2.0],
        ),
        ("huge", vec![1e300, -1e300, 5e299, 1e308]),
        ("tiny", vec![1e-300, 2e-300, -1e-300, 5e-324]),
        ("normal", vec![0.0, 1.0, 4.0, 9.0, 16.0]),
    ]
}

/// A plotting call on fresh axes with data `y` (and `x = 0, 1, 2, ...`).
type Case = fn(&Figure, &Axes, &[f64], &[f64]) -> Outcome;

/// Two rows of `y`, for the functions that need a 2-D field.
fn field(y: &[f64]) -> Result<Grid<f64>, Box<dyn std::error::Error>> {
    Ok(Grid::from_rows(vec![
        y.to_vec(),
        y.iter().map(|v| v * 2.0).collect(),
    ])?)
}

/// Any result as an [`Outcome`], dropping the value.
fn done<T, E: Into<Box<dyn std::error::Error>>>(result: Result<T, E>) -> Outcome {
    result.map(drop).map_err(Into::into)
}

/// Lines, markers and bars.
fn line_cases() -> Vec<(&'static str, Case)> {
    vec![
        ("plot", |_, ax, x, y| done(ax.plot(x.to_vec(), y.to_vec()))),
        ("scatter", |_, ax, x, y| {
            done(ax.scatter(x.to_vec(), y.to_vec()))
        }),
        ("bar", |_, ax, x, y| {
            done(ax.bar(x.to_vec(), y, BarOptions::default()))
        }),
        ("fill_between", |_, ax, x, y| {
            done(ax.fill_between(x, y, vec![0.0; y.len()], FillBetweenOptions::default()))
        }),
        ("errorbar", |_, ax, x, y| {
            done(ax.errorbar(x, y, ErrorbarOptions::default()))
        }),
        ("stem", |_, ax, x, y| {
            done(ax.stem(x, y, &StemOptions::default()))
        }),
        ("step", |_, ax, x, y| {
            done(ax.step(x.to_vec(), y.to_vec(), StepWhere::Pre, LineProps::default()))
        }),
        ("stackplot", |_, ax, x, y| {
            done(ax.stackplot(x, &[y.to_vec()], &StackplotOptions::default()))
        }),
        ("stairs", |_, ax, _, y| {
            done(ax.stairs(y.to_vec(), StairsOptions::default()))
        }),
        ("eventplot", |_, ax, _, y| {
            done(ax.eventplot(&[y.to_vec()], &EventplotOptions::default()))
        }),
    ]
}

/// Statistics and spectra.
fn stat_cases() -> Vec<(&'static str, Case)> {
    vec![
        ("hist", |_, ax, _, y| {
            done(ax.hist(&[y], &HistPlotOptions::default()))
        }),
        ("hist2d", |_, ax, x, y| {
            done(ax.hist2d(x, y, Hist2dOptions::default()))
        }),
        ("hexbin", |_, ax, x, y| {
            done(ax.hexbin(x, y, HexbinOptions::default()))
        }),
        ("boxplot", |_, ax, _, y| {
            done(ax.boxplot(&[y], &BoxplotOptions::default()))
        }),
        ("violinplot", |_, ax, _, y| {
            done(ax.violinplot(&[y], &ViolinplotOptions::default()))
        }),
        ("pie", |_, ax, _, y| done(ax.pie(y, &PieOptions::default()))),
        ("ecdf", |_, ax, _, y| {
            done(ax.ecdf(y, EcdfOptions::default()))
        }),
        ("specgram", |_, ax, _, y| {
            done(ax.specgram(y, SpecgramOptions::default()))
        }),
        ("psd", |_, ax, _, y| {
            done(ax.psd(y, &SpectralPlotOptions::default()))
        }),
    ]
}

/// Images, fields and triangulations.
fn field_cases() -> Vec<(&'static str, Case)> {
    vec![
        ("imshow+colorbar", |fig, ax, _, y| {
            let image = ax.imshow(field(y)?, ImshowOptions::default())?;
            done(fig.colorbar(&image, ColorbarOptions::default()))
        }),
        ("pcolormesh", |_, ax, _, y| {
            done(ax.pcolormesh(Mesh::new(field(y)?), PcolorOptions::default()))
        }),
        ("contour", |_, ax, _, y| {
            done(ax.contour(&QuadField::new(field(y)?), ContourOptions::default()))
        }),
        ("contourf", |_, ax, _, y| {
            done(ax.contourf(&QuadField::new(field(y)?), ContourOptions::default()))
        }),
        ("tripcolor+triplot", |_, ax, x, y| {
            let tri = Triangulation::delaunay(x, y)?;
            ax.triplot(&tri, "o-", LineProps::default())?;
            let values = TriValues::Auto(y.to_vec());
            done(ax.tripcolor(&tri, values, TripcolorOptions::default()))
        }),
    ]
}

/// Scales, limits, decorations and other projections.
fn other_cases() -> Vec<(&'static str, Case)> {
    vec![
        ("log scale", |_, ax, x, y| {
            ax.plot(x.to_vec(), y.to_vec())?;
            ax.set_yscale("log")?;
            done(ax.set_xscale("symlog"))
        }),
        ("text and lines", |_, ax, _, y| {
            for &v in y {
                ax.text(v, v, format!("{v}"));
                ax.axhline(v, LineProps::default());
                ax.axvline(v, LineProps::default());
            }
            Ok(())
        }),
        ("limits", |_, ax, _, y| {
            if let [a, b, ..] = *y {
                ax.set_xlim(a, b).set_ylim(b, a);
            }
            Ok(())
        }),
        ("polar", |fig, _, x, y| {
            done(fig.polar_subplot().plot(x.to_vec(), y.to_vec()))
        }),
        ("scatter 3d", |fig, _, x, y| {
            let ax3 = fig.subplot_3d();
            ax3.scatter(x, y, y)?;
            done(ax3.plot(x, y, y))
        }),
    ]
}

fn cases() -> Vec<(&'static str, Case)> {
    [line_cases(), stat_cases(), field_cases(), other_cases()].concat()
}

/// Renders `fig` to every output format, ignoring errors.
fn render(fig: &Figure) {
    for format in [
        SaveFormat::Png,
        SaveFormat::Svg,
        SaveFormat::Pdf,
        SaveFormat::Eps,
    ] {
        let _ = fig.to_format(format, &plt::figure::SaveOptions::default());
    }
}

#[test]
fn degenerate_data_never_panics() {
    let mut panics = Vec::new();
    let mut failures = Vec::new();
    for (case, run) in cases() {
        for (name, y) in data_sets() {
            let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                let fig = Figure::with_size(2.0, 1.5, 30.0);
                let ax = fig.subplot();
                let result = run(&fig, &ax, &x, &y).map_err(|err| err.to_string());
                render(&fig);
                result
            }));
            match outcome {
                Err(_) => panics.push(format!("{case} with {name} data")),
                // Ordinary data must plot, so that the sweep exercises
                // every function rather than its argument checks.
                Ok(Err(err)) if name == "normal" => failures.push(format!("{case}: {err}")),
                Ok(_) => {}
            }
        }
    }
    assert!(panics.is_empty(), "panicked: {panics:#?}");
    assert!(
        failures.is_empty(),
        "failed on ordinary data: {failures:#?}"
    );
}
