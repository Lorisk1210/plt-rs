//! Push buttons (matplotlib's `widgets.Button`).

use std::cell::Cell;
use std::rc::Rc;

use plt_core::Point;
use plt_text::{HAlign, VAlign};

use super::{Connections, Observers, left_click};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::colors::Color;
use crate::events::{Event, EventKind};
use crate::text::Text;

/// Colours of a [`Button`].
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonOptions {
    /// The background when the pointer is elsewhere.
    pub color: Color,
    /// The background under the pointer.
    pub hovercolor: Color,
}

impl Default for ButtonOptions {
    /// matplotlib's light grey button, lighter when hovered.
    fn default() -> Self {
        Self {
            color: Color::from("0.85"),
            hovercolor: Color::from("0.95"),
        }
    }
}

/// A button filling an axes, with a centred label; it runs its callbacks
/// when pressed and released with the left button over it.
#[derive(Debug)]
pub struct Button {
    ax: Axes,
    label: Text,
    clicked: Observers<()>,
    _connections: Connections,
}

impl Button {
    /// Turns `ax` into a button showing `label`.
    pub fn new(ax: &Axes, label: &str, options: ButtonOptions) -> Self {
        let text = ax.text(0.5, 0.5, label);
        text.set_transform(ax.trans_axes());
        text.set_valign(VAlign::Center).set_halign(HAlign::Center);
        ax.set_navigate(false);
        ax.set_facecolor(options.color.clone());
        ax.set_xticks(Vec::<f64>::new());
        ax.set_yticks(Vec::<f64>::new());
        let clicked = Observers::default();
        let pressed = Rc::new(Cell::new(false));
        let mut connections = Connections::new(ax.figure());
        {
            let (ax, pressed) = (ax.clone(), Rc::clone(&pressed));
            connections.connect(EventKind::ButtonPress, move |event| {
                if left_click(event).is_some_and(|at| ax.contains_point(at)) {
                    pressed.set(true);
                }
            });
        }
        {
            let (ax, clicked) = (ax.clone(), clicked.clone());
            connections.connect(EventKind::ButtonRelease, move |event| {
                let Some(at) = left_click(event) else {
                    return;
                };
                if pressed.replace(false) && ax.contains_point(at) {
                    clicked.process(&());
                }
            });
        }
        {
            let ax = ax.clone();
            connections.connect(EventKind::Motion, move |event| {
                if let Event::Motion(mouse) = event {
                    let over = ax.contains_point(Point::new(mouse.location.x, mouse.location.y));
                    let color = if over {
                        &options.hovercolor
                    } else {
                        &options.color
                    };
                    if ax.facecolor() != *color {
                        ax.set_facecolor(color.clone());
                    }
                }
            });
        }
        Self {
            ax: ax.clone(),
            label: text,
            clicked,
            _connections: connections,
        }
    }

    /// The axes the button fills.
    pub fn ax(&self) -> &Axes {
        &self.ax
    }

    /// The label.
    pub fn label(&self) -> &Text {
        &self.label
    }

    /// Runs `callback` on every click (matplotlib's `on_clicked`).
    pub fn on_clicked(&self, mut callback: impl FnMut() + 'static) {
        self.clicked.connect(move |()| callback());
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::events::EventSource;
    use crate::figure::Figure;
    use crate::navigation::PointerButton;

    #[test]
    fn clicks_and_hover() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.add_axes([0.5, 0.5, 0.4, 0.2]);
        let button = Button::new(&ax, "Reset", ButtonOptions::default());
        let clicks = Rc::new(Cell::new(0));
        let counter = Rc::clone(&clicks);
        button.on_clicked(move || counter.set(counter.get() + 1));
        let mut source = EventSource::new();
        let (x, y) = (280.0, 180.0);
        source.motion(&fig, x, y);
        assert_eq!(ax.facecolor(), Color::from("0.95"), "hovered");
        source.button_press(&fig, x, y, PointerButton::Left, false);
        source.button_release(&fig, x, y, PointerButton::Left);
        assert_eq!(clicks.get(), 1);
        source.button_press(&fig, x, y, PointerButton::Left, false);
        source.button_release(&fig, 10.0, 10.0, PointerButton::Left);
        assert_eq!(clicks.get(), 1, "releasing elsewhere cancels the click");
        drop(button);
        source.button_press(&fig, x, y, PointerButton::Left, false);
        source.button_release(&fig, x, y, PointerButton::Left);
        assert_eq!(clicks.get(), 1, "a dropped button is disconnected");
    }
}
