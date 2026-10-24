//! Other projections: polar axes, geographic maps and a projection of
//! our own, registered by name.

use std::f64::consts::PI;

use plt::axes::{AxisName, ProjectionKind, TickParams};
use plt::colors::colormaps;
use plt::geo::GeoProjection;
use plt::legend::LegendOptions;
use plt::numeric::linspace;
use plt::polar::{Compass, ThetaDirection};
use plt::prelude::*;
use plt::projections::{self, Projection, ProjectionView};
use plt::ticker::{MultipleLocator, NullFormatter, ScalarFormatter};
use plt::transforms::{Affine2D, ResolvedTransform};

use super::data::Samples;
use super::lines::map;
use super::{Built, Entry, background};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure("polar", "A rose curve on polar axes", polar),
    Entry::figure("polar_bars", "A wind rose: bars on polar axes", polar_bars),
    Entry::figure(
        "polar_scatter",
        "A scatter plot on polar axes",
        polar_scatter,
    ),
    Entry::figure(
        "map_projections",
        "Aitoff, Hammer, Lambert and Mollweide maps",
        map_projections,
    ),
    Entry::figure(
        "custom_projection",
        "A projection of our own: skewed x axes for a sounding",
        custom_projection,
    ),
];

/// `r = |cos 4θ|` traced over a full turn.
fn polar() -> Built {
    let fig = Figure::with_size(5.0, 5.0, 100.0);
    let ax = fig.polar_subplot();
    let theta = linspace(0.0, 2.0 * PI, 600);
    let r = map(&theta, |t| (4.0 * t).cos().abs());
    ax.plot(theta, r)?;
    ax.set_title("r = |cos 4θ|");
    Ok(fig)
}

/// How often the wind blew from each of 16 directions, in two speed
/// classes stacked on each other. Compass axes put north at the top and
/// turn clockwise.
fn polar_bars() -> Built {
    let fig = Figure::with_size(5.5, 5.5, 100.0);
    let ax = fig.polar_subplot();
    ax.set_theta_zero_location(Compass::N, 0.0)
        .set_theta_direction(ThetaDirection::Clockwise);
    let theta = linspace(0.0, 2.0 * PI * 15.0 / 16.0, 16);
    let calm = map(&theta, |t| {
        4.0 + 3.0 * (t - 4.0).cos().max(0.0) + (2.0 * t).sin().abs()
    });
    let strong = map(&theta, |t| 1.0 + 5.0 * (t - 4.2).cos().max(0.0).powi(2));
    let width = 2.0 * PI / 16.0 * 0.9;
    let bars = |label: &str| BarOptions {
        thickness: width.into(),
        label: Some(label.into()),
        props: PatchProps::default().edgecolor(background()).linewidth(0.8),
        ..BarOptions::default()
    };
    ax.bar(&theta, &calm, bars("< 5 m/s"))?;
    ax.bar(
        &theta,
        &strong,
        BarOptions {
            base: calm.into(),
            ..bars("≥ 5 m/s")
        },
    )?;
    ax.set_thetagrids_labeled(&[0.0, 90.0, 180.0, 270.0], ["N", "E", "S", "W"])?;
    ax.set_title("Wind rose");
    ax.legend(LegendOptions::default().bbox_to_anchor((1.1, 1.05)))?;
    Ok(fig)
}

/// Points placed by angle and radius, their area growing with the radius
/// and their colour following the angle round a cyclic colormap.
fn polar_scatter() -> Built {
    let fig = Figure::with_size(5.0, 5.0, 100.0);
    let ax = fig.polar_subplot();
    let mut samples = Samples::new(19);
    let r: Vec<f64> = (0..150).map(|_| 2.0 * samples.uniform()).collect();
    let theta: Vec<f64> = (0..150).map(|_| 2.0 * PI * samples.uniform()).collect();
    let options = ScatterOptions {
        sizes: Some(map(&r, |r| 200.0 * r * r)),
        colors: MappedColors::new(theta.clone())
            .cmap(colormaps::get("twilight")?)
            .into(),
        alpha: 0.75.into(),
        ..ScatterOptions::default()
    };
    ax.scatter_with(theta, r, options)?;
    ax.set_title("Polar scatter");
    Ok(fig)
}

/// The same great circle on four map projections of the whole sphere;
/// on Lambert's azimuthal map, centred on it, it is a straight line.
fn map_projections() -> Built {
    let fig = Figure::with_size(8.0, 5.0, 100.0);
    let lambert = GeoProjection::Lambert {
        center_longitude: 0.0,
        center_latitude: 0.0,
    };
    let kinds = [
        ("Aitoff", GeoProjection::Aitoff),
        ("Hammer", GeoProjection::Hammer),
        ("Lambert", lambert),
        ("Mollweide", GeoProjection::Mollweide),
    ];
    // A great circle tilted 50° from the equator, in longitude/latitude.
    let lon = linspace(-PI, PI, 300);
    let lat = map(&lon, |l| (50_f64.to_radians().tan() * l.sin()).atan());
    for (index, (name, kind)) in kinds.into_iter().enumerate() {
        let ax = fig.add_geo_subplot(kind, 2, 2, index + 1)?;
        ax.plot(&lon, &lat)?;
        // Fewer gridlines than the default 30° and 15° keep small maps legible.
        ax.set_longitude_grid(60.0)?.set_latitude_grid(30.0)?;
        if kind == lambert {
            // On the azimuthal map the latitude labels gather at the poles.
            ax.tick_params(&TickParams {
                labelleft: Some(false),
                ..TickParams::default()
            });
        }
        ax.grid(true);
        ax.set_title(name);
    }
    Ok(fig)
}

/// Axes whose data is sheared 30° to the right: lines of constant x
/// lean, as the isotherms of a meteorologist's skew-T diagram do.
#[derive(Debug, Default)]
struct SkewXAxes;

impl SkewXAxes {
    /// The shear, applied in axes coordinates.
    fn skew() -> ResolvedTransform {
        ResolvedTransform::affine(Affine2D::identity().skew_deg(30.0, 0.0))
    }
}

impl Projection for SkewXAxes {
    fn name(&self) -> &'static str {
        "skewx"
    }

    /// Data to axes: the usual scales and limits, then the shear.
    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        view.rectilinear().then(Self::skew())
    }

    /// The x axis (ticks and gridlines) is sheared the same way.
    fn axis_transform(
        &self,
        view: &ProjectionView<'_>,
        axis: AxisName,
    ) -> Option<ResolvedTransform> {
        let across = ResolvedTransform::blend(view.rectilinear(), ResolvedTransform::identity());
        (axis == AxisName::X).then(|| across.then(Self::skew()))
    }
}

/// A temperature and dew-point profile against pressure on the
/// registered `skewx` projection, with a log pressure axis.
fn custom_projection() -> Built {
    let fig = Figure::with_size(5.5, 5.5, 100.0);
    if !projections::projection_names().contains(&"skewx") {
        projections::register(SkewXAxes::default)?;
    }
    let ax = fig.subplot_with_projection(ProjectionKind::Registered("skewx"))?;
    let pressure = linspace(1000.0, 100.0, 60);
    let height = map(&pressure, |p| -7.0 * (p / 1000.0).ln());
    let temperature = map(&height, |h| {
        25.0 - 6.5 * h.min(11.0) + 2.0 * (h * 1.3).sin()
    });
    let dewpoint: Vec<f64> = temperature
        .iter()
        .zip(&height)
        .map(|(t, h)| t - 3.0 - 2.5 * h)
        .collect();
    ax.semilogy(&temperature, &pressure)?
        .set_color("C7")
        .set_label("temperature");
    ax.semilogy(&dewpoint, &pressure)?
        .set_color("C5")
        .set_label("dew point");
    ax.axvline(0.0, LineProps::fmt("--")?.color("C0").label("0 °C"));
    ax.yaxis().set_major_formatter(ScalarFormatter::default());
    ax.yaxis().set_minor_formatter(NullFormatter);
    ax.set_yticks(linspace(100.0, 1000.0, 10));
    ax.set_ylim(1050.0, 100.0)?;
    ax.xaxis().set_major_locator(MultipleLocator::new(10.0)?);
    ax.set_xlim(-50.0, 50.0)?;
    ax.set_xlabel("°C").set_ylabel("hPa");
    ax.set_title("A sounding on skewed axes");
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}
