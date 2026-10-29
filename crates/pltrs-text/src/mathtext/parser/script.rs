//! Sub- and superscripts (matplotlib's `subsuper` action, after TeX's
//! rules 756–759), limits above and below big operators, and primes.

use pltrs_core::numeric::py_max;

use super::Parser;
use super::error::{ErrorKind, Fail, NO_MATCH};
use super::symbol::THIN_SPACE;
use crate::mathtext::boxes::{List, Node, Packing, SHRINK_FACTOR};
use crate::mathtext::data::{is_dropsub_symbol, is_overunder_function, is_overunder_symbol};
use crate::mathtext::fonts::FontConstants;

/// What script placement needs to know about the last item of the
/// nucleus.
#[derive(Clone, Copy, Debug)]
struct LastChar {
    height: f64,
    depth: f64,
    /// An integral: scripts drop and hug the sign.
    dropsub: bool,
    /// Leaning: the superscript moves right.
    slanted: bool,
}

impl LastChar {
    /// Horizontal offsets of the superscript and the subscript.
    fn script_kerns(&self, consts: &FontConstants, x_height: f64) -> (f64, f64) {
        let mut superkern = consts.delta * x_height;
        let mut subkern = consts.delta * x_height;
        if self.slanted {
            superkern += consts.delta * x_height;
            superkern += consts.delta_slanted * (self.height - x_height * 2.0 / 3.0);
            if self.dropsub {
                subkern = (3.0 * consts.delta - consts.delta_integral) * self.height;
                superkern = (3.0 * consts.delta + consts.delta_integral) * self.height;
            } else {
                subkern = 0.0;
            }
        }
        (superkern, subkern)
    }

    fn of(node: &Node) -> Self {
        let (dropsub, slanted) = match node {
            Node::Char(c) => (is_dropsub_symbol(&c.symbol), c.metrics.slanted),
            _ => (false, false),
        };
        Self {
            height: node.height(),
            depth: node.depth(),
            dropsub,
            slanted,
        }
    }
}

/// Whether the nucleus takes limits above and below.
fn is_overunder(nucleus: &Node) -> bool {
    match nucleus {
        Node::Char(c) => is_overunder_symbol(&c.symbol),
        Node::List(list) if !list.vertical => list
            .function_name
            .as_deref()
            .is_some_and(is_overunder_function),
        _ => false,
    }
}

impl Parser<'_, '_> {
    /// Attaches scripts and primes to `nucleus` (an empty box if absent).
    pub(super) fn scripts(
        &mut self,
        loc: usize,
        nucleus: Option<Node>,
        scripts: Vec<(char, Node)>,
        primes: usize,
    ) -> Result<Node, Fail> {
        let nucleus = nucleus.unwrap_or_else(|| Node::hbox(0.0));
        if scripts.is_empty() && primes == 0 {
            return Ok(nucleus);
        }
        let (mut sub, mut sup) = (None, None);
        for (op, arg) in scripts {
            let (slot, msg) = if op == '_' {
                (&mut sub, "Double subscript")
            } else {
                (&mut sup, "Double superscript")
            };
            if slot.is_some() {
                return Err(self.fatal(ErrorKind::Fatal, msg, loc));
            }
            *slot = Some(arg);
        }
        if primes > 0 {
            let mut list = match sup {
                Some(Node::List(list)) if !list.vertical => list,
                Some(other) => List::hlist(vec![other], self.fonts)?,
                None => List::hlist(Vec::new(), self.fonts)?,
            };
            for _ in 0..primes {
                list.children.push(self.symbol(loc, "\\prime")?);
            }
            list.kern(self.fonts)?;
            list.hpack(0.0, Packing::Additional);
            sup = Some(list.into_node());
        }
        if is_overunder(&nucleus) {
            return self.limits(nucleus, sup, sub);
        }
        self.attach(loc, nucleus, sup, sub)
    }

    /// Limits centred above and below a big operator.
    fn limits(
        &mut self,
        nucleus: Node,
        sup: Option<Node>,
        sub: Option<Node>,
    ) -> Result<Node, Fail> {
        let state = *self.state();
        let rule_thickness = self.fonts.underline_thickness(state.fontsize);
        let mut width = nucleus.width();
        let mut shrunk = |node: Option<Node>, width: &mut f64| -> Result<Option<Node>, Fail> {
            node.map(|mut node| {
                node.shrink(self.fonts)?;
                *width = py_max(*width, node.width());
                Ok(node)
            })
            .transpose()
        };
        let sup = shrunk(sup, &mut width)?;
        let sub = shrunk(sub, &mut width)?;
        let centered = |node: Node| {
            let mut list = List::hcentered(vec![node]);
            list.hpack(width, Packing::Exactly);
            list
        };
        let vgap = rule_thickness * 3.0;
        let nucleus_depth = nucleus.depth();
        let mut vlist = Vec::new();
        if let Some(sup) = sup {
            vlist.push(centered(sup).into_node());
            vlist.push(Node::vbox(0.0, vgap));
        }
        vlist.push(centered(nucleus).into_node());
        let mut shift = 0.0;
        if let Some(sub) = sub {
            let sub = centered(sub);
            shift = sub.height + vgap + nucleus_depth;
            vlist.push(Node::vbox(0.0, vgap));
            vlist.push(sub.into_node());
        }
        let mut vlist = List::vlist(vlist);
        vlist.shift_amount = shift;
        self.finish_scripts(vec![vlist.into_node()])
    }

    /// Wraps the scripted nucleus in a list, appending the thin space a
    /// function name asked for, and resets the request.
    fn finish_scripts(&mut self, mut children: Vec<Node>) -> Result<Node, Fail> {
        if self.needs_space_after_subsuper {
            children.push(self.make_space(THIN_SPACE)?);
        }
        self.needs_space_after_subsuper = false;
        Ok(List::hlist(children, self.fonts)?.into_node())
    }

    /// The nucleus as a list whose last character is measured by its
    /// advance (and without its trailing kern).
    fn script_nucleus(&mut self, loc: usize, nucleus: Node) -> Result<(List, LastChar), Fail> {
        match nucleus {
            Node::List(list) if !list.vertical => {
                if list.children.is_empty() {
                    let last = LastChar::of(&Node::List(list));
                    return Ok((List::hlist_unkerned(Vec::new()), last));
                }
                let mut children = list.children;
                let n = children.len();
                if matches!(children[n - 1], Node::Kern { .. }) {
                    // matplotlib indexes the second-to-last child here.
                    let Some(before) = n.checked_sub(2).map(|i| &children[i]) else {
                        return Err(Fail::miss(loc, NO_MATCH));
                    };
                    if matches!(before, Node::Char(_)) {
                        children.pop();
                    }
                }
                if let Some(Node::Char(c)) = children.last_mut() {
                    c.width = c.metrics.advance;
                }
                let last = children
                    .last()
                    .map(LastChar::of)
                    .ok_or(Fail::miss(loc, NO_MATCH))?;
                Ok((List::hlist_unkerned(children), last))
            }
            mut other => {
                if let Node::Char(c) = &mut other {
                    c.width = c.metrics.advance;
                }
                let last = LastChar::of(&other);
                Ok((List::hlist(vec![other], self.fonts)?, last))
            }
        }
    }

    /// Regular sub/superscript placement.
    fn attach(
        &mut self,
        loc: usize,
        nucleus: Node,
        sup: Option<Node>,
        sub: Option<Node>,
    ) -> Result<Node, Fail> {
        let state = *self.state();
        let rule_thickness = self.fonts.underline_thickness(state.fontsize);
        let x_height = self.fonts.x_height(state.font, state.fontsize)?;
        let (nucleus, last) = self.script_nucleus(loc, nucleus)?;
        let consts = *self.fonts.constants();
        let lc_height = last.height;
        let lc_baseline = if last.dropsub { last.depth } else { 0.0 };
        let (superkern, subkern) = last.script_kerns(&consts, x_height);
        let nucleus = nucleus.into_node();
        let (mut shift_up, mut shift_down) = if nucleus.is_char_node() {
            (0.0, 0.0)
        } else {
            let shrunk_x_height = self
                .fonts
                .x_height(state.font, state.fontsize * SHRINK_FACTOR)?;
            (
                nucleus.height() - consts.supdrop * shrunk_x_height,
                nucleus.depth() + consts.subdrop * shrunk_x_height,
            )
        };
        let script = |kern: f64, node: Node, parser: &mut Self| -> Result<List, Fail> {
            let mut list = List::hlist(vec![Node::kern(kern), node], parser.fonts)?;
            list.shrink(parser.fonts)?;
            Ok(list)
        };
        let mut x = match (sup, sub) {
            (None, None) => return Ok(nucleus),
            (None, Some(sub)) => {
                let mut x = script(subkern, sub, self)?;
                shift_down = if last.dropsub {
                    lc_baseline + consts.subdrop * x_height
                } else {
                    py_max(
                        py_max(shift_down, consts.sub1 * x_height),
                        x.height - x_height * 4.0 / 5.0,
                    )
                };
                x.shift_amount = shift_down;
                x
            }
            (Some(sup), sub) => {
                let mut x = script(superkern, sup, self)?;
                shift_up = if last.dropsub {
                    lc_height - consts.subdrop * x_height
                } else {
                    py_max(
                        py_max(shift_up, consts.sup1 * x_height),
                        x.depth + x_height / 4.0,
                    )
                };
                match sub {
                    None => {
                        x.shift_amount = -shift_up;
                        x
                    }
                    Some(sub) => {
                        let y = script(subkern, sub, self)?;
                        shift_down = if last.dropsub {
                            lc_baseline + consts.subdrop * x_height
                        } else {
                            py_max(shift_down, consts.sub2 * x_height)
                        };
                        // Move the subscript down if the scripts are too
                        // close to each other.
                        let mut clr =
                            4.0 * rule_thickness - ((shift_up - x.depth) - (y.height - shift_down));
                        if clr > 0.0 {
                            shift_down += clr;
                            clr = x_height * 4.0 / 5.0 - shift_up + x.depth;
                            if clr > 0.0 {
                                shift_up += clr;
                                shift_down -= clr;
                            }
                        }
                        let gap = (shift_up - x.depth) - (y.height - shift_down);
                        let mut both =
                            List::vlist(vec![x.into_node(), Node::kern(gap), y.into_node()]);
                        both.shift_amount = shift_down;
                        both
                    }
                }
            }
        };
        if !last.dropsub {
            x.width += consts.script_space * x_height;
        }
        self.finish_scripts(vec![nucleus, x.into_node()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn last(slanted: bool, dropsub: bool) -> LastChar {
        LastChar {
            height: 10.0,
            depth: 2.0,
            dropsub,
            slanted,
        }
    }

    #[test]
    fn slanted_nuclei_push_superscripts_right() {
        let consts = FontConstants::BASE;
        let (upright_sup, upright_sub) = last(false, false).script_kerns(&consts, 6.0);
        assert_eq!(upright_sup, upright_sub);
        let (sup, sub) = last(true, false).script_kerns(&consts, 6.0);
        assert!(sup > upright_sup);
        assert_eq!(sub, 0.0);
        let (sup, sub) = last(true, true).script_kerns(&consts, 6.0);
        assert!(sup > 0.0 && sub < 0.0, "integral scripts hug the sign");
    }
}
