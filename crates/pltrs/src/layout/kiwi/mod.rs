//! A port of kiwisolver 1.5, the Cassowary linear constraint solver
//! matplotlib's constrained layout states its layout grids to.
//!
//! The grids' equations determine every edge, but solved any other way
//! the edges agree with matplotlib's only to within the last digits of a
//! double, which moves axes by a pixel. So this is kiwisolver's solver
//! step for step: the same symbols, rows kept in the same order, the same
//! pivots and the same floating-point operations, and the expressions
//! built with the arithmetic of its Python operators.
//!
//! ```text
//! let mut solver = Solver::default();
//! let (left, width) = (solver.new_variable(), solver.new_variable());
//! solver.add_constraint(&Constraint::new(left, Relation::Eq, 0.25, strength::REQUIRED))?;
//! solver.add_edit_variable(width, strength::STRONG)?;
//! solver.suggest_value(width, 0.5)?;
//! solver.update_variables();
//! assert_eq!(solver.value(width), 0.5);
//! ```

mod expression;
mod row;
mod solver;

pub(crate) use expression::{Constraint, Expression, Relation, Variable, strength};
pub(crate) use solver::{Solver, SolverError};
