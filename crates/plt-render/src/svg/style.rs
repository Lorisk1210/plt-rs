//! CSS styles and `transform` attributes as matplotlib's SVG backend
//! writes them (`_get_style_dict`, `_generate_css`, `_generate_transform`).

#![expect(
    clippy::float_cmp,
    reason = "matplotlib's SVG writer leaves out exactly-default values"
)]

use plt_core::{Affine2D, Rgba};

use crate::gc::{CapStyle, GraphicsContext};
use crate::vector::short;

/// An ordered list of CSS properties.
pub(super) type Style = Vec<(&'static str, String)>;

/// `_generate_css`: `key: value` pairs joined by `"; "`.
pub(super) fn css(style: &[(&'static str, String)]) -> String {
    style
        .iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

/// One entry of a `transform` attribute.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum TransformOp {
    /// `translate(x y)`, left out when both are zero.
    Translate(f64, f64),
    /// `scale(s)`, left out when one.
    Scale(f64),
    /// `scale(sx sy)`, left out when both are one.
    ScaleXY(f64, f64),
    /// `rotate(a)`, left out when zero.
    Rotate(f64),
    /// `rotate(a x y)`: a rotation about `(x, y)`, always written.
    RotateAbout(f64, f64, f64),
    /// `matrix(a b c d e f)`.
    Matrix(Affine2D),
}

impl TransformOp {
    /// Whether `_generate_transform` leaves the entry out.
    fn is_identity(self) -> bool {
        match self {
            Self::Translate(x, y) => x == 0.0 && y == 0.0,
            Self::Scale(s) => s == 1.0,
            Self::ScaleXY(sx, sy) => sx == 1.0 && sy == 1.0,
            Self::Rotate(a) => a == 0.0,
            Self::RotateAbout(..) | Self::Matrix(_) => false,
        }
    }
}

/// `_generate_transform`.
pub(super) fn transform_attr(ops: &[TransformOp]) -> String {
    let mut parts = Vec::new();
    for op in ops.iter().filter(|op| !op.is_identity()) {
        let (name, values): (&str, Vec<f64>) = match *op {
            TransformOp::Translate(x, y) => ("translate", vec![x, y]),
            TransformOp::Scale(s) => ("scale", vec![s]),
            TransformOp::ScaleXY(sx, sy) => ("scale", vec![sx, sy]),
            TransformOp::Rotate(a) => ("rotate", vec![a]),
            TransformOp::RotateAbout(a, x, y) => ("rotate", vec![a, x, y]),
            TransformOp::Matrix(m) => ("matrix", m.values().to_vec()),
        };
        let values: Vec<String> = values.into_iter().map(short).collect();
        parts.push(format!("{name}({})", values.join(" ")));
    }
    parts.join(" ")
}

/// matplotlib's names for SVG line caps (`_capstyle_d`).
const fn linecap(cap: CapStyle) -> &'static str {
    match cap {
        CapStyle::Butt => "butt",
        CapStyle::Projecting => "square",
        CapStyle::Round => "round",
    }
}

/// `_get_style_dict`; `hatch` is the id of the face's hatch pattern.
pub(super) fn style_dict(gc: &GraphicsContext, face: Option<Rgba>, hatch: Option<&str>) -> Style {
    let mut style = Style::new();
    let forced_alpha = gc.alpha.is_some();
    let face_opacity = face.filter(|f| f.a != 1.0 && !forced_alpha);
    if let Some(hatch) = hatch {
        style.push(("fill", format!("url(#{hatch})")));
        if let Some(face) = face_opacity {
            style.push(("fill-opacity", short(face.a)));
        }
    } else {
        match face {
            None => style.push(("fill", "none".to_owned())),
            Some(face) => {
                if (face.r, face.g, face.b) != (0.0, 0.0, 0.0) {
                    style.push(("fill", face.to_hex(false)));
                }
                if let Some(face) = face_opacity {
                    style.push(("fill-opacity", short(face.a)));
                }
            }
        }
    }
    if let Some(alpha) = gc.alpha.filter(|&a| a != 1.0) {
        style.push(("opacity", short(alpha)));
    }
    if let Some(pattern) = &gc.dashes.pattern {
        let pattern: Vec<String> = pattern.iter().map(|&v| short(v)).collect();
        style.push(("stroke-dasharray", pattern.join(",")));
        style.push(("stroke-dashoffset", short(gc.dashes.offset)));
    }
    if gc.linewidth != 0.0 {
        let rgb = gc.stroke_color();
        style.push(("stroke", rgb.to_hex(false)));
        if !forced_alpha && rgb.a != 1.0 {
            style.push(("stroke-opacity", short(rgb.a)));
        }
        if gc.linewidth != 1.0 {
            style.push(("stroke-width", short(gc.linewidth)));
        }
        if gc.join.name() != "round" {
            style.push(("stroke-linejoin", gc.join.name().to_owned()));
        }
        if gc.cap != CapStyle::Butt {
            style.push(("stroke-linecap", linecap(gc.cap).to_owned()));
        }
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gc::JoinStyle;

    #[test]
    fn transforms_skip_identities() {
        let ops = [
            TransformOp::Translate(65.881_165, 322.181_656),
            TransformOp::Rotate(-0.0),
            TransformOp::ScaleXY(0.1, -0.1),
            TransformOp::Scale(1.0),
        ];
        assert_eq!(
            transform_attr(&ops),
            "translate(65.881165 322.181656) scale(0.1 -0.1)"
        );
        assert_eq!(transform_attr(&[TransformOp::Translate(0.0, -0.0)]), "");
        assert_eq!(
            transform_attr(&[TransformOp::Matrix(Affine2D::scaling(2.0, 1.0))]),
            "matrix(2 0 0 1 0 0)"
        );
    }

    #[test]
    fn styles_follow_matplotlib() {
        let gc = GraphicsContext {
            linewidth: 0.8,
            ..GraphicsContext::default()
        };
        assert_eq!(
            css(&style_dict(&gc, None, None)),
            "fill: none; stroke: #000000; stroke-width: 0.8"
        );
        let face = GraphicsContext {
            linewidth: 0.0,
            ..GraphicsContext::default()
        };
        assert_eq!(
            css(&style_dict(&face, Some(Rgba::WHITE), None)),
            "fill: #ffffff"
        );
        assert_eq!(css(&style_dict(&face, Some(Rgba::BLACK), None)), "");
        let fancy = GraphicsContext {
            alpha: Some(0.5),
            linewidth: 2.0,
            join: JoinStyle::Miter,
            cap: CapStyle::Projecting,
            dashes: crate::gc::Dashes::new(0.0, vec![3.7, 1.6]),
            ..GraphicsContext::default()
        };
        assert_eq!(
            css(&style_dict(
                &fancy,
                Some(Rgba::new(1.0, 0.0, 0.0, 0.3)),
                Some("h1")
            )),
            "fill: url(#h1); opacity: 0.5; stroke-dasharray: 3.7,1.6; \
             stroke-dashoffset: 0; stroke: #000000; stroke-width: 2; \
             stroke-linejoin: miter; stroke-linecap: square"
        );
    }
}
