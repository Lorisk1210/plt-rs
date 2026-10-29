//! Commands with arguments: phantoms and laps, accents, over/under sets,
//! radicals, over/underlines and stacks.

use pltrs_core::numeric::py_max;

use super::error::{ErrorKind, Fail, NO_MATCH};
use super::{Parsed, Parser};
use crate::error::TextError;
use crate::mathtext::boxes::{
    Char, Dims, GlueSpec, List, Node, Packing, auto_height_char, auto_width_char,
};
use crate::mathtext::data::{accent_symbol, is_wide_accent};

/// What `\phantom`, `\llap` and `\rlap` do with their argument.
#[derive(Clone, Copy)]
enum Lap {
    /// Invisible but taking space.
    Phantom,
    /// Overlapping to the left.
    Left,
    /// Overlapping to the right.
    Right,
}

impl Parser<'_, '_> {
    /// `\phantom`, `\llap` and `\rlap`.
    pub(super) fn phantom_like(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let (after, lap, signature) = [
            ("\\phantom", Lap::Phantom, "\\phantom{value}"),
            ("\\llap", Lap::Left, "\\llap{value}"),
            ("\\rlap", Lap::Right, "\\rlap{value}"),
        ]
        .into_iter()
        .find_map(|(name, lap, signature)| {
            self.literal(loc, name).map(|after| (after, lap, signature))
        })?;
        Some(self.lap(after, lap, signature))
    }

    fn lap(&mut self, after: usize, lap: Lap, signature: &str) -> Parsed<Node> {
        let (end, mut value) = self.args(after, signature, |p, loc| p.braced(loc, false))?;
        let width = value.width;
        let children = match lap {
            Lap::Phantom => {
                value.is_phantom = true;
                return Ok((end, value.into_node()));
            }
            Lap::Left => vec![Node::kern(-width), value.into_node()],
            Lap::Right => vec![value.into_node(), Node::kern(-width)],
        };
        Ok((end, List::hlist(children, self.fonts)?.into_node()))
    }

    /// An accent over the following placeable.
    pub(super) fn accent(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let (after, name) = self.accent_command(loc)?;
        let at = self.skip_ws(after);
        Some(match self.placeable(at) {
            Ok((end, body)) => self.accent_action(&name, body).map(|node| (end, node)),
            Err(Fail::Soft(miss)) => Err(self.fatal(ErrorKind::Syntax, miss.msg, miss.loc)),
            Err(fatal) => Err(fatal),
        })
    }

    fn accent_action(&mut self, name: &str, body: Node) -> Result<Node, Fail> {
        let state = *self.state();
        let thickness = self.fonts.underline_thickness(state.fontsize);
        let width = body.width();
        let mut centered = if is_wide_accent(name) {
            let accent = auto_width_char(&format!("\\{name}"), width, &state, true, self.fonts)?;
            List::hcentered(vec![accent.into_node()])
        } else {
            let symbol = accent_symbol(name).ok_or(Fail::miss(0, NO_MATCH))?;
            let mut accent = Node::Char(Char::accent(symbol, &state, self.fonts)?);
            if name == "mathring" {
                accent.shrink(self.fonts)?;
                accent.shrink(self.fonts)?;
            }
            List::hcentered(vec![Node::hbox(width / 4.0), accent])
        };
        centered.hpack(width, Packing::Exactly);
        let body = List::hlist(vec![body], self.fonts)?;
        Ok(List::vlist(vec![
            centered.into_node(),
            Node::vbox(0.0, thickness * 2.0),
            body.into_node(),
        ])
        .into_node())
    }

    /// `\overset{annotation}{body}`.
    pub(super) fn overset(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\overset")?;
        Some(self.genset(after, "\\overset{annotation}{body}", false))
    }

    /// `\underset{annotation}{body}`.
    pub(super) fn underset(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\underset")?;
        Some(self.genset(after, "\\underset{annotation}{body}", true))
    }

    fn genset(&mut self, after: usize, signature: &str, under: bool) -> Parsed<Node> {
        let (end, (mut annotation, body)) = self.args(after, signature, |p, loc| {
            let (mid, annotation) = p.braced(loc, false)?;
            let (end, body) = p.braced(mid, false)?;
            Ok((end, (annotation, body)))
        })?;
        let thickness = self.fonts.underline_thickness(self.state().fontsize);
        annotation.shrink(self.fonts)?;
        let mut annotation = List::hcentered(vec![annotation.into_node()]);
        let mut body = List::hcentered(vec![body.into_node()]);
        let width = py_max(annotation.width, body.width);
        annotation.hpack(width, Packing::Exactly);
        body.hpack(width, Packing::Exactly);
        let vgap = thickness * 3.0;
        let vlist = if under {
            let shift = body.depth + annotation.height + vgap;
            let mut vlist = List::vlist(vec![
                body.into_node(),
                Node::vbox(0.0, vgap),
                annotation.into_node(),
            ]);
            vlist.shift_amount = shift;
            vlist
        } else {
            List::vlist(vec![
                annotation.into_node(),
                Node::vbox(0.0, vgap),
                body.into_node(),
            ])
        };
        Ok((end, vlist.into_node()))
    }

    /// `\sqrt[root]{value}`.
    pub(super) fn sqrt(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\sqrt")?;
        Some(self.sqrt_body(after))
    }

    /// The optional `[root]`: at least one token and the closing `]`.
    fn sqrt_root(&mut self, loc: usize) -> Parsed<Option<Vec<Node>>> {
        let open = self.skip_ws(loc);
        if self.at(open) != Some('[') {
            return Ok((loc, None));
        }
        let (mut pos, mut nodes, mut count) = (open + 1, Vec::new(), 0);
        while self.at(self.skip_ws(pos)) != Some(']') {
            match self.token(pos) {
                Ok((end, node)) => {
                    nodes.extend(node);
                    count += 1;
                    pos = end;
                }
                Err(Fail::Soft(_)) => break,
                Err(fatal) => return Err(fatal),
            }
        }
        let close = self.skip_ws(pos);
        if count == 0 || self.at(close) != Some(']') {
            return Ok((loc, None));
        }
        Ok((close + 1, Some(nodes)))
    }

    fn sqrt_body(&mut self, after: usize) -> Parsed<Node> {
        let (end, (root, body)) = self.args(after, "\\sqrt{value}", |p, loc| {
            let (pos, root) = p.sqrt_root(loc)?;
            let (end, body) = p.braced(pos, true)?;
            Ok((end, (root, body)))
        })?;
        let state = *self.state();
        let thickness = self.fonts.underline_thickness(state.fontsize);
        // Leave a little room above the body.
        let height = body.height - body.shift_amount + 5.0 * thickness;
        let depth = body.depth + body.shift_amount;
        let check = auto_height_char("\\__sqrt__", height, depth, &state, None, self.fonts)?;
        let height = check.height - check.shift_amount;
        let depth = check.depth + check.shift_amount;
        let padded = List::hlist(
            vec![
                Node::hbox(2.0 * thickness),
                body.into_node(),
                Node::hbox(2.0 * thickness),
            ],
            self.fonts,
        )?;
        let mut rightside = List::vlist(vec![
            Node::hrule(thickness),
            Node::glue(GlueSpec::FILL),
            padded.into_node(),
        ]);
        let extra = (state.fontsize * self.fonts.dpi()) / (100.0 * 12.0);
        rightside.vpack(height + extra, Packing::Exactly, depth);
        let root = match root.filter(|nodes| !nodes.is_empty()) {
            None => Node::Box(Dims::new(0.5 * check.width, 0.0, 0.0)),
            Some(nodes) => {
                let mut root = List::hlist(nodes, self.fonts)?;
                root.shrink(self.fonts)?;
                root.shrink(self.fonts)?;
                root.into_node()
            }
        };
        let mut root = List::vlist(vec![List::hlist(vec![root], self.fonts)?.into_node()]);
        // Raise the root above the tick.
        root.shift_amount = -height * 0.6;
        let check_width = check.width;
        let list = List::hlist(
            vec![
                root.into_node(),
                Node::kern(-0.5 * check_width),
                check.into_node(),
                rightside.into_node(),
            ],
            self.fonts,
        )?;
        Ok((end, list.into_node()))
    }

    /// `\overline{body}`.
    pub(super) fn overline(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\overline")?;
        Some(self.overline_body(after))
    }

    fn overline_body(&mut self, after: usize) -> Parsed<Node> {
        let (end, body) = self.args(after, "\\overline{body}", |p, loc| p.braced(loc, true))?;
        let state = *self.state();
        let thickness = self.fonts.underline_thickness(state.fontsize);
        let height = body.height - body.shift_amount + 3.0 * thickness;
        let depth = body.depth + body.shift_amount;
        let body = List::hlist(vec![body.into_node()], self.fonts)?;
        let mut rightside = List::vlist(vec![
            Node::hrule(thickness),
            Node::glue(GlueSpec::FILL),
            body.into_node(),
        ]);
        let extra = (state.fontsize * self.fonts.dpi()) / (100.0 * 12.0);
        rightside.vpack(height + extra, Packing::Exactly, depth);
        let list = List::hlist(vec![rightside.into_node()], self.fonts)?;
        Ok((end, list.into_node()))
    }

    /// `\underline{body}`.
    pub(super) fn underline(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\underline")?;
        Some(self.underline_body(after))
    }

    fn underline_body(&mut self, after: usize) -> Parsed<Node> {
        let (end, body) = self.args(after, "\\underline{body}", |p, loc| p.braced(loc, true))?;
        let thickness = self.fonts.underline_thickness(self.state().fontsize);
        let body_height = body.height;
        let body = List::hlist(vec![body.into_node()], self.fonts)?;
        let mut vlist = List::vlist(vec![
            body.into_node(),
            Node::kern(3.0 * thickness),
            Node::hrule(thickness),
        ]);
        let delta = vlist.height + vlist.depth + thickness;
        vlist.height = body_height;
        vlist.depth = delta - vlist.height;
        let list = List::hlist(vec![vlist.into_node()], self.fonts)?;
        Ok((end, list.into_node()))
    }

    /// `\substack{row \\ row ...}`: centred rows.
    pub(super) fn substack(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\substack")?;
        Some(self.substack_body(after))
    }

    fn substack_rows(&mut self, loc: usize) -> Parsed<Vec<Vec<Node>>> {
        let mut pos = self.expect_char(loc, '{')?;
        let mut rows = Vec::new();
        loop {
            let at = self.skip_ws(pos);
            if self.at(at) == Some('{') {
                // A nested brace group is a list matplotlib cannot typeset.
                return Err(Fail::Fatal(TextError::Math(
                    "'str' object is not callable".to_owned(),
                )));
            }
            let (end, nodes, count) = self.tokens(pos)?;
            if count == 0 {
                break;
            }
            rows.push(nodes);
            pos = end;
            while let Some(next) = self.row_separator(self.skip_ws(pos)) {
                pos = next;
            }
        }
        Ok((self.expect_char(pos, '}')?, rows))
    }

    fn substack_body(&mut self, after: usize) -> Parsed<Node> {
        let (end, rows) = self.args(after, "\\substack{parts}", Self::substack_rows)?;
        let thickness = self.fonts.underline_thickness(self.state().fontsize);
        let rows = rows
            .into_iter()
            .map(|nodes| List::hlist(nodes, self.fonts))
            .collect::<Result<Vec<_>, _>>()?;
        let max_width = rows
            .iter()
            .map(|row| row.width)
            .reduce(py_max)
            .ok_or_else(Fail::empty_max)?;
        // matplotlib separates the rows with one `Vbox` object repeated.
        let separators = u32::try_from(rows.len().saturating_sub(1)).unwrap_or(u32::MAX);
        let mut stack = Vec::with_capacity(rows.len() * 2);
        for row in rows {
            let mut centered = List::hcentered(vec![row.into_node()]);
            centered.hpack(max_width, Packing::Exactly);
            stack.push(centered.into_node());
            stack.push(Node::shared_vbox(0.0, thickness * 2.0, separators));
        }
        stack.pop();
        let list = List::hlist(vec![List::vlist(stack).into_node()], self.fonts)?;
        Ok((end, list.into_node()))
    }
}
