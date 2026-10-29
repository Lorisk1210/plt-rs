//! The Mollweide projection, pyplot's current axes across figures, and
//! saving with a zero font size.

use pltrs::figure::{Figure, SaveFormat, SaveOptions};
use pltrs::geo::GeoProjection;
use pltrs::pyplot;
use pltrs_core::Point;

use super::{count, made};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::encode::{parse_points, points};
use crate::probes::{Args, Outcome};

pub(super) fn mollweide(args: Args<'_>) -> Outcome {
    styled(args, || {
        let projection = Figure::new()
            .geo_subplot(GeoProjection::Mollweide)
            .geo_projection();
        let (map, input): (fn(GeoProjection, Point) -> Point, _) = match args.get("lonlat") {
            Some(lonlat) => (GeoProjection::forward, lonlat),
            None => (GeoProjection::inverse, args.json("xy")?),
        };
        let mapped: Vec<Point> = parse_points(input)?
            .into_iter()
            .map(|p| map(projection, p))
            .collect();
        Ok(points(&mapped))
    })
}

pub(super) fn sca_figures(args: Args<'_>) -> Outcome {
    styled(args, || {
        pyplot::close_all();
        let outcome = current_after_sca();
        pyplot::close_all();
        outcome
    })
}

/// `fig1, ax1 = plt.subplots(); plt.subplots(); plt.sca(ax1)`: whether
/// `ax1` is the current axes and `fig1` the current figure.
fn current_after_sca() -> Outcome {
    let (fig1, axes1) = made(pyplot::subplots(1, 1))?;
    made(pyplot::subplots(1, 1))?;
    let ax1 = &axes1[(0, 0)];
    pyplot::sca(ax1);
    Ok(Json::Array(vec![
        Json::from(pyplot::gca() == *ax1),
        Json::from(pyplot::gcf() == fig1),
    ]))
}

pub(super) fn savefig_raw(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        made(fig.subplot().plot([0.0, 1.0], [0.0, 1.0]))?;
        let options = SaveOptions {
            dpi: Some(80.0),
            ..SaveOptions::default()
        };
        let raw = made(fig.to_format(SaveFormat::Raw, options))?;
        let (width, height) = fig.width_height();
        Ok(Json::Array(vec![
            Json::from(f64::from(width)),
            Json::from(f64::from(height)),
            count(raw.len() / 4),
        ]))
    })
}
