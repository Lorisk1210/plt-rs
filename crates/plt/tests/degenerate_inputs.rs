//! Plotting degenerate data never panics: every plotting function gets
//! empty, single-valued, constant, NaN, infinite, huge and tiny data, and
//! the figure is rendered to every output format. Functions may reject
//! the data with an error; they must not panic.

mod common;

use plt::axes::{
    BoxplotOptions, EcdfOptions, EventplotOptions, FillBetweenOptions, HexbinOptions,
    Hist2dOptions, HistPlotOptions, ImshowOptions, Mesh, PcolorOptions, PieOptions,
    SpecgramOptions, SpectralPlotOptions, StackplotOptions, StairsOptions, StemOptions, TriValues,
    TripcolorOptions, ViolinplotOptions,
};
use plt::colorbar::ColorbarOptions;
use plt::contour::{ContourOptions, QuadField};
use plt::lines::StepWhere;
use plt::prelude::*;
use plt::tri::Triangulation;

use common::{Outcome, done, field};

/// A plotting call on fresh axes with data `y` (and `x = 0, 1, 2, ...`).
type Case = fn(&Figure, &Axes, &[f64], &[f64]) -> Outcome;

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
            done(ax.stem(x, y, StemOptions::default()))
        }),
        ("step", |_, ax, x, y| {
            done(ax.step(x.to_vec(), y.to_vec(), StepWhere::Pre, LineProps::default()))
        }),
        ("stackplot", |_, ax, x, y| {
            done(ax.stackplot(x, &[y.to_vec()], StackplotOptions::default()))
        }),
        ("stairs", |_, ax, _, y| {
            done(ax.stairs(y.to_vec(), StairsOptions::default()))
        }),
        ("eventplot", |_, ax, _, y| {
            done(ax.eventplot(&[y.to_vec()], EventplotOptions::default()))
        }),
    ]
}

/// Statistics and spectra.
fn stat_cases() -> Vec<(&'static str, Case)> {
    vec![
        ("hist", |_, ax, _, y| {
            done(ax.hist(&[y], HistPlotOptions::default()))
        }),
        ("hist2d", |_, ax, x, y| {
            done(ax.hist2d(x, y, Hist2dOptions::default()))
        }),
        ("hexbin", |_, ax, x, y| {
            done(ax.hexbin(x, y, HexbinOptions::default()))
        }),
        ("boxplot", |_, ax, _, y| {
            done(ax.boxplot(&[y], BoxplotOptions::default()))
        }),
        ("violinplot", |_, ax, _, y| {
            done(ax.violinplot(&[y], ViolinplotOptions::default()))
        }),
        ("pie", |_, ax, _, y| done(ax.pie(y, PieOptions::default()))),
        ("ecdf", |_, ax, _, y| {
            done(ax.ecdf(y, EcdfOptions::default()))
        }),
        ("specgram", |_, ax, _, y| {
            done(ax.specgram(y, SpecgramOptions::default()))
        }),
        ("psd", |_, ax, _, y| {
            done(ax.psd(y, SpectralPlotOptions::default()))
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

#[test]
fn degenerate_data_never_panics() {
    let cases = [line_cases(), stat_cases(), field_cases(), other_cases()].concat();
    common::assert_no_panics(&cases, |run, fig, x, y| run(fig, &fig.subplot(), x, y));
}
