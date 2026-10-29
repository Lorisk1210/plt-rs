//! The label and nearest-contour ops of the `mpl_contour` area: the labels
//! `clabel` makes on the fields of matplotlib's label tests, and
//! `find_nearest_contour` on the Gaussian of `test_find_nearest_contour`.

use std::f64::consts::{FRAC_PI_2, PI};

use pltrs::artist::ArtistHandle;
use pltrs::contour::{
    ClabelFormat, ClabelOptions, ContourOptions, ContourSet, Levels, NearestIn, QuadCoords,
    QuadField,
};
use pltrs::figure::Figure;
use pltrs::fonts::FontSize;
use pltrs::numeric::{Grid, arange, linspace, meshgrid};
use pltrs_core::Point;

use super::{made, unknown};
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};

/// The labels of a scene: level, text, centre in display pixels, rotation
/// and z-order of each.
pub(super) fn clabel(args: Args<'_>) -> Outcome {
    let scene = args.str("scene")?;
    let (name, rest) = scene.split_once(':').unwrap_or((scene, ""));
    let fig = Figure::new();
    let (field, contour, clabel) = match name {
        "outside_levels" => (
            QuadField::new(Grid::from_fn(9, 9, |r, c| (9 * r + c) as f64)),
            ContourOptions {
                levels: Levels::Values(vec![100.0]),
                ..ContourOptions::default()
            },
            ClabelOptions {
                fmt: ClabelFormat::Map(vec![(100.0, "%1.2f".to_owned())]),
                ..ClabelOptions::default()
            },
        ),
        "circular" => {
            let (x, y) = meshgrid(&linspace(-2.0, 2.0, 4), &linspace(-2.0, 2.0, 4));
            let z = Grid::from_fn(4, 4, |r, c| x[(r, c)].hypot(y[(r, c)]));
            (
                QuadField::new(z).with_coords(QuadCoords::Mesh { x, y }),
                ContourOptions::default(),
                ClabelOptions::default(),
            )
        }
        "large_spacing" => {
            let x = arange(-3.0, 3.01, 0.05);
            let z = Grid::from_fn(x.len(), x.len(), |r, c| (-x[c] * x[c] - x[r] * x[r]).exp());
            (
                QuadField::new(z).with_coords(QuadCoords::Axes { x: x.clone(), y: x }),
                ContourOptions {
                    levels: Levels::Values(vec![0.01, 0.2, 0.5, 0.8]),
                    ..ContourOptions::default()
                },
                ClabelOptions {
                    inline_spacing: 100.0,
                    ..ClabelOptions::default()
                },
            )
        }
        "contour_start" => {
            fig.set_dpi(100.0);
            (
                wave(),
                ContourOptions::default(),
                ClabelOptions {
                    fontsize: Some(FontSize::Points(9.0)),
                    ..ClabelOptions::default()
                },
            )
        }
        "zorder" => return zorder_labels(&fig, rest),
        _ => return Err(unknown(scene)),
    };
    let cs = made(fig.subplot().contour(&field, contour))?;
    made(cs.clabel(clabel))?;
    labels(&cs)
}

/// The latitude-longitude wave of `test_label_contour_start`.
fn wave() -> QuadField {
    let (lons, lats) = meshgrid(
        &linspace(-FRAC_PI_2, FRAC_PI_2, 50),
        &linspace(-FRAC_PI_2, FRAC_PI_2, 50),
    );
    let z = Grid::from_fn(50, 50, |r, c| {
        let (lon, lat) = (lons[(r, c)], lats[(r, c)]);
        let wave = 0.75 * (2.0 * lat).sin().powf(8.0) * (4.0 * lon).cos();
        let mean = 0.5 * (2.0 * lat).cos() * ((2.0 * lat).sin().powi(2) + 2.0);
        wave + mean
    });
    QuadField::new(z).with_coords(QuadCoords::Mesh { x: lons, y: lats })
}

/// `test_clabel_zorder`: `max(|x|, |y|)` contoured at z-order 123 and
/// labelled with the scene's `use_clabeltext` and `zorder`.
fn zorder_labels(fig: &Figure, scene: &str) -> Outcome {
    let (clabeltext, zorder) = scene.split_once(':').ok_or_else(|| unknown(scene))?;
    let z = Grid::from_fn(10, 10, |r, c| r.max(c) as f64);
    let cs = made(
        fig.subplot()
            .contour(&QuadField::new(z), ContourOptions::default()),
    )?;
    for collection in cs.collections() {
        collection.set_zorder(123.0);
    }
    made(cs.clabel(ClabelOptions {
        zorder: if zorder == "None" {
            None
        } else {
            Some(made(zorder.parse::<f64>())?)
        },
        use_clabeltext: clabeltext == "True",
        ..ClabelOptions::default()
    }))?;
    labels(&cs)
}

/// The labels of `cs` as matplotlib's label tests read them: the rotation
/// is the text's `get_rotation`, turned by the data transform when the
/// label keeps its angle in data coordinates (`use_clabeltext`).
fn labels(cs: &ContourSet) -> Outcome {
    let data = made(
        cs.axes()
            .figure()
            .resolve_transform(&cs.axes().trans_data()),
    )?;
    let labels = cs
        .labels()
        .iter()
        .map(|label| {
            let text = &label.artist;
            let rotation = if text.transform_rotates_text() {
                data.transform_angle(text.rotation(), text.position())
                    .rem_euclid(360.0)
            } else {
                label.rotation
            };
            object([
                ("level", Json::from(label.level)),
                ("text", Json::from(label.text.clone())),
                (
                    "display",
                    Json::from(vec![label.position.x, label.position.y]),
                ),
                ("rotation", Json::from(rotation)),
                ("zorder", Json::from(text.zorder())),
            ])
        })
        .collect();
    Ok(Json::Array(labels))
}

/// `find_nearest_contour` on `plt.contour(img, 10)` (or `contourf`) of the
/// Gaussian `img` on `np.indices((15, 15))`.
pub(super) fn find_nearest_contour(args: Args<'_>) -> Outcome {
    let img = Grid::from_fn(15, 15, |r, c| {
        let squares = (r as f64 - 5.0).powi(2) + (c as f64 - 5.0).powi(2);
        (-PI * (squares / 25.0)).exp()
    });
    let options = ContourOptions {
        levels: Levels::Count(10),
        ..ContourOptions::default()
    };
    let ax = Figure::new().subplot();
    let field = QuadField::new(img);
    let cs = made(if args.bool("filled")? {
        ax.contourf(&field, options)
    } else {
        ax.contour(&field, options)
    })?;
    let levels = match args.get("indices") {
        None | Some(Json::Null) => None,
        Some(value) => Some(
            value
                .as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad indices {value}")))?
                .into_iter()
                .map(|i| i as usize)
                .collect::<Vec<_>>(),
        ),
    };
    let space = if args.bool("pixel")? {
        NearestIn::Display
    } else {
        NearestIn::Coordinates
    };
    let point = Point::new(args.f64("x")?, args.f64("y")?);
    let near = made(cs.find_nearest_contour(point, levels.as_deref(), space))?
        .ok_or_else(|| ProbeError::rejected("no contour lines"))?;
    let index = |i: usize| Json::Integer(i64::try_from(i).expect("small index"));
    Ok(Json::Array(vec![
        index(near.level),
        index(near.line),
        index(near.vertex),
        Json::from(near.point.x),
        Json::from(near.point.y),
        Json::from(near.distance_squared),
    ]))
}
