//! An interpreter for the PostScript matplotlib writes after its prolog,
//! producing one item per fill, stroke, glyph, image and shading with the
//! graphics state that affects it (paths in device coordinates).

use std::collections::HashMap;

use super::lexer::{Lexer, Tok};
use crate::figures::vector::diff::{Item, Token};
use crate::figures::vector::postscript::{IDENTITY, Matrix, apply, concat};

/// A PostScript value.
#[derive(Clone, Debug, PartialEq)]
enum V {
    Num(f64),
    Name(String),
    Lit(String),
    Str(Vec<u8>),
    Proc(Vec<V>),
    Array(Vec<V>),
    Dict(Vec<(String, V)>),
    Mark,
    Null,
    /// A pattern made by `makepattern`: its description.
    Pattern(Vec<Token>),
}

impl V {
    fn num(&self) -> f64 {
        match self {
            Self::Num(n) => *n,
            _ => 0.0,
        }
    }

    fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Dict(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// Tokens describing a value (for pattern dictionaries).
fn value_tokens(value: &V, out: &mut Vec<Token>) {
    match value {
        V::Num(n) => out.push(Token::Num(*n)),
        V::Name(n) | V::Lit(n) => out.push(Token::word(n.clone())),
        V::Str(s) => out.push(Token::word(format!("<{} bytes>", s.len()))),
        V::Proc(items) | V::Array(items) => {
            out.push(Token::word("["));
            for item in items {
                value_tokens(item, out);
            }
            out.push(Token::word("]"));
        }
        V::Dict(entries) => {
            let mut sorted: Vec<&(String, V)> = entries.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            for (key, value) in sorted {
                out.push(Token::word(format!("/{key}")));
                value_tokens(value, out);
            }
        }
        V::Pattern(tokens) => out.extend(tokens.iter().cloned()),
        V::Mark | V::Null => out.push(Token::word("null")),
    }
}

/// The graphics state.
#[derive(Clone, Debug)]
struct State {
    ctm: Matrix,
    color: Vec<Token>,
    linewidth: f64,
    join: f64,
    cap: f64,
    dash: Vec<Token>,
    clips: Vec<Vec<Token>>,
    path: Vec<Token>,
    point: (f64, f64),
    font: (String, f64),
}

impl Default for State {
    fn default() -> Self {
        Self {
            ctm: IDENTITY,
            color: vec![Token::Num(0.0); 3],
            linewidth: 1.0,
            join: 0.0,
            cap: 0.0,
            dash: vec![Token::word("solid")],
            clips: Vec::new(),
            path: Vec::new(),
            point: (0.0, 0.0),
            font: (String::new(), 0.0),
        }
    }
}

/// The aliases matplotlib's prolog defines (`_psDefs`).
const ALIASES: [(&str, &str); 6] = [
    ("m", "moveto"),
    ("l", "lineto"),
    ("r", "rlineto"),
    ("c", "curveto"),
    ("cl", "closepath"),
    ("ce", "eofill"),
];

/// Runs the page description.
pub(super) struct Interpreter {
    stack: Vec<V>,
    dicts: Vec<HashMap<String, V>>,
    state: State,
    saved: Vec<State>,
    pub(super) items: Vec<Item>,
}

impl Interpreter {
    pub(super) fn new() -> Self {
        Self {
            stack: Vec::new(),
            dicts: vec![HashMap::new()],
            state: State::default(),
            saved: Vec::new(),
            items: Vec::new(),
        }
    }

    fn pop(&mut self) -> V {
        self.stack.pop().unwrap_or(V::Null)
    }

    fn pop_num(&mut self) -> f64 {
        self.pop().num()
    }

    fn pop_nums<const N: usize>(&mut self) -> [f64; N] {
        let mut values = [0.0; N];
        for slot in values.iter_mut().rev() {
            *slot = self.pop_num();
        }
        values
    }

    fn lookup(&self, name: &str) -> Option<V> {
        self.dicts.iter().rev().find_map(|d| d.get(name).cloned())
    }

    fn point(&mut self, op: &str, x: f64, y: f64) {
        let (dx, dy) = apply(&self.state.ctm, x, y);
        self.state.path.push(Token::word(op));
        self.state.path.extend([Token::Num(dx), Token::Num(dy)]);
    }

    /// An item carrying the clip regions.
    fn painted(&self, kind: &str) -> Item {
        let mut item = Item::new(kind);
        for clip in &self.state.clips {
            item.tokens.push(Token::word("clip["));
            item.tokens.extend(clip.iter().cloned());
            item.tokens.push(Token::word("]"));
        }
        item
    }

    fn fill(&mut self) {
        let mut item = self.painted("fill");
        item.tokens.extend(std::mem::take(&mut self.state.path));
        item.tokens.push(Token::word("color:"));
        item.tokens.extend(self.state.color.iter().cloned());
        self.items.push(item);
    }

    fn stroke(&mut self) {
        let s = &self.state;
        let mut item = self.painted("stroke");
        item.tokens.extend(s.path.iter().cloned());
        item.tokens.push(Token::word("color:"));
        item.tokens.extend(s.color.iter().cloned());
        item.tokens.extend([
            Token::Num(s.linewidth),
            Token::Num(s.join),
            Token::Num(s.cap),
        ]);
        item.tokens.extend(s.dash.iter().cloned());
        self.items.push(item);
        self.state.path.clear();
    }

    fn rectangle(&mut self, [x, y, w, h]: [f64; 4]) {
        self.state.path.clear();
        self.point("m", x, y);
        for (px, py) in [(x + w, y), (x + w, y + h), (x, y + h)] {
            self.point("l", px, py);
        }
        self.state.path.push(Token::word("h"));
    }

    fn image(&mut self, lexer: &mut Lexer<'_>) {
        let components = self.pop_num() as usize;
        for _ in 0..4 {
            self.pop();
        }
        let [width, height] = self.pop_nums::<2>().map(|v| v as usize);
        let data = lexer.read_hex(width * height * components);
        let mut item = self.painted("image");
        item.tokens
            .extend(self.state.ctm.iter().map(|&v| Token::Num(v)));
        item.tokens
            .extend([Token::Num(width as f64), Token::Num(height as f64)]);
        let pixels = (width * height).max(1) as f64;
        for channel in 0..components {
            let sum: f64 = data
                .iter()
                .skip(channel)
                .step_by(components.max(1))
                .map(|&v| f64::from(v))
                .sum();
            item.tokens.push(Token::Num(sum / pixels));
        }
        self.items.push(item);
    }

    fn shading(&mut self) {
        let dict = self.pop();
        let decode: Vec<f64> = match dict.get("Decode") {
            Some(V::Array(values)) => values.iter().map(V::num).collect(),
            _ => Vec::new(),
        };
        let mut item = self.painted("shading");
        if let (Some(V::Str(data)), true) = (dict.get("DataSource"), decode.len() >= 4) {
            for vertex in data.as_chunks::<12>().0 {
                let coord = |at: usize, axis: usize| {
                    let raw = u32::from_be_bytes([
                        vertex[at],
                        vertex[at + 1],
                        vertex[at + 2],
                        vertex[at + 3],
                    ]);
                    let (lo, hi) = (decode[2 * axis], decode[2 * axis + 1]);
                    lo + f64::from(raw) * (hi - lo) / f64::from(u32::MAX)
                };
                let (x, y) = apply(&self.state.ctm, coord(1, 0), coord(5, 1));
                item.tokens.extend([Token::Num(x), Token::Num(y)]);
                item.tokens
                    .extend(vertex[9..].iter().map(|&c| Token::Num(f64::from(c))));
            }
        }
        self.items.push(item);
    }

    fn glyph(&mut self) {
        let name = match self.pop() {
            V::Lit(name) => name,
            _ => "?".to_owned(),
        };
        let s = &self.state;
        let mut item = self.painted("glyph");
        item.tokens.push(Token::word(s.font.0.clone()));
        item.tokens.push(Token::word(name));
        item.tokens.push(Token::Num(s.font.1));
        item.tokens
            .extend([Token::Num(s.point.0), Token::Num(s.point.1)]);
        item.tokens
            .extend(s.ctm[..4].iter().map(|&v| Token::Num(v)));
        item.tokens.push(Token::word("color:"));
        item.tokens.extend(s.color.iter().cloned());
        self.items.push(item);
    }

    /// Collection, dictionary and stack operators; `false` when `name` is
    /// not one.
    fn structure_op(&mut self, name: &str) -> bool {
        match name {
            "[" | "<<" => self.stack.push(V::Mark),
            "]" | ">>" => {
                let start = self.stack.iter().rposition(|v| *v == V::Mark).unwrap_or(0);
                let items: Vec<V> = self.stack.drain(start..).skip(1).collect();
                self.stack.push(if name == "]" {
                    V::Array(items)
                } else {
                    let pairs = items
                        .chunks(2)
                        .filter_map(|pair| match pair {
                            [V::Lit(key), value] => Some((key.clone(), value.clone())),
                            _ => None,
                        })
                        .collect();
                    V::Dict(pairs)
                });
            }
            "def" => {
                let value = self.pop();
                if let V::Lit(key) = self.pop()
                    && let Some(dict) = self.dicts.last_mut()
                {
                    dict.insert(key, value);
                }
            }
            "exch" => {
                let (b, a) = (self.pop(), self.pop());
                self.stack.extend([b, a]);
            }
            "pop" => {
                self.pop();
            }
            "begin" => {
                self.pop();
                self.dicts.push(HashMap::new());
            }
            "end" => {
                if self.dicts.len() > 1 {
                    self.dicts.pop();
                }
            }
            "mpldict" | "currentfile" => self.stack.push(V::Null),
            "string" => {
                let length = self.pop_num() as usize;
                self.stack.push(V::Str(vec![0; length]));
            }
            "matrix" => self.stack.push(V::Array(IDENTITY.map(V::Num).to_vec())),
            "bind" | "readonly" | "showpage" => {}
            _ => return false,
        }
        true
    }

    /// Coordinate system, state stack, path and clip operators; `false`
    /// when `name` is not one.
    fn geometry_op(&mut self, name: &str) -> bool {
        match name {
            "translate" => {
                if let Some(V::Array(_)) = self.stack.last() {
                    let matrix = self.pop();
                    let [tx, ty] = self.pop_nums::<2>();
                    if let V::Array(mut m) = matrix {
                        if m.len() == 6 {
                            m[4] = V::Num(tx);
                            m[5] = V::Num(ty);
                        }
                        self.stack.push(V::Array(m));
                    }
                } else {
                    let [tx, ty] = self.pop_nums::<2>();
                    self.state.ctm = concat(&[1.0, 0.0, 0.0, 1.0, tx, ty], &self.state.ctm);
                }
            }
            "rotate" => {
                let (sin, cos) = self.pop_num().to_radians().sin_cos();
                self.state.ctm = concat(&[cos, sin, -sin, cos, 0.0, 0.0], &self.state.ctm);
            }
            "scale" => {
                let [sx, sy] = self.pop_nums::<2>();
                self.state.ctm = concat(&[sx, 0.0, 0.0, sy, 0.0, 0.0], &self.state.ctm);
            }
            "concat" => {
                if let V::Array(m) = self.pop()
                    && m.len() == 6
                {
                    let m: Vec<f64> = m.iter().map(V::num).collect();
                    self.state.ctm = concat(&[m[0], m[1], m[2], m[3], m[4], m[5]], &self.state.ctm);
                }
            }
            "gsave" => self.saved.push(self.state.clone()),
            "grestore" => {
                if let Some(state) = self.saved.pop() {
                    self.state = state;
                }
            }
            "newpath" => self.state.path.clear(),
            "moveto" | "lineto" => {
                let [x, y] = self.pop_nums::<2>();
                self.point(if name == "moveto" { "m" } else { "l" }, x, y);
                self.state.point = apply(&self.state.ctm, x, y);
            }
            "curveto" => {
                let values = self.pop_nums::<6>();
                self.state.path.push(Token::word("c"));
                for pair in values.chunks(2) {
                    let (x, y) = apply(&self.state.ctm, pair[0], pair[1]);
                    self.state.path.extend([Token::Num(x), Token::Num(y)]);
                }
            }
            "closepath" => self.state.path.push(Token::word("h")),
            "clip" => self.state.clips.push(self.state.path.clone()),
            "rectclip" => {
                let rect = self.pop_nums::<4>();
                self.rectangle(rect);
                let path = std::mem::take(&mut self.state.path);
                self.state.clips.push(path);
            }
            _ => return false,
        }
        true
    }

    /// Line, colour, pattern and font operators; `false` when `name` is
    /// not one.
    fn style_op(&mut self, name: &str) -> bool {
        match name {
            "setlinewidth" => self.state.linewidth = self.pop_num(),
            "setlinejoin" => self.state.join = self.pop_num(),
            "setlinecap" => self.state.cap = self.pop_num(),
            "setdash" => {
                let offset = self.pop_num();
                let pattern: Vec<f64> = match self.pop() {
                    V::Array(values) => values.iter().map(V::num).collect(),
                    _ => Vec::new(),
                };
                self.state.dash = if pattern.is_empty() {
                    vec![Token::word("solid")]
                } else {
                    pattern
                        .into_iter()
                        .chain([offset])
                        .map(Token::Num)
                        .collect()
                };
            }
            "setgray" => self.state.color = vec![Token::Num(self.pop_num()); 3],
            "setrgbcolor" => {
                self.state.color = self.pop_nums::<3>().map(Token::Num).to_vec();
            }
            "makepattern" => {
                let matrix = self.pop();
                let dict = self.pop();
                // The pattern space is the matrix in the current space.
                let mut tokens = vec![Token::word("pattern")];
                value_tokens(&dict, &mut tokens);
                value_tokens(&matrix, &mut tokens);
                tokens.extend(self.state.ctm.iter().map(|&v| Token::Num(v)));
                self.stack.push(V::Pattern(tokens));
            }
            "setpattern" => {
                let mut tokens = Vec::new();
                value_tokens(&self.pop(), &mut tokens);
                tokens.extend(self.pop_nums::<3>().map(Token::Num));
                self.state.color = tokens;
            }
            "selectfont" => {
                let size = self.pop_num();
                if let V::Lit(font) = self.pop() {
                    self.state.font = (font, size);
                }
            }
            _ => return false,
        }
        true
    }

    /// Runs a built-in operator; unknown names are pushed.
    fn builtin(&mut self, name: &str, lexer: &mut Lexer<'_>) {
        if self.structure_op(name) || self.geometry_op(name) || self.style_op(name) {
            return;
        }
        match name {
            "fill" | "eofill" => self.fill(),
            "stroke" => self.stroke(),
            "rectfill" => {
                let rect = self.pop_nums::<4>();
                self.rectangle(rect);
                self.fill();
            }
            "glyphshow" => self.glyph(),
            "colorimage" => self.image(lexer),
            "shfill" => self.shading(),
            _ => self.stack.push(V::Name(name.to_owned())),
        }
    }

    /// Runs one value: names are looked up (procedures run), everything
    /// else is pushed.
    fn execute(&mut self, value: V, lexer: &mut Lexer<'_>) {
        let V::Name(name) = value else {
            self.stack.push(value);
            return;
        };
        if let Some(alias) = ALIASES.iter().find(|(a, _)| *a == name).map(|(_, b)| *b) {
            return self.builtin(alias, lexer);
        }
        match self.lookup(&name) {
            Some(V::Proc(body)) => {
                for item in body {
                    self.execute(item, lexer);
                }
            }
            Some(other) => self.stack.push(other),
            None => self.builtin(&name, lexer),
        }
    }

    /// Reads a procedure body up to its closing brace.
    fn procedure(lexer: &mut Lexer<'_>) -> V {
        let mut body = Vec::new();
        while let Some(token) = lexer.next() {
            body.push(match token {
                Tok::ProcEnd => break,
                Tok::ProcStart => Self::procedure(lexer),
                Tok::Num(n) => V::Num(n),
                Tok::Name(n) => V::Name(n),
                Tok::Lit(n) => V::Lit(n),
                Tok::Str(s) => V::Str(s),
            });
        }
        V::Proc(body)
    }

    /// Runs a whole program.
    pub(super) fn run(&mut self, program: &[u8]) {
        let mut lexer = Lexer::new(program);
        while let Some(token) = lexer.next() {
            let value = match token {
                Tok::ProcStart => Self::procedure(&mut lexer),
                Tok::ProcEnd => continue,
                Tok::Num(n) => V::Num(n),
                Tok::Name(n) => V::Name(n),
                Tok::Lit(n) => V::Lit(n),
                Tok::Str(s) => V::Str(s),
            };
            if matches!(value, V::Proc(_)) {
                self.stack.push(value);
            } else {
                self.execute(value, &mut lexer);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_clips_and_fills() {
        let mut ps = Interpreter::new();
        ps.run(
            b"10 20 translate\n/o { gsave newpath translate 0 0 m 1 0 l stroke grestore } bind def\n\
              gsave 0 0 5 5 rectclip 1 2 o 3 4 o grestore\n0.5 setgray 0 0 1 1 rectfill",
        );
        assert_eq!(ps.items.len(), 3);
        assert_eq!(ps.items[0].kind, "stroke");
        let text = ps.items[1].to_string();
        assert!(
            text.starts_with("stroke clip[ m 10 20 l 15 20 l 15 25 l 10 25 h ] m 13 24 l 14 24"),
            "{text}"
        );
        assert_eq!(
            ps.items[2].to_string(),
            "fill m 10 20 l 11 20 l 11 21 l 10 21 h color: 0.5 0.5 0.5"
        );
    }
}
