//! Packing lists: natural sizes and glue settings (TeX's `hpack` and
//! `vpack`).

use pltrs_core::numeric::py_max;

use super::{List, MathNode, Node};
use crate::glue_warnings;

/// How the requested size of a packed list is interpreted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Packing {
    /// Exactly the given size.
    Exactly,
    /// The natural size plus the given amount.
    Additional,
}

/// Glue totals per order of infinity.
#[derive(Default)]
struct Totals {
    stretch: [f64; 4],
    shrink: [f64; 4],
}

impl Totals {
    fn add(&mut self, node: &Node) -> f64 {
        match node {
            Node::Glue { spec, .. } => {
                self.stretch[spec.stretch_order] += spec.stretch;
                self.shrink[spec.shrink_order] += spec.shrink;
                spec.width
            }
            _ => 0.0,
        }
    }
}

impl List {
    /// Computes the width, height and depth of an `Hlist` and sets its
    /// glue so that the width is `width` (exactly or in addition to the
    /// natural width).
    pub(crate) fn hpack(&mut self, width: f64, packing: Packing) {
        let (mut h, mut d, mut x) = (0.0, 0.0, 0.0);
        let mut totals = Totals::default();
        for node in &self.children {
            match node {
                Node::Char(c) => {
                    x += c.width;
                    h = py_max(h, c.height);
                    d = py_max(d, c.depth);
                }
                Node::Box(_) | Node::Rule(_) | Node::List(_) => {
                    x += node.width();
                    let (height, depth) = (node.height(), node.depth());
                    if !height.is_infinite() && !depth.is_infinite() {
                        let s = node.shift_amount();
                        h = py_max(h, height - s);
                        d = py_max(d, depth + s);
                    }
                }
                Node::Glue { .. } => x += totals.add(node),
                Node::Kern { width, .. } => x += width,
            }
        }
        self.height = h;
        self.depth = d;
        let w = match packing {
            Packing::Additional => width + x,
            Packing::Exactly => width,
        };
        self.width = w;
        self.set_glue(w - x, &totals);
    }

    /// Computes the size of a `Vlist` and sets its glue so that the height
    /// is `height` (exactly or additionally); the depth is limited to
    /// `max_depth`.
    pub(crate) fn vpack(&mut self, height: f64, packing: Packing, max_depth: f64) {
        let (mut w, mut d, mut x) = (0.0, 0.0, 0.0);
        let mut totals = Totals::default();
        for node in &self.children {
            match node {
                Node::Box(_) | Node::Rule(_) | Node::List(_) => {
                    x += d + node.height();
                    d = node.depth();
                    let width = node.width();
                    if !width.is_infinite() {
                        w = py_max(w, width + node.shift_amount());
                    }
                }
                Node::Glue { .. } => {
                    x += d;
                    d = 0.0;
                    x += totals.add(node);
                }
                Node::Kern { width, .. } => {
                    x += d + width;
                    d = 0.0;
                }
                // Characters are always wrapped in an Hlist before
                // entering a vertical list.
                Node::Char(_) => {}
            }
        }
        self.width = w;
        if d > max_depth {
            x += d - max_depth;
            self.depth = max_depth;
        } else {
            self.depth = d;
        }
        let h = match packing {
            Packing::Additional => height + x,
            Packing::Exactly => height,
        };
        self.height = h;
        self.set_glue(h - x, &totals);
    }

    /// Sets the glue to absorb `excess` (stretching when positive).
    fn set_glue(&mut self, excess: f64, totals: &Totals) {
        if excess == 0.0 {
            self.glue_sign = 0;
            self.glue_order = 0;
            return;
        }
        let (sign, totals) = if excess > 0.0 {
            (1, &totals.stretch)
        } else {
            (-1, &totals.shrink)
        };
        let order = (0..totals.len())
            .rev()
            .find(|&i| totals[i] != 0.0)
            .unwrap_or(0);
        self.glue_order = order;
        self.glue_sign = sign;
        if totals[order] == 0.0 {
            self.glue_sign = 0;
        } else {
            self.glue_set = excess / totals[order];
        }
        if order == 0 && !self.children.is_empty() {
            // No glue of infinite stretchability takes the excess:
            // matplotlib logs the list.
            let error = if sign > 0 { "Overful" } else { "Underful" };
            glue_warnings::record(format!(
                "{error} {}: {}",
                self.class_name(),
                MathNode::from(&*self).python_repr()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::GlueSpec;
    use super::*;

    #[test]
    fn centred_content_stretches_glue() {
        let mut list = List::hcentered(vec![Node::hbox(4.0)]);
        assert_eq!((list.width, list.glue_sign), (4.0, 0));
        list.hpack(10.0, Packing::Exactly);
        assert_eq!((list.glue_sign, list.glue_order), (1, 1));
        assert_eq!(list.glue_set, 3.0);
        list.hpack(2.0, Packing::Exactly);
        assert_eq!((list.glue_sign, list.glue_set), (-1, 1.0));
    }

    #[test]
    fn rigid_lists_keep_their_glue_ratio() {
        let mut list = List::hlist_unkerned(vec![Node::hbox(1.0), Node::kern(2.0)]);
        list.glue_set = 0.5;
        list.hpack(5.0, Packing::Exactly);
        assert_eq!((list.glue_sign, list.glue_set), (0, 0.5));
    }

    #[test]
    fn vertical_lists_limit_depth() {
        let mut list = List::vlist(vec![
            Node::vbox(1.0, 1.0),
            Node::glue(GlueSpec::FILL),
            Node::vbox(2.0, 3.0),
        ]);
        assert_eq!((list.height, list.depth), (4.0, 3.0));
        list.vpack(10.0, Packing::Exactly, 1.0);
        assert_eq!((list.height, list.depth), (10.0, 1.0));
        assert_eq!(
            (list.glue_sign, list.glue_order, list.glue_set),
            (1, 2, 4.0)
        );
    }
}
