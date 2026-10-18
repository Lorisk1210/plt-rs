//! Check buttons and radio buttons (matplotlib's `widgets._Buttons`,
//! `CheckButtons` and `RadioButtons`, with the default vertical layout).

use std::cell::Cell;
use std::rc::Rc;

use plt_core::Point;
use plt_text::{HAlign, VAlign};

use super::{Connections, Observers, left_click};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, ScatterColors, ScatterOptions};
use crate::collections::{Collection, EdgeColors};
use crate::colors::Color;
use crate::events::EventKind;
use crate::markers::{Marker, MarkerSymbol};
use crate::numeric::linspace;
use crate::text::Text;

/// Extra distance in points within which a click hits a button marker
/// (matplotlib's default collection `pickradius`).
const PICK_RADIUS: f64 = 5.0;

/// The labelled buttons stacked down an axes.
#[derive(Debug)]
struct ButtonColumn {
    ax: Axes,
    labels: Vec<Text>,
    /// The button centres in axes fractions.
    centres: Vec<Point>,
    /// The marker area in points².
    size: f64,
}

impl ButtonColumn {
    /// Sets up `ax` with one label per entry, the buttons at x = 0.15 and
    /// the labels at 0.25, spread evenly from top to bottom.
    fn new(ax: &Axes, labels: &[&str]) -> Self {
        ax.set_xticks(Vec::<f64>::new());
        ax.set_yticks(Vec::<f64>::new());
        ax.set_navigate(false);
        let ys = linspace(1.0, 0.0, labels.len() + 2);
        let ys = &ys[1..=labels.len()];
        let texts: Vec<Text> = labels
            .iter()
            .zip(ys)
            .map(|(label, &y)| {
                let text = ax.text(0.25, y, *label);
                text.set_transform(ax.trans_axes());
                text.set_halign(HAlign::Left).set_valign(VAlign::Center);
                text
            })
            .collect();
        let font_size = texts.first().map_or(10.0, |text| text.font().size);
        Self {
            ax: ax.clone(),
            labels: texts,
            centres: ys.iter().map(|&y| Point::new(0.15, y)).collect(),
            size: (font_size / 2.0).powi(2),
        }
    }

    /// Markers at the button centres.
    fn markers(&self, symbol: MarkerSymbol, options: ScatterOptions) -> Collection {
        let (x, y): (Vec<f64>, Vec<f64>) = self.centres.iter().map(|p| (p.x, p.y)).unzip();
        let options = ScatterOptions {
            sizes: Some(vec![self.size; self.centres.len()]),
            marker: Some(Marker::Symbol(symbol)),
            offset_transform: Some(self.ax.trans_axes()),
            ..options
        };
        self.ax
            .scatter_with(x, y, options)
            .expect("one marker per label")
    }

    /// The button a click at display point `at` selects: among the
    /// buttons whose marker or label contains it, the closest
    /// (matplotlib's `_clicked`).
    fn hit(&self, at: Point) -> Option<usize> {
        if !self.ax.contains_point(at) {
            return None;
        }
        let figure = self.ax.figure();
        figure.layout();
        let bbox = self.ax.window_extent()?;
        let centres: Vec<Point> = self
            .centres
            .iter()
            .map(|c| {
                Point::new(
                    bbox.x0() + c.x * bbox.width(),
                    bbox.y0() + c.y * bbox.height(),
                )
            })
            .collect();
        let reach = (self.size.sqrt() / 2.0 + PICK_RADIUS) * figure.dpi() / 72.0;
        centres
            .iter()
            .enumerate()
            .filter(|(i, c)| {
                (c.x - at.x).abs().max((c.y - at.y).abs()) <= reach
                    || self.labels[*i]
                        .window_extent()
                        .is_some_and(|b| b.contains(at))
            })
            .min_by(|(_, a), (_, b)| {
                let d = |p: &Point| (p.x - at.x).powi(2) + (p.y - at.y).powi(2);
                d(a).total_cmp(&d(b))
            })
            .map(|(i, _)| i)
    }
}

/// Check boxes that each toggle on and off (matplotlib's `CheckButtons`).
#[derive(Debug)]
pub struct CheckButtons {
    column: Rc<ButtonColumn>,
    checks: Collection,
    states: Rc<Vec<Cell<bool>>>,
    clicked: Observers<Option<String>>,
    _connections: Connections,
}

impl CheckButtons {
    /// Turns `ax` into check boxes labelled `labels`, checked where
    /// `actives` says so.
    pub fn new(ax: &Axes, labels: &[&str], actives: &[bool]) -> Self {
        let column = Rc::new(ButtonColumn::new(ax, labels));
        column.markers(
            MarkerSymbol::Square,
            ScatterOptions {
                colors: ScatterColors::Single(Color::none()),
                edgecolors: Some(EdgeColors::Colors(vec![Color::from("black")])),
                linewidths: Some(vec![1.0]),
                ..ScatterOptions::default()
            },
        );
        let checks = column.markers(
            MarkerSymbol::X,
            ScatterOptions {
                colors: ScatterColors::Single(Color::from("black")),
                linewidths: Some(vec![1.0]),
                ..ScatterOptions::default()
            },
        );
        let states: Rc<Vec<Cell<bool>>> = Rc::new(
            (0..labels.len())
                .map(|i| Cell::new(actives.get(i).copied().unwrap_or(false)))
                .collect(),
        );
        show_checks(&checks, &states);
        let clicked = Observers::default();
        let mut connections = Connections::new(ax.figure());
        {
            let (column, checks, states, clicked) = (
                Rc::clone(&column),
                checks.clone(),
                Rc::clone(&states),
                clicked.clone(),
            );
            connections.connect(EventKind::ButtonPress, move |event| {
                if let Some(index) = left_click(event).and_then(|at| column.hit(at)) {
                    toggle(&states, &checks, index, None);
                    clicked.process(&Some(column.labels[index].text()));
                }
            });
        }
        Self {
            column,
            checks,
            states,
            clicked,
            _connections: connections,
        }
    }

    /// Whether each box is checked.
    pub fn status(&self) -> Vec<bool> {
        self.states.iter().map(Cell::get).collect()
    }

    /// Checks (`Some(true)`), unchecks or toggles (`None`) box `index` and
    /// runs the callbacks with its label; `None` for an index out of range.
    pub fn set_active(&self, index: usize, state: Option<bool>) -> Option<()> {
        let label = self.column.labels.get(index)?.text();
        toggle(&self.states, &self.checks, index, state);
        self.clicked.process(&Some(label));
        Some(())
    }

    /// The labels.
    pub fn labels(&self) -> &[Text] {
        &self.column.labels
    }

    /// Runs `callback` with the label of each box clicked (matplotlib's
    /// `on_clicked`).
    pub fn on_clicked(&self, mut callback: impl FnMut(Option<&str>) + 'static) {
        self.clicked
            .connect(move |label: &Option<String>| callback(label.as_deref()));
    }
}

/// Shows the crosses of the checked boxes.
fn show_checks(checks: &Collection, states: &[Cell<bool>]) {
    checks.set_facecolors(
        states
            .iter()
            .map(|state| {
                if state.get() {
                    Color::from("black")
                } else {
                    Color::none()
                }
            })
            .collect(),
    );
}

fn toggle(states: &[Cell<bool>], checks: &Collection, index: usize, state: Option<bool>) {
    let cell = &states[index];
    cell.set(state.unwrap_or(!cell.get()));
    show_checks(checks, states);
}

/// Mutually exclusive choices (matplotlib's `RadioButtons`).
#[derive(Debug)]
pub struct RadioButtons {
    column: Rc<ButtonColumn>,
    buttons: Collection,
    selected: Rc<Cell<usize>>,
    activecolor: Color,
    clicked: Observers<String>,
    _connections: Connections,
}

impl RadioButtons {
    /// Turns `ax` into radio buttons labelled `labels` with button
    /// `active` selected (filled blue).
    pub fn new(ax: &Axes, labels: &[&str], active: usize) -> Self {
        let column = Rc::new(ButtonColumn::new(ax, labels));
        let activecolor = Color::from("blue");
        let buttons = column.markers(
            MarkerSymbol::Circle,
            ScatterOptions {
                colors: ScatterColors::Single(activecolor.clone()),
                edgecolors: Some(EdgeColors::Colors(vec![Color::from("black")])),
                ..ScatterOptions::default()
            },
        );
        let selected = Rc::new(Cell::new(active));
        show_selection(&buttons, labels.len(), active, &activecolor);
        let clicked = Observers::default();
        let mut connections = Connections::new(ax.figure());
        {
            let (column, buttons, selected, clicked, color) = (
                Rc::clone(&column),
                buttons.clone(),
                Rc::clone(&selected),
                clicked.clone(),
                activecolor.clone(),
            );
            connections.connect(EventKind::ButtonPress, move |event| {
                if let Some(index) = left_click(event).and_then(|at| column.hit(at)) {
                    selected.set(index);
                    show_selection(&buttons, column.labels.len(), index, &color);
                    clicked.process(&column.labels[index].text());
                }
            });
        }
        Self {
            column,
            buttons,
            selected,
            activecolor,
            clicked,
            _connections: connections,
        }
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
        show_selection(
            &self.buttons,
            self.column.labels.len(),
            index,
            &self.activecolor,
        );
        self.clicked.process(&label);
        Some(())
    }

    /// The labels.
    pub fn labels(&self) -> &[Text] {
        &self.column.labels
    }

    /// Runs `callback` with the label of each button selected
    /// (matplotlib's `on_clicked`).
    pub fn on_clicked(&self, mut callback: impl FnMut(&str) + 'static) {
        self.clicked.connect(move |label: &String| callback(label));
    }
}

/// Fills the selected button.
fn show_selection(buttons: &Collection, count: usize, selected: usize, color: &Color) {
    buttons.set_facecolors(
        (0..count)
            .map(|i| {
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
}
