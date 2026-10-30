//! Categorical axes (matplotlib's `category` module).

use crate::ticker::{FormattedTicks, Formatter, Locator, TickContext, TickerError};

/// Category names of one axis in order of first appearance; the n-th
/// category sits at position n (matplotlib's `UnitData`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Categories {
    names: Vec<String>,
}

pltrs_core::archive_struct!(Categories { names });

impl Categories {
    /// The names, in position order.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// The position of `name`, if it is a category.
    pub fn position(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|known| known == name)
    }

    /// Positions of `names`, adding the unknown ones as new categories.
    pub fn convert(&mut self, names: &[String]) -> Vec<f64> {
        names
            .iter()
            .map(|name| {
                let index = self.position(name).unwrap_or_else(|| {
                    self.names.push(name.clone());
                    self.names.len() - 1
                });
                index as f64
            })
            .collect()
    }
}

/// A tick at every category (matplotlib's `StrCategoryLocator`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryLocator {
    count: usize,
}

pltrs_core::archive_struct!(CategoryLocator { count });

impl CategoryLocator {
    /// Ticks for the current `categories`.
    pub fn new(categories: &Categories) -> Self {
        Self {
            count: categories.names.len(),
        }
    }
}

impl Locator for CategoryLocator {
    fn tick_values(&self, _: f64, _: f64, _: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        Ok((0..self.count).map(|i| i as f64).collect())
    }
}

/// Labels ticks with the name of the category at the nearest position,
/// or nothing between categories (matplotlib's `StrCategoryFormatter`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryFormatter {
    names: Vec<String>,
}

pltrs_core::archive_struct!(CategoryFormatter { names });

impl CategoryFormatter {
    /// Labels for the current `categories`.
    pub fn new(categories: &Categories) -> Self {
        Self {
            names: categories.names.clone(),
        }
    }

    fn label(&self, value: f64) -> String {
        // Python's round() ties to even.
        let nearest = value.round_ties_even();
        if nearest >= 0.0 && nearest < self.names.len() as f64 {
            self.names[nearest as usize].clone()
        } else {
            String::new()
        }
    }
}

impl Formatter for CategoryFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        _: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        Ok(FormattedTicks {
            labels: locs.iter().map(|&value| self.label(value)).collect(),
            offset: String::new(),
        })
    }

    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(self.label(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|&v| v.to_owned()).collect()
    }

    #[test]
    fn positions_follow_first_appearance() {
        let mut categories = Categories::default();
        assert_eq!(
            categories.convert(&names(&["north", "south", "east"])),
            [0.0, 1.0, 2.0]
        );
        assert_eq!(categories.convert(&names(&["west", "north"])), [3.0, 0.0]);
        assert_eq!(categories.position("west"), Some(3));
        assert_eq!(categories.position("up"), None);
    }

    #[test]
    fn labels_round_to_the_nearest_category() {
        let mut categories = Categories::default();
        categories.convert(&names(&["a", "b", "c"]));
        let formatter = CategoryFormatter::new(&categories);
        assert_eq!(formatter.label(1.2), "b");
        assert_eq!(formatter.label(0.5), "a");
        assert_eq!(formatter.label(2.5), "c");
        assert_eq!(formatter.label(-1.0), "");
        assert_eq!(formatter.label(f64::NAN), "");
    }
}
