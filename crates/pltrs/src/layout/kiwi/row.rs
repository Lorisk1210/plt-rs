//! The symbols of the tableau and its rows (kiwisolver's `symbol.h` and
//! `row.h`).

use std::collections::BTreeMap;

/// What a symbol of the tableau stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Kind {
    /// A variable of the program.
    External,
    /// The slack of an inequality, or an artificial variable.
    Slack,
    /// The error of a constraint that is not required.
    Error,
    /// The marker of a required equality.
    Dummy,
}

/// A symbol of the tableau; symbols order by their ids, which count up
/// from one as they are made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Symbol {
    id: u64,
    kind: Kind,
}

impl Symbol {
    /// The symbol `id` of `kind`.
    pub(super) fn new(kind: Kind, id: u64) -> Self {
        Self { id, kind }
    }

    /// What the symbol stands for.
    pub(super) fn kind(self) -> Kind {
        self.kind
    }
}

/// kiwisolver's test for a coefficient that counts as zero.
pub(super) fn near_zero(value: f64) -> bool {
    const EPS: f64 = 1.0e-8;
    if value < 0.0 {
        -value < EPS
    } else {
        value < EPS
    }
}

/// A row of the tableau: a constant plus symbols with their
/// coefficients, kept in the order of the symbols (kiwisolver's `Row`,
/// whose `AssocVector` is a sorted map).
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Row {
    cells: BTreeMap<Symbol, f64>,
    constant: f64,
}

impl Row {
    /// A row with `constant` and no cells.
    pub(super) fn new(constant: f64) -> Self {
        Self {
            cells: BTreeMap::new(),
            constant,
        }
    }

    /// The cells, in the order of their symbols.
    pub(super) fn cells(&self) -> &BTreeMap<Symbol, f64> {
        &self.cells
    }

    /// The constant.
    pub(super) fn constant(&self) -> f64 {
        self.constant
    }

    /// Adds `value` to the constant and returns the new constant.
    pub(super) fn add(&mut self, value: f64) -> f64 {
        self.constant += value;
        self.constant
    }

    /// Adds `coefficient` to the cell of `symbol`, dropping the cell if
    /// it becomes zero.
    pub(super) fn insert(&mut self, symbol: Symbol, coefficient: f64) {
        let cell = self.cells.entry(symbol).or_insert(0.0);
        *cell += coefficient;
        if near_zero(*cell) {
            self.cells.remove(&symbol);
        }
    }

    /// Adds `other` times `coefficient`, constant and cells, dropping the
    /// cells that become zero.
    pub(super) fn insert_row(&mut self, other: &Self, coefficient: f64) {
        self.constant += other.constant * coefficient;
        for (&symbol, &value) in &other.cells {
            self.insert(symbol, value * coefficient);
        }
    }

    /// Drops the cell of `symbol`.
    pub(super) fn remove(&mut self, symbol: Symbol) {
        self.cells.remove(&symbol);
    }

    /// Negates the constant and every cell.
    pub(super) fn reverse_sign(&mut self) {
        self.constant = -self.constant;
        for value in self.cells.values_mut() {
            *value = -*value;
        }
    }

    /// Solves `0 = row` for `symbol`, which must be in the row: the row
    /// becomes the expression of `symbol`.
    pub(super) fn solve_for(&mut self, symbol: Symbol) {
        let coefficient = -1.0 / self.cells.remove(&symbol).unwrap_or(0.0);
        self.constant *= coefficient;
        for value in self.cells.values_mut() {
            *value *= coefficient;
        }
    }

    /// Solves `lhs = row` for `rhs`, which must be in the row (and `lhs`
    /// not): the row becomes the expression of `rhs`.
    pub(super) fn solve_for_pair(&mut self, lhs: Symbol, rhs: Symbol) {
        self.insert(lhs, -1.0);
        self.solve_for(rhs);
    }

    /// The coefficient of `symbol`, zero if it has no cell.
    pub(super) fn coefficient_for(&self, symbol: Symbol) -> f64 {
        self.cells.get(&symbol).copied().unwrap_or(0.0)
    }

    /// Replaces `symbol` with `row`, if the row has a cell for it.
    pub(super) fn substitute(&mut self, symbol: Symbol, row: &Self) {
        if let Some(coefficient) = self.cells.remove(&symbol) {
            self.insert_row(row, coefficient);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol(id: u64) -> Symbol {
        Symbol::new(Kind::External, id)
    }

    #[test]
    fn cells_that_cancel_are_dropped() {
        let mut row = Row::new(1.0);
        row.insert(symbol(2), 3.0);
        row.insert(symbol(1), 1.0);
        row.insert(symbol(2), -3.0 + 1e-9);
        assert_eq!(row.cells().keys().copied().collect::<Vec<_>>(), [symbol(1)]);
    }

    #[test]
    fn solving_and_substituting() {
        // 0 = 4 + 2 x - y, solved for x: x = -2 + y / 2.
        let (x, y) = (symbol(1), symbol(2));
        let mut row = Row::new(4.0);
        row.insert(x, 2.0);
        row.insert(y, -1.0);
        row.solve_for(x);
        assert_eq!((row.constant(), row.coefficient_for(y)), (-2.0, 0.5));
        // 1 + 3 x with that x: -5 + 1.5 y.
        let mut other = Row::new(1.0);
        other.insert(x, 3.0);
        other.substitute(x, &row);
        assert_eq!(
            (
                other.constant(),
                other.coefficient_for(y),
                other.coefficient_for(x)
            ),
            (-5.0, 1.5, 0.0)
        );
        other.reverse_sign();
        assert_eq!(other.add(1.0), 6.0);
    }

    #[test]
    fn near_zero_is_kiwisolvers() {
        assert!(near_zero(0.0) && near_zero(-0.0) && near_zero(-9e-9));
        assert!(!near_zero(1e-8) && !near_zero(f64::NAN));
    }
}
