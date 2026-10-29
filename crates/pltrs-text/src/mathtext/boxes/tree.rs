//! The box tree of a typeset formula as matplotlib's `Parser.parse`
//! returns it: the nodes named by matplotlib's classes, with the sizes of
//! the lists.

use super::{DimsKind, List, ListClass, Node};

/// A node of the box tree mathtext typesets a formula into, as
/// matplotlib's `_mathtext.Parser.parse` returns it. Sizes are in pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum MathNode {
    /// A character (`Char`, or `Accent`): its symbol (`x`, `\alpha`).
    Char(String),
    /// Fixed space (`Kern`): its width.
    Kern(f64),
    /// Stretchable space (`Glue`).
    Glue,
    /// An empty box or a rule: matplotlib's class name (`Box`, `Hbox`,
    /// `Vbox` or `Hrule`).
    Leaf(&'static str),
    /// A list of nodes.
    List {
        /// matplotlib's class name: `Hlist`, `Vlist`, `HCentered`,
        /// `AutoHeightChar` or `AutoWidthChar`.
        class: &'static str,
        /// Width.
        width: f64,
        /// Height.
        height: f64,
        /// Depth.
        depth: f64,
        /// Offset perpendicular to the enclosing list.
        shift_amount: f64,
        /// The content.
        children: Vec<MathNode>,
    },
}

impl From<&List> for MathNode {
    fn from(list: &List) -> Self {
        let class = match (list.class, list.vertical) {
            (ListClass::Plain, false) => "Hlist",
            (ListClass::Plain, true) => "Vlist",
            (ListClass::Centered, _) => "HCentered",
            (ListClass::AutoHeightChar, _) => "AutoHeightChar",
            (ListClass::AutoWidthChar, _) => "AutoWidthChar",
        };
        Self::List {
            class,
            width: list.width,
            height: list.height,
            depth: list.depth,
            shift_amount: list.shift_amount,
            children: list.children.iter().map(Self::from).collect(),
        }
    }
}

impl From<&Node> for MathNode {
    fn from(node: &Node) -> Self {
        match node {
            Node::Char(c) => Self::Char(c.symbol.clone()),
            Node::Kern { width, .. } => Self::Kern(*width),
            Node::Glue { .. } => Self::Glue,
            Node::Box(dims) | Node::Rule(dims) => Self::Leaf(match dims.kind {
                DimsKind::Box => "Box",
                DimsKind::Hbox => "Hbox",
                DimsKind::Vbox => "Vbox",
                DimsKind::Hrule => "Hrule",
            }),
            Node::List(list) => Self::from(list),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mathtext::boxes::GlueSpec;

    #[test]
    fn nodes_take_matplotlibs_class_names() {
        let centred = List::hcentered(vec![Node::hbox(2.0), Node::kern(1.5)]);
        let list = List::vlist(vec![
            centred.into_node(),
            Node::vbox(1.0, 0.0),
            Node::hrule(0.5),
            Node::glue(GlueSpec::FILL),
        ]);
        let MathNode::List {
            class, children, ..
        } = MathNode::from(&list)
        else {
            panic!("a list");
        };
        assert_eq!(class, "Vlist");
        assert_eq!(
            &children[1..],
            [
                MathNode::Leaf("Vbox"),
                MathNode::Leaf("Hrule"),
                MathNode::Glue
            ]
        );
        let MathNode::List {
            class, children, ..
        } = &children[0]
        else {
            panic!("a list");
        };
        assert_eq!(*class, "HCentered");
        assert_eq!(
            children[1..3],
            [MathNode::Leaf("Hbox"), MathNode::Kern(1.5)]
        );
    }
}
