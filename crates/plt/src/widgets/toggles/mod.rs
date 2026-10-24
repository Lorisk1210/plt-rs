//! Check buttons and radio buttons (matplotlib's `widgets._Buttons`,
//! `CheckButtons` and `RadioButtons`), in matplotlib's legacy column or
//! the 3.11 measured layouts, with labels and markers styled on creation
//! or restyled later.

mod column;
mod options;
mod restyle;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub use options::{
    ButtonLayout, ButtonMarkerProps, ButtonsError, CheckButtonsOptions, RadioButtonsOptions,
};

use super::{CallbackId, Connections, Observers, left_click};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, ScatterColors};
use crate::collections::{Collection, EdgeColors};
use crate::colors::Color;
use crate::events::EventKind;
use crate::markers::MarkerSymbol;
use crate::text::Text;
use column::{ButtonColumn, MarkerLook, cycled};

/// `colors` cycled to one per button, or `fallback` for every button when
/// unset.
fn per_button(colors: Option<&Vec<Color>>, fallback: Color, count: usize) -> Vec<Color> {
    let colors = colors.map_or_else(|| vec![fallback], Clone::clone);
    (0..count)
        .map(|i| cycled(&colors, i).cloned().unwrap_or_else(Color::none))
        .collect()
}

/// Check boxes that each toggle on and off (matplotlib's `CheckButtons`).
#[derive(Debug)]
pub struct CheckButtons {
    column: Rc<ButtonColumn>,
    frames: Collection,
    checks: Collection,
    states: Rc<Vec<Cell<bool>>>,
    /// The colour of each cross when checked (matplotlib's
    /// `_active_check_colors`).
    active_colors: Rc<RefCell<Vec<Color>>>,
    clicked: Observers<Option<String>>,
    _connections: Connections,
}

impl CheckButtons {
    /// Turns `ax` into check boxes labelled `labels`, checked where
    /// `actives` says so, in matplotlib's default style.
    pub fn new(ax: &Axes, labels: &[&str], actives: &[bool]) -> Self {
        Self::with_options(ax, labels, actives, CheckButtonsOptions::default())
            .expect("the legacy layout measures no label")
    }

    /// Check boxes arranged and styled as `options` say.
    ///
    /// # Errors
    ///
    /// When a grid layout has fewer cells than labels, or a label cannot
    /// be measured for a measured layout.
    pub fn with_options(
        ax: &Axes,
        labels: &[&str],
        actives: &[bool],
        options: CheckButtonsOptions,
    ) -> Result<Self, ButtonsError> {
        let column = Rc::new(ButtonColumn::new(
            ax,
            labels,
            options.layout,
            &options.label_props,
        )?);
        let frame = &options.frame_props;
        let frames = column.markers(
            MarkerSymbol::Square,
            MarkerLook {
                sizes: column.sizes_or_default(frame.sizes.as_ref()),
                colors: ScatterColors::PerPoint(per_button(
                    frame.facecolors.as_ref(),
                    frame.color.clone().unwrap_or_else(Color::none),
                    labels.len(),
                )),
                edgecolors: Some(EdgeColors::Colors(per_button(
                    frame.edgecolors.as_ref(),
                    frame.color.clone().unwrap_or_else(|| Color::from("black")),
                    labels.len(),
                ))),
                linewidths: Some(frame.linewidths.clone().unwrap_or_else(|| vec![1.0])),
            },
        );
        let check = &options.check_props;
        let active_colors = Rc::new(RefCell::new(per_button(
            check.facecolors.as_ref(),
            check.color.clone().unwrap_or_else(|| Color::from("black")),
            labels.len(),
        )));
        let checks = column.markers(
            MarkerSymbol::X,
            MarkerLook {
                sizes: column.sizes_or_default(check.sizes.as_ref()),
                colors: ScatterColors::PerPoint(active_colors.borrow().clone()),
                edgecolors: check.edgecolors.clone().map(EdgeColors::Colors),
                linewidths: Some(check.linewidths.clone().unwrap_or_else(|| vec![1.0])),
            },
        );
        let states: Rc<Vec<Cell<bool>>> = Rc::new(
            (0..labels.len())
                .map(|i| Cell::new(actives.get(i).copied().unwrap_or(false)))
                .collect(),
        );
        show_checks(&checks, &states, &active_colors.borrow());
        let clicked = Observers::default();
        let mut connections = Connections::new(ax.figure());
        {
            let (column, checks, states, colors, clicked) = (
                Rc::clone(&column),
                checks.clone(),
                Rc::clone(&states),
                Rc::clone(&active_colors),
                clicked.clone(),
            );
            connections.connect(EventKind::ButtonPress, move |event| {
                if let Some(index) = left_click(event).and_then(|at| column.hit(at, &checks)) {
                    toggle(&states, &checks, &colors.borrow(), index, None);
                    clicked.process(&Some(column.labels[index].text()));
                }
            });
        }
        Ok(Self {
            column,
            frames,
            checks,
            states,
            active_colors,
            clicked,
            _connections: connections,
        })
    }

    /// Whether each box is checked.
    pub fn status(&self) -> Vec<bool> {
        self.states.iter().map(Cell::get).collect()
    }

    /// Checks (`Some(true)`), unchecks or toggles (`None`) box `index` and
    /// runs the callbacks with its label; `None` for an index out of range.
    pub fn set_active(&self, index: usize, state: Option<bool>) -> Option<()> {
        let label = self.column.labels.get(index)?.text();
        toggle(
            &self.states,
            &self.checks,
            &self.active_colors.borrow(),
            index,
            state,
        );
        self.clicked.process(&Some(label));
        Some(())
    }

    /// The labels.
    pub fn labels(&self) -> &[Text] {
        &self.column.labels
    }

    /// Runs `callback` with the label of each box clicked (matplotlib's
    /// `on_clicked`); returns its id for [`CheckButtons::disconnect`].
    pub fn on_clicked(&self, mut callback: impl FnMut(Option<&str>) + 'static) -> CallbackId {
        self.clicked
            .connect(move |label: &Option<String>| callback(label.as_deref()))
    }

    /// Disconnects the callback with id `id` (matplotlib's `disconnect`);
    /// returns whether it was connected.
    pub fn disconnect(&self, id: CallbackId) -> bool {
        self.clicked.disconnect(id)
    }
}

/// Shows the crosses of the checked boxes in their active colours.
fn show_checks(checks: &Collection, states: &[Cell<bool>], colors: &[Color]) {
    checks.set_facecolors(
        states
            .iter()
            .zip(colors)
            .map(|(state, color)| {
                if state.get() {
                    color.clone()
                } else {
                    Color::none()
                }
            })
            .collect(),
    );
}

fn toggle(
    states: &[Cell<bool>],
    checks: &Collection,
    colors: &[Color],
    index: usize,
    state: Option<bool>,
) {
    let cell = &states[index];
    cell.set(state.unwrap_or(!cell.get()));
    show_checks(checks, states, colors);
}

/// Mutually exclusive choices (matplotlib's `RadioButtons`).
#[derive(Debug)]
pub struct RadioButtons {
    column: Rc<ButtonColumn>,
    buttons: Collection,
    selected: Rc<Cell<usize>>,
    /// The fill of each button when selected (matplotlib's
    /// `_active_colors`).
    active_colors: Rc<RefCell<Vec<Color>>>,
    clicked: Observers<String>,
    _connections: Connections,
}

impl RadioButtons {
    /// Turns `ax` into radio buttons labelled `labels` with button
    /// `active` selected, in matplotlib's default style (filled blue).
    pub fn new(ax: &Axes, labels: &[&str], active: usize) -> Self {
        Self::with_options(ax, labels, active, RadioButtonsOptions::default())
            .expect("the legacy layout measures no label")
    }

    /// Radio buttons arranged and styled as `options` say.
    ///
    /// # Errors
    ///
    /// When a grid layout has fewer cells than labels, or a label cannot
    /// be measured for a measured layout.
    pub fn with_options(
        ax: &Axes,
        labels: &[&str],
        active: usize,
        options: RadioButtonsOptions,
    ) -> Result<Self, ButtonsError> {
        let column = Rc::new(ButtonColumn::new(
            ax,
            labels,
            options.layout,
            &options.label_props,
        )?);
        let radio = &options.radio_props;
        let activecolor = options
            .activecolor
            .clone()
            .unwrap_or_else(|| Color::from("blue"));
        let active_colors = Rc::new(RefCell::new(per_button(
            radio.facecolors.as_ref(),
            radio.color.clone().unwrap_or(activecolor),
            labels.len(),
        )));
        let buttons = column.markers(
            MarkerSymbol::Circle,
            MarkerLook {
                sizes: column.sizes_or_default(radio.sizes.as_ref()),
                colors: ScatterColors::PerPoint(active_colors.borrow().clone()),
                edgecolors: Some(EdgeColors::Colors(per_button(
                    radio.edgecolors.as_ref(),
                    radio.color.clone().unwrap_or_else(|| Color::from("black")),
                    labels.len(),
                ))),
                linewidths: radio.linewidths.clone(),
            },
        );
        let selected = Rc::new(Cell::new(active));
        show_selection(&buttons, &active_colors.borrow(), active);
        let clicked = Observers::default();
        let mut connections = Connections::new(ax.figure());
        {
            let (column, buttons, selected, colors, clicked) = (
                Rc::clone(&column),
                buttons.clone(),
                Rc::clone(&selected),
                Rc::clone(&active_colors),
                clicked.clone(),
            );
            connections.connect(EventKind::ButtonPress, move |event| {
                if let Some(index) = left_click(event).and_then(|at| column.hit(at, &buttons)) {
                    selected.set(index);
                    show_selection(&buttons, &colors.borrow(), index);
                    clicked.process(&column.labels[index].text());
                }
            });
        }
        Ok(Self {
            column,
            buttons,
            selected,
            active_colors,
            clicked,
            _connections: connections,
        })
    }

    /// The index of the selected button.
    pub fn index_selected(&self) -> usize {
        self.selected.get()
    }

    /// The label of the selected button.
    pub fn value_selected(&self) -> String {
        self.column.labels[self.selected.get()].text()
    }

    /// Selects button `index` and runs the callbacks with its label;
    /// `None` for an index out of range.
    pub fn set_active(&self, index: usize) -> Option<()> {
        let label = self.column.labels.get(index)?.text();
        self.selected.set(index);
        show_selection(&self.buttons, &self.active_colors.borrow(), index);
        self.clicked.process(&label);
        Some(())
    }

    /// The labels.
    pub fn labels(&self) -> &[Text] {
        &self.column.labels
    }

    /// Runs `callback` with the label of each button selected
    /// (matplotlib's `on_clicked`); returns its id for
    /// [`RadioButtons::disconnect`].
    pub fn on_clicked(&self, mut callback: impl FnMut(&str) + 'static) -> CallbackId {
        self.clicked.connect(move |label: &String| callback(label))
    }

    /// Disconnects the callback with id `id` (matplotlib's `disconnect`);
    /// returns whether it was connected.
    pub fn disconnect(&self, id: CallbackId) -> bool {
        self.clicked.disconnect(id)
    }
}

/// Fills the selected button with its active colour.
fn show_selection(buttons: &Collection, colors: &[Color], selected: usize) {
    buttons.set_facecolors(
        colors
            .iter()
            .enumerate()
            .map(|(i, color)| {
                if i == selected {
                    color.clone()
                } else {
                    Color::none()
                }
            })
            .collect(),
    );
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::events::EventSource;
    use crate::figure::Figure;
    use crate::navigation::PointerButton;

    #[test]
    fn check_buttons_toggle() {
        let fig = Figure::with_size(6.0, 3.0, 100.0);
        let ax = fig.add_axes([0.05, 0.3, 0.25, 0.5]);
        let checks = CheckButtons::new(&ax, &["red", "green", "blue"], &[true, false, true]);
        assert_eq!(checks.status(), [true, false, true]);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&seen);
        checks.on_clicked(move |label| log.borrow_mut().push(label.map(str::to_owned)));
        checks.set_active(1, None);
        assert_eq!(checks.status(), [true, true, true]);
        // Click the middle box: x = 0.05 + 0.15 * 0.25 of 600 px, y = middle.
        let mut source = EventSource::new();
        source.button_press(
            &fig,
            (0.05 + 0.15 * 0.25) * 600.0,
            0.55 * 300.0,
            PointerButton::Left,
            false,
        );
        assert_eq!(checks.status(), [true, false, true]);
        assert_eq!(
            *seen.borrow(),
            [Some("green".to_owned()), Some("green".to_owned())]
        );
    }

    #[test]
    fn radio_buttons_select_one() {
        let fig = Figure::with_size(6.0, 3.0, 100.0);
        let ax = fig.add_axes([0.38, 0.3, 0.25, 0.5]);
        let radio = RadioButtons::new(&ax, &["one", "two", "three"], 1);
        assert_eq!(radio.value_selected(), "two");
        radio.set_active(2);
        assert_eq!(radio.index_selected(), 2);
        assert_eq!(radio.set_active(7), None);
    }

    #[test]
    fn styled_buttons_cycle_label_props_and_marker_colours() {
        use crate::text::TextProps;

        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
        let options = RadioButtonsOptions {
            label_props: vec![
                TextProps::default().color("red"),
                TextProps::default().color("blue"),
            ],
            radio_props: ButtonMarkerProps {
                facecolors: Some(vec!["C0".into(), "C1".into(), "C2".into()]),
                ..ButtonMarkerProps::default()
            },
            ..RadioButtonsOptions::default()
        };
        let radio = RadioButtons::with_options(&ax, &["a", "b", "c"], 1, options).unwrap();
        assert_eq!(radio.labels()[2].color(), Color::from("red"));
        assert_eq!(radio.active_colors.borrow()[1], Color::from("C1"));
        radio.set_active(2);
        assert_eq!(radio.value_selected(), "c");
    }

    #[test]
    fn measured_layouts_place_labels_in_rows_and_columns() {
        let fig = Figure::with_size(6.0, 3.0, 100.0);
        let ax = fig.add_axes([0.05, 0.1, 0.9, 0.35]);
        let grid = CheckButtonsOptions {
            layout: ButtonLayout::Grid { rows: 2, cols: 2 },
            ..CheckButtonsOptions::default()
        };
        let checks = CheckButtons::with_options(&ax, &["a", "bb", "c"], &[true], grid).unwrap();
        let at: Vec<plt_core::Point> = checks.labels().iter().map(Text::position).collect();
        assert_eq!(at[0].y, at[1].y);
        assert!(at[1].x > at[0].x);
        assert_eq!(at[2].x, at[0].x);
        assert!(at[2].y < at[0].y);
        assert_eq!(checks.status(), [true, false, false]);
        let too_small = CheckButtonsOptions {
            layout: ButtonLayout::Grid { rows: 1, cols: 2 },
            ..CheckButtonsOptions::default()
        };
        let refused = CheckButtons::with_options(&ax, &["a", "b", "c"], &[], too_small).err();
        assert_eq!(
            refused,
            Some(ButtonsError::TooFewPositions {
                positions: 2,
                labels: 3
            })
        );
    }
}
