//! Restyling check and radio buttons once they exist (matplotlib's
//! `set_label_props`, `set_frame_props`, `set_check_props`,
//! `set_radio_props` and the `activecolor` setter).

use super::column::{ButtonColumn, cycled};
use super::options::ButtonMarkerProps;
use super::{CheckButtons, RadioButtons, show_checks, show_selection};
use crate::collections::{Collection, CollectionColorError, EdgeColors};
use crate::colors::Color;
use crate::text::TextProps;

impl ButtonColumn {
    /// Updates label `i` with entry `i % len` of `props` (matplotlib's
    /// `set_label_props`); no entries change nothing.
    fn restyle_labels(&self, props: &[TextProps]) {
        for (i, label) in self.labels.iter().enumerate() {
            if let Some(props) = cycled(props, i) {
                label.update_props(props.clone());
            }
        }
    }
}

/// Updates `markers` with the set properties of `props` (matplotlib's
/// `Collection.update`), the sizes cycled to one per button of `count`.
fn update_markers(markers: &Collection, props: &ButtonMarkerProps, count: usize) {
    if let Some(sizes) = &props.sizes {
        let sizes = (0..count).filter_map(|i| cycled(sizes, i).copied());
        markers.set_sizes(sizes.collect());
    }
    if let Some(color) = &props.color {
        markers
            .set_facecolor(color.clone())
            .set_edgecolor(color.clone());
    }
    if let Some(faces) = &props.facecolors {
        markers.set_facecolors(faces.clone());
    }
    if let Some(edges) = &props.edgecolors {
        markers.set_edgecolors(EdgeColors::Colors(edges.clone()));
    }
    if let Some(widths) = &props.linewidths {
        markers.set_linewidths(widths.clone());
    }
}

/// The face colours of `markers`, a single one repeated for each of
/// `count` buttons: the colours the buttons show when on (matplotlib's
/// `_active_colors` taken from `get_facecolor()`).
fn active_colors(markers: &Collection, count: usize) -> Result<Vec<Color>, CollectionColorError> {
    let faces = markers.colors()?.faces;
    let faces = if faces.len() == 1 {
        vec![faces[0]; count]
    } else {
        faces
    };
    Ok(faces.into_iter().map(Color::from).collect())
}

impl CheckButtons {
    /// Updates the labels with the cycled `props` (matplotlib's
    /// `set_label_props`); the boxes keep their size.
    pub fn set_label_props(&self, props: &[TextProps]) -> &Self {
        self.column.restyle_labels(props);
        self
    }

    /// Restyles the square frames (matplotlib's `set_frame_props`).
    pub fn set_frame_props(&self, props: &ButtonMarkerProps) -> &Self {
        update_markers(&self.frames, props, self.column.labels.len());
        self
    }

    /// Restyles the crosses; the checked ones then show their new face
    /// colours (matplotlib's `set_check_props`).
    ///
    /// # Errors
    ///
    /// When a colour is invalid.
    pub fn set_check_props(
        &self,
        props: &ButtonMarkerProps,
    ) -> Result<&Self, CollectionColorError> {
        update_markers(&self.checks, props, self.column.labels.len());
        let colors = active_colors(&self.checks, self.column.labels.len())?;
        show_checks(&self.checks, &self.states, &colors);
        *self.active_colors.borrow_mut() = colors;
        Ok(self)
    }
}

impl RadioButtons {
    /// Updates the labels with the cycled `props` (matplotlib's
    /// `set_label_props`); the buttons keep their size.
    pub fn set_label_props(&self, props: &[TextProps]) -> &Self {
        self.column.restyle_labels(props);
        self
    }

    /// Restyles the round buttons; the selected one then shows its new
    /// face colour (matplotlib's `set_radio_props`). As in matplotlib,
    /// without new face colours the buttons take the faces they show, so
    /// unselected ones stay empty when selected later.
    ///
    /// # Errors
    ///
    /// When a colour is invalid.
    pub fn set_radio_props(
        &self,
        props: &ButtonMarkerProps,
    ) -> Result<&Self, CollectionColorError> {
        update_markers(&self.buttons, props, self.column.labels.len());
        let colors = active_colors(&self.buttons, self.column.labels.len())?;
        show_selection(&self.buttons, &colors, self.selected.get());
        *self.active_colors.borrow_mut() = colors;
        Ok(self)
    }

    /// Fills every button with `color` when selected (matplotlib's
    /// `activecolor` setter).
    ///
    /// # Errors
    ///
    /// When the colour is invalid.
    pub fn set_activecolor(&self, color: impl Into<Color>) -> Result<&Self, CollectionColorError> {
        self.set_radio_props(&ButtonMarkerProps {
            facecolors: Some(vec![color.into()]),
            ..ButtonMarkerProps::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::colors::Color;
    use crate::figure::Figure;
    use crate::text::TextProps;
    use crate::widgets::{ButtonMarkerProps, CheckButtons, RadioButtons};

    #[test]
    fn radio_props_recolour_the_selection_and_resize_the_buttons() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let radio = RadioButtons::new(&fig.add_axes([0.1, 0.1, 0.8, 0.8]), &["a", "b"], 1);
        radio
            .set_radio_props(&ButtonMarkerProps {
                sizes: Some(vec![144.0]),
                facecolors: Some(vec![Color::from("green")]),
                ..ButtonMarkerProps::default()
            })
            .unwrap();
        assert_eq!(radio.buttons.sizes(), Some(vec![144.0, 144.0]));
        let faces = radio.buttons.colors().unwrap().faces;
        assert!(faces[0].is_transparent() && faces[1] == Color::from("green").resolve().unwrap());
        radio.set_activecolor("red").unwrap();
        radio.set_active(0);
        let faces = radio.buttons.colors().unwrap().faces;
        assert_eq!(faces[0], Color::from("red").resolve().unwrap());
    }

    #[test]
    fn check_props_keep_the_states_and_labels_cycle() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let checks = CheckButtons::new(
            &fig.add_axes([0.1, 0.1, 0.8, 0.8]),
            &["a", "b"],
            &[true, false],
        );
        checks.set_label_props(&[TextProps::default().color("red")]);
        assert!(
            checks
                .labels()
                .iter()
                .all(|label| label.color() == Color::from("red"))
        );
        checks
            .set_check_props(&ButtonMarkerProps {
                facecolors: Some(vec![Color::from("blue")]),
                ..ButtonMarkerProps::default()
            })
            .unwrap();
        assert_eq!(checks.status(), [true, false]);
        let faces = checks.checks.colors().unwrap().faces;
        assert_eq!(faces[0], Color::from("blue").resolve().unwrap());
        assert!(faces[1].is_transparent());
        checks.set_active(1, Some(true));
        let faces = checks.checks.colors().unwrap().faces;
        assert_eq!(faces[1], Color::from("blue").resolve().unwrap());
    }
}
