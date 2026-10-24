//! Probe operations for the `contour_behaviour` area: colours, widths and
//! dashes of line contours, labels and legend entries, ported from
//! matplotlib's `test_contour.py` (see
//! `spec/reference/probes/test_contour_behaviour.py`). Every op runs under
//! the reference configuration.

use plt::artist::ArtistHandle;
use plt::colors::Color;
use plt::contour::{ClabelOptions, ContourOptions, ContourSet, Levels, QuadCoords, QuadField};
use plt::figure::Figure;
use plt::lines::LineStyle;
use plt::numeric::{Grid, arange, linspace};
use plt::rc;
use plt::ticker::pyfmt;
use plt_core::Rgba;

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::{object, rgba};
use crate::probes::{Args, Op, Outcome, ProbeError, made};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("contour.style", |args| with_rc(args, || style(args))),
    Op::new("contour.labels", |args| with_rc(args, || labels(args))),
    Op::new("contour.legend_elements", |args| {
        with_rc(args, || legend_elements(args))
    }),
];

fn rows(rows: Vec<Vec<f64>>) -> Result<Grid<f64>, ProbeError> {
    made(Grid::from_rows(rows))
}

/// The colours of `test_single_color_and_extend`.
fn single_color(index: &str) -> Result<Color, ProbeError> {
    Ok(match index {
        "0" => "darkred".into(),
        "1" => Rgba::new(1.0, 0.0, 0.0, 0.5).into(),
        "2" => (0.1, 0.2, 0.5, 0.3).into(),
        other => return Err(ProbeError::BadArgs(format!("unknown colour {other:?}"))),
    })
}

/// `(exp(-X² - Y²) - exp(-(X - 1)² - (Y - 1)²)) * 2` on the grid of
/// `test_linestyles`.
fn two_gaussians() -> QuadField {
    let (x, y) = (arange(-3.0, 3.0, 0.25), arange(-2.0, 2.0, 0.25));
    let z = Grid::from_fn(y.len(), x.len(), |r, c| {
        let (x, y) = (x[c], y[r]);
        ((-x * x - y * y).exp() - (-(x - 1.0).powi(2) - (y - 1.0).powi(2)).exp()) * 2.0
    });
    QuadField::new(z).with_coords(QuadCoords::Axes { x, y })
}

fn parse_style(name: &str) -> Result<Option<LineStyle>, ProbeError> {
    if name == "-" {
        Ok(None)
    } else {
        made(name.parse()).map(Some)
    }
}

/// The line contour set a `contour.style` scene names.
fn style_set(scene: &str) -> Result<ContourSet, ProbeError> {
    let fig = Figure::new();
    let ax = fig.subplot();
    let parts: Vec<&str> = scene.split(':').collect();
    let set = match parts.as_slice() {
        ["color", index, extend] => {
            let options = ContourOptions {
                levels: Levels::Values(vec![0.5, 0.75, 1.0, 1.25, 1.5]),
                colors: Some(vec![single_color(index)?]),
                extend: made(extend.parse())?,
                ..ContourOptions::default()
            };
            let z = rows(vec![vec![0.0, 1.0], vec![1.0, 2.0]])?;
            ax.contour(&QuadField::new(z), options)
        }
        ["linewidth", case] => {
            let (contour, call) = match *case {
                "rc_lines" => ("None", None),
                "rc_contour" => ("4.24", None),
                "explicit" => ("4.24", Some(vec![5.02])),
                other => return Err(ProbeError::BadArgs(format!("unknown case {other:?}"))),
            };
            made(rc::set("lines.linewidth", "1.23"))?;
            made(rc::set("contour.linewidth", contour))?;
            let z = Grid::from_fn(4, 3, |r, c| (3 * r + c) as f64);
            let options = ContourOptions {
                linewidths: call,
                ..ContourOptions::default()
            };
            ax.contour(&QuadField::new(z), options)
        }
        ["styles", linestyles, negative, rc_style] => {
            if let Some(style) = parse_style(rc_style)? {
                made(rc::set("contour.negative_linestyle", &style.to_string()))?;
            }
            let options = ContourOptions {
                levels: Levels::Count(6),
                colors: Some(vec!["k".into()]),
                linestyles: parse_style(linestyles)?.map(|style| vec![style]),
                negative_linestyles: parse_style(negative)?,
                ..ContourOptions::default()
            };
            ax.contour(&two_gaussians(), options)
        }
        ["all_nan"] => ax.contour(
            &QuadField::new(Grid::filled(2, 2, f64::NAN)),
            ContourOptions::default(),
        ),
        _ => return Err(ProbeError::BadArgs(format!("unknown scene {scene:?}"))),
    };
    made(set)
}

/// `items` cycled to `n` entries.
fn cycled<T: Clone>(items: &[T], n: usize) -> Vec<T> {
    (0..n).map(|i| items[i % items.len()].clone()).collect()
}

fn style(args: Args<'_>) -> Outcome {
    let set = style_set(args.str("scene")?)?;
    let levels = set.levels();
    let n = levels.len();
    let colors = cycled(&set.colors(), n);
    let widths = cycled(&set.linewidths(), n);
    let dashed = cycled(&set.dash_patterns(), n)
        .iter()
        .map(|dashes| Json::Bool(!dashes.is_solid()))
        .collect();
    Ok(object([
        ("levels", Json::from(levels)),
        (
            "colors",
            Json::Array(colors.into_iter().map(rgba).collect()),
        ),
        ("linewidths", Json::from(widths)),
        ("dashed", Json::Array(dashed)),
    ]))
}

/// The labels `clabel` makes for a scene.
fn labels(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let scene = args.str("scene")?;
    let (set, options) = match scene.split_once(':').unwrap_or((scene, "")) {
        ("powerlimits", _) => {
            let z = Grid::from_fn(10, 10, |r, c| {
                let t = (10 * r + c) as f64 / 99.0;
                1e-6 * 100f64.powf(t)
            });
            let options = ContourOptions {
                levels: Levels::Values(vec![0.25e-5, 1e-5, 4e-5]),
                ..ContourOptions::default()
            };
            (
                ax.contour(&QuadField::new(z), options),
                ClabelOptions::default(),
            )
        }
        ("zorder", zorder) => {
            let z = Grid::from_fn(10, 10, |r, c| r.max(c) as f64);
            let set = made(ax.contour(&QuadField::new(z), ContourOptions::default()))?;
            for collection in set.collections() {
                collection.set_zorder(123.0);
            }
            let options = ClabelOptions {
                zorder: if zorder == "-" {
                    None
                } else {
                    Some(made(zorder.parse::<f64>())?)
                },
                ..ClabelOptions::default()
            };
            (Ok(set), options)
        }
        ("manual", _) => {
            let x = linspace(-10.0, 10.0, 50);
            let z = Grid::from_fn(50, 50, |r, c| x[c] * x[c] / (x[r] * x[r]) - 1.0);
            let field = QuadField::new(z).with_coords(QuadCoords::Axes { x: x.clone(), y: x });
            let options = ContourOptions {
                levels: Levels::Values(vec![0.0, 100.0]),
                ..ContourOptions::default()
            };
            let manual = ClabelOptions {
                manual: Some(vec![(1.3, 1.0)]),
                ..ClabelOptions::default()
            };
            (ax.contour(&field, options), manual)
        }
        ("manual_subset", _) => {
            let options = ContourOptions {
                levels: Levels::Values(vec![1.5, 2.5, 3.5]),
                ..ContourOptions::default()
            };
            let z = rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]])?;
            let manual = ClabelOptions {
                levels: Some(vec![2.5]),
                manual: Some(vec![(0.5, 0.5)]),
                ..ClabelOptions::default()
            };
            (ax.contour(&QuadField::new(z), options), manual)
        }
        _ => return Err(ProbeError::BadArgs(format!("unknown scene {scene:?}"))),
    };
    let texts = made(made(set)?.clabel(options))?;
    let labels = texts
        .iter()
        .map(|text| {
            object([
                ("text", text.text().into()),
                ("zorder", text.zorder().into()),
            ])
        })
        .collect();
    Ok(Json::Array(labels))
}

/// The labels of `legend_elements()` and the number of handles.
fn legend_elements(args: Args<'_>) -> Outcome {
    let filled = args.bool("filled")?;
    let fig = Figure::new();
    let ax = fig.subplot();
    let z = Grid::from_fn(9, 9, |r, c| ((r + 1) * (c + 1)) as f64);
    let colors: [&str; 3] = if filled {
        ["#FFFF00", "#FF00FF", "#00FFFF"]
    } else {
        ["blue", "#00FF00", "red"]
    };
    let options = ContourOptions {
        levels: Levels::Values(vec![10.0, 30.0, 50.0]),
        colors: Some(colors.map(Color::from).to_vec()),
        extend: plt::colors::Extend::Both,
        ..ContourOptions::default()
    };
    let field = QuadField::new(z);
    let set = made(if filled {
        ax.contourf(&field, options)
    } else {
        ax.contour(&field, options)
    })?;
    let (handles, labels) = set.legend_elements("x", pyfmt::repr);
    Ok(object([
        ("labels", Json::from(labels)),
        (
            "handles",
            Json::Integer(i64::try_from(handles.len()).expect("few handles")),
        ),
    ]))
}
