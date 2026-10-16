//! Fractions: `\frac`, `\dfrac`, `\binom` and `\genfrac`.

use super::delims::Item;
use super::error::{Fail, NO_MATCH};
use super::{Parsed, Parser, py_max};
use crate::mathtext::boxes::{List, Node, Packing, SHRINK_FACTOR};

/// TeX's math styles, which set how much a fraction shrinks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MathStyle {
    Display = 0,
    Text = 1,
    Script = 2,
    ScriptScript = 3,
}

impl MathStyle {
    fn from_digit(c: char) -> Option<Self> {
        Some(match c {
            '0' => Self::Display,
            '1' => Self::Text,
            '2' => Self::Script,
            '3' => Self::ScriptScript,
            _ => return None,
        })
    }
}

/// The arguments of a fraction.
struct Fraction {
    left: &'static str,
    right: &'static str,
    rule: Option<f64>,
    style: MathStyle,
    num: List,
    den: List,
}

impl Parser<'_, '_> {
    /// `\frac{num}{den}`.
    pub(super) fn fraction(&mut self, loc: usize) -> Option<Parsed<Node>> {
        self.simple_fraction(loc, "\\frac", MathStyle::Text, true)
    }

    /// `\dfrac{num}{den}`.
    pub(super) fn display_fraction(&mut self, loc: usize) -> Option<Parsed<Node>> {
        self.simple_fraction(loc, "\\dfrac", MathStyle::Display, true)
    }

    /// `\binom{num}{den}`.
    pub(super) fn binomial(&mut self, loc: usize) -> Option<Parsed<Node>> {
        self.simple_fraction(loc, "\\binom", MathStyle::Text, false)
    }

    fn simple_fraction(
        &mut self,
        loc: usize,
        name: &str,
        style: MathStyle,
        ruled: bool,
    ) -> Option<Parsed<Node>> {
        let after = self.literal(loc, name)?;
        let signature = format!("{name}{{num}}{{den}}");
        Some(self.simple_fraction_body(after, &signature, style, ruled))
    }

    fn simple_fraction_body(
        &mut self,
        after: usize,
        signature: &str,
        style: MathStyle,
        ruled: bool,
    ) -> Parsed<Node> {
        let (end, (num, den)) = self.args(after, signature, Self::two_groups)?;
        let fraction = if ruled {
            Fraction {
                left: "",
                right: "",
                rule: Some(self.fonts.underline_thickness(self.state().fontsize)),
                style,
                num,
                den,
            }
        } else {
            Fraction {
                left: "(",
                right: ")",
                rule: None,
                style,
                num,
                den,
            }
        };
        Ok((end, self.genfrac(fraction)?))
    }

    fn two_groups(&mut self, loc: usize) -> Parsed<(List, List)> {
        let (mid, first) = self.braced(loc, true)?;
        let (end, second) = self.braced(mid, true)?;
        Ok((end, (first, second)))
    }

    /// `\genfrac{ldelim}{rdelim}{rulesize}{style}{num}{den}`.
    pub(super) fn generalized_fraction(&mut self, loc: usize) -> Option<Parsed<Node>> {
        let after = self.literal(loc, "\\genfrac")?;
        let signature = "\\genfrac{ldelim}{rdelim}{rulesize}{style}{num}{den}";
        Some(
            self.args(after, signature, Self::genfrac_args)
                .and_then(|(end, fraction)| Ok((end, self.genfrac(fraction)?))),
        )
    }

    /// `{` optional delimiter `}`.
    fn optional_delimiter(&mut self, loc: usize) -> Parsed<&'static str> {
        let open = self.expect_char(loc, '{')?;
        let (pos, delim) = self.delimiter(self.skip_ws(open)).unwrap_or((open, ""));
        Ok((self.expect_char(pos, '}')?, delim))
    }

    fn genfrac_args(&mut self, loc: usize) -> Parsed<Fraction> {
        let (pos, left) = self.optional_delimiter(loc)?;
        let (pos, right) = self.optional_delimiter(pos)?;
        let open = self.expect_char(pos, '{')?;
        let at = self.skip_ws(open);
        let (end, rule) = self.float_literal(at).ok_or(Fail::miss(at, NO_MATCH))?;
        let open = self.expect_char(self.expect_char(end, '}')?, '{')?;
        let at = self.skip_ws(open);
        let (pos, style) = match self.at(at).and_then(MathStyle::from_digit) {
            Some(style) => (at + 1, style),
            None => (open, MathStyle::Text),
        };
        let close = self.expect_char(pos, '}')?;
        let (end, (num, den)) = self.two_groups(close)?;
        let rule = (rule != 0.0).then_some(rule);
        Ok((
            end,
            Fraction {
                left,
                right,
                rule,
                style,
                num,
                den,
            },
        ))
    }

    /// matplotlib's `_genfrac`: `num` over `den` around the math axis,
    /// with a rule if any, optionally between auto-sized delimiters.
    fn genfrac(&mut self, fraction: Fraction) -> Result<Node, Fail> {
        let Fraction {
            left,
            right,
            rule,
            style,
            mut num,
            mut den,
        } = fraction;
        let state = *self.state();
        let thickness = self.fonts.underline_thickness(state.fontsize);
        let axis_height = self.fonts.axis_height(state.font, state.fontsize)?;
        let consts = *self.fonts.constants();
        let mut x_height = self.fonts.x_height(state.font, state.fontsize)?;
        for _ in 0..style as usize {
            x_height *= SHRINK_FACTOR;
            num.shrink(self.fonts)?;
            den.shrink(self.fonts)?;
        }
        let width = py_max(num.width, den.width);
        let mut cnum = List::hcentered(vec![num.into_node()]);
        let mut cden = List::hcentered(vec![den.into_node()]);
        cnum.hpack(width, Packing::Exactly);
        cden.hpack(width, Packing::Exactly);
        let display = style == MathStyle::Display;
        let vlist = if let Some(rule) = rule {
            let (num_shift_up, den_shift_down, clr) = if display {
                (consts.num1 * x_height, consts.denom1 * x_height, 3.0 * rule)
            } else {
                (consts.num2 * x_height, consts.denom2 * x_height, rule)
            };
            let delta = rule / 2.0;
            let num_clr = py_max((num_shift_up - cnum.depth) - (axis_height + delta), clr);
            let den_clr = py_max((axis_height - delta) - (cden.height - den_shift_down), clr);
            let shift = cden.height + den_clr + delta - axis_height;
            let mut vlist = List::vlist(vec![
                cnum.into_node(),
                Node::vbox(0.0, num_clr),
                Node::hrule(rule),
                Node::vbox(0.0, den_clr),
                cden.into_node(),
            ]);
            vlist.shift_amount = shift;
            vlist
        } else {
            let (num_shift_up, den_shift_down, min_clr) = if display {
                (
                    consts.num1 * x_height,
                    consts.denom1 * x_height,
                    7.0 * thickness,
                )
            } else {
                (
                    consts.num3 * x_height,
                    consts.denom2 * x_height,
                    3.0 * thickness,
                )
            };
            let def_clr = (num_shift_up - cnum.depth) - (cden.height - den_shift_down);
            let clr = py_max(def_clr, min_clr);
            let mut vlist = List::vlist(vec![
                cnum.into_node(),
                Node::vbox(0.0, clr),
                cden.into_node(),
            ]);
            vlist.shift_amount = den_shift_down;
            if def_clr < min_clr {
                vlist.shift_amount += (min_clr - def_clr) / 2.0;
            }
            vlist
        };
        let result = List::hlist(
            vec![
                Node::hbox(thickness),
                vlist.into_node(),
                Node::hbox(thickness),
            ],
            self.fonts,
        )?;
        if left.is_empty() && right.is_empty() {
            return Ok(result.into_node());
        }
        let or_none = |delim: &'static str| if delim.is_empty() { "." } else { delim };
        let middle = vec![Item::Node(result.into_node())];
        Ok(self
            .auto_sized_delimiter(or_none(left), middle, or_none(right))?
            .into_node())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_from_genfrac_digits() {
        assert_eq!(MathStyle::from_digit('0'), Some(MathStyle::Display));
        assert_eq!(MathStyle::from_digit('3'), Some(MathStyle::ScriptScript));
        assert_eq!(MathStyle::from_digit('4'), None);
        assert_eq!(MathStyle::Script as usize, 2);
    }
}
