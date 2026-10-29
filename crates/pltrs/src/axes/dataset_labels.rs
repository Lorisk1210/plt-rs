//! Legend labels of the datasets of one plotting call (matplotlib's
//! `label` given as a string or as a list).

/// The legend labels of the datasets a call plots, such as the datasets
/// of [`Axes::hist`](super::Axes::hist): one label converts into a label
/// for the first dataset, a list into one label per dataset, in order.
///
/// ```
/// use pltrs::axes::DatasetLabels;
/// let one = DatasetLabels::from("samples");
/// assert_eq!(one.get(0), Some("samples"));
/// assert_eq!(one.get(1), None);
/// let each = DatasetLabels::from(["first", "second"]);
/// assert_eq!(each.get(1), Some("second"));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DatasetLabels(Vec<String>);

impl DatasetLabels {
    /// The label of dataset `index`, if it has one.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.0.get(index).map(String::as_str)
    }

    /// The labels, the first dataset's first.
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

/// A label for the first dataset (matplotlib's `label="..."`).
impl From<&str> for DatasetLabels {
    fn from(label: &str) -> Self {
        Self(vec![label.to_owned()])
    }
}

/// A label for the first dataset (matplotlib's `label="..."`).
impl From<String> for DatasetLabels {
    fn from(label: String) -> Self {
        Self(vec![label])
    }
}

/// One label per dataset (matplotlib's `label=[...]`).
impl<S: Into<String>> From<Vec<S>> for DatasetLabels {
    fn from(labels: Vec<S>) -> Self {
        labels.into_iter().collect()
    }
}

/// One label per dataset (matplotlib's `label=[...]`).
impl<S: Into<String>, const N: usize> From<[S; N]> for DatasetLabels {
    fn from(labels: [S; N]) -> Self {
        labels.into_iter().collect()
    }
}

/// One label per dataset, in the order of the iterator.
impl<S: Into<String>> FromIterator<S> for DatasetLabels {
    fn from_iter<I: IntoIterator<Item = S>>(labels: I) -> Self {
        Self(labels.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_label_names_the_first_dataset_and_a_list_each() {
        assert_eq!(DatasetLabels::default().get(0), None);
        assert_eq!(DatasetLabels::from(String::from("a")).as_slice(), ["a"]);
        let each = DatasetLabels::from(vec![String::from("a"), String::from("b")]);
        assert_eq!(each.as_slice(), ["a", "b"]);
        assert_eq!(
            DatasetLabels::from(Vec::<String>::new()),
            DatasetLabels::default()
        );
        let counted: DatasetLabels = (1..=2).map(|n| format!("run {n}")).collect();
        assert_eq!(counted.as_slice(), ["run 1", "run 2"]);
    }
}
