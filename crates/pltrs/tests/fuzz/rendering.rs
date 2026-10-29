//! Rendering: figures of random paths (any codes, any coordinates, very
//! long), with dashes, hatches, path effects, sketches, clip paths and
//! rasterization, and texts at extreme sizes and rotations, drawn at
//! extreme figure sizes and resolutions to every save format.

use pltrs::artist::Sketch;
use pltrs::artist::{ArtistHandle, ClipPathSpec};
use pltrs::figure::{BboxInches, Figure, SaveFormat, SaveOptions};
use pltrs::lines::{CapStyle, JoinStyle, LineStyle};
use pltrs::patches::{PatchProps, PathShape};
use pltrs::patheffects::{GcUpdate, LineShadow, PatchShadow, PathEffect, Stroke, TickedStroke};
use pltrs::transforms::Point;
use pltrs_core::{Path, PathCode};

use crate::harness::{Gen, fuzz};
use crate::support::FORMATS;

/// A path as vertices and codes (not necessarily valid ones).
#[derive(Debug)]
struct PathSpec {
    vertices: Vec<Point>,
    codes: Vec<PathCode>,
}

/// A path effect.
#[derive(Debug)]
enum EffectSpec {
    Stroke(f64),
    PatchShadow(f64, f64),
    LineShadow(f64),
    Ticked(f64, f64, f64),
}

/// A patch: its path and how it is drawn.
#[derive(Debug)]
struct PatchSpec {
    path: PathSpec,
    linewidth: f64,
    dashes: Option<(f64, Vec<f64>)>,
    hatch: Option<String>,
    alpha: f64,
    zorder: f64,
    effects: Vec<EffectSpec>,
    sketch: Option<(f64, f64, f64)>,
    clip: Option<PathSpec>,
    rasterized: bool,
    fill: bool,
    caps: (CapStyle, JoinStyle),
}

/// A text at an extreme size or rotation.
#[derive(Debug)]
struct TextSpec {
    text: String,
    size: f64,
    rotation: f64,
    position: (f64, f64),
}

/// A figure and how it is saved.
#[derive(Debug)]
struct Scene {
    size: (f64, f64),
    dpi: f64,
    save_dpi: Option<f64>,
    tight: bool,
    transparent: bool,
    format: SaveFormat,
    patches: Vec<PatchSpec>,
    texts: Vec<TextSpec>,
}

/// A coordinate in the unit square around the axes mostly, else any.
fn coordinate(g: &mut Gen) -> f64 {
    g.number_in(-0.2, 1.2)
}

/// A path: well-formed segments most of the time, any codes otherwise,
/// sometimes very long.
fn path(g: &mut Gen) -> PathSpec {
    let segments = if g.chance(0.02) {
        2000 + g.below(8000)
    } else {
        g.len(12)
    };
    let mut vertices = Vec::new();
    let mut codes = Vec::new();
    let anarchic = g.chance(0.1);
    for i in 0..segments {
        let code = if anarchic {
            g.pick(&[
                PathCode::MoveTo,
                PathCode::LineTo,
                PathCode::Curve3,
                PathCode::Curve4,
                PathCode::ClosePoly,
            ])
        } else if i == 0 {
            PathCode::MoveTo
        } else {
            g.pick(&[
                PathCode::LineTo,
                PathCode::LineTo,
                PathCode::Curve3,
                PathCode::Curve4,
                PathCode::ClosePoly,
                PathCode::MoveTo,
            ])
        };
        let repeat = if anarchic { 1 } else { code.vertex_count() };
        for _ in 0..repeat {
            vertices.push(Point::new(coordinate(g), coordinate(g)));
            codes.push(code);
        }
    }
    PathSpec { vertices, codes }
}

/// Dashes: ordinary, tiny, huge, zero, negative or NaN.
fn dashes(g: &mut Gen) -> Option<(f64, Vec<f64>)> {
    g.chance(0.3).then(|| {
        let count = g.len(6);
        let pattern = (0..count).map(|_| g.number_in(0.0, 10.0)).collect();
        (g.number_in(0.0, 5.0), pattern)
    })
}

/// A path effect with random parameters.
fn effect(g: &mut Gen) -> EffectSpec {
    match g.below(4) {
        0 => EffectSpec::Stroke(g.number_in(0.0, 10.0)),
        1 => EffectSpec::PatchShadow(g.number_in(-5.0, 5.0), g.number_in(0.0, 1.0)),
        2 => EffectSpec::LineShadow(g.number_in(-5.0, 5.0)),
        _ => EffectSpec::Ticked(
            g.number_in(0.1, 20.0),
            g.number_in(-180.0, 180.0),
            g.number_in(0.0, 3.0),
        ),
    }
}

/// A patch with random styling.
fn patch(g: &mut Gen) -> PatchSpec {
    let hatches = [
        "/", "\\\\", "|", "-", "+", "x", "o", "O", ".", "*", "////////", "xo*",
    ];
    PatchSpec {
        path: path(g),
        linewidth: g.number_in(0.0, 5.0),
        dashes: dashes(g),
        hatch: g.chance(0.3).then(|| g.text(&hatches, &hatches)),
        alpha: g.number_in(0.0, 1.0),
        zorder: g.number_in(-5.0, 5.0),
        effects: (0..g.len(3)).map(|_| effect(g)).collect(),
        sketch: g.chance(0.15).then(|| {
            (
                g.number_in(0.0, 5.0),
                g.number_in(0.0, 200.0),
                g.number_in(0.0, 5.0),
            )
        }),
        clip: g.chance(0.2).then(|| path(g)),
        rasterized: g.chance(0.15),
        fill: g.chance(0.6),
        caps: (
            g.pick(&[CapStyle::Butt, CapStyle::Round, CapStyle::Projecting]),
            g.pick(&[JoinStyle::Miter, JoinStyle::Round, JoinStyle::Bevel]),
        ),
    }
}

/// A text with an extreme size or rotation.
fn text(g: &mut Gen) -> TextSpec {
    let texts = ["Ag", "$\\frac{a}{b}$", "", "\n\n", "\u{1F600}x\u{301}"];
    let text = match g.below(8) {
        0 => "long text ".repeat(1 + g.below(60)),
        _ => g.pick(&texts).to_owned(),
    };
    TextSpec {
        text,
        size: g.number_in(0.0, 200.0),
        rotation: g.number_in(-720.0, 720.0),
        position: (coordinate(g), coordinate(g)),
    }
}

/// A scene: sizes and resolutions extreme a third of the time.
fn scene(g: &mut Gen) -> Scene {
    let extreme = g.chance(0.33);
    let length = |g: &mut Gen| {
        if extreme {
            g.number()
        } else {
            g.between(0.5, 3.0)
        }
    };
    let dpi = if extreme {
        g.number()
    } else {
        g.between(10.0, 80.0)
    };
    Scene {
        size: (length(g), length(g)),
        dpi,
        save_dpi: g.chance(0.2).then(|| g.number_in(1.0, 100.0)),
        tight: g.chance(0.2),
        transparent: g.chance(0.2),
        format: g.pick(&FORMATS),
        patches: (0..g.len(5)).map(|_| patch(g)).collect(),
        texts: (0..g.len(3)).map(|_| text(g)).collect(),
    }
}

/// The path of a spec, if its codes are valid.
fn build(spec: &PathSpec) -> Option<Path> {
    Path::new(spec.vertices.clone(), spec.codes.clone()).ok()
}

/// The effects of the specs.
fn effects(specs: &[EffectSpec]) -> Vec<PathEffect> {
    specs
        .iter()
        .map(|spec| match *spec {
            EffectSpec::Stroke(width) => {
                PathEffect::with_stroke(Stroke::new(GcUpdate::default().linewidth(width)))
            }
            EffectSpec::PatchShadow(offset, alpha) => {
                PathEffect::with_simple_patch_shadow(PatchShadow {
                    offset: (offset, -offset),
                    alpha,
                    ..PatchShadow::default()
                })
            }
            EffectSpec::LineShadow(offset) => PathEffect::simple_line_shadow(LineShadow {
                offset: (offset, offset),
                ..LineShadow::default()
            }),
            EffectSpec::Ticked(spacing, angle, length) => {
                PathEffect::with_ticked_stroke(TickedStroke {
                    spacing,
                    angle,
                    length,
                    ..TickedStroke::default()
                })
            }
        })
        .collect()
}

/// Draws the scene and saves it.
fn draw(scene: &Scene) {
    let fig = Figure::with_size(scene.size.0, scene.size.1, scene.dpi);
    let ax = fig.subplot();
    for spec in &scene.patches {
        let Some(path) = build(&spec.path) else {
            continue;
        };
        let mut props = PatchProps::default()
            .linewidth(spec.linewidth)
            .fill(spec.fill)
            .capstyle(spec.caps.0)
            .joinstyle(spec.caps.1);
        props.base.alpha = Some(spec.alpha);
        props.base.zorder = Some(spec.zorder);
        props.hatch.clone_from(&spec.hatch);
        if let Some((offset, pattern)) = &spec.dashes {
            props = props.linestyle(LineStyle::dashes(*offset, pattern.clone()));
        }
        let patch = ax.add_patch_with(PathShape(path), props);
        patch.set_path_effects(effects(&spec.effects));
        patch.set_rasterized(spec.rasterized);
        if let Some((scale, length, randomness)) = spec.sketch {
            patch.set_sketch_params(Some(Sketch {
                scale,
                length,
                randomness,
            }));
        }
        if let Some(clip) = spec.clip.as_ref().and_then(build) {
            patch.set_clip_path(Some(ClipPathSpec::Path {
                path: clip,
                transform: ax.trans_data(),
            }));
        }
    }
    for spec in &scene.texts {
        let text = ax.text(spec.position.0, spec.position.1, spec.text.as_str());
        text.set_fontsize(spec.size).set_rotation(spec.rotation);
    }
    let options = SaveOptions {
        dpi: scene.save_dpi,
        bbox_inches: scene.tight.then_some(BboxInches::Tight),
        transparent: Some(scene.transparent),
        ..SaveOptions::default()
    };
    let _ = fig.to_format(scene.format, options);
}

#[test]
fn random_scenes() {
    fuzz("rendering", 100, scene, draw);
}
