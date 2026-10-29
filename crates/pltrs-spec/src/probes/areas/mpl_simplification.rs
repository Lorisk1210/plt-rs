//! Probe operations for the `mpl_simplification` area: matplotlib's own
//! non-image tests of path simplification and clipping (see
//! `spec/reference/probes/test_mpl_simplification.py`). The path ops are
//! the `path` area's, under the names the tests call.

use std::f64::consts::PI;

use pltrs::artist::ArtistHandle;
use pltrs::figure::Figure;
use pltrs::lines::JoinStyle;
use pltrs::mlab::{Complex, blackman, fft};
use pltrs::numeric::linspace;
use pltrs_core::Path;
use pltrs_core::path::convert::CleanupOptions;

use super::mpl_tests::in_test_style;
use super::path::{cleaned, iter_segments};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};
use crate::rng::SpecRng;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Path.cleaned", cleaned),
    Op::new("Path.iter_segments", iter_segments),
    Op::new("Line2D.simplified", line_simplified),
];

/// The x values (`None`: the indices), y values and line style of a
/// `Line2D.simplified` scene.
struct Scene {
    x: Option<Vec<f64>>,
    y: Vec<f64>,
    /// Drawn 2 pt wide with round joins, as the noise tests do.
    wide_round: bool,
}

impl Scene {
    fn of(args: Args<'_>) -> Result<Self, ProbeError> {
        let noise = || SpecRng::new(0).uniform(50_000, 0.0, 1.0);
        Ok(match args.str("scene")? {
            "antiparallel" => Self {
                x: Some(vec![0.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
                y: args.f64_vec("y")?,
                wide_round: false,
            },
            "noise" => Self {
                x: None,
                y: noise().into_iter().map(|u| u * 50.0).collect(),
                wide_round: true,
            },
            "sine_plus_noise" => Self {
                x: None,
                y: (linspace(0.0, PI * 2.0, 50_000).into_iter().zip(noise()))
                    .map(|(t, u)| t.sin() + u * 0.01)
                    .collect(),
                wide_round: true,
            },
            "fft_peaks" => Self {
                x: None,
                y: fft_peaks(),
                wide_round: false,
            },
            other => return Err(ProbeError::BadArgs(format!("unknown scene {other}"))),
        })
    }
}

/// `abs(fft(sin(2π·0.01·t) · blackman(65536)))` for `t` in `0..65536`.
fn fft_peaks() -> Vec<f64> {
    let signal: Vec<Complex> = blackman(65_536)
        .into_iter()
        .enumerate()
        .map(|(t, w)| Complex::new((2.0 * PI * 0.01 * t as f64).sin() * w, 0.0))
        .collect();
    fft(&signal).into_iter().map(Complex::abs).collect()
}

/// The display path of a scene's line cleaned with `simplify=True`: the
/// vertices but matplotlib's last two mapped back to data coordinates for
/// `antiparallel`, else the size of matplotlib's vertex array (which ends
/// with a `STOP` vertex).
fn line_simplified(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let scene = Scene::of(args)?;
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = match scene.x {
            Some(x) => ax.plot(x, scene.y).map_err(ProbeError::rejected)?,
            None => ax.plot_y(scene.y),
        };
        if scene.wide_round {
            line.set_solid_joinstyle(JoinStyle::Round)
                .set_linewidth(2.0);
        }
        let transform = line.transform().unwrap_or_else(|| ax.trans_data());
        let resolved = fig
            .resolve_transform(&transform)
            .map_err(ProbeError::rejected)?;
        let options = CleanupOptions {
            simplify: Some(pltrs::rc::params().path_simplify_threshold),
            ..CleanupOptions::default()
        };
        let path = line.path().map_err(ProbeError::rejected)?;
        let simplified = resolved.transform_path(&path).cleaned(&options);
        if args.str("scene")? != "antiparallel" {
            return Ok(Json::from((2 * (simplified.len() + 1)) as f64));
        }
        let back = resolved
            .inverted()
            .ok_or_else(|| ProbeError::rejected("singular data transform"))?
            .transform_path(&simplified);
        Ok(vertices_but_last(&back))
    })
}

/// The vertices of `path` but its last, as `[[x, y], ...]`: matplotlib's
/// `vertices[:-2]` of the same path with its trailing `STOP`.
fn vertices_but_last(path: &Path) -> Json {
    let vertices = path.vertices();
    Json::Array(
        vertices[..vertices.len().saturating_sub(1)]
            .iter()
            .map(|p| Json::Array(vec![Json::from(p.x), Json::from(p.y)]))
            .collect(),
    )
}
