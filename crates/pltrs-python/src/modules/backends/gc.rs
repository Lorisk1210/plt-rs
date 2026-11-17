//! matplotlib's `GraphicsContextBase` as pltrs (Rust)'s
//! `GraphicsContext`, both ways: from the drawing methods of the renderers
//! pltrs hands Python (`_native.drawing`), and to the Python code pltrs
//! (Rust) calls while it draws (path effects and renderers of the
//! program's).

use pltrs::colors::Color;
use pltrs::transforms::Bbox;
use pltrs_core::Rgba;
use pltrs_render::{
    CapStyle, ClipPath, Dashes, GraphicsContext, Hatch, JoinStyle, Sketch, SnapMode,
};

use super::super::path::{PyPath, path_arrays};
use super::super::transforms::{affine, matrix};
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
pub(in crate::modules) struct PyGc(pub(in crate::modules) GraphicsContext);

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
                .map(|(path, matrix)| Ok::<_, PyErr>(ClipPath::new(path.path, affine(&matrix)?)))
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

/// The colour `rgba` as an `(r, g, b, a)` tuple.
fn rgba_tuple(rgba: Rgba) -> (f64, f64, f64, f64) {
    (rgba.r, rgba.g, rgba.b, rgba.a)
}

/// A graphics context for Python, as `_native_gc` describes one: what
/// `backends._native_drawing.gc_from_native` fills a
/// `GraphicsContextBase` from (a path effect of the program's, a
/// renderer of the program's), the colours as RGBA tuples and the clip
/// path as its `(vertices, codes)` arrays.
impl IntoPy for PyGc {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let gc = self.0;
        let clip_path = gc
            .clip_path
            .map(|clip| Ok::<_, PyErr>((path_arrays(&clip.path, false)?, matrix(clip.transform)?)))
            .transpose()?;
        let (pattern, hatch_color, hatch_linewidth) = match gc.hatch {
            Some(hatch) => (
                Some(hatch.pattern),
                Some(rgba_tuple(hatch.color)),
                Some(hatch.linewidth),
            ),
            None => (None, None, None),
        };
        let snap = match gc.snap {
            SnapMode::Auto => None,
            SnapMode::Always => Some(true),
            SnapMode::Never => Some(false),
        };
        (
            (
                rgba_tuple(gc.foreground),
                gc.alpha,
                gc.antialiased,
                gc.cap.name(),
                gc.join.name(),
                gc.linewidth,
            ),
            (gc.dashes.offset, gc.dashes.pattern),
            (
                gc.clip_rect
                    .map(|clip| (clip.x0(), clip.y0(), clip.x1(), clip.y1())),
                clip_path,
            ),
            (pattern, hatch_color, hatch_linewidth),
            (
                snap,
                gc.sketch
                    .map(|sketch| (sketch.scale, sketch.length, sketch.randomness)),
                gc.url,
                gc.gid,
            ),
        )
            .into_py(py)
    }
}
