//! Shared helpers for the matplotlib reference tests: the case corpus
//! (`fixtures/cases.txt`), the reference images (`fixtures/agg_reference.bin`)
//! and the deterministic test images both sides use.

pub(crate) mod backend;

use std::collections::HashMap;

use plt_core::{Path, PathBuilder, Point, Rgba};
use plt_raster::{Mask, Pixmap};

/// One drawing operation of a case.
#[derive(Debug)]
pub(crate) struct Op {
    /// `fill`, `stroke`, `hatch`, `markers`, `gouraud`, `image` or `text`.
    pub(crate) kind: String,
    /// The path the op draws, if any.
    pub(crate) target: Option<String>,
    /// `key=value` arguments.
    pub(crate) args: HashMap<String, String>,
}

impl Op {
    pub(crate) fn arg(&self, key: &str) -> Option<&str> {
        self.args.get(key).map(String::as_str)
    }

    pub(crate) fn floats(&self, key: &str) -> Option<Vec<f64>> {
        self.arg(key).map(parse_floats)
    }

    pub(crate) fn color(&self, key: &str) -> Option<Rgba> {
        self.floats(key).map(|v| Rgba::new(v[0], v[1], v[2], v[3]))
    }

    pub(crate) fn number(&self, key: &str, default: f64) -> f64 {
        self.arg(key)
            .map_or(default, |v| v.parse().expect("number"))
    }
}

/// A reference case.
#[derive(Debug)]
pub(crate) struct Case {
    pub(crate) name: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) paths: HashMap<String, Path>,
    pub(crate) ops: Vec<Op>,
}

pub(crate) fn parse_floats(text: &str) -> Vec<f64> {
    text.split(',').map(|v| v.parse().expect("float")).collect()
}

/// Parses `M x y L x y Q .. C .. Z` tokens.
pub(crate) fn parse_path(tokens: &[&str]) -> Path {
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
        let cmd = tokens[i];
        i += 1;
        b = match cmd {
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

/// Parses `fixtures/cases.txt`.
pub(crate) fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    let mut current: Option<Case> = None;
    for raw in include_str!("../fixtures/cases.txt").lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        match tokens[0] {
            "case" => {
                current = Some(Case {
                    name: tokens[1].to_owned(),
                    width: tokens[2].parse().expect("width"),
                    height: tokens[3].parse().expect("height"),
                    paths: HashMap::new(),
                    ops: Vec::new(),
                });
            }
            "end" => cases.push(current.take().expect("case before end")),
            "path" => {
                let case = current.as_mut().expect("path inside case");
                case.paths
                    .insert(tokens[1].to_owned(), parse_path(&tokens[2..]));
            }
            kind => {
                let case = current.as_mut().expect("op inside case");
                let rest = &tokens[1..];
                let target = rest
                    .first()
                    .filter(|t| !t.contains('='))
                    .map(|t| (*t).to_owned());
                let args = rest
                    .iter()
                    .filter_map(|t| t.split_once('='))
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect();
                case.ops.push(Op {
                    kind: kind.to_owned(),
                    target,
                    args,
                });
            }
        }
    }
    cases
}

/// Decodes `fixtures/agg_reference.bin` into named pixmaps.
pub(crate) fn references() -> HashMap<String, Pixmap> {
    let bytes: &[u8] = include_bytes!("../fixtures/agg_reference.bin");
    assert_eq!(&bytes[..8], b"PLTAGG1\n", "bad reference header");
    let mut pos = 8;
    let mut take = |n: usize| {
        let s = &bytes[pos..pos + n];
        pos += n;
        s
    };
    let u32_at = |s: &[u8]| u32::from_le_bytes(s.try_into().expect("u32"));
    let count = u32_at(take(4));
    let mut out = HashMap::new();
    for _ in 0..count {
        let name_len = u16::from_le_bytes(take(2).try_into().expect("u16")) as usize;
        let name = String::from_utf8(take(name_len).to_vec()).expect("utf-8 name");
        let width = u32_at(take(4));
        let height = u32_at(take(4));
        let len = u32_at(take(4)) as usize;
        let mut data = Vec::with_capacity(width as usize * height as usize * 4);
        for &[count, r, g, b, a] in take(len).as_chunks::<5>().0 {
            for _ in 0..count {
                data.extend_from_slice(&[r, g, b, a]);
            }
        }
        out.insert(
            name,
            Pixmap::from_vec(width, height, data).expect("reference size"),
        );
    }
    out
}

/// The deterministic RGBA test image (mirrors `image_pixels` in generate.py).
pub(crate) fn test_image(width: u32, height: u32) -> Pixmap {
    let mut data = Vec::new();
    for j in 0..height {
        for i in 0..width {
            data.extend(
                [
                    (i * 37 + j * 11) % 256,
                    (i * 5 + j * 53 + 7) % 256,
                    (i * i + j * 3) % 256,
                    (i * 19 + j * 29 + 40) % 256,
                ]
                .map(|v| v as u8),
            );
        }
    }
    Pixmap::from_vec(width, height, data).expect("image size")
}

/// The deterministic coverage bitmap (mirrors `coverage_pixels`).
pub(crate) fn test_coverage(width: u32, height: u32) -> Mask {
    let data = (0..height)
        .flat_map(|j| (0..width).map(move |i| ((i * 47 + j * 31 + i * j * 3) % 256) as u8))
        .collect();
    Mask::from_vec(width, height, data).expect("coverage size")
}
