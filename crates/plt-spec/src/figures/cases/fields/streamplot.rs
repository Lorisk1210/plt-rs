//! Streamplot cases of `figures/cases_fields.py`.

use plt::axes::ImshowOptions;
use plt::colors::colormaps;
use plt::image::{ColorizerOptions, Extent, ImageAlpha, Origin, ScalarField};
use plt::lines::LineProps;
use plt::numeric::{Grid, linspace};
use plt::rc::Aspect;
use plt::streamplot::{
    IntegrationDirection, StreamColorMap, StreamColors, StreamField, StreamWidths,
    StreamplotOptions,
};
use plt::transforms::Point;

use crate::figures::FigureCase;

/// `_stream_field(n)`: `u = -1 - x² + y`, `v = 1 + x - y²` on an `n × n`
/// grid over `[-3, 3]²`.
fn stream_field(n: usize) -> StreamField {
    let x = linspace(-3.0, 3.0, n);
    let u = Grid::from_fn(n, n, |i, j| -1.0 - x[j] * x[j] + x[i]);
    let v = Grid::from_fn(n, n, |i, j| 1.0 + x[j] - x[i] * x[i]);
    StreamField::new(x.clone(), x, u, v)
}

/// The speed `hypot(u, v)` of a field.
fn speed(field: &StreamField) -> Grid<f64> {
    Grid::from_fn(field.u.rows(), field.u.cols(), |i, j| {
        field.u[(i, j)].hypot(field.v[(i, j)])
    })
}

/// The same density in both directions.
fn density(d: f64) -> (f64, f64) {
    (d, d)
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("fields/streamplot_masked", |fig| {
        let ax = fig.subplot();
        let masked = |i: usize, j: usize| (15..25).contains(&i) && (15..25).contains(&j);
        let mut field = stream_field(40);
        field.u = Grid::from_fn(40, 40, |i, j| {
            if masked(i, j) {
                f64::NAN
            } else {
                field.u[(i, j)]
            }
        });
        let options = StreamplotOptions {
            color: Some(StreamColors::Single("r".into())),
            ..StreamplotOptions::default()
        };
        ax.streamplot(&field, options)?;
        let visible = Grid::from_fn(40, 40, |i, j| !masked(i, j));
        let image = ImshowOptions {
            colors: ColorizerOptions::default().cmap(colormaps::get("gray")?),
            extent: Some(Extent {
                left: -3.0,
                right: 3.0,
                bottom: -3.0,
                top: 3.0,
            }),
            alpha: Some(ImageAlpha::Uniform(0.5)),
            aspect: Some(Aspect::Auto),
            origin: Some(Origin::Lower),
            ..ImshowOptions::default()
        };
        ax.imshow(ScalarField::booleans(&visible), image)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/streamplot_default", |fig| {
        fig.subplot()
            .streamplot(&stream_field(40), StreamplotOptions::default())?;
        Ok(())
    }),
    FigureCase::new("fields/streamplot_density_color", |fig| {
        let field = stream_field(40);
        let options = StreamplotOptions {
            density: (0.5, 1.5),
            color: Some(StreamColors::Mapped(Box::new(StreamColorMap {
                values: speed(&field),
                cmap: Some(colormaps::get("autumn")?),
                norm: None,
            }))),
            ..StreamplotOptions::default()
        };
        fig.subplot().streamplot(&field, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/streamplot_linewidth_array", |fig| {
        let field = stream_field(40);
        let speed = speed(&field);
        let max = speed.as_slice().iter().copied().fold(f64::MIN, f64::max);
        let widths = speed.map(|s| 5.0 * s / max);
        let options = StreamplotOptions {
            density: density(0.6),
            color: Some(StreamColors::Single("k".into())),
            linewidth: Some(StreamWidths::Varying(widths)),
            ..StreamplotOptions::default()
        };
        fig.subplot().streamplot(&field, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/streamplot_start_points", |fig| {
        let ax = fig.subplot();
        let starts = [
            Point::new(-2.0, -2.0),
            Point::new(-1.0, 0.0),
            Point::new(0.0, 2.0),
            Point::new(1.5, -1.5),
            Point::new(2.5, 2.5),
        ];
        let options = StreamplotOptions {
            start_points: Some(starts.to_vec()),
            color: Some(StreamColors::Single("tab:blue".into())),
            linewidth: Some(StreamWidths::Uniform(2.0)),
            ..StreamplotOptions::default()
        };
        ax.streamplot(&stream_field(40), options)?;
        let xs: Vec<f64> = starts.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = starts.iter().map(|p| p.y).collect();
        ax.plot_with(&xs, &ys, LineProps::fmt("ko")?)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/streamplot_unbroken", |fig| {
        let options = StreamplotOptions {
            density: density(0.6),
            broken_streamlines: false,
            ..StreamplotOptions::default()
        };
        fig.subplot().streamplot(&stream_field(40), options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/streamplot_forward_only", |fig| {
        let options = StreamplotOptions {
            density: density(0.8),
            integration_direction: IntegrationDirection::Forward,
            ..StreamplotOptions::default()
        };
        fig.subplot().streamplot(&stream_field(40), options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/streamplot_arrowstyle", |fig| {
        let options = StreamplotOptions {
            density: density(0.7),
            arrowstyle: "->".parse()?,
            arrowsize: 2.0,
            num_arrows: 3,
            ..StreamplotOptions::default()
        };
        fig.subplot().streamplot(&stream_field(40), options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
];
