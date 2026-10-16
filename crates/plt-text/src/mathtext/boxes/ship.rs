//! Shipping out a box: absolute positions of every glyph and rule (TeX's
//! `hlist_out` / `vlist_out`).

use super::{List, Node};
use crate::mathtext::fonts::{FontInfo, Fonts, GlyphError};

/// The positioned content of a shipped box, y downwards from its top.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Shipped {
    /// Glyph origins and the glyphs drawn there.
    pub(crate) glyphs: Vec<(f64, f64, FontInfo)>,
    /// Rules as `(x, y, width, height)` with `y` the top edge.
    pub(crate) rects: Vec<(f64, f64, f64, f64)>,
}

/// Ships `root` out with its top-left corner at the origin.
pub(crate) fn ship(root: &List, fonts: &mut Fonts<'_>) -> Result<Shipped, GlyphError> {
    let mut shipper = Shipper {
        fonts,
        cur_h: 0.0,
        cur_v: 0.0,
        off_v: root.height,
        phantom: Vec::new(),
        out: Shipped::default(),
    };
    shipper.hlist_out(root)?;
    Ok(shipper.out)
}

struct Shipper<'a, 'e> {
    fonts: &'a mut Fonts<'e>,
    cur_h: f64,
    cur_v: f64,
    off_v: f64,
    phantom: Vec<bool>,
    out: Shipped,
}

/// Limits a glue amount to ±10⁹ like TeX.
fn clamp(value: f64) -> f64 {
    value.clamp(-1e9, 1e9)
}

/// The glue amount used so far, rounded like Python's `round`.
fn glue_amount(list: &List, cur_glue: &mut f64, cur_g: f64, spec: &super::GlueSpec) -> f64 {
    match list.glue_sign {
        1 if spec.stretch_order == list.glue_order => {
            *cur_glue += spec.stretch;
            clamp(list.glue_set * *cur_glue).round_ties_even()
        }
        -1 if spec.shrink_order == list.glue_order => {
            *cur_glue += spec.shrink;
            clamp(list.glue_set * *cur_glue).round_ties_even()
        }
        _ => cur_g,
    }
}

impl Shipper<'_, '_> {
    fn visible(&self) -> bool {
        !self.phantom.iter().any(|&hidden| hidden)
    }

    fn glyph(&mut self, node: &Node, x: f64, y: f64) -> Result<(), GlyphError> {
        if let Node::Char(c) = node
            && self.visible()
        {
            let info = self.fonts.info(c.key())?;
            let (x, y) = if c.accent {
                (x - c.metrics.xmin, y + c.metrics.ymin)
            } else {
                (x, y)
            };
            self.out.glyphs.push((x, y, info));
        }
        Ok(())
    }

    fn rule(&mut self, node: &Node, x: f64, y: f64, width: f64, height: f64) {
        if matches!(node, Node::Rule(_)) && self.visible() {
            self.out.rects.push((x, y - height, width, height));
        }
    }

    fn list_out(&mut self, list: &List) -> Result<(), GlyphError> {
        if list.vertical {
            self.vlist_out(list)
        } else {
            self.hlist_out(list)
        }
    }

    fn hlist_out(&mut self, list: &List) -> Result<(), GlyphError> {
        let mut cur_g = 0.0;
        let mut cur_glue = 0.0;
        let base_line = self.cur_v;
        self.phantom.push(list.is_phantom);
        for node in &list.children {
            match node {
                Node::Char(c) => {
                    self.glyph(node, self.cur_h, self.cur_v + self.off_v)?;
                    self.cur_h += c.width;
                }
                Node::Kern { width, .. } => self.cur_h += width,
                Node::List(inner) if inner.children.is_empty() => self.cur_h += inner.width,
                Node::List(inner) => {
                    let edge = self.cur_h;
                    self.cur_v = base_line + inner.shift_amount;
                    self.list_out(inner)?;
                    self.cur_h = edge + inner.width;
                    self.cur_v = base_line;
                }
                Node::Box(dims) | Node::Rule(dims) => {
                    let height = if dims.height.is_infinite() {
                        list.height
                    } else {
                        dims.height
                    };
                    let depth = if dims.depth.is_infinite() {
                        list.depth
                    } else {
                        dims.depth
                    };
                    if height > 0.0 && dims.width > 0.0 {
                        self.cur_v = base_line + depth;
                        self.rule(
                            node,
                            self.cur_h,
                            self.cur_v + self.off_v,
                            dims.width,
                            height,
                        );
                        self.cur_v = base_line;
                    }
                    self.cur_h += dims.width;
                }
                Node::Glue { spec, .. } => {
                    let mut width = spec.width - cur_g;
                    cur_g = glue_amount(list, &mut cur_glue, cur_g, spec);
                    width += cur_g;
                    self.cur_h += width;
                }
            }
        }
        self.phantom.pop();
        Ok(())
    }

    fn vlist_out(&mut self, list: &List) -> Result<(), GlyphError> {
        let mut cur_g = 0.0;
        let mut cur_glue = 0.0;
        let left_edge = self.cur_h;
        self.cur_v -= list.height;
        for node in &list.children {
            match node {
                Node::Kern { width, .. } => self.cur_v += width,
                Node::List(inner) if inner.children.is_empty() => {
                    self.cur_v += inner.height + inner.depth;
                }
                Node::List(inner) => {
                    self.cur_v += inner.height;
                    self.cur_h = left_edge + inner.shift_amount;
                    let save_v = self.cur_v;
                    self.list_out(inner)?;
                    self.cur_v = save_v + inner.depth;
                    self.cur_h = left_edge;
                }
                Node::Box(dims) | Node::Rule(dims) => {
                    let width = if dims.width.is_infinite() {
                        list.width
                    } else {
                        dims.width
                    };
                    let height = dims.height + dims.depth;
                    if height > 0.0 && dims.depth > 0.0 {
                        self.cur_v += height;
                        self.rule(node, self.cur_h, self.cur_v + self.off_v, width, height);
                    }
                }
                Node::Glue { spec, .. } => {
                    let mut height = spec.width - cur_g;
                    cur_g = glue_amount(list, &mut cur_glue, cur_g, spec);
                    height += cur_g;
                    self.cur_v += height;
                }
                // Characters are always wrapped in an Hlist inside a Vlist.
                Node::Char(_) => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_and_round_like_python() {
        assert_eq!(clamp(2e9), 1e9);
        assert_eq!(clamp(-2e9), -1e9);
        assert_eq!(2.5_f64.round_ties_even(), 2.0);
        assert_eq!(3.5_f64.round_ties_even(), 4.0);
    }
}
