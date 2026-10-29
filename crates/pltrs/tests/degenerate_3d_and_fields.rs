//! Plotting degenerate data never panics, continued: the 3-D plots, the
//! vector fields, triangulated contours, spectra and correlations, and
//! the decorations, with the data sets and checks of
//! `degenerate_inputs.rs`.

mod common;

use pltrs::axes::{
    BandAlign, ImshowOptions, SpectralPlotOptions, SpectrumPlotOptions, SpectrumScale, SpyOptions,
};
use pltrs::contour::ContourOptions;
use pltrs::lines::AxLineThrough;
use pltrs::mlab::XcorrOptions;
use pltrs::mplot3d::{
    Array3, Axes3D, Bar3DOptions, Boxes, Contour3DOptions, Errorbar3DOptions, FillBetween3DOptions,
    Quiver3DOptions, Stem3DOptions, SurfaceOptions, TrisurfOptions, Vec3, VoxelOptions,
    WireframeOptions,
};
use pltrs::prelude::*;
use pltrs::quiver::{BarbsOptions, QuiverOptions, VectorField};
use pltrs::streamplot::{StreamField, StreamplotOptions};
use pltrs::text::AnnotateOptions;
use pltrs::transforms::Point;
use pltrs::tri::Triangulation;

use common::{Outcome, done, field};

/// A plotting call on fresh 3-D axes with data `y` (and `x = 0, 1, ...`).
type Case3D = fn(&Axes3D, &[f64], &[f64]) -> Outcome;

/// A plotting call on fresh axes with data `y` (and `x = 0, 1, ...`).
type Case = fn(&Axes, &[f64], &[f64]) -> Outcome;

/// The points `(x, y, y)`.
fn points(x: &[f64], y: &[f64]) -> Vec<Vec3> {
    x.iter().zip(y).map(|(&a, &b)| [a, b, b]).collect()
}

/// Surfaces, bars, fields and decorations of 3-D axes.
fn cases_3d() -> Vec<(&'static str, Case3D)> {
    vec![
        ("plot_surface", |ax, x, y| {
            let (gx, gy) = (field(x)?, field(y)?);
            done(ax.plot_surface(&gx, &gy, &gy, SurfaceOptions::default()))
        }),
        ("plot_wireframe", |ax, x, y| {
            let (gx, gy) = (field(x)?, field(y)?);
            done(ax.plot_wireframe(&gx, &gy, &gy, WireframeOptions::default()))
        }),
        ("plot_trisurf", |ax, x, y| {
            let tri = Triangulation::delaunay(x, y)?;
            done(ax.plot_trisurf(&tri, y, TrisurfOptions::default()))
        }),
        ("contour and contourf", |ax, x, y| {
            let (gx, gy) = (field(x)?, field(y)?);
            ax.contour(&gx, &gy, &gy, Contour3DOptions::default())?;
            done(ax.contourf(&gx, &gy, &gy, Contour3DOptions::default()))
        }),
        ("bar3d", |ax, x, y| {
            let z = vec![0.0; y.len()];
            let boxes = Boxes::new(x, y, &z, [0.5], [0.5], y);
            done(ax.bar3d(boxes, Bar3DOptions::default()))
        }),
        ("voxels", |ax, _, y| {
            let filled = Array3::from_fn([y.len().min(4), 2, 2], |[i, _, _]| y[i] > 1.0);
            done(ax.voxels(&filled, VoxelOptions::default()))
        }),
        ("quiver", |ax, x, y| {
            let at = points(x, y);
            done(ax.quiver(&at, &at, Quiver3DOptions::default()))
        }),
        ("stem and errorbar", |ax, x, y| {
            ax.stem(x, y, y, Stem3DOptions::default())?;
            done(ax.errorbar(x, y, y, Errorbar3DOptions::default()))
        }),
        ("fill_between", |ax, x, y| {
            let z = vec![0.0; y.len()];
            done(ax.fill_between((x, y, y), (x, y, &z), FillBetween3DOptions::default()))
        }),
        ("text, limits and log scale", |ax, x, y| {
            ax.scatter(x, y, y)?;
            for &v in y {
                ax.text(v, v, v, format!("{v}"), None);
            }
            if let [a, b, ..] = *y {
                ax.set_xlim(a, b)?;
            }
            done(ax.set_zscale("log"))
        }),
    ]
}

/// Vector fields, triangulated contours and images.
fn field_cases() -> Vec<(&'static str, Case)> {
    vec![
        ("quiver", |ax, x, y| {
            let field = VectorField::new(x, y, y, x);
            done(ax.quiver(field, QuiverOptions::default()))
        }),
        ("barbs", |ax, x, y| {
            let field = VectorField::new(x, y, y, x);
            done(ax.barbs(field, BarbsOptions::default()))
        }),
        ("streamplot", |ax, x, y| {
            let stream = StreamField::new(x.to_vec(), vec![0.0, 1.0], field(y)?, field(x)?);
            done(ax.streamplot(&stream, StreamplotOptions::default()))
        }),
        ("tricontour and tricontourf", |ax, x, y| {
            let tri = Triangulation::delaunay(x, y)?;
            ax.tricontour(&tri, y, ContourOptions::default())?;
            done(ax.tricontourf(&tri, y, ContourOptions::default()))
        }),
        ("matshow and spy", |ax, _, y| {
            ax.matshow(field(y)?, ImshowOptions::default())?;
            done(ax.spy(&field(y)?, SpyOptions::default()))
        }),
    ]
}

/// Spectra, correlations, bars, spans and annotations.
fn other_cases() -> Vec<(&'static str, Case)> {
    vec![
        ("csd and cohere", |ax, _, y| {
            let mut options = SpectralPlotOptions::default();
            // Coherence averages over segments, so short data needs short ones.
            options.spectral.nfft = 2;
            ax.csd(y, y, options.clone())?;
            done(ax.cohere(y, y, options))
        }),
        ("magnitude and angle spectra", |ax, _, y| {
            let options = SpectrumPlotOptions::default();
            ax.magnitude_spectrum(y, SpectrumScale::Decibels, options.clone())?;
            done(ax.angle_spectrum(y, options))
        }),
        ("xcorr", |ax, x, y| {
            let options = XcorrOptions {
                maxlags: None,
                ..XcorrOptions::default()
            };
            done(ax.xcorr(x, y, options, true, LineProps::default()))
        }),
        ("barh and broken_barh", |ax, x, y| {
            ax.barh(x.to_vec(), y, BarOptions::default())?;
            let ranges: Vec<(f64, f64)> = x.iter().copied().zip(y.iter().copied()).collect();
            ax.broken_barh(&ranges, (0.0, 1.0), BandAlign::default());
            Ok(())
        }),
        ("fill, hlines and vlines", |ax, x, y| {
            ax.fill(x, y, PatchProps::default())?;
            ax.hlines(y, x, y)?;
            done(ax.vlines(x, y, x))
        }),
        ("loglog", |ax, x, y| done(ax.loglog(x.to_vec(), y.to_vec()))),
        ("annotate and axline", |ax, _, y| {
            for &v in y {
                let options = AnnotateOptions {
                    xytext: Some(Point::new(v / 2.0, v * 2.0).into()),
                    ..AnnotateOptions::default()
                };
                ax.annotate(format!("{v}"), Point::new(v, v), options)?;
                ax.axline(
                    Point::new(v, 0.0),
                    AxLineThrough::Slope(v),
                    LineProps::default(),
                )?;
            }
            Ok(())
        }),
    ]
}

#[test]
fn degenerate_3d_data_never_panics() {
    common::assert_no_panics(&cases_3d(), |run, fig, x, y| run(&fig.subplot_3d(), x, y));
}

#[test]
fn degenerate_fields_and_decorations_never_panic() {
    let cases = [field_cases(), other_cases()].concat();
    common::assert_no_panics(&cases, |run, fig, x, y| run(&fig.subplot(), x, y));
}
