//! Quiver and quiverkey cases of `figures/cases_fields.py`.

use pltrs::axes::MappedColors;
use pltrs::colors::{Colorize, colormaps};
use pltrs::lines::LineProps;
use pltrs::numeric::{Grid, linspace, meshgrid};
use pltrs::quiver::{
    Angles, KeyCoordinates, LabelPosition, Pivot, QuiverArrowShape, QuiverKeyOptions,
    QuiverOptions, QuiverUnits, VectorField,
};

use crate::figures::FigureCase;

/// `_grid(n, extent)`: an `n × n` mesh over `[-extent, extent]²`.
fn grid(n: usize) -> (Grid<f64>, Grid<f64>) {
    let x = linspace(-2.0, 2.0, n);
    meshgrid(&x, &x)
}

/// `_swirl(xx, yy)`: a rotational field with a sink.
fn swirl(xx: &Grid<f64>, yy: &Grid<f64>) -> (Grid<f64>, Grid<f64>) {
    let u = Grid::from_fn(xx.rows(), xx.cols(), |i, j| -yy[(i, j)] - 0.3 * xx[(i, j)]);
    let v = Grid::from_fn(xx.rows(), xx.cols(), |i, j| xx[(i, j)] - 0.3 * yy[(i, j)]);
    (u, v)
}

/// The swirl on an `n × n` mesh, placed at the mesh points.
fn swirl_field(n: usize) -> VectorField {
    let (xx, yy) = grid(n);
    let (u, v) = swirl(&xx, &yy);
    VectorField::on_mesh(&xx, &yy, &u, &v)
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("fields/quiver_units_xy", |fig| {
        let ax = fig.subplot();
        let options = QuiverOptions {
            units: QuiverUnits::Xy,
            width: Some(0.05),
            scale: Some(3.0),
            ..QuiverOptions::default()
        };
        ax.quiver(swirl_field(5), options)?;
        ax.set_aspect(pltrs::rc::Aspect::Equal)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/quiver_default", |fig| {
        fig.subplot()
            .quiver(swirl_field(9), QuiverOptions::default())?;
        Ok(())
    }),
    FigureCase::new("fields/quiver_uv_only", |fig| {
        let (xx, yy) = grid(6);
        let (u, v) = swirl(&xx, &yy);
        fig.subplot()
            .quiver(VectorField::on_index_grid(&u, &v), QuiverOptions::default())?;
        Ok(())
    })
    .sized(4.0, 3.0, 100.0),
    FigureCase::new("fields/quiver_xy_scale_units", |fig| {
        let ax = fig.subplot();
        let field = VectorField::new(
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 2.0, -1.0],
            [1.0, -0.5, 1.0],
        );
        let options = QuiverOptions {
            angles: Angles::Xy,
            scale_units: Some(QuiverUnits::Xy),
            scale: Some(1.0),
            ..QuiverOptions::default()
        };
        ax.quiver(field, options)?;
        ax.set_xlim(-1.0, 3.0)?;
        ax.set_ylim(-1.0, 3.0)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/quiver_scale_width", |fig| {
        let options = QuiverOptions {
            scale: Some(40.0),
            width: Some(0.012),
            color: "tab:red".into(),
            ..QuiverOptions::default()
        };
        fig.subplot().quiver(swirl_field(7), options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/quiver_units_dots_inches", |fig| {
        let axs = fig.subplots(1, 2)?;
        let dots = QuiverOptions {
            units: QuiverUnits::Dots,
            width: Some(2.0),
            ..QuiverOptions::default()
        };
        axs[(0, 0)].quiver(swirl_field(5), dots)?;
        let inches = QuiverOptions {
            units: QuiverUnits::Inches,
            width: Some(0.03),
            scale_units: Some(QuiverUnits::Inches),
            scale: Some(4.0),
            ..QuiverOptions::default()
        };
        axs[(0, 1)].quiver(swirl_field(5), inches)?;
        Ok(())
    })
    .sized(5.0, 2.5, 100.0),
    FigureCase::new("fields/quiver_pivot", |fig| {
        let axs = fig.subplots(1, 3)?;
        let (xx, yy) = grid(5);
        let pivots = [
            (Pivot::Tail, "tail"),
            (Pivot::Middle, "middle"),
            (Pivot::Tip, "tip"),
        ];
        for (ax, (pivot, name)) in axs.as_slice().iter().zip(pivots) {
            let options = QuiverOptions {
                pivot,
                scale: Some(12.0),
                ..QuiverOptions::default()
            };
            ax.quiver(swirl_field(5), options)?;
            ax.plot_with(
                xx.as_slice(),
                yy.as_slice(),
                LineProps::fmt("r.")?.markersize(3.0),
            )?;
            ax.set_title(name);
        }
        Ok(())
    })
    .sized(6.0, 2.5, 100.0),
    FigureCase::new("fields/quiver_head_shape", |fig| {
        let options = QuiverOptions {
            shape: QuiverArrowShape {
                head_width: 6.0,
                head_length: 8.0,
                head_axis_length: 7.0,
                ..QuiverArrowShape::default()
            },
            width: Some(0.008),
            ..QuiverOptions::default()
        };
        fig.subplot().quiver(swirl_field(5), options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/quiver_flat_head_edges", |fig| {
        let options = QuiverOptions {
            shape: QuiverArrowShape {
                head_axis_length: 0.0,
                head_width: 4.0,
                ..QuiverArrowShape::default()
            },
            color: "gold".into(),
            props: pltrs::collections::CollectionProps::default()
                .edgecolor("k")
                .linewidth(0.8),
            ..QuiverOptions::default()
        };
        fig.subplot().quiver(swirl_field(5), options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/quiver_color_by_c", |fig| {
        let (xx, yy) = grid(8);
        let (u, v) = swirl(&xx, &yy);
        let speed: Vec<f64> = u
            .as_slice()
            .iter()
            .zip(v.as_slice())
            .map(|(u, v)| u.hypot(*v))
            .collect();
        let colors = MappedColors::new(speed).cmap(colormaps::get("viridis")?);
        let field = VectorField::on_mesh(&xx, &yy, &u, &v).with_colors(colors);
        fig.subplot().quiver(field, QuiverOptions::default())?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/quiver_small_vectors", |fig| {
        let (xx, yy) = grid(6);
        let (u, v) = swirl(&xx, &yy);
        let scale = Grid::from_fn(6, 6, |i, j| {
            let r = xx[(i, j)].hypot(yy[(i, j)]);
            r * r
        });
        let scaled = |g: &Grid<f64>| Grid::from_fn(6, 6, |i, j| g[(i, j)] * scale[(i, j)]);
        let field = VectorField::on_mesh(&xx, &yy, &scaled(&u), &scaled(&v));
        let options = QuiverOptions {
            shape: QuiverArrowShape {
                min_shaft: 2.0,
                min_length: 0.5,
                ..QuiverArrowShape::default()
            },
            ..QuiverOptions::default()
        };
        fig.subplot().quiver(field, options)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("fields/quiverkey", |fig| {
        let ax = fig.subplot();
        let q = ax.quiver(swirl_field(7), QuiverOptions::default())?;
        let east = QuiverKeyOptions {
            labelpos: LabelPosition::East,
            coordinates: KeyCoordinates::Axes,
            ..QuiverKeyOptions::default()
        };
        ax.quiverkey(&q, (0.85, 1.03), 2.0, r"$2\ \mathrm{m/s}$", east);
        let north = QuiverKeyOptions {
            labelpos: LabelPosition::North,
            coordinates: KeyCoordinates::Data,
            color: Some("r".into()),
            ..QuiverKeyOptions::default()
        };
        ax.quiverkey(&q, (0.1, 0.1), 1.0, "1 unit", north);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
