//! `\left ... \middle ... \right`: delimiters sized to their content.

use pltrs_core::numeric::py_max;

use super::error::{ErrorKind, Fail};
use super::{Parsed, Parser};
use crate::error::TextError;
use crate::mathtext::boxes::{List, Node, auto_height_char};

/// An item between `\left` and `\right`: a node, or a `\middle`
/// delimiter not yet sized.
#[derive(Debug)]
pub(super) enum Item {
    /// Typeset content.
    Node(Node),
    /// The `\middle` marker.
    Middle,
    /// The delimiter following `\middle`.
    Delimiter(&'static str),
}

impl Parser<'_, '_> {
    /// A delimiter after `loc`, or the fatal "Expected a delimiter".
    fn expect_delimiter(&self, loc: usize) -> Parsed<&'static str> {
        let at = self.skip_ws(loc);
        self.delimiter(at)
            .ok_or_else(|| self.fatal(ErrorKind::Syntax, "Expected a delimiter", at))
    }

    /// `\left delim (simple | \middle delim)* \right delim`.
    pub(super) fn auto_delim(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\left")?;
        Some(self.auto_delim_body(after))
    }

    fn auto_delim_body(&mut self, after: usize) -> Parsed<Node> {
        let (mut pos, left) = self.expect_delimiter(after)?;
        let mut items = Vec::new();
        loop {
            match self.simple(pos) {
                Ok((end, node)) => {
                    items.extend(node.map(Item::Node));
                    pos = end;
                    continue;
                }
                Err(Fail::Soft(_)) => {}
                Err(fatal) => return Err(fatal),
            }
            let Some(after_middle) = self.literal(self.skip_ws(pos), "\\middle") else {
                break;
            };
            let (end, delim) = self.expect_delimiter(after_middle)?;
            items.push(Item::Middle);
            items.push(Item::Delimiter(delim));
            pos = end;
        }
        let at = self.skip_ws(pos);
        let Some(after_right) = self.literal(at, "\\right") else {
            return Err(self.fatal(ErrorKind::Syntax, "Expected '\\\\right'", at));
        };
        let (end, right) = self.expect_delimiter(after_right)?;
        Ok((
            end,
            self.auto_sized_delimiter(left, items, right)?.into_node(),
        ))
    }

    /// matplotlib's `_auto_sized_delimiter`: delimiters as tall as the
    /// items between them (`.` draws nothing).
    pub(super) fn auto_sized_delimiter(
        &mut self,
        front: &str,
        mut middle: Vec<Item>,
        back: &str,
    ) -> Result<List, Fail> {
        let state = *self.state();
        let (height, depth, factor) = if middle.is_empty() {
            (0.0, 0.0, Some(1.0))
        } else {
            let nodes = || {
                middle.iter().filter_map(|item| match item {
                    Item::Node(node) => Some(node),
                    _ => None,
                })
            };
            let height = nodes()
                .map(Node::height)
                .reduce(py_max)
                .ok_or_else(Fail::empty_max)?;
            let depth = nodes()
                .map(Node::depth)
                .reduce(py_max)
                .ok_or_else(Fail::empty_max)?;
            // Python edits the list while enumerating it: removing an item
            // makes the loop skip the one after it.
            let mut index = 0;
            while index < middle.len() {
                if matches!(middle[index], Item::Middle) {
                    match middle.get(index + 1) {
                        Some(&Item::Delimiter(".")) => {
                            // `list.remove` drops the first equal item.
                            let first = middle
                                .iter()
                                .position(|item| matches!(item, Item::Delimiter(".")))
                                .unwrap_or(index + 1);
                            middle.remove(first);
                        }
                        Some(&Item::Delimiter(delim)) => {
                            let sized =
                                auto_height_char(delim, height, depth, &state, None, self.fonts)?;
                            middle[index + 1] = Item::Node(sized.into_node());
                        }
                        _ => {}
                    }
                    middle.remove(index);
                }
                index += 1;
            }
            (height, depth, None)
        };
        let mut parts = Vec::with_capacity(middle.len() + 2);
        if front != "." {
            parts.push(
                auto_height_char(front, height, depth, &state, factor, self.fonts)?.into_node(),
            );
        }
        for item in middle {
            match item {
                Item::Node(node) => parts.push(node),
                Item::Middle | Item::Delimiter(_) => {
                    return Err(Fail::Fatal(TextError::Math(
                        "'str' object has no attribute 'get_kerning'".to_owned(),
                    )));
                }
            }
        }
        if back != "." {
            parts.push(
                auto_height_char(back, height, depth, &state, factor, self.fonts)?.into_node(),
            );
        }
        Ok(List::hlist(parts, self.fonts)?)
    }
}
