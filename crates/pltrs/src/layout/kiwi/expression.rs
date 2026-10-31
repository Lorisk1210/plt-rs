//! Variables, linear expressions and constraints, built with the
//! arithmetic of kiwisolver's Python operators, so that every coefficient
//! and constant comes out as matplotlib's `_layoutgrid` computes it, down
//! to the sign of zero.

use std::collections::BTreeMap;
use std::ops::{Add, Div, Mul, Sub};

/// A variable of a [`Solver`](super::Solver) (kiwisolver's `Variable`).
///
/// Variables order by creation. kiwisolver orders the terms of a
/// constraint by the addresses of its variables, which fresh allocations
/// make increasing in the common case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Variable(pub(super) usize);

/// A variable times a coefficient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Term {
    pub(super) variable: Variable,
    pub(super) coefficient: f64,
}

/// A linear expression: terms plus a constant.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Expression {
    pub(super) terms: Vec<Term>,
    pub(super) constant: f64,
}

impl Expression {
    /// The expression of two terms (kiwisolver's `Term + Term`).
    fn pair(first: Term, second: Term) -> Self {
        Self {
            terms: vec![first, second],
            constant: 0.0,
        }
    }

    /// The expression with `term` appended (kiwisolver's
    /// `Expression + Term`, whose constant is the expression's).
    fn with(mut self, term: Term) -> Self {
        self.terms.push(term);
        self
    }

    /// One term per variable, in the order of the variables, with the
    /// coefficients of repeated variables added up in the order of the
    /// terms (kiwisolver's `reduce`).
    fn reduce(self) -> Self {
        let mut coefficients = BTreeMap::new();
        for term in self.terms {
            *coefficients.entry(term.variable).or_insert(0.0) += term.coefficient;
        }
        Self {
            terms: coefficients
                .into_iter()
                .map(|(variable, coefficient)| Term {
                    variable,
                    coefficient,
                })
                .collect(),
            constant: self.constant,
        }
    }
}

impl Variable {
    /// The variable with a coefficient of one.
    fn term(self) -> Term {
        self.times(1.0)
    }

    /// The variable times `coefficient`.
    fn times(self, coefficient: f64) -> Term {
        Term {
            variable: self,
            coefficient,
        }
    }
}

impl Add for Variable {
    type Output = Expression;

    fn add(self, rhs: Self) -> Expression {
        Expression::pair(self.term(), rhs.term())
    }
}

impl Sub for Variable {
    type Output = Expression;

    fn sub(self, rhs: Self) -> Expression {
        Expression::pair(self.term(), rhs.times(-1.0))
    }
}

impl Sub<f64> for Variable {
    type Output = Expression;

    fn sub(self, rhs: f64) -> Expression {
        Expression {
            terms: vec![self.term()],
            constant: -rhs,
        }
    }
}

impl Sub<Expression> for Variable {
    type Output = Expression;

    /// kiwisolver adds the negated expression and the variable's term,
    /// in that order.
    fn sub(self, rhs: Expression) -> Expression {
        (rhs * -1.0).with(self.term())
    }
}

impl Add<Variable> for Expression {
    type Output = Self;

    fn add(self, rhs: Variable) -> Self {
        self.with(rhs.term())
    }
}

impl Sub<Variable> for Expression {
    type Output = Self;

    fn sub(self, rhs: Variable) -> Self {
        self.with(rhs.times(-1.0))
    }
}

impl Sub for Expression {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self {
        let rhs = rhs * -1.0;
        self.constant += rhs.constant;
        self.terms.extend(rhs.terms);
        self
    }
}

impl Mul<f64> for Expression {
    type Output = Self;

    fn mul(mut self, rhs: f64) -> Self {
        for term in &mut self.terms {
            term.coefficient *= rhs;
        }
        self.constant *= rhs;
        self
    }
}

impl Div<f64> for Expression {
    type Output = Self;

    /// Multiplies by the reciprocal, as kiwisolver does.
    fn div(self, rhs: f64) -> Self {
        self * (1.0 / rhs)
    }
}

/// How the expression of a constraint compares to zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Relation {
    /// `expression == 0`.
    Eq,
    /// `expression >= 0`.
    Ge,
}

/// The strengths of constraints (kiwisolver's `strength`).
pub(crate) mod strength {
    /// Constraints the solution must satisfy.
    pub(crate) const REQUIRED: f64 = create(1000.0, 1000.0, 1000.0);
    /// Constraints satisfied unless required ones prevent it.
    pub(crate) const STRONG: f64 = create(1.0, 0.0, 0.0);

    /// kiwisolver's `strength::create` with a weight of one.
    const fn create(a: f64, b: f64, c: f64) -> f64 {
        clamp(a) * 1_000_000.0 + clamp(b) * 1000.0 + clamp(c)
    }

    /// `max(0, min(1000, value))`.
    const fn clamp(value: f64) -> f64 {
        value.clamp(0.0, 1000.0)
    }
}

/// A linear constraint with its strength (kiwisolver's `Constraint`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Constraint {
    pub(super) expression: Expression,
    pub(super) relation: Relation,
    pub(super) strength: f64,
}

impl Constraint {
    /// `lhs <relation> rhs` with `strength` (kiwisolver's `lhs == rhs`
    /// or `lhs >= rhs`, then `| strength`): the expression `lhs - rhs`,
    /// reduced.
    pub(crate) fn new<L, R>(lhs: L, relation: Relation, rhs: R, strength: f64) -> Self
    where
        L: Sub<R, Output = Expression>,
    {
        Self {
            expression: (lhs - rhs).reduce(),
            relation,
            strength: strength.clamp(0.0, strength::REQUIRED),
        }
    }

    /// `variable == 0` with `strength`, the constraint of an edit
    /// variable.
    pub(super) fn edit(variable: Variable, strength: f64) -> Self {
        Self {
            expression: Expression {
                terms: vec![variable.term()],
                constant: 0.0,
            },
            relation: Relation::Eq,
            strength,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strengths_are_kiwisolvers() {
        assert_eq!(strength::REQUIRED, 1_001_001_000.0);
        assert_eq!(strength::STRONG, 1_000_000.0);
    }

    #[test]
    fn constraints_reduce_to_one_term_per_variable_in_creation_order() {
        let (a, b, c) = (Variable(0), Variable(1), Variable(2));
        let lhs = (c - a) - b;
        let constraint = Constraint::new(lhs, Relation::Ge, a + c, strength::STRONG);
        let terms: Vec<_> = constraint
            .expression
            .terms
            .iter()
            .map(|t| (t.variable, t.coefficient))
            .collect();
        // A cancelled variable keeps its zero coefficient, as in kiwisolver.
        assert_eq!(terms, [(a, -2.0), (b, -1.0), (c, 0.0)]);
        assert_eq!(constraint.expression.constant.to_bits(), 0.0f64.to_bits());
    }

    #[test]
    fn constants_keep_kiwisolvers_sign_of_zero() {
        let (a, b, c) = (Variable(0), Variable(1), Variable(2));
        // `a == 0` keeps `-0.0`; `a == b + c` negates a zero constant.
        assert!((a - 0.0).constant.is_sign_negative());
        assert!((a - (b + c)).constant.is_sign_negative());
        let width = (b - a) / 3.0;
        assert_eq!(width.terms[0].coefficient, 1.0 / 3.0);
        assert!(!(width.clone() - width * 2.0).constant.is_sign_negative());
    }
}
