//! The axes [`Figure::subplot_mosaic`](super::Figure::subplot_mosaic)
//! makes, by label.

use std::ops::Index;

use crate::axes::Axes;

/// The axes of a mosaic, by label, in creation order.
///
/// Index it with a label, as matplotlib's dictionary of axes, to get the
/// axes or fail at once; [`get`](Self::get) returns an `Option` instead.
///
/// ```
/// use pltrs::figure::{Figure, MosaicOptions};
/// let axd = Figure::new().subplot_mosaic("AB;CC", MosaicOptions::default())?;
/// axd["C"].set_title("C spans the bottom row");
/// assert!(axd.get("D").is_none());
/// # Ok::<(), pltrs::figure::MosaicError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MosaicAxes {
    entries: Vec<(String, Axes)>,
}

impl MosaicAxes {
    /// The axes made for a mosaic, by label in creation order.
    pub(super) fn new(entries: Vec<(String, Axes)>) -> Self {
        Self { entries }
    }

    /// The axes labelled `label`.
    pub fn get(&self, label: &str) -> Option<&Axes> {
        self.entries
            .iter()
            .find(|(name, _)| name == label)
            .map(|(_, ax)| ax)
    }

    /// Labels and axes in creation order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Axes)> {
        self.entries.iter().map(|(name, ax)| (name.as_str(), ax))
    }

    /// The number of axes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the mosaic has no axes.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Index<&str> for MosaicAxes {
    type Output = Axes;

    /// The axes labelled `label`.
    ///
    /// # Panics
    ///
    /// When the mosaic has no such label, as indexing a map with a missing
    /// key does; the message lists the labels it has.
    fn index(&self, label: &str) -> &Axes {
        self.get(label).unwrap_or_else(|| {
            let labels: Vec<&str> = self.iter().map(|(name, _)| name).collect();
            panic!("the mosaic has no axes labelled {label:?}, only {labels:?}")
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::figure::{Figure, MosaicOptions};

    #[test]
    #[should_panic(expected = r#"the mosaic has no axes labelled "Z", only ["A", "B"]"#)]
    fn a_missing_label_names_the_labels_there_are() {
        let axd = Figure::new()
            .subplot_mosaic("AB", MosaicOptions::default())
            .unwrap();
        assert_eq!(axd["B"], *axd.get("B").unwrap());
        let _ = &axd["Z"];
    }
}
