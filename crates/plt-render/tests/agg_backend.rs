//! Pixel comparison of `RasterRenderer` against matplotlib's `RendererAgg`.
//!
//! `fixtures/backend_cases.txt` describes renderer-level calls;
//! `fixtures/generate.py` made the same calls on matplotlib 3.11's
//! `RendererAgg` and saved `fixtures/agg_backend/<case>.png`. Every case
//! must match exactly unless listed in [`TOLERANCES`]. Set
//! `DUMP=<case name>` and run with `--nocapture` to print differing pixels.

use std::collections::HashMap;
use std::path::PathBuf;

use plt_core::{Affine2D, Bbox, Path, PathBuilder, Point, Rgba};
use plt_render::{
    CapStyle, ClipPath, Dashes, GraphicsContext, Hatch, JoinStyle, PathCollection, RasterRenderer,
    Renderer, RgbaImage, Sketch, SnapMode,
};

/// Cases allowed to differ: name, maximum channel difference, reason.
const TOLERANCES: &[(&str, u8, &str)] = &[];

fn floats(text: &str) -> Vec<f64> {
    text.split(',')
        .map(|v| v.parse().expect("number"))
        .collect()
}

fn color(text: &str) -> Rgba {
    let v = floats(text);
    Rgba::new(v[0], v[1], v[2], v[3])
}

fn colors(text: &str) -> Vec<Rgba> {
    text.split(';').map(color).collect()
}

fn parse_path(tokens: &[&str]) -> Path {
    let mut b = PathBuilder::default();
    let mut i = 0;
    let next = |i: &mut usize| {
        let p = Point::new(
            tokens[*i].parse().expect("x"),
            tokens[*i + 1].parse().expect("y"),
        );
        *i += 2;
        p
    };
    while i < tokens.len() {
        i += 1;
        b = match tokens[i - 1] {
            "M" => b.move_to(next(&mut i)),
            "L" => b.line_to(next(&mut i)),
            "Q" => {
                let c = next(&mut i);
                b.quad_to(c, next(&mut i))
            }
            "C" => {
                let c1 = next(&mut i);
                let c2 = next(&mut i);
                b.cubic_to(c1, c2, next(&mut i))
            }
            "Z" => b.close(),
            other => panic!("unknown path command {other}"),
        };
    }
    b.build()
}

/// The deterministic test image (mirrors `image_pixels` in generate.py).
fn test_image(width: usize, height: usize) -> RgbaImage {
    let mut data = Vec::with_capacity(width * height * 4);
    for j in 0..height {
        for i in 0..width {
            data.extend(
                [
                    (i * 37 + j * 11),
                    (i * 5 + j * 53 + 7),
                    (i * i + j * 3),
                    (i * 19 + j * 29 + 40),
                ]
                .map(|v| (v % 256) as u8),
            );
        }
    }
    RgbaImage::new(width, height, data).expect("image size")
}

/// One case being replayed.
struct Case {
    name: String,
    renderer: RasterRenderer,
    paths: HashMap<String, Path>,
    affines: HashMap<String, Affine2D>,
}

impl Case {
    fn affine(&self, args: &HashMap<&str, &str>, key: &str) -> Affine2D {
        args.get(key)
            .map_or_else(Affine2D::identity, |id| self.affines[*id])
    }

    /// The graphics context `generate.py` configures from the same keys.
    fn gc(&self, args: &HashMap<&str, &str>) -> GraphicsContext {
        let arg = |key: &str| args.get(key).copied();
        GraphicsContext {
            foreground: arg("color").map_or(Rgba::BLACK, color),
            alpha: arg("alpha").map(|a| a.parse().expect("alpha")),
            antialiased: arg("aa") != Some("0"),
            cap: arg("cap").map_or(CapStyle::Butt, |c| CapStyle::from_name(c).expect("cap")),
            join: arg("join").map_or(JoinStyle::Round, |j| JoinStyle::from_name(j).expect("join")),
            linewidth: arg("lw").map_or(1.0, |w| w.parse().expect("lw")),
            dashes: arg("dashes").map_or_else(Dashes::solid, |spec| {
                let (offset, lengths) = spec.split_once(':').expect("offset:lengths");
                Dashes {
                    offset: offset.parse().expect("offset"),
                    pattern: floats(lengths),
                }
            }),
            clip_rect: arg("clip").map(|v| {
                let v = floats(v);
                Bbox::from_extents(v[0], v[1], v[2], v[3])
            }),
            clip_path: arg("clippath").map(|id| ClipPath {
                path: self.paths[id].clone(),
                transform: self.affine(args, "clipt"),
            }),
            hatch: arg("hatch").map(|pattern| Hatch {
                pattern: pattern.to_owned(),
                color: color(arg("hcolor").expect("hatch colour")),
                linewidth: arg("hlw").expect("hatch width").parse().expect("hlw"),
            }),
            snap: match arg("snap") {
                Some("true") => SnapMode::Always,
                Some("false") => SnapMode::Never,
                _ => SnapMode::Auto,
            },
            sketch: arg("sketch").map(|v| {
                let v = floats(v);
                Sketch {
                    scale: v[0],
                    length: v[1],
                    randomness: v[2],
                }
            }),
            ..GraphicsContext::default()
        }
    }

    fn run(&mut self, op: &str, targets: &[&str], args: &HashMap<&str, &str>) {
        let gc = self.gc(args);
        let face = args.get("face").map(|f| color(f));
        match op {
            "draw_path" => {
                let path = &self.paths[targets[0]];
                let transform = self.affine(args, "transform");
                self.renderer.draw_path(&gc, path, &transform, face);
            }
            "draw_markers" => {
                let (marker, positions) = (&self.paths[targets[0]], &self.paths[targets[1]]);
                let marker_transform = self.affine(args, "marker_transform");
                let transform = self.affine(args, "transform");
                self.renderer.draw_markers(
                    &gc,
                    marker,
                    &marker_transform,
                    positions,
                    &transform,
                    face,
                );
            }
            "collection" => self.collection(&gc, args),
            "gouraud" => {
                let triangles: Vec<[Point; 3]> = args["tris"]
                    .split(';')
                    .map(|t| {
                        let v = floats(t);
                        [0, 1, 2].map(|k| Point::new(v[2 * k], v[2 * k + 1]))
                    })
                    .collect();
                let corner_colors: Vec<[Rgba; 3]> = args["colors"]
                    .split(';')
                    .map(|c| {
                        let v = floats(c);
                        [0, 1, 2]
                            .map(|k| Rgba::new(v[4 * k], v[4 * k + 1], v[4 * k + 2], v[4 * k + 3]))
                    })
                    .collect();
                let transform = self.affine(args, "transform");
                self.renderer
                    .draw_gouraud_triangles(&gc, &triangles, &corner_colors, &transform);
            }
            "image" => {
                let at = floats(args["at"]);
                let size = floats(args["size"]);
                let image = test_image(size[0] as usize, size[1] as usize);
                self.renderer
                    .draw_image(&gc, Point::new(at[0], at[1]), &image, None);
            }
            other => panic!("unknown op {other}"),
        }
    }

    fn collection(&mut self, gc: &GraphicsContext, args: &HashMap<&str, &str>) {
        let paths: Vec<Path> = args["paths"]
            .split(',')
            .map(|p| self.paths[p].clone())
            .collect();
        let transforms: Vec<Affine2D> = args
            .get("transforms")
            .map(|t| t.split(',').map(|id| self.affines[id]).collect())
            .unwrap_or_default();
        let offsets: Vec<Point> = args
            .get("offsets")
            .map(|o| {
                floats(o)
                    .chunks(2)
                    .map(|c| Point::new(c[0], c[1]))
                    .collect()
            })
            .unwrap_or_default();
        let faces = args.get("faces").map(|f| colors(f)).unwrap_or_default();
        let edges = args.get("edges").map(|e| colors(e)).unwrap_or_default();
        let widths = floats(args.get("lws").unwrap_or(&"1"));
        let antialiased: Vec<bool> = args
            .get("aa")
            .unwrap_or(&"1")
            .split(',')
            .map(|v| v == "1")
            .collect();
        let collection = PathCollection {
            master_transform: self.affine(args, "master"),
            paths: &paths,
            path_transforms: &transforms,
            offsets: &offsets,
            facecolors: &faces,
            edgecolors: &edges,
            linewidths: &widths,
            antialiased: &antialiased,
            ..PathCollection::default()
        };
        self.renderer.draw_path_collection(gc, &collection);
    }
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Replays every case of `backend_cases.txt`.
fn cases() -> Vec<Case> {
    let text = std::fs::read_to_string(fixtures().join("backend_cases.txt")).expect("case file");
    let mut done = Vec::new();
    let mut current: Option<Case> = None;
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        match tokens[0] {
            "case" => {
                let n = |i: usize| tokens[i].parse::<f64>().expect("size");
                current = Some(Case {
                    name: tokens[1].to_owned(),
                    renderer: RasterRenderer::new(n(2), n(3), n(4)).expect("canvas"),
                    paths: HashMap::new(),
                    affines: HashMap::new(),
                });
            }
            "end" => done.push(current.take().expect("case before end")),
            "path" => {
                let case = current.as_mut().expect("path inside case");
                case.paths
                    .insert(tokens[1].to_owned(), parse_path(&tokens[2..]));
            }
            "affine" => {
                let case = current.as_mut().expect("affine inside case");
                let v: Vec<f64> = tokens[2..]
                    .iter()
                    .map(|t| t.parse().expect("coefficient"))
                    .collect();
                let affine = Affine2D::from_values(v[0], v[1], v[2], v[3], v[4], v[5]);
                case.affines.insert(tokens[1].to_owned(), affine);
            }
            op => {
                let case = current.as_mut().expect("op inside case");
                let targets: Vec<&str> = tokens[1..]
                    .iter()
                    .copied()
                    .filter(|t| !t.contains('='))
                    .collect();
                let args = tokens[1..]
                    .iter()
                    .filter_map(|t| t.split_once('='))
                    .collect();
                case.run(op, &targets, &args);
            }
        }
    }
    done
}

#[test]
fn matches_matplotlib_renderer_agg() {
    let mut failures = Vec::new();
    for mut case in cases() {
        let png = std::fs::read(fixtures().join(format!("agg_backend/{}.png", case.name)))
            .expect("reference");
        let expected = plt_codec::png::decode(&png)
            .expect("decodable reference")
            .to_rgba8();
        assert_eq!(
            case.renderer.take_errors(),
            [],
            "{}: raster errors",
            case.name
        );
        let actual = case.renderer.pixmap().data();
        assert_eq!(actual.len(), expected.len(), "{}: canvas size", case.name);
        let width = case.renderer.pixmap().width() as usize;
        let (mut max, mut pixels) = (0u8, 0usize);
        for (i, (a, e)) in actual.chunks(4).zip(expected.chunks(4)).enumerate() {
            let worst = a
                .iter()
                .zip(e)
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
            if worst > 0 {
                pixels += 1;
                max = max.max(worst);
                if std::env::var("DUMP").ok().as_deref() == Some(case.name.as_str()) {
                    println!(
                        "  ({}, {}) actual {a:?} expected {e:?}",
                        i % width,
                        i / width
                    );
                }
            }
        }
        let allowed = TOLERANCES
            .iter()
            .find(|t| t.0 == case.name)
            .map_or(0, |t| t.1);
        println!(
            "{:<30} max diff {max:>3}, {pixels:>5} pixels differ",
            case.name
        );
        if max > allowed {
            failures.push(format!(
                "{}: max diff {max} > {allowed} ({pixels} pixels)",
                case.name
            ));
        }
    }
    assert!(failures.is_empty(), "mismatches:\n{}", failures.join("\n"));
}
