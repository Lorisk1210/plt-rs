//! The core grammar: tokens, simple items, scripts, placeables and groups.

use super::error::{EXPECTED_END_GROUP, EXPECTED_PLACEABLE, ErrorKind, Fail, Miss, NO_MATCH};
use super::{MAX_NESTING, Parsed, Parser};
use crate::error::TextError;
use crate::mathtext::boxes::{List, Node};
use crate::mathtext::font_name::FontName;

/// A placeable alternative: `None` when its leading literal does not
/// match, else its (successful or fatal) result.
type Alternative = Option<Parsed<Node>>;

impl Parser<'_, '_> {
    /// `ZeroOrMore(token)`: the end, the nodes and how many tokens matched
    /// (font switches match without producing a node).
    pub(super) fn tokens(&mut self, loc: usize) -> Result<(usize, Vec<Node>, usize), Fail> {
        let (mut pos, mut nodes, mut count) = (loc, Vec::new(), 0);
        loop {
            match self.token(pos) {
                Ok((end, node)) => {
                    nodes.extend(node);
                    count += 1;
                    pos = end;
                }
                Err(Fail::Soft(_)) => return Ok((pos, nodes, count)),
                Err(fatal) => return Err(fatal),
            }
        }
    }

    /// `token`: a simple item, else an unclosed group or an unknown
    /// command, which are errors.
    pub(super) fn token(&mut self, loc: usize) -> Parsed<Option<Node>> {
        let start = self.skip_ws(loc);
        match self.simple(start) {
            Err(Fail::Soft(_)) => {}
            other => return other,
        }
        if let Some(&stop) = self.group_failures.get(&start)
            && self.skip_ws(stop) == self.len()
        {
            return Err(self.fatal(ErrorKind::Fatal, "Expected '}'", self.len()));
        }
        if let Some((_, name)) = self.command(start) {
            let msg = format!("Unknown symbol: \\{name}");
            return Err(self.fatal(ErrorKind::Fatal, &msg, start));
        }
        Err(Fail::miss(start, NO_MATCH))
    }

    /// `simple`: a space, `\hspace`, a font switch or a placeable with
    /// optional scripts.
    pub(super) fn simple(&mut self, loc: usize) -> Parsed<Option<Node>> {
        let start = self.skip_ws(loc);
        if let Some((end, width)) = self.space_command(start) {
            return Ok((end, Some(self.make_space(width)?)));
        }
        if let Some(after) = self.literal(start, "\\hspace") {
            let (end, width) = self.args(after, "\\hspace{space}", |p, loc| {
                let open = p.expect_char(loc, '{')?;
                let at = p.skip_ws(open);
                let (end, width) = p.float_literal(at).ok_or(Fail::miss(at, NO_MATCH))?;
                Ok((p.expect_char(end, '}')?, width))
            })?;
            return Ok((end, Some(self.make_space(width)?)));
        }
        if let Some((end, font)) = self.font_command(start) {
            self.state_mut().set_font(font);
            return Ok((end, None));
        }
        self.subsuper(start).map(|(end, node)| (end, Some(node)))
    }

    /// Matches `c` after whitespace.
    pub(super) fn expect_char(&self, loc: usize, c: char) -> Result<usize, Fail> {
        let at = self.skip_ws(loc);
        if self.at(at) == Some(c) {
            Ok(at + 1)
        } else {
            Err(Fail::miss(at, NO_MATCH))
        }
    }

    /// Command arguments after a `-` in the grammar: a mismatch becomes
    /// the fatal error `Expected <signature>`.
    pub(super) fn args<T>(
        &mut self,
        after: usize,
        signature: &str,
        parse: impl FnOnce(&mut Self, usize) -> Parsed<T>,
    ) -> Parsed<T> {
        match parse(self, after) {
            Err(Fail::Soft(_)) => {
                let msg = format!("Expected {signature}");
                Err(self.fatal(ErrorKind::Syntax, &msg, self.skip_ws(after)))
            }
            other => other,
        }
    }

    /// `subsuper`: a placeable followed by `_`/`^` scripts and primes.
    fn subsuper(&mut self, loc: usize) -> Parsed<Node> {
        let start = self.skip_ws(loc);
        let (nucleus, miss) = match self.placeable(start) {
            Ok((end, node)) => (Some((end, node)), None),
            Err(Fail::Soft(miss)) => (None, Some(miss)),
            Err(fatal) => return Err(fatal),
        };
        let mut pos = nucleus.as_ref().map_or(start, |(end, _)| *end);
        let mut scripts = Vec::new();
        loop {
            let at = self.skip_ws(pos);
            let Some(op) = self.at(at).filter(|c| matches!(c, '_' | '^')) else {
                break;
            };
            match self.placeable(at + 1) {
                Ok((end, arg)) => {
                    scripts.push((op, arg));
                    pos = end;
                }
                Err(Fail::Soft(miss)) => {
                    return Err(self.fatal(ErrorKind::Syntax, miss.msg, miss.loc));
                }
                Err(fatal) => return Err(fatal),
            }
        }
        let (nucleus, primes_from) = if !scripts.is_empty() {
            (nucleus.map(|(_, node)| node), pos)
        } else if self.at(start) == Some('\'') {
            (None, start)
        } else if let Some((end, node)) = nucleus {
            (Some(node), end)
        } else {
            return Err(Fail::Soft(miss.unwrap_or(Miss {
                loc: start,
                msg: NO_MATCH,
            })));
        };
        let primes_at = self.skip_ws(primes_from);
        let mut end = primes_at;
        while self.at(end) == Some('\'') {
            end += 1;
        }
        let node = self.scripts(start, nucleus, scripts, end - primes_at)?;
        Ok((end, node))
    }

    /// `placeable`: everything that can carry scripts, tried in
    /// matplotlib's order. Only `symbol` keeps leading whitespace (so a
    /// space after `^` is a superscript space).
    pub(super) fn placeable(&mut self, loc: usize) -> Parsed<Node> {
        if self.nesting >= MAX_NESTING {
            let msg = "maximum recursion depth exceeded".to_owned();
            return Err(Fail::Fatal(TextError::Math(msg)));
        }
        self.nesting += 1;
        let result = self.placeable_alternatives(loc);
        self.nesting -= 1;
        result
    }

    fn placeable_alternatives(&mut self, loc: usize) -> Parsed<Node> {
        let ws = self.skip_ws(loc);
        let mut furthest = Miss {
            loc,
            msg: EXPECTED_PLACEABLE,
        };
        if let Some(result) = self.phantom_like(ws) {
            return result;
        }
        if let Some(result) = self.accent(ws) {
            return result;
        }
        if let Some((end, symbol)) = self.scan_symbol(loc) {
            return self.symbol(loc, &symbol).map(|node| (end, node));
        }
        if let Some((end, name)) = self.function_command(ws) {
            return self.function(ws, end, &name);
        }
        if let Some(result) = self.operatorname(ws) {
            return result;
        }
        match self.group(ws) {
            Err(Fail::Soft(miss)) => {
                if miss.loc > furthest.loc {
                    furthest = miss;
                }
            }
            other => return other,
        }
        let commands: [fn(&mut Self, usize) -> Alternative; 13] = [
            Self::fraction,
            Self::display_fraction,
            Self::binomial,
            Self::generalized_fraction,
            Self::overset,
            Self::underset,
            Self::sqrt,
            Self::overline,
            Self::underline,
            Self::text,
            Self::boldsymbol,
            Self::substack,
            Self::auto_delim,
        ];
        for command in commands {
            if let Some(result) = command(self, ws) {
                return result;
            }
        }
        Err(Fail::Soft(furthest))
    }

    /// `group`: `{...}` or `\mathXX{...}`, in a new font state.
    fn group(&mut self, loc: usize) -> Parsed<Node> {
        let mut pos = loc;
        let mut font = None;
        if let Some(after) = self.literal(loc, "\\math")
            && let Some((end, name)) = self.font_name(self.skip_ws(after))
        {
            font = Some(name);
            pos = end;
        }
        let open = self.skip_ws(pos);
        if self.at(open) != Some('{') {
            return Err(Fail::miss(loc, NO_MATCH));
        }
        self.push_state();
        if let Some(font) = font {
            self.state_mut().set_font(font);
        }
        let (stop, nodes, _) = self.tokens(open + 1)?;
        let close = self.skip_ws(stop);
        if self.at(close) != Some('}') {
            self.group_failures.insert(loc, stop);
            return Err(Fail::miss(close, EXPECTED_END_GROUP));
        }
        self.pop_state();
        Ok((close + 1, List::hlist(nodes, self.fonts)?.into_node()))
    }

    /// `{` tokens `}` without a new font state: at least one token for a
    /// required group, any number for an optional one.
    pub(super) fn braced(&mut self, loc: usize, required: bool) -> Parsed<List> {
        let open = self.expect_char(loc, '{')?;
        let (stop, nodes, count) = self.tokens(open)?;
        if required && count == 0 {
            return Err(Fail::miss(open, NO_MATCH));
        }
        let close = self.expect_char(stop, '}')?;
        Ok((close, List::hlist(nodes, self.fonts)?))
    }

    /// `{ simple* }`: the argument of `\operatorname` and `\boldsymbol`,
    /// with the number of items matched.
    pub(super) fn simple_items(&mut self, loc: usize) -> Parsed<Vec<Node>> {
        let mut pos = self.expect_char(loc, '{')?;
        let mut nodes = Vec::new();
        loop {
            match self.simple(pos) {
                Ok((end, node)) => {
                    nodes.extend(node);
                    pos = end;
                }
                Err(Fail::Soft(_)) => break,
                Err(fatal) => return Err(fatal),
            }
        }
        Ok((self.expect_char(pos, '}')?, nodes))
    }

    /// Sets the font of a character inside `\operatorname` or
    /// `\boldsymbol` (the font class stays).
    pub(super) fn refont(&mut self, node: &mut Node, font: FontName) -> Result<(), Fail> {
        if let Node::Char(c) = node {
            c.font = font;
            c.update_metrics(self.fonts)?;
        }
        Ok(())
    }
}
