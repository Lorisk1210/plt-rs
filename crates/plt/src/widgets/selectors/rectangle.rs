//! Selecting a rectangle or an ellipse (matplotlib's `RectangleSelector`
//! and `EllipseSelector`).

use plt_core::Point;
use plt_core::path::PolygonOptions;

use super::driver::{Core, Driver};
use super::region::{Region, RegionSettings, SelectorUnits};
use super::shape::RegionShape;
use super::{ModifierKeys, SelectorError, SelectorEvent, selector_methods};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::colors::Color;
use crate::lines::{Line2D, LineProps};
use crate::navigation::PointerButton;
use crate::patches::{Ellipse, Patch, PatchProps, PatchShape, Rectangle};
use crate::rc::ColorOrAuto;
use crate::widgets::CallbackId;

/// Options of a [`RectangleSelector`] or [`EllipseSelector`] (matplotlib's
/// keyword arguments).
#[derive(Clone, Debug)]
pub struct RectangleSelectorOptions {
    /// Shapes no wider than this are removed (`minspanx`).
    pub minspanx: f64,
    /// Shapes no taller than this are removed (`minspany`).
    pub minspany: f64,
    /// Look of the shape (`props`); its `visible` decides whether
    /// selections show.
    pub props: PatchProps,
    /// The units of `minspanx` and `minspany` (`spancoords`).
    pub spancoords: SelectorUnits,
    /// The buttons that select; all when `None` (`button`).
    pub buttons: Option<Vec<PointerButton>>,
    /// Distance in pixels within which a handle is grabbed (`grab_range`).
    pub grab_range: f64,
    /// Look of the handle markers (`handle_props`); unset edge colours
    /// follow the shape's edge colour.
    pub handle_props: LineProps,
    /// Whether the shape stays after the release, with handles to resize,
    /// move and rotate it (`interactive`).
    pub interactive: bool,
    /// The keys the selector reacts to (`state_modifier_keys`).
    pub modifier_keys: ModifierKeys,
    /// Whether a press anywhere inside the shape moves it
    /// (`drag_from_anywhere`).
    pub drag_from_anywhere: bool,
    /// Whether presses outside the shape leave it alone instead of starting
    /// a new one (`ignore_event_outside`).
    pub ignore_event_outside: bool,
    /// The units squares are square in: display pixels, or data units
    /// (matplotlib's `use_data_coordinates`).
    pub square_units: SelectorUnits,
}

impl Default for RectangleSelectorOptions {
    /// matplotlib's defaults: a translucent red shape with a black edge,
    /// without handles.
    fn default() -> Self {
        Self {
            minspanx: 0.0,
            minspany: 0.0,
            props: PatchProps::default()
                .facecolor("red")
                .edgecolor("black")
                .alpha(0.2)
                .fill(true),
            spancoords: SelectorUnits::Data,
            buttons: None,
            grab_range: 10.0,
            handle_props: LineProps::default(),
            interactive: false,
            modifier_keys: ModifierKeys::default(),
            drag_from_anywhere: false,
            ignore_event_outside: false,
            square_units: SelectorUnits::Pixels,
        }
    }
}

/// The selector driver for `shape` on `ax` with `options`.
fn region<S: RegionShape>(ax: &Axes, options: &RectangleSelectorOptions) -> Driver<Region<S>> {
    let mut props = options.props.clone();
    let visible = props.base.visible.take();
    let patch = ax.add_patch(S::initial());
    patch.set_visible(false);
    patch.update_props(props);
    let edge = options
        .props
        .edgecolor
        .clone()
        .unwrap_or_else(|| Color::from("black"));
    let handle_props = LineProps::default()
        .markeredgecolor(ColorOrAuto::Color(edge))
        .overlay(options.handle_props.clone());
    let settings = RegionSettings {
        minspanx: options.minspanx,
        minspany: options.minspany,
        spancoords: options.spancoords,
        grab_range: options.grab_range,
        interactive: options.interactive,
        drag_from_anywhere: options.drag_from_anywhere,
        ignore_event_outside: options.ignore_event_outside,
        square_units: options.square_units,
    };
    let tool = Region::new(ax, patch, settings, handle_props);
    let mut core = Core::new(ax, options.buttons.clone(), options.modifier_keys.clone());
    if let Some(visible) = visible {
        core.visible = visible;
    }
    Driver::new(core, tool)
}

/// The methods both region selectors have.
macro_rules! region_methods {
    ($selector:ty, $shape:ty) => {
        selector_methods!($selector);

        impl $selector {
            /// A selector on `ax`.
            pub fn new(ax: &Axes, options: RectangleSelectorOptions) -> Self {
                Self {
                    driver: region::<$shape>(ax, &options),
                }
            }

            /// Calls `callback` after each release that makes, changes or
            /// removes a selection, with events at its lower-left and
            /// upper-right corners (matplotlib's `onselect(eclick,
            /// erelease)`); returns its id for `disconnect`.
            pub fn on_select(
                &self,
                mut callback: impl FnMut(&SelectorEvent, &SelectorEvent) + 'static,
            ) -> CallbackId {
                self.driver
                    .selected
                    .connect(move |(press, release)| callback(press, release))
            }

            /// Disconnects the callback with id `id`; returns whether it
            /// was connected.
            pub fn disconnect(&self, id: CallbackId) -> bool {
                self.driver.disconnect(id)
            }

            /// `(xmin, xmax, ymin, ymax)` of the unrotated shape in data
            /// coordinates (matplotlib's `extents`).
            pub fn extents(&self) -> (f64, f64, f64, f64) {
                self.driver.shared.borrow().tool.extents()
            }

            /// Shows the shape over `(xmin, xmax, ymin, ymax)` (matplotlib's
            /// `extents` setter).
            pub fn set_extents(&self, extents: (f64, f64, f64, f64)) {
                let shared = self.driver.shared.borrow();
                shared.tool.set_extents(&shared.core, extents);
            }

            /// The centre in data coordinates (matplotlib's `center`).
            pub fn center(&self) -> (f64, f64) {
                self.driver.shared.borrow().tool.center()
            }

            /// The x and y data coordinates of the rotated corners, from the
            /// lower left counter-clockwise (matplotlib's `corners`).
            pub fn corners(&self) -> (Vec<f64>, Vec<f64>) {
                let shared = self.driver.shared.borrow();
                shared.tool.corners(&shared.core.ax)
            }

            /// The x and y data coordinates of the rotated edge centres, from
            /// the left counter-clockwise (matplotlib's `edge_centers`).
            pub fn edge_centers(&self) -> (Vec<f64>, Vec<f64>) {
                let shared = self.driver.shared.borrow();
                shared.tool.edge_centers(&shared.core.ax)
            }

            /// The rotation in degrees (matplotlib's `rotation`).
            pub fn rotation(&self) -> f64 {
                self.driver.shared.borrow().tool.rotation.to_degrees()
            }

            /// Rotates the shape about its centre; angles outside ±45° are
            /// ignored (matplotlib's `rotation` setter).
            pub fn set_rotation(&self, degrees: f64) {
                let mut shared = self.driver.shared.borrow_mut();
                let shared = &mut *shared;
                shared.tool.set_rotation(&shared.core, degrees);
            }

            /// The outline of the shape in data coordinates, closed
            /// (matplotlib's `geometry`, as points rather than rows of y and
            /// x values).
            pub fn geometry(&self) -> Vec<Point> {
                let shared = self.driver.shared.borrow();
                outline(&shared.core.ax, &shared.tool.patch.shape())
            }

            /// The patch showing the selection.
            pub fn artist(&self) -> Patch<$shape> {
                self.driver.shared.borrow().tool.patch.clone()
            }

            /// The handle markers of an interactive selector: the centre, the
            /// corners and the edge centres.
            pub fn handles(&self) -> Vec<Line2D> {
                self.driver
                    .shared
                    .borrow()
                    .tool
                    .handles
                    .as_ref()
                    .map_or_else(Vec::new, |handles| {
                        handles
                            .each()
                            .iter()
                            .map(|markers| markers.line().clone())
                            .collect()
                    })
            }

            /// Changes the shape's look (matplotlib's `set_props`).
            pub fn set_props(&self, props: PatchProps) {
                self.driver.shared.borrow().tool.patch.update_props(props);
            }

            /// Changes the look of the handles (matplotlib's
            /// `set_handle_props`).
            ///
            /// # Errors
            ///
            /// When the selector is not interactive and has no handles.
            pub fn set_handle_props(&self, props: LineProps) -> Result<(), SelectorError> {
                let mut shared = self.driver.shared.borrow_mut();
                let handles = shared
                    .tool
                    .handles
                    .as_mut()
                    .ok_or(SelectorError::NoHandles)?;
                for markers in handles.each() {
                    markers.update_props(&props);
                }
                handles.props = std::mem::take(&mut handles.props).overlay(props);
                Ok(())
            }
        }
    };
}

/// The closed outline of `shape` in the data coordinates of `ax`
/// (matplotlib's `get_verts` mapped back to data).
fn outline<S: PatchShape>(ax: &Axes, shape: &S) -> Vec<Point> {
    let Ok(trans) = ax.figure().resolve_transform(&ax.trans_data()) else {
        return Vec::new();
    };
    let Some(inverse) = trans.inverted() else {
        return Vec::new();
    };
    let display = trans.transform_path(&shape.path().transformed(&shape.patch_transform()));
    let options = PolygonOptions {
        closed_only: true,
        ..PolygonOptions::default()
    };
    display
        .to_polygons(&options)
        .into_iter()
        .next()
        .map(|polygon| {
            polygon
                .into_iter()
                .map(|p| inverse.transform_point(p))
                .collect()
        })
        .unwrap_or_default()
}

/// A rectangle selected by dragging the mouse (matplotlib's
/// `RectangleSelector`); interactive rectangles can be resized, moved,
/// squared (shift), grown about their centre (control) and rotated (`r`).
///
/// ```
/// use plt::figure::Figure;
/// use plt::widgets::{RectangleSelector, RectangleSelectorOptions};
/// let ax = Figure::new().subplot();
/// let selector = RectangleSelector::new(&ax, RectangleSelectorOptions::default());
/// selector.on_select(|press, release| {
///     println!("({}, {}) to ({}, {})", press.xdata, press.ydata, release.xdata, release.ydata);
/// });
/// selector.set_extents((0.25, 0.5, 0.25, 0.75));
/// assert_eq!(selector.center(), (0.375, 0.5));
/// ```
#[derive(Debug)]
pub struct RectangleSelector {
    driver: Driver<Region<Rectangle>>,
}

region_methods!(RectangleSelector, Rectangle);

impl RectangleSelector {
    /// `(x0, y0, width, height)` of the unrotated rectangle (matplotlib's
    /// `_rect_bbox`).
    pub(super) fn rect_bbox(&self) -> (f64, f64, f64, f64) {
        self.driver.shared.borrow().tool.patch.shape().rect_bbox()
    }

    /// Makes the selector only edit its current rectangle, as the bounding
    /// box of a polygon selector does.
    pub(super) fn edit_only(&self) {
        let mut shared = self.driver.shared.borrow_mut();
        shared.tool.allow_creation = false;
        shared.core.selection_completed = true;
    }
}

/// An ellipse selected by dragging the mouse (matplotlib's
/// `EllipseSelector`), inscribed in the rectangle a [`RectangleSelector`]
/// would select.
#[derive(Debug)]
pub struct EllipseSelector {
    driver: Driver<Region<Ellipse>>,
}

region_methods!(EllipseSelector, Ellipse);

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::events::EventSource;
    use crate::figure::Figure;
    use crate::widgets::SelectorState;

    /// A 400 × 400 figure with one axes over the data square [0, 10]².
    fn axes() -> (Figure, Axes) {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
        ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
        (fig, ax)
    }

    fn drag(fig: &Figure, from: (f64, f64), to: (f64, f64), key: Option<&str>) {
        let mut source = EventSource::new();
        if let Some(key) = key {
            source.key_press(fig, key, from.0 * 40.0, from.1 * 40.0);
        }
        source.button_press(
            fig,
            from.0 * 40.0,
            from.1 * 40.0,
            PointerButton::Left,
            false,
        );
        source.motion(fig, to.0 * 40.0, to.1 * 40.0);
        source.button_release(fig, to.0 * 40.0, to.1 * 40.0, PointerButton::Left);
        if let Some(key) = key {
            source.key_release(fig, key, to.0 * 40.0, to.1 * 40.0);
        }
    }

    fn close(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
        [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)]
            .iter()
            .all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn dragging_selects_and_resizes() {
        let (fig, ax) = axes();
        let options = RectangleSelectorOptions {
            interactive: true,
            ..RectangleSelectorOptions::default()
        };
        let selector = RectangleSelector::new(&ax, options);
        let log = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&log);
        selector.on_select(move |press, release| {
            seen.borrow_mut()
                .push((press.xdata, press.ydata, release.xdata, release.ydata));
        });
        drag(&fig, (6.0, 2.0), (2.0, 5.0), None);
        assert!(close(selector.extents(), (2.0, 6.0, 2.0, 5.0)));
        assert_eq!(log.borrow().len(), 1);
        assert!(selector.handles()[1].visible());
        // Dragging the north-east corner resizes.
        drag(&fig, (6.0, 5.0), (8.0, 7.0), None);
        assert!(close(selector.extents(), (2.0, 8.0, 2.0, 7.0)));
        // Dragging the centre moves.
        drag(&fig, (5.0, 4.5), (6.0, 5.5), None);
        assert!(close(selector.extents(), (3.0, 9.0, 3.0, 8.0)));
        assert_eq!(selector.geometry().len(), 5);
    }

    #[test]
    fn squares_and_ellipses() {
        let (fig, ax) = axes();
        let selector = EllipseSelector::new(&ax, RectangleSelectorOptions::default());
        drag(&fig, (2.0, 2.0), (6.0, 3.0), Some("shift"));
        assert!(close(selector.extents(), (2.0, 6.0, 2.0, 6.0)));
        assert!(!selector.artist().visible(), "not interactive");
        selector.add_state(SelectorState::Center).unwrap();
        drag(&fig, (5.0, 5.0), (6.0, 7.0), None);
        assert!(close(selector.extents(), (4.0, 6.0, 3.0, 7.0)));
        selector.set_rotation(30.0);
        assert!((selector.rotation() - 30.0).abs() < 1e-9);
        selector.set_rotation(60.0);
        assert!(
            (selector.rotation() - 30.0).abs() < 1e-9,
            "beyond 45° is ignored"
        );
    }

    #[test]
    fn states_follow_keys_and_names() {
        let (fig, ax) = axes();
        let options = RectangleSelectorOptions {
            interactive: true,
            ..RectangleSelectorOptions::default()
        };
        let selector = RectangleSelector::new(&ax, options);
        drag(&fig, (2.0, 2.0), (6.0, 5.0), None);
        assert!(selector.selection_completed());
        let mut source = EventSource::new();
        source.key_press(&fig, "r", 0.0, 0.0);
        assert_eq!(selector.states(), [SelectorState::Rotate]);
        source.key_press(&fig, "r", 0.0, 0.0);
        assert!(selector.states().is_empty(), "the rotate key toggles");
        for name in ["center", "square"] {
            selector.add_state(name.parse().unwrap()).unwrap();
        }
        assert_eq!(
            selector.states(),
            [SelectorState::Square, SelectorState::Center]
        );
        assert_eq!(
            "clear".parse::<SelectorState>(),
            Err(crate::widgets::UnknownState("clear".into()))
        );
        selector.clear();
        assert!(!selector.selection_completed() && !selector.visible());
    }
}
