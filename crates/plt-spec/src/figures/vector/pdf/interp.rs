//! A content-stream interpreter producing one item per painting
//! operation with the graphics state that affects it, so that documents
//! setting the same state with different operators compare equal.
//!
//! Paths are kept in device coordinates (transformed by the current
//! matrix), form `XObject`s run in place, text is reduced to the glyphs
//! shown and their device positions, and clip regions travel with every
//! painted item.

use std::collections::HashMap;

use super::samples::{image_tokens, shading_tokens};
use super::syntax::{Document, Value, operations};
use super::{base_name, value_tokens};
use crate::figures::vector::diff::{Item, Token};
use crate::figures::vector::postscript::{IDENTITY, Matrix, apply, concat};

fn numbers(operands: &[Value]) -> Vec<f64> {
    operands.iter().filter_map(Value::as_num).collect()
}

/// A Type 3 or Type 0 font as text placement needs it.
#[derive(Clone, Debug, Default)]
struct Font {
    name: String,
    glyphs: HashMap<u32, String>,
    widths: HashMap<u32, f64>,
    /// Horizontal scale of the font matrix.
    scale: f64,
    /// Bytes per character code (2 for `Identity-H` Type 0 fonts).
    code_bytes: usize,
}

/// The widths of a CID font's `W` array (`c [w1 w2 ...]` and
/// `c_first c_last w` entries).
fn cid_widths(entries: &[Value]) -> HashMap<u32, f64> {
    let mut widths = HashMap::new();
    let mut i = 0;
    while i < entries.len() {
        let first = entries[i].as_num().unwrap_or(0.0) as u32;
        match entries.get(i + 1) {
            Some(Value::Array(run)) => {
                for (code, width) in (first..).zip(run) {
                    widths.insert(code, width.as_num().unwrap_or(0.0));
                }
                i += 2;
            }
            Some(last) => {
                let last = last.as_num().unwrap_or(0.0) as u32;
                let width = entries.get(i + 2).and_then(Value::as_num).unwrap_or(0.0);
                widths.extend((first..=last).map(|code| (code, width)));
                i += 3;
            }
            None => break,
        }
    }
    widths
}

/// The graphics state.
#[derive(Clone, Debug)]
struct State {
    ctm: Matrix,
    stroke: Vec<Token>,
    fill: Vec<Token>,
    stroke_alpha: f64,
    fill_alpha: f64,
    soft_mask: Vec<Token>,
    linewidth: f64,
    cap: f64,
    join: f64,
    dash: Vec<Token>,
    clips: Vec<Vec<Token>>,
    font: Font,
    font_size: f64,
    rise: f64,
}

impl Default for State {
    fn default() -> Self {
        let black = vec![Token::Num(0.0), Token::Num(0.0), Token::Num(0.0)];
        Self {
            ctm: IDENTITY,
            stroke: black.clone(),
            fill: black,
            stroke_alpha: 1.0,
            fill_alpha: 1.0,
            soft_mask: Vec::new(),
            linewidth: 1.0,
            cap: 0.0,
            join: 0.0,
            dash: vec![Token::word("solid")],
            clips: Vec::new(),
            font: Font::default(),
            font_size: 0.0,
            rise: 0.0,
        }
    }
}

/// Interprets content streams into items.
pub(super) struct Interpreter<'a> {
    doc: &'a Document,
    pub(super) items: Vec<Item>,
    state: State,
    stack: Vec<State>,
    path: Vec<Token>,
    clip_pending: bool,
    text_matrix: Matrix,
    line_matrix: Matrix,
}

impl<'a> Interpreter<'a> {
    pub(super) fn new(doc: &'a Document) -> Self {
        Self {
            doc,
            items: Vec::new(),
            state: State::default(),
            stack: Vec::new(),
            path: Vec::new(),
            clip_pending: false,
            text_matrix: IDENTITY,
            line_matrix: IDENTITY,
        }
    }

    /// The resource `name` of `category` in `resources`.
    fn resource(&self, resources: &'a Value, category: &str, name: &str) -> &'a Value {
        let dict = self
            .doc
            .resolve(resources.get(category).unwrap_or(&Value::Null));
        dict.get(name).unwrap_or(&Value::Null)
    }

    fn point(&mut self, x: f64, y: f64) {
        let (x, y) = apply(&self.state.ctm, x, y);
        self.path.push(Token::Num(x));
        self.path.push(Token::Num(y));
    }

    fn clip_tokens(&self, out: &mut Vec<Token>) {
        for clip in &self.state.clips {
            out.push(Token::word("clip["));
            out.extend(clip.iter().cloned());
            out.push(Token::word("]"));
        }
    }

    fn fill_tokens(&self, out: &mut Vec<Token>) {
        out.push(Token::word("fill:"));
        out.extend(self.state.fill.iter().cloned());
        out.push(Token::Num(self.state.fill_alpha));
    }

    fn stroke_tokens(&self, out: &mut Vec<Token>) {
        let s = &self.state;
        out.push(Token::word("stroke:"));
        out.extend(s.stroke.iter().cloned());
        out.extend([
            Token::Num(s.stroke_alpha),
            Token::Num(s.linewidth),
            Token::Num(s.cap),
            Token::Num(s.join),
        ]);
        out.extend(s.dash.iter().cloned());
    }

    /// An item with the state every painted item carries.
    fn painted(&self, kind: &str) -> Item {
        let mut item = Item::new(kind);
        if !self.state.soft_mask.is_empty() {
            item.tokens.push(Token::word("smask:"));
            item.tokens.extend(self.state.soft_mask.iter().cloned());
        }
        self.clip_tokens(&mut item.tokens);
        item
    }

    /// Ends the current path with a painting operator.
    fn paint(&mut self, op: &str) {
        let path = std::mem::take(&mut self.path);
        if self.clip_pending {
            self.clip_pending = false;
            self.state.clips.push(path.clone());
        }
        let (fill, stroke) = match op {
            "f" | "F" | "f*" => (true, false),
            "S" | "s" => (false, true),
            "B" | "B*" | "b" | "b*" => (true, true),
            _ => return,
        };
        let kind = match (fill, stroke) {
            (true, true) => "fill+stroke",
            (true, false) => "fill",
            _ => "stroke",
        };
        let mut item = self.painted(kind);
        item.tokens.extend(path);
        if fill {
            self.fill_tokens(&mut item.tokens);
        }
        if stroke {
            self.stroke_tokens(&mut item.tokens);
        }
        self.items.push(item);
    }

    /// A colour operand list: grey levels become RGB.
    fn color(operands: &[Value]) -> Vec<Token> {
        let values = numbers(operands);
        match values.as_slice() {
            [gray] => vec![Token::Num(*gray); 3],
            _ => values.into_iter().map(Token::Num).collect(),
        }
    }

    fn graphics_state(&mut self, resources: &'a Value, name: &str) {
        let state = self
            .doc
            .resolve(self.resource(resources, "ExtGState", name));
        if let Some(alpha) = state.get("CA").and_then(Value::as_num) {
            self.state.stroke_alpha = alpha;
        }
        if let Some(alpha) = state.get("ca").and_then(Value::as_num) {
            self.state.fill_alpha = alpha;
        }
        if let Some(mask) = state.get("SMask") {
            let group = self.doc.resolve(mask.get("G").unwrap_or(&Value::Null));
            let shading = group
                .get("Resources")
                .and_then(|r| r.get("Shading"))
                .and_then(|s| s.get("S"))
                .unwrap_or(&Value::Null);
            self.state.soft_mask = shading_tokens(self.doc, shading, &IDENTITY);
        }
    }

    fn font(&mut self, resources: &'a Value, name: &str, size: f64) {
        let font = self.doc.resolve(self.resource(resources, "Font", name));
        let base = font
            .get("BaseFont")
            .and_then(Value::as_name)
            .unwrap_or(name);
        self.state.font_size = size;
        if font.get("Subtype").and_then(Value::as_name) == Some("Type0") {
            let descendant = font
                .get("DescendantFonts")
                .map(|d| self.doc.resolve(d))
                .and_then(Value::as_array)
                .and_then(<[Value]>::first)
                .map_or(&Value::Null, |d| self.doc.resolve(d));
            let widths = descendant
                .get("W")
                .map(|w| self.doc.resolve(w))
                .and_then(Value::as_array)
                .map(cid_widths)
                .unwrap_or_default();
            self.state.font = Font {
                name: base_name(base).to_owned(),
                glyphs: HashMap::new(),
                widths,
                scale: 0.001,
                code_bytes: 2,
            };
            return;
        }
        let mut glyphs = HashMap::new();
        let differences = self
            .doc
            .resolve(font.get("Encoding").unwrap_or(&Value::Null))
            .get("Differences")
            .and_then(Value::as_array)
            .unwrap_or_default();
        let mut code = 0u32;
        for entry in differences {
            match entry {
                Value::Num(n) => code = *n as u32,
                Value::Name(glyph) => {
                    glyphs.insert(code, glyph.clone());
                    code += 1;
                }
                _ => {}
            }
        }
        let first = font.get("FirstChar").and_then(Value::as_num).unwrap_or(0.0) as u32;
        let widths = self
            .doc
            .resolve(font.get("Widths").unwrap_or(&Value::Null))
            .as_array()
            .unwrap_or_default()
            .iter()
            .enumerate()
            .filter_map(|(i, w)| Some((first + i as u32, w.as_num()?)))
            .collect();
        let scale = font
            .get("FontMatrix")
            .and_then(Value::as_array)
            .and_then(|m| m.first())
            .and_then(Value::as_num)
            .unwrap_or(0.001);
        self.state.font = Font {
            name: base_name(base).to_owned(),
            glyphs,
            widths,
            scale,
            code_bytes: 1,
        };
    }

    /// Shows the codes of a string, one item per glyph.
    fn show(&mut self, bytes: &[u8]) {
        let width = self.state.font.code_bytes.max(1);
        for chunk in bytes.chunks(width) {
            let code = chunk.iter().fold(0u32, |code, &b| code << 8 | u32::from(b));
            let s = &self.state;
            let render = concat(
                &[s.font_size, 0.0, 0.0, s.font_size, 0.0, s.rise],
                &concat(&self.text_matrix, &s.ctm),
            );
            let (x, y) = apply(&render, 0.0, 0.0);
            let mut item = self.painted("glyph");
            item.tokens.push(Token::word(s.font.name.clone()));
            item.tokens.push(Token::word(
                s.font
                    .glyphs
                    .get(&code)
                    .cloned()
                    .unwrap_or_else(|| format!("#{code}")),
            ));
            item.tokens
                .extend([Token::Num(s.font_size), Token::Num(x), Token::Num(y)]);
            item.tokens
                .extend(render[..4].iter().map(|v| Token::Num(v / s.font_size)));
            self.fill_tokens(&mut item.tokens);
            self.items.push(item);
            let advance =
                s.font.widths.get(&code).copied().unwrap_or(0.0) * s.font.scale * s.font_size;
            self.text_matrix = concat(&[1.0, 0.0, 0.0, 1.0, advance, 0.0], &self.text_matrix);
        }
    }

    fn text_operation(&mut self, resources: &'a Value, operands: &[Value], op: &str) {
        let n = numbers(operands);
        match op {
            "BT" => {
                self.text_matrix = IDENTITY;
                self.line_matrix = IDENTITY;
            }
            "Tf" => {
                let name = operands.first().and_then(Value::as_name).unwrap_or("");
                self.font(resources, name, n.first().copied().unwrap_or(0.0));
            }
            "Td" if n.len() == 2 => {
                self.line_matrix = concat(&[1.0, 0.0, 0.0, 1.0, n[0], n[1]], &self.line_matrix);
                self.text_matrix = self.line_matrix;
            }
            "Tm" if n.len() == 6 => {
                self.line_matrix = [n[0], n[1], n[2], n[3], n[4], n[5]];
                self.text_matrix = self.line_matrix;
            }
            "Ts" => self.state.rise = n.first().copied().unwrap_or(0.0),
            "Tj" => {
                if let Some(Value::Str(bytes)) = operands.first() {
                    self.show(&bytes.clone());
                }
            }
            "TJ" => {
                let parts = operands
                    .first()
                    .and_then(Value::as_array)
                    .unwrap_or_default()
                    .to_vec();
                for part in parts {
                    match part {
                        Value::Str(bytes) => self.show(&bytes),
                        Value::Num(kern) => {
                            let shift = -kern / 1000.0 * self.state.font_size;
                            self.text_matrix =
                                concat(&[1.0, 0.0, 0.0, 1.0, shift, 0.0], &self.text_matrix);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    /// Runs an `XObject`: a form in place, or an image.
    fn xobject(&mut self, resources: &'a Value, name: &str) {
        let reference = self.resource(resources, "XObject", name);
        let dict = self.doc.resolve(reference);
        if dict.get("Subtype").and_then(Value::as_name) == Some("Form") {
            let matrix = dict
                .get("Matrix")
                .and_then(Value::as_array)
                .map(numbers)
                .filter(|m| m.len() == 6)
                .map_or(IDENTITY, |m| [m[0], m[1], m[2], m[3], m[4], m[5]]);
            let inner = dict
                .get("Resources")
                .map_or(resources, |r| self.doc.resolve(r));
            let data = self.doc.stream(reference).unwrap_or_default();
            self.stack.push(self.state.clone());
            self.state.ctm = concat(&matrix, &self.state.ctm);
            self.run(inner, &data);
            if let Some(state) = self.stack.pop() {
                self.state = state;
            }
        } else {
            let mut item = self.painted("image");
            item.tokens
                .extend(self.state.ctm.iter().map(|&v| Token::Num(v)));
            item.tokens.push(Token::Num(self.state.fill_alpha));
            item.tokens.extend(image_tokens(self.doc, reference));
            self.items.push(item);
        }
    }

    /// A tiling pattern as fill colour: its matrix and tile.
    fn pattern(&self, resources: &'a Value, name: &str) -> Vec<Token> {
        let reference = self.resource(resources, "Pattern", name);
        let dict = self.doc.resolve(reference);
        let mut tokens = vec![Token::word("pattern")];
        for key in ["BBox", "Matrix", "XStep", "YStep"] {
            if let Some(value) = dict.get(key) {
                value_tokens(value, &mut tokens);
            }
        }
        let data = self.doc.stream(reference).unwrap_or_default();
        let mut tile = Interpreter::new(self.doc);
        tile.run(resources, &data);
        for item in tile.items {
            tokens.push(Token::word(item.kind));
            tokens.extend(item.tokens);
        }
        tokens
    }

    fn operation(&mut self, resources: &'a Value, operands: &[Value], op: &str) {
        let n = numbers(operands);
        let name = operands.first().and_then(Value::as_name);
        match op {
            "q" => self.stack.push(self.state.clone()),
            "Q" => {
                if let Some(state) = self.stack.pop() {
                    self.state = state;
                }
            }
            "cm" if n.len() == 6 => {
                self.state.ctm = concat(&[n[0], n[1], n[2], n[3], n[4], n[5]], &self.state.ctm);
            }
            "m" | "l" if n.len() == 2 => {
                self.path.push(Token::word(op));
                self.point(n[0], n[1]);
            }
            "c" if n.len() == 6 => {
                self.path.push(Token::word("c"));
                for pair in n.chunks(2) {
                    self.point(pair[0], pair[1]);
                }
            }
            "re" if n.len() == 4 => {
                let (x, y, w, h) = (n[0], n[1], n[2], n[3]);
                self.path.push(Token::word("re"));
                for (px, py) in [(x, y), (x + w, y), (x + w, y + h), (x, y + h)] {
                    self.point(px, py);
                }
            }
            "h" => self.path.push(Token::word("h")),
            "W" | "W*" => self.clip_pending = true,
            "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => self.paint(op),
            "w" => self.state.linewidth = n.first().copied().unwrap_or(1.0),
            "J" => self.state.cap = n.first().copied().unwrap_or(0.0),
            "j" => self.state.join = n.first().copied().unwrap_or(0.0),
            "d" => {
                let pattern = operands
                    .first()
                    .and_then(Value::as_array)
                    .map(numbers)
                    .unwrap_or_default();
                self.state.dash = if pattern.is_empty() {
                    vec![Token::word("solid")]
                } else {
                    let mut dash: Vec<Token> = pattern.into_iter().map(Token::Num).collect();
                    dash.push(Token::Num(n.first().copied().unwrap_or(0.0)));
                    dash
                };
            }
            "G" | "RG" => self.state.stroke = Self::color(operands),
            "g" | "rg" => self.state.fill = Self::color(operands),
            "scn" => {
                if let Some(name) = name {
                    self.state.fill = self.pattern(resources, name);
                } else {
                    self.state.fill = Self::color(operands);
                }
            }
            "gs" => {
                if let Some(name) = name {
                    self.graphics_state(resources, name);
                }
            }
            "Do" => {
                if let Some(name) = name {
                    self.xobject(resources, name);
                }
            }
            "sh" => {
                if let Some(name) = name {
                    let shading = self.resource(resources, "Shading", name);
                    let mut item = self.painted("shading");
                    item.tokens.push(Token::Num(self.state.fill_alpha));
                    item.tokens
                        .extend(shading_tokens(self.doc, shading, &self.state.ctm));
                    self.items.push(item);
                }
            }
            "d0" | "d1" => {
                let mut item = Item::new(op);
                item.tokens.extend(n.into_iter().map(Token::Num));
                self.items.push(item);
            }
            _ => self.text_operation(resources, operands, op),
        }
    }

    /// Interprets `data` with `resources`.
    pub(super) fn run(&mut self, resources: &'a Value, data: &[u8]) {
        match operations(data) {
            Ok(ops) => {
                for (operands, op) in ops {
                    self.operation(resources, &operands, &op);
                }
            }
            Err(err) => self.items.push(Item {
                kind: "unreadable".to_owned(),
                tokens: vec![Token::word(err)],
            }),
        }
    }
}
