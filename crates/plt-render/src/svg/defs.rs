//! Definitions referenced by drawn elements and written once at the end of
//! the document: clip paths and hatch patterns (`_get_clip_attrs`,
//! `_write_clips`, `_get_hatch`, `_write_hatches`).

use plt_core::path::convert::{StringCodes, StringOptions};
use plt_core::{Affine2D, Path, Rgba};

use super::SvgRenderer;
use super::style::{Style, css};
use super::xml::XmlWriter;
use crate::gc::GraphicsContext;
use crate::vector::{hex_color, repr, repr_str, short};

/// What a clip definition contains.
#[derive(Clone, Debug, PartialEq)]
enum ClipShape {
    /// A clip path and its transform to (flipped) SVG coordinates.
    Path(Path, Affine2D),
    /// A clip rectangle `(x, y, width, height)` in SVG coordinates.
    Rect([f64; 4]),
}

/// The identity of a clip definition (matplotlib's `dictkey`).
#[derive(Clone, Debug, PartialEq)]
enum ClipKey {
    Path(usize, Affine2D),
    Rect([f64; 4]),
}

/// Clip definitions in first-use order.
#[derive(Debug, Default)]
pub(super) struct Clips {
    /// Distinct clip paths; the index is matplotlib's clip path id.
    paths: Vec<Path>,
    entries: Vec<(ClipKey, ClipShape, String)>,
}

/// One hatch pattern: the pattern, face, hatch colour and line width.
#[derive(Clone, Debug, PartialEq)]
struct HatchKey {
    pattern: String,
    face: Option<Rgba>,
    color: Rgba,
    linewidth: f64,
}

/// Hatch patterns in first-use order.
#[derive(Debug, Default)]
pub(super) struct Hatches {
    entries: Vec<(HatchKey, String)>,
}

/// The side of a hatch tile in points.
const HATCH_SIZE: f64 = 72.0;

/// Python's `str` of a colour tuple.
fn color_repr(color: Rgba) -> String {
    format!(
        "({}, {}, {}, {})",
        repr(color.r),
        repr(color.g),
        repr(color.b),
        repr(color.a)
    )
}

/// `_convert_path` without clipping or simplification.
pub(super) fn path_data(path: &Path, transform: Affine2D) -> String {
    path.convert_to_string(&StringOptions {
        transform,
        codes: StringCodes::SVG,
        ..StringOptions::default()
    })
}

impl SvgRenderer {
    /// `_get_clip_attrs`: the `clip-path` attribute for `gc` (a clip path
    /// takes precedence over the clip rectangle), registering the
    /// definition on first use.
    pub(super) fn clip_attribute(
        &mut self,
        gc: &GraphicsContext,
    ) -> Option<(&'static str, String)> {
        let clip_path = gc
            .clip_path
            .as_ref()
            .filter(|clip| clip.path.vertices().iter().all(|p| p.is_finite()));
        let (key, shape, content) = if let Some(clip) = clip_path {
            let flipped = clip.transform.then(self.flip());
            let known = self.clips.paths.iter().position(|p| *p == clip.path);
            let index = known.unwrap_or_else(|| {
                self.clips.paths.push(clip.path.clone());
                self.clips.paths.len() - 1
            });
            let values: Vec<String> = flipped.values().iter().map(|&v| repr(v)).collect();
            (
                ClipKey::Path(index, flipped),
                ClipShape::Path(clip.path.clone(), flipped),
                format!("({index}, '{}')", values.join(" ")),
            )
        } else {
            let rect = gc.clip_rect?;
            let (x, y, w, h) = rect.bounds();
            let values = [x, self.height - (y + h), w, h];
            let content: Vec<String> = values
                .iter()
                .map(|&v| format!("np.float64({})", repr(v)))
                .collect();
            (
                ClipKey::Rect(values),
                ClipShape::Rect(values),
                format!("({})", content.join(", ")),
            )
        };
        let known = self.clips.entries.iter().find(|(k, _, _)| *k == key);
        let id = if let Some((_, _, id)) = known {
            id.clone()
        } else {
            let id = self.make_id("p", &content);
            self.clips.entries.push((key, shape, id.clone()));
            id
        };
        Some(("clip-path", format!("url(#{id})")))
    }

    /// `_get_hatch`: the id of the hatch pattern filling `face` with the
    /// hatch of `gc`, if it has one.
    pub(super) fn hatch_id(&mut self, gc: &GraphicsContext, face: Option<Rgba>) -> Option<String> {
        let hatch = gc.hatch.as_ref()?;
        let key = HatchKey {
            pattern: hatch.pattern.clone(),
            face,
            color: hatch.color,
            linewidth: hatch.linewidth,
        };
        if let Some((_, id)) = self.hatches.entries.iter().find(|(k, _)| *k == key) {
            return Some(id.clone());
        }
        let content = format!(
            "({}, {}, {}, {})",
            repr_str(&key.pattern),
            face.map_or_else(|| "None".to_owned(), color_repr),
            color_repr(key.color),
            repr(key.linewidth)
        );
        let id = self.make_id("h", &content);
        self.hatches.entries.push((key, id.clone()));
        Some(id)
    }
}

impl Clips {
    /// `_write_clips`.
    pub(super) fn write(&self, writer: &mut XmlWriter) {
        if self.entries.is_empty() {
            return;
        }
        writer.start("defs", &[]);
        for (_, shape, id) in &self.entries {
            writer.start("clipPath", &[("id", id.clone())]);
            match shape {
                ClipShape::Path(path, transform) => {
                    writer.element("path", "", &[("d", path_data(path, *transform))]);
                }
                ClipShape::Rect([x, y, w, h]) => writer.element(
                    "rect",
                    "",
                    &[
                        ("x", short(*x)),
                        ("y", short(*y)),
                        ("width", short(*w)),
                        ("height", short(*h)),
                    ],
                ),
            }
            writer.end(true);
        }
        writer.end(true);
    }
}

impl Hatches {
    /// `_write_hatches`: one 72 × 72 pattern tile per hatch.
    pub(super) fn write(&self, writer: &mut XmlWriter) {
        if self.entries.is_empty() {
            return;
        }
        let size = HATCH_SIZE.to_string();
        writer.start("defs", &[]);
        for (key, id) in &self.entries {
            writer.start(
                "pattern",
                &[
                    ("id", id.clone()),
                    ("patternUnits", "userSpaceOnUse".to_owned()),
                    ("x", "0".to_owned()),
                    ("y", "0".to_owned()),
                    ("width", size.clone()),
                    ("height", size.clone()),
                ],
            );
            let tile = Affine2D::scaling(HATCH_SIZE, HATCH_SIZE)
                .scale(1.0, -1.0)
                .translate(0.0, HATCH_SIZE);
            let path =
                plt_core::hatch::pattern_path(&key.pattern, plt_core::hatch::DEFAULT_DENSITY)
                    .unwrap_or_default();
            let data = path_data(&path, tile);
            let fill = key.face.map_or_else(|| "none".to_owned(), hex_color);
            let edge = (HATCH_SIZE + 1.0).to_string();
            writer.element(
                "rect",
                "",
                &[
                    ("x", "0".to_owned()),
                    ("y", "0".to_owned()),
                    ("width", edge.clone()),
                    ("height", edge),
                    ("fill", fill),
                ],
            );
            let mut style: Style = vec![
                ("fill", hex_color(key.color)),
                ("stroke", hex_color(key.color)),
                ("stroke-width", repr(key.linewidth)),
                ("stroke-linecap", "butt".to_owned()),
                ("stroke-linejoin", "miter".to_owned()),
            ];
            if key.color.a < 1.0 {
                style.push(("stroke-opacity", repr(key.color.a)));
            }
            writer.element("path", "", &[("d", data), ("style", css(&style))]);
            writer.end(true);
        }
        writer.end(true);
    }
}
