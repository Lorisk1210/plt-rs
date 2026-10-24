//! Options of check and radio buttons: their layout, the properties of
//! their labels and markers, and what can go wrong building them.

use std::fmt;

use crate::colors::Color;
use crate::text::{TextError, TextProps};

/// How check or radio buttons are arranged (matplotlib's `layout`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonLayout {
    /// matplotlib's legacy column: buttons at 0.15 and labels at 0.25 of
    /// the axes width, evenly spread from top to bottom (`None`).
    #[default]
    Legacy,
    /// One column, the labels a fixed distance in points right of the
    /// buttons (`"vertical"`).
    Vertical,
    /// One row, each column as wide as its label (`"horizontal"`).
    Horizontal,
    /// A grid filled left to right, then top to bottom, each column as
    /// wide as its widest label (`(rows, cols)`).
    Grid {
        /// Number of rows.
        rows: usize,
        /// Number of columns.
        cols: usize,
    },
}

/// Properties of the markers of check or radio buttons (matplotlib's
/// `frame_props`, `check_props` and `radio_props`, which are scatter
/// keywords); unset ones keep the widget's defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ButtonMarkerProps {
    /// Marker areas in points², one for all or one per button (`s`);
    /// by default the square of half the label's font size.
    pub sizes: Option<Vec<f64>>,
    /// The colour the face and edge default to (`color`).
    pub color: Option<Color>,
    /// Face colours, cycled over the buttons.
    pub facecolors: Option<Vec<Color>>,
    /// Edge colours, cycled over the buttons.
    pub edgecolors: Option<Vec<Color>>,
    /// Edge widths in points, cycled over the buttons.
    pub linewidths: Option<Vec<f64>>,
}

/// Options of [`CheckButtons`](super::CheckButtons) (matplotlib's keyword
/// arguments).
#[derive(Clone, Debug, Default)]
pub struct CheckButtonsOptions {
    /// The arrangement of the buttons.
    pub layout: ButtonLayout,
    /// Text properties of the labels, cycled: label `i` gets entry
    /// `i % len` (matplotlib's `label_props`).
    pub label_props: Vec<TextProps>,
    /// The square frames: no face and edges 1 pt wide in `plt.ink`
    /// (black) by default.
    pub frame_props: ButtonMarkerProps,
    /// The crosses of checked boxes: 1 pt wide in `plt.ink` (black) by
    /// default.
    pub check_props: ButtonMarkerProps,
}

/// Options of [`RadioButtons`](super::RadioButtons) (matplotlib's keyword
/// arguments).
#[derive(Clone, Debug, Default)]
pub struct RadioButtonsOptions {
    /// The arrangement of the buttons.
    pub layout: ButtonLayout,
    /// Text properties of the labels, cycled: label `i` gets entry
    /// `i % len` (matplotlib's `label_props`).
    pub label_props: Vec<TextProps>,
    /// The fill of the selected button,
    /// `plt.widgets.radiobuttons.activecolor` (blue) by default; face
    /// colours in `radio_props` take precedence (matplotlib's
    /// `activecolor`).
    pub activecolor: Option<Color>,
    /// The round buttons: edges in `plt.ink` (black) and the active colour
    /// by default (matplotlib's `radio_props`).
    pub radio_props: ButtonMarkerProps,
}

/// Why check or radio buttons cannot be built.
#[derive(Clone, Debug, PartialEq)]
pub enum ButtonsError {
    /// A grid layout with fewer cells than labels.
    TooFewPositions {
        /// Cells of the grid.
        positions: usize,
        /// Labels to place.
        labels: usize,
    },
    /// A label cannot be measured for the layout (malformed math).
    Text(TextError),
}

impl fmt::Display for ButtonsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewPositions { positions, labels } => write!(
                f,
                "the layout has {positions} positions but {labels} labels were provided"
            ),
            Self::Text(err) => write!(f, "cannot measure a label: {err}"),
        }
    }
}

impl std::error::Error for ButtonsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Text(err) => Some(err),
            Self::TooFewPositions { .. } => None,
        }
    }
}

impl From<TextError> for ButtonsError {
    fn from(err: TextError) -> Self {
        Self::Text(err)
    }
}
