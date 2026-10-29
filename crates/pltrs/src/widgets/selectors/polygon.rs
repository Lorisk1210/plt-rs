//! Selecting a polygon vertex by vertex (matplotlib's `PolygonSelector`).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use pltrs_core::Point;

use super::driver::{Core, Driver, Pointer, Shared, Tool};
use super::handles::MarkerHandles;
use super::rectangle::RectangleSelector;
use super::{ModifierKeys, SelectorError, SelectorEvent, SelectorState, selector_methods};
use crate::artist::{Artist, ArtistHandle};
use crate::axes::Axes;
use crate::lines::{Line2D, LineArtist, LineProps, LineStyle};
use crate::markers::MarkerSymbol;
use crate::navigation::PointerButton;
use crate::patches::{Patch, PatchProps, Rectangle};
use crate::rc::ColorOrAuto;
use crate::widgets::CallbackId;

mod bounding;

use self::bounding::BoundingBox;

/// Options of a [`PolygonSelector`] (matplotlib's keyword arguments).
#[derive(Clone, Debug)]
pub struct PolygonSelectorOptions {
    /// Look of the polygon's line (`props`).
    pub props: LineProps,
    /// Look of the vertex markers (`handle_props`); by default black edges
    /// filled with the line's colour.
    pub handle_props: Option<LineProps>,
    /// Distance in pixels within which a vertex is grabbed, or the first
    /// vertex closes the polygon (`grab_range`).
    pub grab_range: f64,
    /// Whether a completed polygon gets a box to move and scale it
    /// (`draw_bounding_box`).
    pub draw_bounding_box: bool,
    /// Look of the box's markers, over the vertex markers'
    /// (`box_handle_props`).
    pub box_handle_props: LineProps,
    /// Look of the box (`box_props`); the rectangle selector's by default.
    pub box_props: Option<PatchProps>,
}

impl Default for PolygonSelectorOptions {
    /// matplotlib's defaults: a translucent black line two points wide,
    /// without a box.
    fn default() -> Self {
        Self {
            props: LineProps::default()
                .color("k")
                .linestyle(LineStyle::Solid)
                .linewidth(2.0)
                .alpha(0.5),
            handle_props: None,
            grab_range: 10.0,
            draw_bounding_box: false,
            box_handle_props: LineProps::default(),
            box_props: None,
        }
    }
}

/// The polygon selector's own state.
#[derive(Debug)]
struct Polygon {
    line: Line2D,
    handles: MarkerHandles,
    handle_props: LineProps,
    /// The vertices, the last one following the pointer (or repeating the
    /// first once closed).
    xys: Vec<(f64, f64)>,
    xys_at_press: Vec<(f64, f64)>,
    /// The vertex being dragged.
    grabbed: Option<usize>,
    grab_range: f64,
    draw_box: bool,
    box_handle_props: LineProps,
    box_props: Option<PatchProps>,
    bounding: Option<BoundingBox>,
    /// The selector's shared state, for the box's callbacks.
    this: Weak<RefCell<Shared<Polygon>>>,
}

impl Polygon {
    /// The vertices of the polygon (matplotlib's `verts`).
    fn verts(&self) -> Vec<(f64, f64)> {
        self.xys[..self.xys.len().saturating_sub(1)].to_vec()
    }

    /// Whether the last vertex closes the polygon.
    fn closed(&self) -> bool {
        self.xys.len() > 3 && self.xys.last() == self.xys.first()
    }

    /// Redraws the line, the box and the markers (matplotlib's
    /// `_draw_polygon_without_update`).
    fn draw(&mut self, core: &Core) {
        let (xs, ys): (Vec<f64>, Vec<f64>) = self.xys.iter().copied().unzip();
        self.line.set_data(&xs, &ys);
        self.update_box();
        let markers = if core.selection_completed || self.closed() {
            xs.len().saturating_sub(1)
        } else {
            xs.len()
        };
        self.handles
            .set_data((xs[..markers].to_vec(), ys[..markers].to_vec()));
    }

    /// Removes vertex `i`, reopening polygons left with too few
    /// (matplotlib's `_remove_vertex`).
    fn remove_vertex(&mut self, core: &mut Core, i: usize) {
        let n = self.xys.len();
        if n > 2 && core.selection_completed && (i == 0 || i == n - 1) {
            self.xys.remove(0);
            self.xys.pop();
            if let Some(&first) = self.xys.first() {
                self.xys.push(first);
            }
        } else if i < n {
            self.xys.remove(i);
        }
        if self.xys.len() <= 2 {
            core.selection_completed = false;
            self.remove_box();
        }
    }

    /// Completes the polygon with `verts` (matplotlib's `verts` setter).
    fn set_verts(&mut self, core: &mut Core, verts: &[(f64, f64)]) {
        self.xys = verts.to_vec();
        self.xys.push(verts[0]);
        core.selection_completed = true;
        core.visible = true;
        self.show(true);
        if self.draw_box && self.bounding.is_none() {
            self.add_box(core);
        }
        self.draw(core);
    }
}

impl Tool for Polygon {
    type Selection = Vec<(f64, f64)>;

    fn show(&self, visible: bool) {
        self.line.set_visible(visible);
        self.handles.set_visible(visible);
    }

    fn press(&mut self, core: &mut Core, event: &SelectorEvent) {
        let editing = core.selection_completed || core.state.contains(&SelectorState::MoveVertex);
        if editing
            && !self.xys.is_empty()
            && let Some((index, distance)) = self.handles.closest(event.x, event.y)
            && distance < self.grab_range
        {
            self.grabbed = Some(index);
        }
        self.xys_at_press.clone_from(&self.xys);
    }

    fn drag(&mut self, core: &mut Core, event: &SelectorEvent) -> Option<Self::Selection> {
        let point = (event.xdata, event.ydata);
        let move_all = core.state.contains(&SelectorState::MoveAll);
        // A key that lifts the free vertex or clears the polygon can take
        // the held vertex away: it is let go.
        self.grabbed = self.grabbed.filter(|&index| index < self.xys.len());
        if let Some(index) = self.grabbed {
            self.xys[index] = point;
            if index == 0
                && core.selection_completed
                && let Some(last) = self.xys.last_mut()
            {
                *last = point;
            }
        } else if move_all && let Some(press) = &core.eventpress {
            let (dx, dy) = (event.xdata - press.xdata, event.ydata - press.ydata);
            for (xy, &(x, y)) in self.xys.iter_mut().zip(&self.xys_at_press) {
                *xy = (x + dx, y + dy);
            }
        } else if core.selection_completed
            || move_all
            || core.state.contains(&SelectorState::MoveVertex)
        {
            return None;
        } else {
            let first = self.xys.first().copied().unwrap_or(point);
            let near_first = core
                .ax
                .figure()
                .resolve_transform(&core.ax.trans_data())
                .is_ok_and(|trans| {
                    let p = trans.transform_point(Point::new(first.0, first.1));
                    (p.x - event.x).hypot(p.y - event.y) < self.grab_range
                });
            let follow = if self.xys.len() > 3 && near_first {
                first
            } else {
                point
            };
            if let Some(last) = self.xys.last_mut() {
                *last = follow;
            }
        }
        self.draw(core);
        None
    }

    fn release(&mut self, core: &mut Core, event: &SelectorEvent) -> Option<Self::Selection> {
        if let Some(index) = self.grabbed.take() {
            if event.button == Some(PointerButton::Right) {
                self.remove_vertex(core, index);
                self.draw(core);
            }
        } else if self.closed() {
            core.selection_completed = true;
            if self.draw_box && self.bounding.is_none() {
                self.add_box(core);
            }
        } else if !core.selection_completed
            && !core.state.contains(&SelectorState::MoveAll)
            && !core.state.contains(&SelectorState::MoveVertex)
        {
            let at = self.xys.len().saturating_sub(1);
            self.xys.insert(at, (event.xdata, event.ydata));
        }
        core.selection_completed.then(|| self.verts())
    }

    /// Motion with or without a button: the free vertex follows the
    /// pointer (matplotlib's `PolygonSelector.onmove`).
    fn motion(&mut self, core: &mut Core, pointer: &Pointer) -> Option<Self::Selection> {
        if core.ignore(pointer) {
            if !core.lock_available() && !self.xys.is_empty() {
                if let Some(last) = self.xys.last_mut() {
                    *last = (f64::NAN, f64::NAN);
                }
                self.draw(core);
            }
            return None;
        }
        let event = core.clean(pointer)?;
        self.drag(core, &event)
    }

    /// Holding a move key lifts the free vertex off the pointer.
    fn key_press(&mut self, core: &mut Core, _pointer: &Pointer) {
        let moving = core.state.contains(&SelectorState::MoveVertex)
            || core.state.contains(&SelectorState::MoveAll);
        if !core.selection_completed && moving {
            self.xys.pop();
            self.draw(core);
        }
    }

    /// Releasing a move key puts the free vertex back; the clear key starts
    /// a new polygon at the pointer.
    fn key_release(&mut self, core: &mut Core, pointer: &Pointer) {
        let key = pointer.key.as_deref();
        let move_key = [SelectorState::MoveVertex, SelectorState::MoveAll]
            .iter()
            .any(|&state| key.is_some() && core.key(state) == key);
        if !core.selection_completed && move_key {
            if let Some(data) = pointer.data {
                self.xys.push((data.x, data.y));
                self.draw(core);
            }
        } else if key.is_some() && key == core.clear_key() {
            if let Some(event) = core.clean(pointer) {
                self.xys = vec![(event.xdata, event.ydata)];
            }
            core.selection_completed = false;
            self.remove_box();
            core.visible = true;
            self.show(true);
        }
    }

    /// Starts over with one vertex at the origin (matplotlib's
    /// `PolygonSelector._clear_without_update`).
    fn clear(&mut self, core: &mut Core) {
        core.selection_completed = false;
        self.xys = vec![(0.0, 0.0)];
        self.draw(core);
    }
}

/// A polygon placed vertex by vertex (matplotlib's `PolygonSelector`):
/// each click adds a vertex, clicking the first one completes it. Then
/// vertices can be dragged, removed with the right button, or all moved
/// with shift held; control moves a vertex of an unfinished polygon and
/// escape starts a new one. The callbacks receive the vertices of the
/// completed polygon in data coordinates.
#[derive(Debug)]
pub struct PolygonSelector {
    driver: Driver<Polygon>,
}

selector_methods!(PolygonSelector);

impl PolygonSelector {
    /// A polygon selector on `ax`.
    pub fn new(ax: &Axes, options: PolygonSelectorOptions) -> Self {
        let mut line = LineArtist::new(Vec::new(), Vec::new(), crate::rc::params());
        options.props.clone().apply(&mut line);
        line.base_mut().transform = Some(ax.trans_data());
        let line = ax.add_line_artist(line);
        let handle_props = options.handle_props.clone().unwrap_or_else(|| {
            let face = options.props.color.clone().unwrap_or_else(|| "k".into());
            LineProps::default()
                .markeredgecolor(ColorOrAuto::Color("k".into()))
                .markerfacecolor(ColorOrAuto::Color(face))
        });
        let handles = MarkerHandles::new(
            ax,
            (Vec::new(), Vec::new()),
            MarkerSymbol::Circle,
            &handle_props,
        );
        let polygon = Polygon {
            line,
            handles,
            box_handle_props: handle_props
                .clone()
                .overlay(options.box_handle_props.clone()),
            handle_props,
            xys: vec![(0.0, 0.0)],
            xys_at_press: Vec::new(),
            grabbed: None,
            grab_range: options.grab_range,
            draw_box: options.draw_bounding_box,
            box_props: options.box_props.clone(),
            bounding: None,
            this: Weak::new(),
        };
        let core = Core::new(ax, None, ModifierKeys::polygon());
        let driver = Driver::new(core, polygon);
        {
            let mut shared = driver.shared.borrow_mut();
            shared.tool.this = Rc::downgrade(&driver.shared);
            shared.set_visible(true);
        }
        Self { driver }
    }

    /// Calls `callback` with the vertices on each release once the polygon
    /// is complete (matplotlib's `onselect`); returns its id for
    /// [`PolygonSelector::disconnect`].
    pub fn on_select(&self, mut callback: impl FnMut(&[(f64, f64)]) + 'static) -> CallbackId {
        self.driver.selected.connect(move |verts| callback(verts))
    }

    /// Disconnects the callback with id `id` (matplotlib's `disconnect`);
    /// returns whether it was connected.
    pub fn disconnect(&self, id: CallbackId) -> bool {
        self.driver.disconnect(id)
    }

    /// The vertices placed so far (matplotlib's `verts`).
    pub fn verts(&self) -> Vec<(f64, f64)> {
        self.driver.shared.borrow().tool.verts()
    }

    /// Replaces the polygon by the completed polygon through `verts`
    /// (matplotlib's `verts` setter).
    ///
    /// # Errors
    ///
    /// When `verts` is empty.
    pub fn set_verts(&self, verts: &[(f64, f64)]) -> Result<&Self, SelectorError> {
        if verts.is_empty() {
            return Err(SelectorError::NoVertices);
        }
        let mut shared = self.driver.shared.borrow_mut();
        let shared = &mut *shared;
        shared.tool.set_verts(&mut shared.core, verts);
        Ok(self)
    }

    /// The line showing the polygon.
    pub fn artist(&self) -> Line2D {
        self.driver.shared.borrow().tool.line.clone()
    }

    /// The vertex markers.
    pub fn handles(&self) -> Line2D {
        self.driver.shared.borrow().tool.handles.line().clone()
    }

    /// `(xmin, xmax, ymin, ymax)` of the box around the completed
    /// polygon, while it is drawn (matplotlib's `_box.extents`).
    pub fn box_extents(&self) -> Option<(f64, f64, f64, f64)> {
        self.with_box(RectangleSelector::extents)
    }

    /// The patch of the box around the completed polygon, while it is
    /// drawn.
    pub fn box_artist(&self) -> Option<Patch<Rectangle>> {
        self.with_box(RectangleSelector::artist)
    }

    /// The handle markers of the box around the completed polygon; none
    /// without a box.
    pub fn box_handles(&self) -> Vec<Line2D> {
        self.with_box(RectangleSelector::handles)
            .unwrap_or_default()
    }

    /// `f` of the box selector around the completed polygon, while it is
    /// drawn.
    fn with_box<R>(&self, f: impl FnOnce(&RectangleSelector) -> R) -> Option<R> {
        let shared = self.driver.shared.borrow();
        shared
            .tool
            .bounding
            .as_ref()
            .map(|bounding| f(&bounding.selector))
    }

    /// Changes the line's look (matplotlib's `set_props`).
    pub fn set_props(&self, props: LineProps) -> &Self {
        self.driver.shared.borrow().tool.line.update_props(props);
        self
    }

    /// Changes the look of the vertex markers (matplotlib's
    /// `set_handle_props`).
    pub fn set_handle_props(&self, props: LineProps) -> &Self {
        let mut shared = self.driver.shared.borrow_mut();
        let polygon = &mut shared.tool;
        polygon.handles.update_props(&props);
        polygon.handle_props = std::mem::take(&mut polygon.handle_props).overlay(props);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventSource;
    use crate::figure::Figure;

    fn click(fig: &Figure, source: &mut EventSource, (x, y): (f64, f64), button: PointerButton) {
        let (x, y) = (x * 40.0, y * 40.0);
        source.motion(fig, x, y);
        source.button_press(fig, x, y, button, false);
        source.button_release(fig, x, y, button);
    }

    #[test]
    fn clicks_place_vertices_until_the_polygon_closes() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let options = PolygonSelectorOptions {
            draw_bounding_box: true,
            ..PolygonSelectorOptions::default()
        };
        let selector = PolygonSelector::new(&ax, options);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&seen);
        selector.on_select(move |verts| log.borrow_mut().push(verts.to_vec()));
        let mut source = EventSource::new();
        for xy in [(2.0, 2.0), (8.0, 2.0), (5.0, 8.0), (2.0, 2.0)] {
            click(&fig, &mut source, xy, PointerButton::Left);
        }
        let triangle = vec![(2.0, 2.0), (8.0, 2.0), (5.0, 8.0)];
        assert_eq!(selector.verts(), triangle);
        assert_eq!(seen.borrow().last(), Some(&triangle));
        // Right-clicking a vertex removes it.
        click(&fig, &mut source, (5.0, 8.0), PointerButton::Right);
        assert_eq!(selector.verts(), [(2.0, 2.0), (8.0, 2.0)]);
        selector
            .set_verts(&[(1.0, 1.0), (3.0, 1.0), (3.0, 3.0)])
            .unwrap();
        assert_eq!(selector.verts().len(), 3);
        assert_eq!(
            selector.set_verts(&[]).err(),
            Some(SelectorError::NoVertices)
        );
    }

    #[test]
    fn a_vertex_cleared_while_held_is_let_go() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let selector = PolygonSelector::new(&ax, PolygonSelectorOptions::default());
        let mut source = EventSource::new();
        click(&fig, &mut source, (2.0, 2.0), PointerButton::Left);
        click(&fig, &mut source, (8.0, 2.0), PointerButton::Left);
        // Found by fuzzing: control lifts the free vertex, the press then
        // holds vertex 1, which escape clears before the pointer drags it.
        source.key_press(&fig, "control", 320.0, 80.0);
        source.motion(&fig, 320.0, 80.0);
        source.button_press(&fig, 320.0, 80.0, PointerButton::Left, false);
        source.key_release(&fig, "control", 320.0, 80.0);
        source.key_press(&fig, "escape", 320.0, 80.0);
        source.key_release(&fig, "escape", 320.0, 80.0);
        source.motion(&fig, 200.0, 200.0);
        source.button_release(&fig, 200.0, 200.0, PointerButton::Left);
        assert!(!selector.selection_completed());
    }

    #[test]
    fn completed_polygons_show_their_bounding_box() {
        let ax = Figure::new().subplot();
        let boxed = |draw_bounding_box| {
            let options = PolygonSelectorOptions {
                draw_bounding_box,
                ..PolygonSelectorOptions::default()
            };
            let selector = PolygonSelector::new(&ax, options);
            assert_eq!(selector.box_extents(), None, "no box before completion");
            selector
                .set_verts(&[(0.25, 0.25), (0.75, 0.25), (0.5, 0.75)])
                .unwrap();
            selector
        };
        let selector = boxed(true);
        assert_eq!(selector.box_extents(), Some((0.25, 0.75, 0.25, 0.75)));
        assert!(selector.box_artist().is_some());
        assert_eq!(selector.box_handles().len(), 3, "centre, corners, edges");
        assert!(selector.selection_completed());
        let plain = boxed(false);
        assert_eq!(plain.box_extents(), None);
        assert!(plain.box_artist().is_none() && plain.box_handles().is_empty());
    }
}
