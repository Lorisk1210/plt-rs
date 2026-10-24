//! matplotlib's `GraphicsContextPdf`: the graphics state the content
//! stream is in, and the operators (`delta`) that turn it into the state
//! a drawing call needs. Clipping can only be widened by restoring a
//! saved state, so clip changes pop and push the state stack.

#![allow(
    clippy::float_cmp,
    reason = "matplotlib detects graphics-state changes with exact comparisons"
)]

use plt_core::{Bbox, Rgba};

use super::object::Obj;
use super::path_ops;
use super::resources::{HatchStyle, Resources, cap_code, join_code};
use crate::gc::{CapStyle, ClipPath, GraphicsContext, JoinStyle};

/// The fill colour of a state: matplotlib starts from the three-element
/// black `(0, 0, 0)`, which differs from every RGBA colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Fill {
    InitialBlack,
    None,
    Color(Rgba),
}

impl Fill {
    fn rgb(self) -> Option<[f64; 3]> {
        match self {
            Self::InitialBlack => Some([0.0; 3]),
            Self::None => None,
            Self::Color(c) => Some([c.r, c.g, c.b]),
        }
    }

    /// `GraphicsContextPdf.fill`: whether a path with this fill colour is
    /// filled.
    fn paints(self) -> bool {
        match self {
            Self::InitialBlack => true,
            Self::None => false,
            Self::Color(c) => c.a != 0.0,
        }
    }
}

/// A clip region together with the drawing generation it was set in:
/// matplotlib compares clip boxes and paths by object identity, and every
/// artist owns its own (see `PdfRenderer::generation`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Clip<T> {
    pub(super) value: T,
    pub(super) generation: u64,
}

/// A graphics state.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PdfState {
    pub(super) clip_rect: Option<Clip<Bbox>>,
    pub(super) clip_path: Option<Clip<ClipPath>>,
    pub(super) alpha: f64,
    pub(super) forced_alpha: bool,
    pub(super) effective_alphas: (f64, f64),
    pub(super) cap: CapStyle,
    pub(super) fill: Fill,
    pub(super) join: JoinStyle,
    pub(super) linewidth: f64,
    pub(super) dashes: (f64, Option<Vec<f64>>),
    pub(super) rgb: Rgba,
    pub(super) hatch: Option<String>,
    pub(super) hatch_color: Option<Rgba>,
    pub(super) hatch_linewidth: f64,
}

impl Default for PdfState {
    /// `GraphicsContextPdf()`.
    fn default() -> Self {
        Self {
            clip_rect: None,
            clip_path: None,
            alpha: 1.0,
            forced_alpha: false,
            effective_alphas: (1.0, 1.0),
            cap: CapStyle::Butt,
            fill: Fill::InitialBlack,
            join: JoinStyle::Round,
            linewidth: 1.0,
            dashes: (0.0, None),
            rgb: Rgba::BLACK,
            hatch: None,
            hatch_color: None,
            hatch_linewidth: 1.0,
        }
    }
}

impl PdfState {
    /// The state a drawing call with `gc` and fill colour `fill` needs
    /// (`check_gc`), with clips tagged by `generation`.
    pub(super) fn from_gc(gc: &GraphicsContext, fill: Option<Rgba>, generation: u64) -> Self {
        let rgb = gc.stroke_color();
        let effective_alphas = match (gc.alpha, fill) {
            (Some(alpha), _) => (alpha, alpha),
            (None, None) => (rgb.a, 1.0),
            (None, Some(face)) => (rgb.a, face.a),
        };
        Self {
            clip_rect: gc.clip_rect.map(|value| Clip { value, generation }),
            clip_path: gc
                .clip_path
                .clone()
                .filter(|clip| clip.path.vertices().iter().all(|p| p.is_finite()))
                .map(|value| Clip { value, generation }),
            alpha: gc.alpha.unwrap_or(1.0),
            forced_alpha: gc.alpha.is_some(),
            effective_alphas,
            cap: gc.cap,
            fill: fill.map_or(Fill::None, Fill::Color),
            join: gc.join,
            linewidth: gc.linewidth,
            dashes: (gc.dashes.offset, gc.dashes.pattern.clone()),
            rgb,
            hatch: gc.hatch.as_ref().map(|h| h.pattern.clone()),
            hatch_color: gc.hatch.as_ref().map(|h| h.color),
            hatch_linewidth: gc.hatch.as_ref().map_or(1.0, |h| h.linewidth),
        }
    }

    /// `stroke`: whether paths are stroked.
    pub(super) fn strokes(&self) -> bool {
        self.linewidth > 0.0 && self.alpha > 0.0 && self.rgb.a != 0.0
    }

    /// `fill`: whether paths are filled.
    pub(super) fn fills(&self) -> bool {
        self.hatch.is_some() || self.fill.paints()
    }
}

/// `fillcolor_cmd` / `rgb_cmd`: a grey or RGB colour operator.
fn color_cmd(rgb: [f64; 3], gray: &str, color: &str) -> Vec<Obj> {
    if rgb[0] == rgb[1] && rgb[1] == rgb[2] {
        vec![Obj::Real(rgb[0]), Obj::op(gray)]
    } else {
        vec![
            Obj::Real(rgb[0]),
            Obj::Real(rgb[1]),
            Obj::Real(rgb[2]),
            Obj::op(color),
        ]
    }
}

/// The current state plus the saved states below it.
#[derive(Debug, Default)]
pub(super) struct StateStack {
    pub(super) current: PdfState,
    parents: Vec<PdfState>,
    /// `pdf.inheritcolor`: leave colours to the including document.
    pub(super) inherit_color: bool,
}

impl StateStack {
    /// The initial state; `inherit_color` is `pdf.inheritcolor`.
    pub(super) fn new(inherit_color: bool) -> Self {
        Self {
            inherit_color,
            ..Self::default()
        }
    }

    /// `push`: saves the state (`q`).
    pub(super) fn push(&mut self) -> Vec<Obj> {
        self.parents.push(self.current.clone());
        vec![Obj::op("q")]
    }

    /// `pop`: restores the saved state (`Q`).
    pub(super) fn pop(&mut self) -> Vec<Obj> {
        match self.parents.pop() {
            Some(parent) => {
                self.current = parent;
                vec![Obj::op("Q")]
            }
            None => Vec::new(),
        }
    }

    /// `finalize`: restores every saved state.
    pub(super) fn finish(&mut self) -> Vec<Obj> {
        let mut cmds = Vec::new();
        while !self.parents.is_empty() {
            cmds.extend(self.pop());
        }
        cmds
    }

    /// `clip_cmd`: pops until the clip matches or nothing is saved, then
    /// pushes and narrows the clip if it still differs.
    fn clip_cmd(&mut self, target: &PdfState) -> Vec<Obj> {
        let same =
            |s: &PdfState| s.clip_rect == target.clip_rect && s.clip_path == target.clip_path;
        let mut cmds = Vec::new();
        while !same(&self.current) && !self.parents.is_empty() {
            cmds.extend(self.pop());
        }
        if !same(&self.current) || self.parents.is_empty() {
            cmds.extend(self.push());
            if self.current.clip_rect != target.clip_rect
                && let Some(rect) = &target.clip_rect
            {
                let (x, y, w, h) = rect.value.bounds();
                cmds.push(Obj::Seq(vec![
                    Obj::Real(x),
                    Obj::Real(y),
                    Obj::Real(w),
                    Obj::Real(h),
                ]));
                cmds.extend([Obj::op("re"), Obj::op("W"), Obj::op("n")]);
            }
            if self.current.clip_path != target.clip_path
                && let Some(clip) = &target.clip_path
            {
                let ops = path_ops(&clip.value.path, clip.value.transform, None, None, None);
                cmds.extend([Obj::Raw(ops), Obj::op("W"), Obj::op("n")]);
            }
        }
        cmds
    }

    /// `hatch_cmd`: the fill colour space, colour or hatch pattern.
    fn hatch_cmd(&self, target: &PdfState, resources: &mut Resources) -> Vec<Obj> {
        match &target.hatch {
            Some(pattern) => {
                let name = resources.hatch_pattern(HatchStyle {
                    edge: target.hatch_color,
                    face: match self.current.fill {
                        Fill::Color(c) => Some(c),
                        Fill::InitialBlack => Some(Rgba::BLACK),
                        Fill::None => None,
                    },
                    pattern: pattern.clone(),
                    linewidth: target.hatch_linewidth,
                });
                vec![
                    Obj::name("Pattern"),
                    Obj::op("cs"),
                    Obj::name(name),
                    Obj::op("scn"),
                ]
            }
            _ => match self.current.fill.rgb() {
                Some(rgb) if !self.inherit_color => color_cmd(rgb, "g", "rg"),
                Some(_) => Vec::new(),
                None => vec![Obj::name("DeviceRGB"), Obj::op("cs")],
            },
        }
    }

    /// `delta`: the operators turning the current state into `target`,
    /// which becomes current.
    pub(super) fn delta(&mut self, target: &PdfState, resources: &mut Resources) -> Vec<Obj> {
        let mut cmds = Vec::new();
        let s = &self.current;
        if s.clip_rect != target.clip_rect || s.clip_path != target.clip_path {
            cmds.extend(self.clip_cmd(target));
            self.current.clip_rect.clone_from(&target.clip_rect);
            self.current.clip_path.clone_from(&target.clip_path);
        }
        let s = &self.current;
        if s.alpha != target.alpha
            || s.forced_alpha != target.forced_alpha
            || s.effective_alphas != target.effective_alphas
        {
            let name = resources.alpha_state(target.effective_alphas);
            cmds.extend([Obj::name(name), Obj::op("gs")]);
            self.current.alpha = target.alpha;
            self.current.forced_alpha = target.forced_alpha;
            self.current.effective_alphas = target.effective_alphas;
        }
        if self.current.cap != target.cap {
            cmds.extend([Obj::Int(cap_code(target.cap)), Obj::op("J")]);
            self.current.cap = target.cap;
        }
        let fill_changed = self.current.fill != target.fill;
        if fill_changed {
            if let (Some(rgb), false) = (target.fill.rgb(), self.inherit_color) {
                cmds.extend(color_cmd(rgb, "g", "rg"));
            }
            self.current.fill = target.fill;
        }
        if self.current.join != target.join {
            cmds.extend([Obj::Int(join_code(target.join)), Obj::op("j")]);
            self.current.join = target.join;
        }
        if self.current.linewidth != target.linewidth {
            cmds.extend([Obj::Real(target.linewidth), Obj::op("w")]);
            self.current.linewidth = target.linewidth;
        }
        if self.current.dashes != target.dashes {
            let (offset, pattern) = match &target.dashes {
                (offset, Some(pattern)) => (*offset, Obj::reals(pattern.iter().copied())),
                (_, None) => (0.0, Obj::Array(Vec::new())),
            };
            cmds.extend([pattern, Obj::Real(offset), Obj::op("d")]);
            self.current.dashes.clone_from(&target.dashes);
        }
        if self.current.rgb != target.rgb {
            if !self.inherit_color {
                let c = target.rgb;
                cmds.extend(color_cmd([c.r, c.g, c.b], "G", "RG"));
            }
            self.current.rgb = target.rgb;
        }
        let s = &self.current;
        if fill_changed
            || s.hatch != target.hatch
            || s.hatch_color != target.hatch_color
            || s.hatch_linewidth != target.hatch_linewidth
        {
            cmds.extend(self.hatch_cmd(target, resources));
            self.current.hatch.clone_from(&target.hatch);
            self.current.hatch_color = target.hatch_color;
            self.current.hatch_linewidth = target.hatch_linewidth;
        }
        cmds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gc::GraphicsContext;

    fn ops(cmds: &[Obj]) -> String {
        cmds.iter()
            .map(|c| String::from_utf8(c.repr().unwrap()).unwrap())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn deltas_follow_matplotlib() {
        let mut stack = StateStack::default();
        let mut resources = Resources::default();
        let patch = GraphicsContext {
            foreground: Rgba::WHITE,
            linewidth: 0.0,
            join: JoinStyle::Miter,
            ..GraphicsContext::default()
        };
        let target = PdfState::from_gc(&patch, Some(Rgba::WHITE), 0);
        assert_eq!(
            ops(&stack.delta(&target, &mut resources)),
            "1 g 0 j 0 w 1 G 1 g"
        );
        assert_eq!(ops(&stack.delta(&target, &mut resources)), "");
        assert!(stack.current.fills() && !stack.current.strokes());

        let line = GraphicsContext {
            foreground: Rgba::opaque(0.5, 0.25, 1.0),
            linewidth: 1.5,
            clip_rect: Some(Bbox::from_bounds(1.0, 2.0, 3.0, 4.0)),
            dashes: crate::gc::Dashes::new(0.0, vec![3.7, 1.6]),
            ..GraphicsContext::default()
        };
        let target = PdfState::from_gc(&line, None, 1);
        assert_eq!(
            ops(&stack.delta(&target, &mut resources)),
            "q 1 2 3 4 re W n 1 j 1.5 w [ 3.7 1.6 ] 0 d 0.5 0.25 1 RG /DeviceRGB cs"
        );
        let unclipped = PdfState::from_gc(&GraphicsContext::default(), None, 2);
        let cmds = ops(&stack.delta(&unclipped, &mut resources));
        assert!(cmds.starts_with("Q q "), "{cmds}");
        assert_eq!(ops(&stack.finish()), "Q");
    }

    #[test]
    fn forced_alpha_sets_both_alphas() {
        let gc = GraphicsContext {
            alpha: Some(0.25),
            ..GraphicsContext::default()
        };
        let state = PdfState::from_gc(&gc, Some(Rgba::WHITE), 0);
        assert_eq!(state.effective_alphas, (0.25, 0.25));
        let mut stack = StateStack::default();
        let mut resources = Resources::default();
        assert!(ops(&stack.delta(&state, &mut resources)).starts_with("/A1 gs"));
    }
}
