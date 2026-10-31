//! matplotlib's `GraphicsContextBase` as pltrs (Rust)'s
//! `GraphicsContext`, for the renderers pltrs hands Python
//! (`RendererAgg.draw_path`).

use pltrs::colors::Color;
use pltrs::transforms::Bbox;
use pltrs_core::Rgba;
use pltrs_render::{
    CapStyle, ClipPath, Dashes, GraphicsContext, Hatch, JoinStyle, Sketch, SnapMode,
};

use super::super::path::PyPath;
use super::super::transforms::affine;
use crate::binding::prelude::*;

/// The stroke of a graphics context: `(rgb, forced_alpha, antialiased,
/// capstyle, joinstyle, linewidth)`.
type Stroke = (Color, Option<f64>, bool, String, String, f64);

/// The clipping of a graphics context: its rectangle's extents and its
/// path with the matrix of its transform.
type Clipping = (Option<(f64, f64, f64, f64)>, Option<(PyPath, NdArray)>);

/// The hatch of a graphics context: `(pattern, color, linewidth)`.
type HatchSpec = (Option<String>, Option<Color>, f64);

/// The rest of a graphics context: `(snap, sketch, url, gid)`.
type Extras = (
    Option<bool>,
    Option<(f64, f64, f64)>,
    Option<String>,
    Option<String>,
);

/// The dashes of a graphics context: `(dash_offset, dash_list)`.
type DashSpec = (f64, Option<Vec<f64>>);

/// A graphics context as `backend_agg._native_gc` describes it: `(stroke,
/// dashes, clipping, hatch, extras)` (see [`Stroke`], [`DashSpec`],
/// [`Clipping`], [`HatchSpec`] and [`Extras`]), the styles by name and
/// `None` for what is unset.
pub(super) struct PyGc(pub(super) GraphicsContext);

/// The colour `color` resolves to.
fn rgba(color: &Color) -> PyResult<Rgba> {
    color
        .resolve()
        .map_err(|err| PyErr::value_error(err.to_string()))
}

impl FromPy<'_> for PyGc {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (stroke, dashes, clipping, hatch, extras): (
            Stroke,
            DashSpec,
            Clipping,
            HatchSpec,
            Extras,
        ) = object.extract()?;
        let (rgb, alpha, antialiased, cap, join, linewidth) = stroke;
        let foreground = rgba(&rgb)?;
        let (clip_rect, clip_path) = clipping;
        let (pattern, hatch_color, hatch_linewidth) = hatch;
        let (snap, sketch, url, gid) = extras;
        let unknown =
            |kind: &str, name: &str| PyErr::value_error(format!("unknown {kind} {name:?}"));
        Ok(Self(GraphicsContext {
            foreground,
            alpha,
            antialiased,
            cap: CapStyle::from_name(&cap).ok_or_else(|| unknown("capstyle", &cap))?,
            join: JoinStyle::from_name(&join).ok_or_else(|| unknown("joinstyle", &join))?,
            linewidth,
            dashes: match dashes {
                (offset, Some(pattern)) => Dashes::new(offset, pattern),
                (_, None) => Dashes::solid(),
            },
            clip_rect: clip_rect.map(|(x0, y0, x1, y1)| Bbox::from_extents(x0, y0, x1, y1)),
            clip_path: clip_path
                .map(|(path, matrix)| {
                    Ok::<_, PyErr>(ClipPath {
                        path: path.path,
                        transform: affine(&matrix)?,
                    })
                })
                .transpose()?,
            hatch: pattern
                .map(|pattern| {
                    Ok::<_, PyErr>(Hatch {
                        pattern,
                        color: hatch_color.as_ref().map_or(Ok(foreground), rgba)?,
                        linewidth: hatch_linewidth,
                    })
                })
                .transpose()?,
            snap: match snap {
                None => SnapMode::Auto,
                Some(true) => SnapMode::Always,
                Some(false) => SnapMode::Never,
            },
            sketch: sketch.map(|(scale, length, randomness)| Sketch {
                scale,
                length,
                randomness,
            }),
            url,
            gid,
        }))
    }
}
