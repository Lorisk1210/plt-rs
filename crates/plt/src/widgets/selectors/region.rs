//! Selecting a rectangular or elliptical region (the interaction of
//! matplotlib's `RectangleSelector`, which `EllipseSelector` shares).

use plt_core::numeric::np_sign;
use plt_core::{Affine2D, Point};

use super::driver::{Core, Tool};
use super::handles::MarkerHandles;
use super::shape::{Extents, RegionShape};
use super::{SelectorEvent, SelectorState};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::events::CursorShape;
use crate::lines::LineProps;
use crate::markers::MarkerSymbol;
use crate::patches::Patch;
use crate::widgets::ordered;

/// The units a region selector measures in: minimum spans (matplotlib's
/// `spancoords`) and squares (`use_data_coordinates`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectorUnits {
    /// Data units.
    Data,
    /// Display pixels.
    Pixels,
}

/// matplotlib's `Axes._get_aspect_ratio`: the axes' display height over
/// width, in data units.
fn aspect_ratio(ax: &Axes) -> f64 {
    let (width, height) = ax.figure().size_inches();
    let pos = ax.position();
    let w = pos.x1() * width - pos.x0() * width;
    let h = pos.y1() * height - pos.y0() * height;
    h / (w * ax.data_ratio())
}

/// A corner, from the lower left counter-clockwise (matplotlib's
/// `_corner_order`).
const CORNERS: [Grab; 4] = [
    Grab::SouthWest,
    Grab::SouthEast,
    Grab::NorthEast,
    Grab::NorthWest,
];

/// An edge centre, from the left counter-clockwise (matplotlib's
/// `_edge_order`).
const EDGES: [Grab; 4] = [Grab::West, Grab::South, Grab::East, Grab::North];

/// What a press grabbed (matplotlib's `_active_handle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grab {
    SouthWest,
    SouthEast,
    NorthEast,
    NorthWest,
    West,
    South,
    East,
    North,
    /// The whole shape (`"C"`).
    Center,
}

impl Grab {
    fn is_corner(self) -> bool {
        CORNERS.contains(&self)
    }

    /// Whether dragging it moves the left or right side.
    fn moves_x(self) -> bool {
        self.is_corner() || matches!(self, Self::East | Self::West)
    }

    /// Whether dragging it moves the bottom or top side.
    fn moves_y(self) -> bool {
        self.is_corner() || matches!(self, Self::North | Self::South)
    }

    /// matplotlib's `'W' in handle`.
    fn west(self) -> bool {
        matches!(self, Self::SouthWest | Self::NorthWest | Self::West)
    }

    /// matplotlib's `'S' in handle`.
    fn south(self) -> bool {
        matches!(self, Self::SouthWest | Self::SouthEast | Self::South)
    }
}

/// What a drag does (matplotlib's `_RectangleSelectorAction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Rotate,
    Move,
    Resize,
    Create,
}

/// The markers of an interactive region selector.
#[derive(Debug)]
pub(super) struct RegionHandles {
    pub(super) center: MarkerHandles,
    pub(super) corners: MarkerHandles,
    pub(super) edges: MarkerHandles,
    pub(super) props: LineProps,
}

impl RegionHandles {
    pub(super) fn each(&self) -> [&MarkerHandles; 3] {
        [&self.center, &self.corners, &self.edges]
    }
}

/// The settings of a region selector that do not change while it is used.
#[derive(Clone, Debug)]
pub(super) struct RegionSettings {
    pub(super) minspanx: f64,
    pub(super) minspany: f64,
    pub(super) spancoords: SelectorUnits,
    pub(super) grab_range: f64,
    pub(super) interactive: bool,
    pub(super) drag_from_anywhere: bool,
    pub(super) ignore_event_outside: bool,
    /// The units squares are square in.
    pub(super) square_units: SelectorUnits,
}

/// A region selector's own state.
#[derive(Debug)]
pub(super) struct Region<S: RegionShape> {
    pub(super) patch: Patch<S>,
    pub(super) handles: Option<RegionHandles>,
    pub(super) settings: RegionSettings,
    /// Rotation in radians.
    pub(super) rotation: f64,
    aspect_ratio_correction: f64,
    /// Whether presses may start a new shape.
    pub(super) allow_creation: bool,
    grab: Option<Grab>,
    extents_on_press: Option<Extents>,
    rotation_on_press: f64,
}

/// Whether a drag by `(refx, refy)` is mostly horizontal (matplotlib's
/// `max(refx, refy, key=abs) == refx`).
fn mostly_horizontal(refx: f64, refy: f64) -> bool {
    refy.abs() <= refx.abs()
}

impl<S: RegionShape> Region<S> {
    /// A hidden region on `ax`, with markers when interactive.
    pub(super) fn new(
        ax: &Axes,
        patch: Patch<S>,
        settings: RegionSettings,
        handle_props: LineProps,
    ) -> Self {
        let mut region = Self {
            patch,
            handles: None,
            settings,
            rotation: 0.0,
            aspect_ratio_correction: 1.0,
            allow_creation: true,
            grab: None,
            extents_on_press: None,
            rotation_on_press: 0.0,
        };
        region.set_aspect_ratio_correction(ax);
        if region.settings.interactive {
            let (cx, cy) = region.center();
            region.handles = Some(RegionHandles {
                corners: MarkerHandles::new(
                    ax,
                    region.corners(ax),
                    MarkerSymbol::Circle,
                    &handle_props,
                ),
                edges: MarkerHandles::new(
                    ax,
                    region.edge_centers(ax),
                    MarkerSymbol::Square,
                    &handle_props,
                ),
                center: MarkerHandles::new(
                    ax,
                    (vec![cx], vec![cy]),
                    MarkerSymbol::Square,
                    &handle_props,
                ),
                props: handle_props,
            });
        }
        region
    }

    /// The unrotated extent (matplotlib's `extents`).
    pub(super) fn extents(&self) -> Extents {
        let (x0, y0, width, height) = self.patch.shape().rect_bbox();
        let (xmin, xmax) = ordered((x0, x0 + width));
        let (ymin, ymax) = ordered((y0, y0 + height));
        (xmin, xmax, ymin, ymax)
    }

    /// The centre in data coordinates.
    pub(super) fn center(&self) -> (f64, f64) {
        let (x0, y0, width, height) = self.patch.shape().rect_bbox();
        (x0 + width / 2.0, y0 + height / 2.0)
    }

    /// The rotation about the centre in data coordinates (matplotlib's
    /// `_get_rotation_transform`).
    fn rotation_transform(&self, ax: &Axes) -> Affine2D {
        let aspect = aspect_ratio(ax);
        let (cx, cy) = self.center();
        Affine2D::translation(-cx, -cy)
            .scale(1.0, aspect)
            .rotate(self.rotation)
            .scale(1.0, 1.0 / aspect)
            .translate(cx, cy)
    }

    /// `points` of the unrotated shape, rotated.
    fn rotated(&self, ax: &Axes, points: [(f64, f64); 4]) -> (Vec<f64>, Vec<f64>) {
        let transform = self.rotation_transform(ax);
        points
            .iter()
            .map(|&(x, y)| {
                let p = transform.transform_point(Point::new(x, y));
                (p.x, p.y)
            })
            .unzip()
    }

    /// The corners from the lower left counter-clockwise (matplotlib's
    /// `corners`).
    pub(super) fn corners(&self, ax: &Axes) -> (Vec<f64>, Vec<f64>) {
        let (x0, y0, w, h) = self.patch.shape().rect_bbox();
        self.rotated(ax, [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)])
    }

    /// The edge centres from the left counter-clockwise (matplotlib's
    /// `edge_centers`).
    pub(super) fn edge_centers(&self, ax: &Axes) -> (Vec<f64>, Vec<f64>) {
        let (x0, y0, w, h) = self.patch.shape().rect_bbox();
        let (hw, hh) = (w / 2.0, h / 2.0);
        self.rotated(
            ax,
            [
                (x0, y0 + hh),
                (x0 + hw, y0),
                (x0 + w, y0 + hh),
                (x0 + hw, y0 + h),
            ],
        )
    }

    /// Updates the rotation correction of the shape (matplotlib's
    /// `_set_aspect_ratio_correction`).
    fn set_aspect_ratio_correction(&mut self, ax: &Axes) {
        let aspect = aspect_ratio(ax);
        self.patch
            .update_shape(|shape| shape.set_aspect_ratio_correction(aspect));
        self.aspect_ratio_correction = match self.settings.square_units {
            SelectorUnits::Data => 1.0,
            SelectorUnits::Pixels => aspect,
        };
    }

    /// Shows the shape over `extents` with its markers (matplotlib's
    /// `extents` setter).
    pub(super) fn set_extents(&self, core: &Core, extents: Extents) {
        let ax = &core.ax;
        let view = (ax.xlim(), ax.ylim());
        let angle = self.rotation.to_degrees();
        self.patch
            .update_shape(|shape| shape.draw(extents, angle, view));
        if let Some(handles) = &self.handles {
            handles.corners.set_data(self.corners(ax));
            handles.edges.set_data(self.edge_centers(ax));
            let (cx, cy) = self.center();
            handles.center.set_data((vec![cx], vec![cy]));
        }
        self.show(core.visible);
    }

    /// Rotates the shape to `degrees` within ±45° (matplotlib's `rotation`
    /// setter); other angles are ignored.
    pub(super) fn set_rotation(&mut self, core: &Core, degrees: f64) {
        if (-45.0..=45.0).contains(&degrees) {
            self.rotation = degrees.to_radians();
            self.set_extents(core, self.extents());
        }
    }

    fn action(&self, core: &Core) -> Action {
        match self.grab {
            Some(grab) if core.state.contains(&SelectorState::Rotate) && grab.is_corner() => {
                Action::Rotate
            }
            Some(Grab::Center) => Action::Move,
            Some(_) => Action::Resize,
            None => Action::Create,
        }
    }

    /// What a press at `event` grabs (matplotlib's `_set_active_handle`).
    fn set_grab(&mut self, core: &Core, event: &SelectorEvent) {
        let Some(handles) = &self.handles else {
            self.grab = None;
            return;
        };
        let far = (0, f64::INFINITY);
        let (c_idx, c_dist) = handles.corners.closest(event.x, event.y).unwrap_or(far);
        let (e_idx, e_dist) = handles.edges.closest(event.x, event.y).unwrap_or(far);
        let (_, m_dist) = handles.center.closest(event.x, event.y).unwrap_or(far);
        let range = self.settings.grab_range;
        self.grab = if core.state.contains(&SelectorState::Move) || m_dist < range * 2.0 {
            Some(Grab::Center)
        } else if c_dist > range && e_dist > range {
            (self.settings.drag_from_anywhere
                && self
                    .patch
                    .contains_display_point(Point::new(event.x, event.y)))
            .then_some(Grab::Center)
        } else if c_dist < e_dist {
            Some(CORNERS[c_idx])
        } else {
            Some(EDGES[e_idx])
        };
    }

    /// The extents a resize drag gives (matplotlib's `_onmove`, resize
    /// branch).
    fn resized(
        &self,
        core: &Core,
        grab: Grab,
        (xdata, ydata): (f64, f64),
        (refx, refy): (f64, f64),
    ) -> Extents {
        let (mut x0, mut x1, mut y0, mut y1) =
            self.extents_on_press.unwrap_or_else(|| self.extents());
        let arc = self.aspect_ratio_correction;
        let square = core.state.contains(&SelectorState::Square);
        let along_x = matches!(grab, Grab::East | Grab::West)
            || (grab.is_corner() && mostly_horizontal(refx, refy));
        if core.state.contains(&SelectorState::Center) {
            let (w, h) = (x1 - x0, y1 - y0);
            let (cx, cy) = (x0 + w / 2.0, y0 + h / 2.0);
            let (hw, hh) = if square {
                if along_x {
                    let hw = xdata - cx;
                    (hw, hw / arc)
                } else {
                    let hh = ydata - cy;
                    (hh * arc, hh)
                }
            } else {
                let hw = if grab.moves_x() {
                    (xdata - cx).abs()
                } else {
                    w / 2.0
                };
                let hh = if grab.moves_y() {
                    (ydata - cy).abs()
                } else {
                    h / 2.0
                };
                (hw, hh)
            };
            return (cx - hw, cx + hw, cy - hh, cy + hh);
        }
        if grab.west() {
            x0 = x1;
        }
        if grab.south() {
            y0 = y1;
        }
        if grab.moves_x() {
            x1 = xdata;
        }
        if grab.moves_y() {
            y1 = ydata;
        }
        if square {
            if along_x {
                y1 = y0 + np_sign(ydata - y0) * (x1 - x0).abs() / arc;
            } else {
                x1 = x0 + np_sign(xdata - x0) * (y1 - y0).abs() * arc;
            }
        }
        (x0, x1, y0, y1)
    }

    /// The extents a creating drag gives, or `None` when creating is not
    /// allowed (matplotlib's `_onmove`, create branch).
    fn created(
        &mut self,
        core: &Core,
        press: (f64, f64),
        (xdata, ydata): (f64, f64),
        (refx, refy): (f64, f64),
    ) -> Option<Extents> {
        self.rotation = 0.0;
        if (self.settings.ignore_event_outside && core.selection_completed) || !self.allow_creation
        {
            return None;
        }
        let arc = self.aspect_ratio_correction;
        let (mut cx, mut cy) = press;
        let mut dx = (xdata - cx) / 2.0;
        let mut dy = (ydata - cy) / 2.0;
        if core.state.contains(&SelectorState::Square) {
            if mostly_horizontal(refx, refy) {
                dy = np_sign(dy) * dx.abs() / arc;
            } else {
                dx = np_sign(dx) * dy.abs() * arc;
            }
        }
        if core.state.contains(&SelectorState::Center) {
            dx *= 2.0;
            dy *= 2.0;
        } else {
            cx += dx;
            cy += dy;
        }
        Some((cx - dx, cx + dx, cy - dy, cy + dy))
    }
}

impl<S: RegionShape> Tool for Region<S> {
    type Selection = (SelectorEvent, SelectorEvent);

    fn show(&self, visible: bool) {
        self.patch.set_visible(visible);
        if let Some(handles) = &self.handles {
            for markers in handles.each() {
                markers.set_visible(visible);
            }
        }
    }

    fn press(&mut self, core: &mut Core, event: &SelectorEvent) {
        if self.settings.interactive && self.patch.visible() {
            self.set_grab(core, event);
        } else {
            self.grab = None;
        }
        if self.grab.is_none() && !self.settings.ignore_event_outside && self.allow_creation {
            // A press outside starts a new, still hidden, shape.
            core.visible = false;
            let (x, y) = (event.xdata, event.ydata);
            self.set_extents(core, (x, x, y, y));
            core.visible = true;
        } else {
            core.visible = true;
            self.show(true);
        }
        self.extents_on_press = Some(self.extents());
        self.rotation_on_press = self.rotation;
        self.set_aspect_ratio_correction(&core.ax);
        let cursor = match self.action(core) {
            Action::Move => CursorShape::Move,
            Action::Create => CursorShape::SelectRegion,
            Action::Rotate | Action::Resize => return,
        };
        core.ax.figure().set_cursor(cursor);
    }

    fn drag(&mut self, core: &mut Core, event: &SelectorEvent) -> Option<Self::Selection> {
        let action = self.action(core);
        let mut data = (event.xdata, event.ydata);
        let press = core.eventpress.as_mut()?;
        if action == Action::Resize {
            // Resizing works on the unrotated shape; like matplotlib, the
            // press is unrotated in place.
            if let Some(inverse) = self.rotation_transform(&core.ax).inverted() {
                let p = inverse.transform_point(Point::new(data.0, data.1));
                data = (p.x, p.y);
                let q = inverse.transform_point(Point::new(press.xdata, press.ydata));
                (press.xdata, press.ydata) = (q.x, q.y);
            }
        }
        let (px, py) = (press.xdata, press.ydata);
        let reference = match self.settings.square_units {
            SelectorUnits::Data => (data.0 - px, data.1 - py),
            SelectorUnits::Pixels => (event.x - press.x, event.y - press.y),
        };
        let on_press = self.extents_on_press.unwrap_or_else(|| self.extents());
        let extents = match (action, self.grab) {
            (Action::Rotate, _) => {
                let (bx, by) = self.center();
                let angle = (data.1 - by).atan2(data.0 - bx) - (py - by).atan2(px - bx);
                let degrees = (self.rotation_on_press + angle).to_degrees();
                self.set_rotation(core, degrees);
                on_press
            }
            (Action::Resize, Some(grab)) => self.resized(core, grab, data, reference),
            (Action::Move, _) => {
                let (dx, dy) = (data.0 - px, data.1 - py);
                let (x0, x1, y0, y1) = on_press;
                (x0 + dx, x1 + dx, y0 + dy, y1 + dy)
            }
            _ => self.created(core, (px, py), data, reference)?,
        };
        self.set_extents(core, extents);
        None
    }

    fn release(&mut self, core: &mut Core, _event: &SelectorEvent) -> Option<Self::Selection> {
        core.ax.figure().set_cursor(CursorShape::Pointer);
        if !self.settings.interactive {
            self.patch.set_visible(false);
        }
        if self.grab.is_none() && core.selection_completed && self.settings.ignore_event_outside {
            return None;
        }
        let (x0, x1, y0, y1) = self.extents();
        let trans = core
            .ax
            .figure()
            .resolve_transform(&core.ax.trans_data())
            .ok()?;
        let corner = |template: &SelectorEvent, x: f64, y: f64| {
            let at = trans.transform_point(Point::new(x, y));
            SelectorEvent {
                x: at.x,
                y: at.y,
                xdata: x,
                ydata: y,
                ..template.clone()
            }
        };
        let press = corner(core.eventpress.as_ref()?, x0, y0);
        let release = corner(core.eventrelease.as_ref()?, x1, y1);
        let (spanx, spany) = match self.settings.spancoords {
            SelectorUnits::Data => (
                (press.xdata - release.xdata).abs(),
                (press.ydata - release.ydata).abs(),
            ),
            SelectorUnits::Pixels => ((press.x - release.x).abs(), (press.y - release.y).abs()),
        };
        let selection = if spanx <= self.settings.minspanx || spany <= self.settings.minspany {
            let existed = core.selection_completed;
            self.clear(core);
            existed.then_some((press, release))
        } else {
            core.selection_completed = true;
            Some((press, release))
        };
        self.grab = None;
        self.extents_on_press = None;
        selection
    }
}
