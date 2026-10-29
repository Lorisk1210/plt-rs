//! Line contours and their labels: `contour_corner_mask`, `contour_demo`,
//! `contour_label_demo` and `contours_in_optimization_demo`.

use pltrs::axes::{GridOptions, ImshowOptions};
use pltrs::colorbar::ColorbarOptions;
use pltrs::colors::{Color, Colorize, ColorizerOptions, colormaps};
use pltrs::contour::{
    ClabelFormat, ClabelOptions, ContourOptions, ContourOrigin, ContourSet, QuadCoords, QuadField,
};
use pltrs::figure::Figure;
use pltrs::fonts::FontSize;
use pltrs::image::{Extent, Interpolation};
use pltrs::lines::LineStyle;
use pltrs::numeric::{Grid, arange, linspace, meshgrid};
use pltrs::orientation::Orientation;
use pltrs::patheffects::{PathEffect, TickedStroke};
use pltrs::prelude::ArtistHandle;
use pltrs::ticker::{FuncFormatter, LogFormatter, LogLabelStyle, LogLocator};

use crate::figures::cases::common::{colors, contour_field, count, percent_label, values};
use crate::figures::{CaseResult, FigureCase};
use std::rc::Rc;

/// `contour_demo`'s field with its coordinates.
fn demo_field() -> QuadField {
    let (x, y, z) = contour_field();
    QuadField::new(z).with_coords(QuadCoords::Mesh { x, y })
}

/// Labels of the given size on every level.
fn sized_labels(size: f64) -> ClabelOptions {
    ClabelOptions {
        fontsize: Some(FontSize::Points(size)),
        ..ClabelOptions::default()
    }
}

fn contour_corner_mask(fig: &Figure) -> CaseResult {
    let (x, y) = meshgrid(&arange(0.0, 7.0, 1.0), &arange(0.0, 10.0, 1.0));
    let z = Grid::from_fn(10, 7, |r, c| {
        (0.5 * x[(r, c)]).sin() * (0.52 * y[(r, c)]).cos()
    });
    let masked = [(2, 3), (2, 4), (3, 4), (4, 4), (7, 2), (5, 0), (0, 6)];
    let mask = Grid::from_fn(10, 7, |r, c| masked.contains(&(r, c)));
    let axs = fig.subplots(1, 2)?;
    for (ax, corner_mask) in axs.as_slice().iter().zip([false, true]) {
        let field = QuadField {
            corner_mask: Some(corner_mask),
            ..QuadField::new(z.clone())
                .with_coords(QuadCoords::Mesh {
                    x: x.clone(),
                    y: y.clone(),
                })
                .with_mask(mask.clone())
        };
        let cs = ax.contourf(&field, ContourOptions::default())?;
        // `ax.contour(cs, ...)`: the same field at the levels of `cs`.
        let lines = ContourOptions {
            colors: Some(colors(&["k"])),
            ..values(&cs.levels())
        };
        ax.contour(&field, lines)?;
        let printed = if corner_mask { "True" } else { "False" };
        ax.set_title(format!("corner_mask={printed}"));
        ax.grid_with(
            true,
            GridOptions {
                color: Some("k".into()),
                linestyle: Some(LineStyle::Solid),
                alpha: Some(0.3),
                ..GridOptions::default()
            },
        );
        // `plot(np.ma.array(x, mask=~mask), y, 'ro')`: one line per
        // column, showing the masked points only.
        for c in 0..7 {
            let xs: Vec<f64> = (0..10)
                .map(|r| if mask[(r, c)] { x[(r, c)] } else { f64::NAN })
                .collect();
            ax.plot_fmt(xs, y.column(c), "ro")?;
        }
    }
    Ok(())
}

fn contour_demo_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let cs = ax.contour(&demo_field(), ContourOptions::default())?;
    cs.clabel(sized_labels(10.0))?;
    ax.set_title("Simplest default with labels");
    Ok(())
}

fn contour_demo_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let cs = ax.contour(&demo_field(), ContourOptions::default())?;
    cs.clabel(ClabelOptions {
        manual: Some(vec![
            (-1.0, -1.4),
            (-0.62, -0.7),
            (-2.0, 0.5),
            (1.7, 1.2),
            (2.0, 1.4),
            (2.4, 1.7),
        ]),
        ..sized_labels(10.0)
    })?;
    ax.set_title("labels at selected locations");
    Ok(())
}

/// Six black levels labelled in 9 points, titled `title`.
fn single_color(fig: &Figure, title: &str) -> CaseResult {
    let ax = fig.subplot();
    let options = ContourOptions {
        colors: Some(colors(&["k"])),
        ..count(6)
    };
    ax.contour(&demo_field(), options)?
        .clabel(sized_labels(9.0))?;
    ax.set_title(title);
    Ok(())
}

fn contour_demo_3(fig: &Figure) -> CaseResult {
    single_color(fig, "Single color - negative contours dashed")
}

fn contour_demo_4(fig: &Figure) -> CaseResult {
    single_color(fig, "Single color - negative contours solid")
}

fn contour_demo_5(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = ContourOptions {
        linewidths: Some(arange(0.5, 4.0, 0.5)),
        colors: Some(vec![
            Color::from("r"),
            Color::from("green"),
            Color::from("blue"),
            Color::from([1.0, 1.0, 0.0]),
            Color::from("#afeeee"),
            Color::from("0.5"),
        ]),
        ..count(6)
    };
    ax.contour(&demo_field(), options)?
        .clabel(sized_labels(9.0))?;
    ax.set_title("Crazy lines");
    Ok(())
}

fn contour_demo_6(fig: &Figure) -> CaseResult {
    let (_, _, z) = contour_field();
    let ax = fig.subplot();
    let image = ImshowOptions {
        origin: Some(pltrs::image::Origin::Lower),
        extent: Some(Extent::new(-3.0, 3.0, -2.0, 2.0)),
        interpolation: Some(Interpolation::Bilinear),
        ..ImshowOptions::default()
    }
    .cmap(colormaps::get("gray")?);
    let im = ax.imshow(z.clone(), image)?;
    let levels = arange(-1.2, 1.6, 0.2);
    let field = QuadField::new(z).with_coords(QuadCoords::Implicit {
        origin: Some(ContourOrigin::Lower),
        extent: Some([-3.0, 3.0, -2.0, 2.0]),
    });
    let options = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("flag")?),
            ..ColorizerOptions::default()
        },
        extend: pltrs::colors::Extend::Both,
        linewidths: Some(vec![2.0]),
        ..values(&levels)
    };
    let cs = ax.contour(&field, options)?;
    let mut widths = vec![2.0; levels.len()];
    widths[6] = 4.0;
    cs.set_linewidths(&widths);
    cs.clabel(ClabelOptions {
        levels: Some(levels.iter().copied().skip(1).step_by(2).collect()),
        fmt: ClabelFormat::Label("%1.1f".into()),
        ..sized_labels(14.0)
    })?;
    let cb = fig.colorbar(
        &cs,
        ColorbarOptions {
            shrink: 0.8,
            ..ColorbarOptions::default()
        },
    )?;
    ax.set_title("Lines with colorbar");
    fig.colorbar(
        &im,
        ColorbarOptions {
            orientation: Some(Orientation::Horizontal),
            shrink: 0.8,
            ..ColorbarOptions::default()
        },
    )?;
    let (_, b, _, h) = ax.position().bounds();
    let (ll, _, ww, _) = cb.ax().position().bounds();
    cb.ax().set_position([ll, b + 0.1 * h, ww, h * 0.8]);
    Ok(())
}

/// The default line contours of `contour_demo`'s field.
fn default_lines(fig: &Figure) -> Result<ContourSet, Box<dyn std::error::Error>> {
    Ok(fig
        .subplot()
        .contour(&demo_field(), ContourOptions::default())?)
}

fn contour_label_demo_1(fig: &Figure) -> CaseResult {
    let cs = default_lines(fig)?;
    cs.clabel(ClabelOptions {
        levels: Some(cs.levels()),
        fmt: ClabelFormat::Formatter(Rc::new(FuncFormatter::new(|x, _| percent_label(x)))),
        ..sized_labels(10.0)
    })?;
    Ok(())
}

fn contour_label_demo_2(fig: &Figure) -> CaseResult {
    let cs = default_lines(fig)?;
    let names = [
        "first", "second", "third", "fourth", "fifth", "sixth", "seventh",
    ];
    let levels = cs.levels();
    cs.clabel(ClabelOptions {
        levels: Some(levels.iter().copied().step_by(2).collect()),
        fmt: ClabelFormat::Map(
            levels
                .iter()
                .zip(names)
                .map(|(&level, name)| (level, name.to_owned()))
                .collect(),
        ),
        ..sized_labels(10.0)
    })?;
    Ok(())
}

fn contour_label_demo_3(fig: &Figure) -> CaseResult {
    let (x, y, z) = contour_field();
    let ax = fig.subplot();
    let powers = QuadField::new(z.map(|v| 100f64.powf(*v))).with_coords(QuadCoords::Mesh { x, y });
    let cs = ax.contour(
        &powers,
        ContourOptions {
            locator: Some(Box::new(LogLocator::new(10.0)?)),
            ..ContourOptions::default()
        },
    )?;
    cs.clabel(ClabelOptions {
        levels: Some(cs.levels()),
        fmt: ClabelFormat::Formatter(Rc::new(LogFormatter::new(LogLabelStyle::Mathtext, 10.0))),
        ..ClabelOptions::default()
    })?;
    ax.set_title("$100^Z$");
    Ok(())
}

/// A constraint of `contours_in_optimization_demo`: the function whose
/// zero contour bounds the feasible region, its colour and its ticks.
type Constraint<'a> = (&'a dyn Fn(f64, f64) -> f64, &'a str, TickedStroke);

fn contours_in_optimization_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let (x1, x2) = meshgrid(&linspace(0.001, 4.0, 101), &linspace(0.001, 4.0, 105));
    let field = |f: &dyn Fn(f64, f64) -> f64| {
        let z = Grid::from_fn(105, 101, |r, c| f(x1[(r, c)], x2[(r, c)]));
        QuadField::new(z).with_coords(QuadCoords::Mesh {
            x: x1.clone(),
            y: x2.clone(),
        })
    };
    let objective = field(&|a, b| a * a + b * b - 2.0 * a - 2.0 * b + 2.0);
    let cntr = ax.contour(
        &objective,
        ContourOptions {
            colors: Some(colors(&["black"])),
            ..values(&[0.01, 0.1, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0])
        },
    )?;
    cntr.clabel(ClabelOptions {
        fmt: ClabelFormat::Label("%2.1f".into()),
        use_clabeltext: true,
        ..ClabelOptions::default()
    })?;
    let constraints: [Constraint<'_>; 3] = [
        (
            &|a, b| -(3.0 * a + b - 5.5),
            "sandybrown",
            TickedStroke {
                angle: 135.0,
                ..TickedStroke::default()
            },
        ),
        (
            &|a, b| -(a + 2.0 * b - 4.5),
            "orangered",
            TickedStroke {
                angle: 60.0,
                length: 2.0,
                ..TickedStroke::default()
            },
        ),
        (
            &|a, b| 0.8 + a.powf(-3.0) - b,
            "mediumblue",
            TickedStroke {
                spacing: 7.0,
                ..TickedStroke::default()
            },
        ),
    ];
    for (g, color, ticks) in constraints {
        let cg = ax.contour(
            &field(g),
            ContourOptions {
                colors: Some(colors(&[color])),
                ..values(&[0.0])
            },
        )?;
        for collection in cg.collections() {
            collection.set_path_effects(vec![PathEffect::with_ticked_stroke(ticks.clone())]);
        }
    }
    ax.set_xlim(0.0, 4.0)?;
    ax.set_ylim(0.0, 4.0)?;
    Ok(())
}

/// `contour_demo` sets this before its fourth figure and keeps it.
const SOLID_NEGATIVE: &[(&str, &str)] = &[("contour.negative_linestyle", "solid")];

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_images/contour_corner_mask", contour_corner_mask),
    FigureCase::new("gallery_images/contour_demo_1", contour_demo_1),
    FigureCase::new("gallery_images/contour_demo_2", contour_demo_2),
    FigureCase::new("gallery_images/contour_demo_3", contour_demo_3),
    FigureCase::new("gallery_images/contour_demo_4", contour_demo_4).with_rc(SOLID_NEGATIVE),
    FigureCase::new("gallery_images/contour_demo_5", contour_demo_5).with_rc(SOLID_NEGATIVE),
    FigureCase::new("gallery_images/contour_demo_6", contour_demo_6).with_rc(SOLID_NEGATIVE),
    FigureCase::new("gallery_images/contour_label_demo_1", contour_label_demo_1),
    FigureCase::new("gallery_images/contour_label_demo_2", contour_label_demo_2),
    FigureCase::new("gallery_images/contour_label_demo_3", contour_label_demo_3),
    FigureCase::new(
        "gallery_images/contours_in_optimization_demo",
        contours_in_optimization_demo,
    )
    .sized(6.0, 6.0, 100.0),
];
