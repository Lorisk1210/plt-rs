//! A selector connected to its figure: the driver every selector holds,
//! which hands the figure's mouse, key and draw events to its shared state
//! and callbacks and runs the updates they ask for.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use super::super::blit::{self, Background, Updater};
use super::{Core, Shared, Tool};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::{Event, EventKind};
use crate::figure::Figure;
use crate::widgets::{CallbackId, Connections, Observers};

/// A selector connected to its figure: the shared state, the callbacks
/// and the event connections.
pub(in crate::widgets::selectors) struct Driver<T: Tool> {
    pub(in crate::widgets::selectors) shared: Rc<RefCell<Shared<T>>>,
    /// Called with each completed selection (matplotlib's `onselect`).
    pub(in crate::widgets::selectors) selected: Observers<T::Selection>,
    /// Called while a selection changes (matplotlib's `onmove_callback`).
    pub(in crate::widgets::selectors) moved: Observers<T::Selection>,
    /// What a blitting selector draws its artists over.
    background: Rc<Background>,
    /// The connections to the events of the selector's figure.
    connections: RefCell<Connections>,
}

impl<T: Tool + std::fmt::Debug> std::fmt::Debug for Driver<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Driver")
            .field("shared", &self.shared)
            .finish_non_exhaustive()
    }
}

impl<T: Tool> Driver<T> {
    /// Disconnects callback `id` from either signal; returns whether it
    /// was connected.
    pub(in crate::widgets::selectors) fn disconnect(&self, id: CallbackId) -> bool {
        self.selected.disconnect(id) || self.moved.disconnect(id)
    }

    /// Connects a selector made of `core` and `tool` to the figure's
    /// mouse, key and draw events; a blitting selector's artists are
    /// animated.
    pub(in crate::widgets::selectors) fn new(core: Core, tool: T) -> Self {
        let figure = core.ax.figure();
        let shared = Rc::new(RefCell::new(Shared { core, tool }));
        let background = Rc::new(Background::default());
        let updater = {
            let (shared, background) = (Rc::downgrade(&shared), Rc::clone(&background));
            Updater::new(move || {
                if let Some(shared) = shared.upgrade() {
                    show_changes(&shared, &background);
                }
            })
        };
        shared.borrow_mut().core.updater = updater;
        let selected = Observers::default();
        let moved = Observers::default();
        let connections = RefCell::new(connect_events(
            figure,
            &shared,
            &background,
            &selected,
            &moved,
        ));
        let driver = Self {
            shared,
            selected,
            moved,
            background,
            connections,
        };
        driver.animate_artists();
        driver
    }

    /// Whether the selector blits (matplotlib's `useblit`).
    pub(in crate::widgets::selectors) fn useblit(&self) -> bool {
        self.shared.borrow().core.useblit
    }

    /// Makes the artists of a blitting selector animated, as matplotlib
    /// makes them (`animated=self.useblit`): figure draws leave them out.
    pub(in crate::widgets::selectors) fn animate_artists(&self) {
        let shared = self.shared.borrow();
        if shared.core.useblit {
            for artist in shared.tool.artists() {
                artist.set_animated(true);
            }
        }
    }

    /// Turns the selector on or off (matplotlib's `set_active`): one turned
    /// on saves its background again.
    pub(in crate::widgets::selectors) fn set_active(&self, active: bool) {
        let (scene, useblit) = {
            let mut shared = self.shared.borrow_mut();
            shared.core.active = active;
            (shared.scene(), shared.core.useblit)
        };
        if active && useblit {
            self.background.save(&scene.0, &scene.1);
        }
    }

    /// Makes the selector work in `ax` from now on, taking the events of
    /// its figure (the event routing of matplotlib's `new_axes`, which
    /// connects to the canvas of the new axes when it is another).
    pub(in crate::widgets::selectors) fn move_to(&self, ax: &Axes) {
        self.shared.borrow_mut().core.ax = ax.clone();
        let figure = ax.figure();
        if *self.connections.borrow().figure() != figure {
            let connections = connect_events(
                figure,
                &self.shared,
                &self.background,
                &self.selected,
                &self.moved,
            );
            // The old connections end as they are dropped.
            drop(self.connections.replace(connections));
        }
        self.animate_artists();
    }
}

/// Shows the changes of the selector behind `shared` (matplotlib's
/// `update`): unless its axes is hidden, its artists blitted over
/// `background` or, without blitting, a full draw asked for.
fn show_changes<T: Tool>(shared: &RefCell<Shared<T>>, background: &Background) {
    let ((ax, artists), useblit) = {
        let shared = shared.borrow();
        (shared.scene(), shared.core.useblit)
    };
    if !ax.visible() {
        return;
    }
    if useblit {
        background.update(&ax, &artists);
    } else {
        let canvas = ax.figure().canvas();
        canvas.report(canvas.draw_idle());
    }
}

/// Connects the selector behind `shared` to the mouse, key and draw events
/// of `figure`, its selections reported to `selected` and `moved`, the
/// updates it asks for run after each event.
fn connect_events<T: Tool>(
    figure: Figure,
    shared: &Rc<RefCell<Shared<T>>>,
    background: &Rc<Background>,
    selected: &Observers<T::Selection>,
    moved: &Observers<T::Selection>,
) -> Connections {
    let mut connections = Connections::new(figure);
    let weak = || Rc::downgrade(shared);
    connect(
        &mut connections,
        EventKind::ButtonPress,
        weak(),
        |shared, event| {
            if let Event::ButtonPress(mouse) = event {
                shared.borrow_mut().press(mouse);
            }
        },
    );
    let observers = selected.clone();
    connect(
        &mut connections,
        EventKind::ButtonRelease,
        weak(),
        move |shared, event| {
            if let Event::ButtonRelease(mouse) = event {
                let selection = shared.borrow_mut().release(mouse);
                if let Some(selection) = selection {
                    observers.process(&selection);
                }
            }
        },
    );
    let observers = moved.clone();
    connect(
        &mut connections,
        EventKind::Motion,
        weak(),
        move |shared, event| {
            if let Event::Motion(mouse) = event {
                let selection = shared.borrow_mut().motion(mouse);
                if let Some(selection) = selection {
                    observers.process(&selection);
                }
            }
        },
    );
    connect(
        &mut connections,
        EventKind::KeyPress,
        weak(),
        |shared, event| {
            if let Event::KeyPress(key) = event {
                shared.borrow_mut().key_press(key);
            }
        },
    );
    connect(
        &mut connections,
        EventKind::KeyRelease,
        weak(),
        |shared, event| {
            if let Event::KeyRelease(key) = event {
                shared.borrow_mut().key_release(key);
            }
        },
    );
    let background = Rc::clone(background);
    connect(
        &mut connections,
        EventKind::Draw,
        weak(),
        move |shared, _| {
            // matplotlib's `update_background` on `draw_event`. A draw made
            // while the selector's state is borrowed (by a selector it holds)
            // keeps the background; the next update saves one.
            let Ok(shared) = shared.try_borrow() else {
                return;
            };
            if shared.core.useblit {
                let (ax, artists) = shared.scene();
                drop(shared);
                background.save(&ax, &artists);
            }
        },
    );
    connections
}

/// Connects `handler` for events of `kind`, for as long as the selector
/// behind `shared` lives, running the updates it asks for after it.
fn connect<T: Tool>(
    connections: &mut Connections,
    kind: EventKind,
    shared: Weak<RefCell<Shared<T>>>,
    handler: impl Fn(&RefCell<Shared<T>>, &Event) + 'static,
) {
    connections.connect(kind, move |event| {
        if let Some(shared) = shared.upgrade() {
            handler(&shared, event);
            blit::run_requested();
        }
    });
}
