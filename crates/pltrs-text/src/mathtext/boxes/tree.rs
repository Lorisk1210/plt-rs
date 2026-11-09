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

impl MathNode {
    /// The node as matplotlib's `repr` writes it (its `Node.__repr__`):
    /// a character as `` `x` ``, a kern as `k1.50`, an empty box, a rule
    /// or glue by its class name, a list by its class name and sizes,
    /// then its children indented, one per line.
    ///
    /// ```
    /// use pltrs_text::mathtext::MathNode;
    /// let list = MathNode::List {
    ///     class: "Hlist", width: f64::NAN, height: 7.42, depth: 0.0, shift_amount: -0.0,
    ///     children: vec![MathNode::Kern(f64::NAN), MathNode::Char("2".into())],
    /// };
    /// assert_eq!(list.python_repr(), "Hlist<w=nan h=7.42 d=0.00 s=-0.00>[\n  knan,\n  `2`,\n]");
    /// ```
    pub fn python_repr(&self) -> String {
        match self {
            Self::Char(symbol) => format!("`{symbol}`"),
            Self::Kern(width) => format!("k{}", python_fixed(*width)),
            Self::Glue => "Glue".to_owned(),
            Self::Leaf(class) => (*class).to_owned(),
            Self::List {
                class,
                width,
                height,
                depth,
                shift_amount,
                children,
            } => {
                let content = if children.is_empty() {
                    String::new()
                } else {
                    let lines: Vec<String> = children
                        .iter()
                        .map(|child| format!("{},", child.python_repr()))
                        .collect();
                    format!("\n{}\n", indented(&lines.join("\n")))
                };
                format!(
                    "{class}<w={} h={} d={} s={}>[{content}]",
                    python_fixed(*width),
                    python_fixed(*height),
                    python_fixed(*depth),
                    python_fixed(*shift_amount)
                )
            }
        }
    }
}

/// `value` with two decimals, as Python's `"{:.02f}"` writes it (`nan`,
/// `inf`, `-inf`, a sign kept on negative zero).
fn python_fixed(value: f64) -> String {
    if value.is_nan() {
        "nan".to_owned()
    } else if value.is_infinite() {
        if value > 0.0 { "inf" } else { "-inf" }.to_owned()
    } else {
        format!("{value:.2}")
    }
}

/// `text` with every line that is not blank indented by two spaces
/// (Python's `textwrap.indent(text, "  ")`).
fn indented(text: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| {
            if line.trim().is_empty() {
                line.to_owned()
            } else {
                format!("  {line}")
            }
        })
        .collect()
}

impl List {
    /// The class matplotlib made the list as.
    pub(crate) fn class_name(&self) -> &'static str {
        match (self.class, self.vertical) {
            (ListClass::Plain, false) => "Hlist",
            (ListClass::Plain, true) => "Vlist",
            (ListClass::Centered, _) => "HCentered",
            (ListClass::AutoHeightChar, _) => "AutoHeightChar",
            (ListClass::AutoWidthChar, _) => "AutoWidthChar",
        }
    }
}

impl From<&List> for MathNode {
    fn from(list: &List) -> Self {
        Self::List {
            class: list.class_name(),
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
    fn reprs_write_as_matplotlibs() {
        let inner = MathNode::List {
            class: "Hlist",
            width: 5.92,
            height: 5.47,
            depth: 0.0,
            shift_amount: 0.0,
            children: vec![MathNode::Char("x".to_owned())],
        };
        let outer = MathNode::List {
            class: "Hlist",
            width: f64::NAN,
            height: f64::INFINITY,
            depth: 0.0,
            shift_amount: 0.0,
            children: vec![inner, MathNode::Glue, MathNode::Leaf("Hbox")],
        };
        assert_eq!(
            outer.python_repr(),
            "Hlist<w=nan h=inf d=0.00 s=0.00>[\n  Hlist<w=5.92 h=5.47 d=0.00 s=0.00>[\n    `x`,\n  ],\n  Glue,\n  Hbox,\n]"
        );
        let empty = MathNode::List {
            class: "Vlist",
            width: 0.0,
            height: 0.0,
            depth: 0.0,
            shift_amount: 0.0,
            children: Vec::new(),
        };
        assert_eq!(empty.python_repr(), "Vlist<w=0.00 h=0.00 d=0.00 s=0.00>[]");
    }

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
