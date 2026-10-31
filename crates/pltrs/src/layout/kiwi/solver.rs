//! The solver: an incremental dual simplex over a tableau of rows
//! (kiwisolver's `SolverImpl`), step for step.

use std::collections::BTreeMap;
use std::fmt;

use super::expression::{Constraint, Relation, Variable, strength};
use super::row::{Kind, Row, Symbol, near_zero};

/// Why the solver refused a request (kiwisolver's exceptions).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SolverError {
    /// A required constraint contradicts the others.
    UnsatisfiableConstraint,
    /// The objective has no minimum.
    Unbounded,
    /// The dual simplex found no symbol to enter the basis.
    DualOptimizeFailed,
    /// An edit variable was added twice.
    DuplicateEditVariable,
    /// An edit variable was given the required strength.
    BadRequiredStrength,
    /// A value was suggested for a variable that is no edit variable.
    UnknownEditVariable,
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsatisfiableConstraint => "unable to satisfy a required constraint",
            Self::Unbounded => "the objective is unbounded",
            Self::DualOptimizeFailed => "dual optimize failed",
            Self::DuplicateEditVariable => "duplicate edit variable",
            Self::BadRequiredStrength => "an edit variable cannot have the required strength",
            Self::UnknownEditVariable => "unknown edit variable",
        })
    }
}

impl std::error::Error for SolverError {}

/// The symbols tracking a constraint in the tableau: its marker and,
/// for constraints that are not required, its other error symbol.
#[derive(Clone, Copy, Debug)]
struct Tag {
    marker: Symbol,
    other: Option<Symbol>,
}

/// An edit variable: the tag of its constraint and its suggested value.
#[derive(Clone, Copy, Debug)]
struct Edit {
    tag: Tag,
    constant: f64,
}

/// Which objective [`Solver::optimize`] minimises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Objective {
    /// The objective of the constraints' errors.
    Main,
    /// The artificial objective of a row being added.
    Artificial,
}

/// A linear constraint solver (kiwisolver's `Solver`): required
/// constraints hold exactly, the others as well as their strengths allow,
/// and edit variables take suggested values.
#[derive(Debug)]
pub(crate) struct Solver {
    /// The symbol of each variable used, by variable.
    symbols: Vec<Option<Symbol>>,
    /// The value of each variable at the last update, by variable.
    values: Vec<f64>,
    /// The edit variables.
    edits: BTreeMap<Variable, Edit>,
    /// The basic rows, by their basic symbol.
    rows: BTreeMap<Symbol, Row>,
    infeasible_rows: Vec<Symbol>,
    objective: Row,
    artificial: Option<Row>,
    id_tick: u64,
}

impl Default for Solver {
    fn default() -> Self {
        Self {
            symbols: Vec::new(),
            values: Vec::new(),
            edits: BTreeMap::new(),
            rows: BTreeMap::new(),
            infeasible_rows: Vec::new(),
            objective: Row::default(),
            artificial: None,
            id_tick: 1,
        }
    }
}

impl Solver {
    /// A new variable, valued zero.
    pub(crate) fn new_variable(&mut self) -> Variable {
        self.symbols.push(None);
        self.values.push(0.0);
        Variable(self.symbols.len() - 1)
    }

    /// The value of `variable` at the last [`update_variables`](Self::update_variables).
    pub(crate) fn value(&self, variable: Variable) -> f64 {
        self.values[variable.0]
    }

    /// Adds `constraint` (kiwisolver's `addConstraint`).
    pub(crate) fn add_constraint(&mut self, constraint: &Constraint) -> Result<(), SolverError> {
        self.add_constraint_tagged(constraint).map(|_| ())
    }

    /// Adds `constraint` and returns its tag.
    fn add_constraint_tagged(&mut self, constraint: &Constraint) -> Result<Tag, SolverError> {
        let (mut row, tag) = self.create_row(constraint);
        let mut subject = choose_subject(&row, tag);
        // A row of dummies only is redundant if its constant is zero, and
        // unsatisfiable otherwise.
        if subject.is_none() && all_dummies(&row) {
            if !near_zero(row.constant()) {
                return Err(SolverError::UnsatisfiableConstraint);
            }
            subject = Some(tag.marker);
        }
        match subject {
            None => {
                if !self.add_with_artificial_variable(&row)? {
                    return Err(SolverError::UnsatisfiableConstraint);
                }
            }
            Some(subject) => {
                row.solve_for(subject);
                self.substitute(subject, &row);
                self.rows.insert(subject, row);
            }
        }
        self.optimize(Objective::Main)?;
        Ok(tag)
    }

    /// Makes `variable` an edit variable of `strength`, which must not be
    /// required (kiwisolver's `addEditVariable`).
    pub(crate) fn add_edit_variable(
        &mut self,
        variable: Variable,
        strength: f64,
    ) -> Result<(), SolverError> {
        if self.edits.contains_key(&variable) {
            return Err(SolverError::DuplicateEditVariable);
        }
        let strength = strength.clamp(0.0, strength::REQUIRED);
        if strength == strength::REQUIRED {
            return Err(SolverError::BadRequiredStrength);
        }
        let tag = self.add_constraint_tagged(&Constraint::edit(variable, strength))?;
        self.edits.insert(variable, Edit { tag, constant: 0.0 });
        Ok(())
    }

    /// Suggests `value` for the edit variable `variable` (kiwisolver's
    /// `suggestValue`), then restores feasibility with the dual simplex.
    pub(crate) fn suggest_value(
        &mut self,
        variable: Variable,
        value: f64,
    ) -> Result<(), SolverError> {
        let edit = self
            .edits
            .get_mut(&variable)
            .ok_or(SolverError::UnknownEditVariable)?;
        let delta = value - edit.constant;
        edit.constant = value;
        let tag = edit.tag;
        self.shift_edit_rows(tag, delta);
        self.dual_optimize()
    }

    /// Moves the rows of an edit constraint by `delta`, noting those that
    /// become infeasible.
    fn shift_edit_rows(&mut self, tag: Tag, delta: f64) {
        // The positive error symbol may be basic, then the negative one.
        if let Some(row) = self.rows.get_mut(&tag.marker) {
            if row.add(-delta) < 0.0 {
                self.infeasible_rows.push(tag.marker);
            }
            return;
        }
        if let Some(other) = tag.other
            && let Some(row) = self.rows.get_mut(&other)
        {
            if row.add(delta) < 0.0 {
                self.infeasible_rows.push(other);
            }
            return;
        }
        // Otherwise every row holding the error symbols moves.
        for (&symbol, row) in &mut self.rows {
            let coefficient = row.coefficient_for(tag.marker);
            if coefficient != 0.0
                && row.add(delta * coefficient) < 0.0
                && symbol.kind() != Kind::External
            {
                self.infeasible_rows.push(symbol);
            }
        }
    }

    /// Stores the value of every variable used by a constraint
    /// (kiwisolver's `updateVariables`).
    pub(crate) fn update_variables(&mut self) {
        for (value, symbol) in self.values.iter_mut().zip(&self.symbols) {
            if let Some(symbol) = symbol {
                *value = self.rows.get(symbol).map_or(0.0, Row::constant);
            }
        }
    }

    /// The symbol of `variable`, made on first use.
    fn var_symbol(&mut self, variable: Variable) -> Symbol {
        if let Some(symbol) = self.symbols[variable.0] {
            return symbol;
        }
        let symbol = self.next_symbol(Kind::External);
        self.symbols[variable.0] = Some(symbol);
        symbol
    }

    /// A new symbol of `kind`.
    fn next_symbol(&mut self, kind: Kind) -> Symbol {
        let symbol = Symbol::new(kind, self.id_tick);
        self.id_tick += 1;
        symbol
    }

    /// The row of `constraint` (kiwisolver's `createRow`): its terms
    /// with the basic variables substituted, plus the slack, error or
    /// dummy symbols of its relation and strength, with a constant made
    /// non-negative; and the tag tracking it.
    fn create_row(&mut self, constraint: &Constraint) -> (Row, Tag) {
        let expression = &constraint.expression;
        let mut row = Row::new(expression.constant);
        for term in &expression.terms {
            if near_zero(term.coefficient) {
                continue;
            }
            let symbol = self.var_symbol(term.variable);
            match self.rows.get(&symbol) {
                Some(basic) => row.insert_row(basic, term.coefficient),
                None => row.insert(symbol, term.coefficient),
            }
        }
        let required = constraint.strength >= strength::REQUIRED;
        let tag = match constraint.relation {
            Relation::Ge => {
                let slack = self.next_symbol(Kind::Slack);
                row.insert(slack, -1.0);
                let other = (!required).then(|| {
                    let error = self.next_symbol(Kind::Error);
                    row.insert(error, 1.0);
                    self.objective.insert(error, constraint.strength);
                    error
                });
                Tag {
                    marker: slack,
                    other,
                }
            }
            Relation::Eq if required => {
                let dummy = self.next_symbol(Kind::Dummy);
                row.insert(dummy, 1.0);
                Tag {
                    marker: dummy,
                    other: None,
                }
            }
            Relation::Eq => {
                let plus = self.next_symbol(Kind::Error);
                let minus = self.next_symbol(Kind::Error);
                row.insert(plus, -1.0);
                row.insert(minus, 1.0);
                self.objective.insert(plus, constraint.strength);
                self.objective.insert(minus, constraint.strength);
                Tag {
                    marker: plus,
                    other: Some(minus),
                }
            }
        };
        if row.constant() < 0.0 {
            row.reverse_sign();
        }
        (row, tag)
    }

    /// Adds `row` through an artificial variable (kiwisolver's
    /// `addWithArtificialVariable`); `false` if the row cannot be
    /// satisfied.
    fn add_with_artificial_variable(&mut self, row: &Row) -> Result<bool, SolverError> {
        let art = self.next_symbol(Kind::Slack);
        self.rows.insert(art, row.clone());
        self.artificial = Some(row.clone());
        self.optimize(Objective::Artificial)?;
        let success = self
            .artificial
            .take()
            .is_some_and(|artificial| near_zero(artificial.constant()));
        // An artificial variable still basic leaves the basis, unless its
        // row is constant.
        if let Some(mut basic) = self.rows.remove(&art) {
            if basic.cells().is_empty() {
                return Ok(success);
            }
            let Some(entering) = any_pivotable_symbol(&basic) else {
                return Ok(false);
            };
            basic.solve_for_pair(art, entering);
            self.substitute(entering, &basic);
            self.rows.insert(entering, basic);
        }
        for row in self.rows.values_mut() {
            row.remove(art);
        }
        self.objective.remove(art);
        Ok(success)
    }

    /// Replaces `symbol` with `row` throughout the tableau and the
    /// objectives, noting the restricted rows that become infeasible.
    fn substitute(&mut self, symbol: Symbol, row: &Row) {
        for (&basic, basic_row) in &mut self.rows {
            basic_row.substitute(symbol, row);
            if basic.kind() != Kind::External && basic_row.constant() < 0.0 {
                self.infeasible_rows.push(basic);
            }
        }
        self.objective.substitute(symbol, row);
        if let Some(artificial) = &mut self.artificial {
            artificial.substitute(symbol, row);
        }
    }

    /// The symbol to enter the basis to lower the objective `which`.
    fn entering(&self, which: Objective) -> Option<Symbol> {
        match which {
            Objective::Main => entering_symbol(&self.objective),
            Objective::Artificial => self.artificial.as_ref().and_then(entering_symbol),
        }
    }

    /// Minimises the objective `which` with the primal simplex
    /// (kiwisolver's `optimize`).
    fn optimize(&mut self, which: Objective) -> Result<(), SolverError> {
        while let Some(entering) = self.entering(which) {
            let leaving = self.leaving_row(entering).ok_or(SolverError::Unbounded)?;
            self.pivot(leaving, entering);
        }
        Ok(())
    }

    /// Moves `entering` into the basis in place of `leaving`.
    fn pivot(&mut self, leaving: Symbol, entering: Symbol) {
        if let Some(mut row) = self.rows.remove(&leaving) {
            row.solve_for_pair(leaving, entering);
            self.substitute(entering, &row);
            self.rows.insert(entering, row);
        }
    }

    /// Restores feasibility with the dual simplex, the objective staying
    /// optimal (kiwisolver's `dualOptimize`).
    fn dual_optimize(&mut self) -> Result<(), SolverError> {
        while let Some(leaving) = self.infeasible_rows.pop() {
            let Some(row) = self.rows.get(&leaving) else {
                continue;
            };
            if !near_zero(row.constant()) && row.constant() < 0.0 {
                let entering = self
                    .dual_entering_symbol(row)
                    .ok_or(SolverError::DualOptimizeFailed)?;
                self.pivot(leaving, entering);
            }
        }
        Ok(())
    }

    /// The symbol of the infeasible `row` with a positive coefficient
    /// and the smallest ratio of its objective coefficient to it
    /// (kiwisolver's `getDualEnteringSymbol`).
    fn dual_entering_symbol(&self, row: &Row) -> Option<Symbol> {
        let mut entering = None;
        let mut ratio = f64::MAX;
        for (&symbol, &value) in row.cells() {
            if value > 0.0 && symbol.kind() != Kind::Dummy {
                let r = self.objective.coefficient_for(symbol) / value;
                if r < ratio {
                    ratio = r;
                    entering = Some(symbol);
                }
            }
        }
        entering
    }

    /// The restricted row limiting `entering` most, by the ratio of its
    /// constant to its coefficient (kiwisolver's `getLeavingRow`).
    fn leaving_row(&self, entering: Symbol) -> Option<Symbol> {
        let mut ratio = f64::MAX;
        let mut found = None;
        for (&symbol, row) in &self.rows {
            if symbol.kind() == Kind::External {
                continue;
            }
            let coefficient = row.coefficient_for(entering);
            if coefficient < 0.0 {
                let r = -row.constant() / coefficient;
                if r < ratio {
                    ratio = r;
                    found = Some(symbol);
                }
            }
        }
        found
    }
}

/// The subject to solve a new row for (kiwisolver's `chooseSubject`): its
/// first external symbol, else a slack or error marker with a negative
/// coefficient.
fn choose_subject(row: &Row, tag: Tag) -> Option<Symbol> {
    if let Some(&symbol) = row.cells().keys().find(|s| s.kind() == Kind::External) {
        return Some(symbol);
    }
    [Some(tag.marker), tag.other]
        .into_iter()
        .flatten()
        .find(|&symbol| {
            matches!(symbol.kind(), Kind::Slack | Kind::Error) && row.coefficient_for(symbol) < 0.0
        })
}

/// Whether every symbol of `row` is a dummy.
fn all_dummies(row: &Row) -> bool {
    row.cells().keys().all(|s| s.kind() == Kind::Dummy)
}

/// The first slack or error symbol of `row`.
fn any_pivotable_symbol(row: &Row) -> Option<Symbol> {
    row.cells()
        .keys()
        .copied()
        .find(|s| matches!(s.kind(), Kind::Slack | Kind::Error))
}

/// The first symbol of the objective, other than a dummy, with a
/// negative coefficient (kiwisolver's `getEnteringSymbol`).
fn entering_symbol(objective: &Row) -> Option<Symbol> {
    objective
        .cells()
        .iter()
        .find(|&(symbol, &value)| symbol.kind() != Kind::Dummy && value < 0.0)
        .map(|(&symbol, _)| symbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_constraints_hold_and_edits_follow_suggestions() {
        let mut solver = Solver::default();
        let (left, width, right) = (
            solver.new_variable(),
            solver.new_variable(),
            solver.new_variable(),
        );
        solver
            .add_constraint(&Constraint::new(
                left,
                Relation::Eq,
                1.0,
                strength::REQUIRED,
            ))
            .unwrap();
        solver.add_edit_variable(width, strength::STRONG).unwrap();
        let right_edge = Constraint::new(right, Relation::Eq, left + width, strength::REQUIRED);
        solver.add_constraint(&right_edge).unwrap();
        solver.suggest_value(width, 2.5).unwrap();
        solver.update_variables();
        assert_eq!(
            [left, width, right].map(|v| solver.value(v)),
            [1.0, 2.5, 3.5]
        );
    }

    #[test]
    fn strong_edits_give_way_to_required_inequalities() {
        let mut solver = Solver::default();
        let (low, high) = (solver.new_variable(), solver.new_variable());
        solver.add_edit_variable(low, strength::STRONG).unwrap();
        solver
            .add_constraint(&Constraint::new(
                high,
                Relation::Eq,
                2.0,
                strength::REQUIRED,
            ))
            .unwrap();
        solver
            .add_constraint(&Constraint::new(
                high,
                Relation::Ge,
                low,
                strength::REQUIRED,
            ))
            .unwrap();
        solver.suggest_value(low, 5.0).unwrap();
        solver.update_variables();
        assert_eq!((solver.value(low), solver.value(high)), (2.0, 2.0));
        solver.suggest_value(low, 1.0).unwrap();
        solver.update_variables();
        assert_eq!(solver.value(low), 1.0);
    }

    #[test]
    fn contradictions_and_misuse_are_errors() {
        let mut solver = Solver::default();
        let x = solver.new_variable();
        solver
            .add_constraint(&Constraint::new(x, Relation::Eq, 1.0, strength::REQUIRED))
            .unwrap();
        assert_eq!(
            solver.add_constraint(&Constraint::new(x, Relation::Eq, 2.0, strength::REQUIRED)),
            Err(SolverError::UnsatisfiableConstraint)
        );
        assert_eq!(
            solver.suggest_value(x, 1.0),
            Err(SolverError::UnknownEditVariable)
        );
        assert_eq!(
            solver.add_edit_variable(x, strength::REQUIRED * 2.0),
            Err(SolverError::BadRequiredStrength)
        );
        let y = solver.new_variable();
        solver.add_edit_variable(y, strength::STRONG).unwrap();
        assert_eq!(
            solver.add_edit_variable(y, strength::STRONG),
            Err(SolverError::DuplicateEditVariable)
        );
        assert_eq!(solver.value(y), 0.0, "no update yet");
    }
}
