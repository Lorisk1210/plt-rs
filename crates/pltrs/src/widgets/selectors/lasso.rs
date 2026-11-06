//! Free-hand selections (matplotlib's `LassoSelector` and `Lasso`).

use std::cell::RefCell;
use std::rc::Rc;

use pltrs_core::Point;

use super::driver::{Core, Driver, Tool};
use super::{ModifierKeys, SelectorEvent, selector_methods};
use crate::artist::{Artist, ArtistHandle, RawHandle};
use crate::axes::Axes;
use crate::events::{Event, EventKind};
use crate::figure::BufferRegion;
use crate::lines::{Line2D, LineArtist, LineProps, LineStyle};
use crate::navigation::PointerButton;
use crate::widgets::{CallbackId, Connections, Observers, data_in};

/// Options of a [`LassoSelector`] (matplotlib's keyword arguments).
#[derive(Clone, Debug)]
pub struct LassoSelectorOptions {
    /// Look of the curve (`props`).
    pub props: LineProps,
    /// The buttons that select; all when `None` (`button`).
    pub buttons: Option<Vec<PointerButton>>,
    /// Whether the curve is shown by blitting it, animated, rather than by
    /// full draws (`useblit`).
    pub useblit: bool,
}

impl Default for LassoSelectorOptions {
    /// matplotlib's defaults: a line of the rcParams' look, blitted,
    /// started by any button.
    fn default() -> Self {
        Self {
            props: LineProps::default(),
            buttons: None,
            useblit: true,
        }
    }
}

/// Options of a [`Lasso`] (matplotlib's keyword arguments).
#[derive(Clone, Debug)]
pub struct LassoOptions {
    /// Look of the curve over matplotlib's solid black line two points
    /// wide (`props`).
    pub props: LineProps,
    /// Whether each motion blits the curve over the background the lasso
    /// saved when it was made, rather than asking for a full draw
    /// (`useblit`).
    pub useblit: bool,
}

impl Default for LassoOptions {
    /// matplotlib's defaults: the black line, blitted.
    fn default() -> Self {
        Self {
            props: LineProps::default(),
            useblit: true,
        }
    }
}

/// A line of `props` through `points` added to `ax` without autoscaling
/// (matplotlib's `ax.add_line(Line2D(...))`).
fn add_line(ax: &Axes, (x, y): (Vec<f64>, Vec<f64>), props: LineProps) -> Line2D {
    let mut line = LineArtist::new(x, y, crate::rc::params());
    props.apply(&mut line);
    line.base_mut().transform = Some(ax.trans_data());
    ax.add_line_artist(line)
}

/// `points` split into x and y values.
fn unzip(points: &[(f64, f64)]) -> (Vec<f64>, Vec<f64>) {
    points.iter().copied().unzip()
}

/// The lasso selector's own state.
#[derive(Debug)]
struct LassoTool {
    line: Line2D,
    verts: Option<Vec<(f64, f64)>>,
}

impl Tool for LassoTool {
    type Selection = Vec<(f64, f64)>;

    fn artists(&self) -> Vec<RawHandle> {
        vec![self.line.raw().clone()]
    }

    fn show(&self, visible: bool) {
        self.line.set_visible(visible);
    }

    fn press(&mut self, _core: &mut Core, event: &SelectorEvent) {
        self.verts = Some(vec![(event.xdata, event.ydata)]);
        self.line.set_visible(true);
    }

    fn drag(&mut self, core: &mut Core, event: &SelectorEvent) -> Option<Self::Selection> {
        let verts = self.verts.as_mut()?;
        verts.push((event.xdata, event.ydata));
        let (x, y) = unzip(verts);
        self.line.set_data(&x, &y);
        core.request_update();
        None
    }

    fn release(&mut self, _core: &mut Core, event: &SelectorEvent) -> Option<Self::Selection> {
        let selection = self.verts.take().map(|mut verts| {
            verts.push((event.xdata, event.ydata));
            verts
        });
        self.line.set_data(Vec::<f64>::new(), Vec::<f64>::new());
        self.line.set_visible(false);
        selection
    }
}

/// A free-hand curve drawn while a button is held (matplotlib's
/// `LassoSelector`); the callbacks receive its vertices in data
/// coordinates, for example to pick the points inside it.
#[derive(Debug)]
pub struct LassoSelector {
    driver: Driver<LassoTool>,
}

selector_methods!(LassoSelector);

impl LassoSelector {
    /// A lasso selector on `ax`.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::widgets::{LassoSelector, LassoSelectorOptions};
    /// let ax = Figure::new().subplot();
    /// let lasso = LassoSelector::new(&ax, LassoSelectorOptions::default());
    /// lasso.on_select(|verts| println!("{} vertices", verts.len()));
    /// assert!(lasso.useblit(), "matplotlib's default");
    /// ```
    pub fn new(ax: &Axes, options: LassoSelectorOptions) -> Self {
        let line = add_line(ax, (Vec::new(), Vec::new()), options.props.visible(false));
        let core = Core::new(
            ax,
            options.buttons,
            ModifierKeys::default(),
            options.useblit,
        );
        Self {
            driver: Driver::new(core, LassoTool { line, verts: None }),
        }
    }

    /// Calls `callback` with the curve's vertices on each release
    /// (matplotlib's `onselect`); returns its id for
    /// [`LassoSelector::disconnect`].
    pub fn on_select(&self, mut callback: impl FnMut(&[(f64, f64)]) + 'static) -> CallbackId {
        self.driver.selected.connect(move |verts| callback(verts))
    }

    /// Disconnects the callback with id `id` (matplotlib's `disconnect`);
    /// returns whether it was connected.
    pub fn disconnect(&self, id: CallbackId) -> bool {
        self.driver.disconnect(id)
    }

    /// The line showing the curve while it is drawn.
    pub fn artist(&self) -> Line2D {
        self.driver.shared.borrow().tool.line.clone()
    }

    /// Changes the curve's look (matplotlib's `set_props`).
    pub fn set_props(&self, props: LineProps) -> &Self {
        {
            let shared = self.driver.shared.borrow();
            shared.tool.line.update_props(props);
            shared.core.request_blit();
        }
        super::blit::run_requested();
        self
    }
}

/// The state of a [`Lasso`] while it is drawn.
#[derive(Debug)]
struct LassoState {
    ax: Axes,
    line: Line2D,
    active: bool,
    /// The vertices, until the release.
    verts: Option<Vec<(f64, f64)>>,
    /// The background of the axes saved when the lasso was made, when it
    /// blits.
    background: Option<BufferRegion>,
}

impl LassoState {
    /// Shows the line as it now is (matplotlib's `Lasso.onmove`): drawn
    /// over the saved background and blitted, or a full draw asked for.
    fn show(&self) {
        let canvas = self.ax.figure().canvas();
        match &self.background {
            Some(background) => {
                canvas.report(canvas.restore_region(background));
                canvas.report(self.ax.draw_artist(&self.line));
                canvas.report(canvas.blit(Some(&self.ax.bbox())));
            }
            None => canvas.report(canvas.draw_idle()),
        }
    }
}

/// A one-shot free-hand curve started at a point, typically from a press
/// handler (matplotlib's `Lasso`): it follows the pointer with the left
/// button held and, on the release, passes its vertices to the callbacks
/// (when there are more than two) and removes itself.
#[derive(Debug)]
pub struct Lasso {
    state: Rc<RefCell<LassoState>>,
    selected: Observers<Vec<(f64, f64)>>,
    _connections: Connections,
}

impl Lasso {
    /// A lasso on `ax` starting at data point `xy`; a blitting one saves
    /// the background of `ax` now, before its line is drawn.
    pub fn new(ax: &Axes, xy: (f64, f64), options: LassoOptions) -> Self {
        let canvas = ax.figure().canvas();
        let background = options
            .useblit
            .then(|| canvas.copy_from_bbox(&ax.bbox()))
            .and_then(|saved| saved.map_err(|err| canvas.report(Err(err))).ok());
        let style = LineProps::default()
            .linestyle(LineStyle::Solid)
            .color("black")
            .linewidth(2.0)
            .overlay(options.props);
        let line = add_line(ax, (vec![xy.0], vec![xy.1]), style);
        let state = Rc::new(RefCell::new(LassoState {
            ax: ax.clone(),
            line,
            active: true,
            verts: Some(vec![xy]),
            background,
        }));
        let selected = Observers::default();
        let mut connections = Connections::new(ax.figure());
        {
            let (state, selected) = (Rc::clone(&state), selected.clone());
            connections.connect(EventKind::ButtonRelease, move |event| {
                let Event::ButtonRelease(mouse) = event else {
                    return;
                };
                let verts = {
                    let mut state = state.borrow_mut();
                    if !state.active {
                        return;
                    }
                    let point = data_in(&state.ax, &mouse.location);
                    let verts = state.verts.take().map(|mut verts| {
                        verts.push(point.map_or((f64::NAN, f64::NAN), |p| (p.x, p.y)));
                        verts
                    });
                    if verts.is_some() {
                        state.line.remove();
                    }
                    verts
                };
                if let Some(verts) = verts.filter(|verts| verts.len() > 2) {
                    selected.process(&verts);
                }
            });
        }
        {
            let state = Rc::clone(&state);
            connections.connect(EventKind::Motion, move |event| {
                let Event::Motion(mouse) = event else {
                    return;
                };
                let moved = {
                    let mut state = state.borrow_mut();
                    let location = &mouse.location;
                    let inside = state.ax.contains_point(Point::new(location.x, location.y));
                    if !state.active || mouse.button != Some(PointerButton::Left) || !inside {
                        return;
                    }
                    let point = data_in(&state.ax, location);
                    let state = &mut *state;
                    match (state.verts.as_mut(), point) {
                        (Some(verts), Some(p)) => {
                            verts.push((p.x, p.y));
                            let (x, y) = unzip(verts);
                            state.line.set_data(&x, &y);
                            true
                        }
                        _ => false,
                    }
                };
                if moved {
                    state.borrow().show();
                }
            });
        }
        Self {
            state,
            selected,
            _connections: connections,
        }
    }

    /// Calls `callback` with the vertices on the release (matplotlib's
    /// `callback`); returns its id for [`Lasso::disconnect`].
    pub fn on_select(&self, mut callback: impl FnMut(&[(f64, f64)]) + 'static) -> CallbackId {
        self.selected.connect(move |verts| callback(verts))
    }

    /// Disconnects the callback with id `id` (matplotlib's `disconnect`);
    /// returns whether it was connected.
    pub fn disconnect(&self, id: CallbackId) -> bool {
        self.selected.disconnect(id)
    }

    /// Whether the lasso reacts to events (matplotlib's `active`).
    pub fn active(&self) -> bool {
        self.state.borrow().active
    }

    /// Turns the lasso on or off.
    pub fn set_active(&self, active: bool) -> &Self {
        self.state.borrow_mut().active = active;
        self
    }

    /// Whether each motion blits the curve over the background saved when
    /// the lasso was made, rather than asking for a full draw
    /// (matplotlib's `useblit`).
    pub fn useblit(&self) -> bool {
        self.state.borrow().background.is_some()
    }

    /// The line showing the curve.
    pub fn line(&self) -> Line2D {
        self.state.borrow().line.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventSource;
    use crate::figure::Figure;

    #[test]
    fn lassos_report_their_vertices() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let selector = LassoSelector::new(&ax, LassoSelectorOptions::default());
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&seen);
        selector.on_select(move |verts| log.borrow_mut().push(verts.len()));
        let lasso = Lasso::new(&ax, (1.0, 1.0), LassoOptions::default());
        let log = Rc::clone(&seen);
        lasso.on_select(move |verts| log.borrow_mut().push(verts.len()));
        let mut source = EventSource::new();
        source.button_press(&fig, 40.0, 40.0, PointerButton::Left, false);
        for x in [80.0, 120.0, 160.0] {
            source.motion(&fig, x, 80.0);
        }
        assert!(selector.artist().visible());
        source.button_release(&fig, 160.0, 120.0, PointerButton::Left);
        assert_eq!(*seen.borrow(), [5, 5]);
        assert!(!selector.artist().visible());
        assert!(
            !lasso.line().raw().is_attached(),
            "the lasso removed itself"
        );
    }

    #[test]
    fn lasso_selectors_restyle_their_curve() {
        let ax = Figure::new().subplot();
        let options = LassoSelectorOptions {
            props: LineProps::default().color("b"),
            ..LassoSelectorOptions::default()
        };
        let selector = LassoSelector::new(&ax, options);
        selector.set_props(LineProps::default().color("r").alpha(0.3));
        let line = selector.artist();
        assert_eq!(line.color(), "r".into());
        assert_eq!(line.alpha(), Some(0.3));
    }
}
