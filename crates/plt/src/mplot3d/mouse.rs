//! Turning, zooming and panning a 3-D axes with the mouse (matplotlib's
//! `Axes3D.mouse_init`, `_on_move`, `drag_pan` and `_scale_axis_limits`):
//! the buttons doing each, and the view or limits a drag produces.

use std::fmt;
use std::str::FromStr;

use plt_core::Point;

use super::camera::{DIST, ViewAngles};
use super::error::Axes3DError;
use super::handle::Axes3D;
use super::math::{Vec3, norm};
use super::quaternion::Quaternion;
use super::state::Axes3DState;
use crate::navigation::PointerButton;
use crate::rc::RcParams;

/// How dragging with the rotate button turns the view
/// (`axes3d.mouserotationstyle`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RotationStyle {
    /// Horizontal motion changes the azimuth, vertical motion the
    /// elevation.
    Azel,
    /// The scene turns about the axis perpendicular to the motion.
    Trackball,
    /// The point under the pointer follows it on a virtual sphere.
    Sphere,
    /// Ken Shoemake's arcball: twice the rotation of `Sphere`, and
    /// path-independent.
    #[default]
    Arcball,
}

impl RotationStyle {
    /// matplotlib's name (`azel`, `trackball`, `sphere` or `arcball`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Azel => "azel",
            Self::Trackball => "trackball",
            Self::Sphere => "sphere",
            Self::Arcball => "arcball",
        }
    }
}

impl fmt::Display for RotationStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for RotationStyle {
    type Err = Axes3DError;

    /// Parses matplotlib's name ([`RotationStyle::name`]).
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        [Self::Azel, Self::Trackball, Self::Sphere, Self::Arcball]
            .into_iter()
            .find(|style| style.name() == name)
            .ok_or_else(|| Axes3DError::UnknownRotationStyle(name.to_owned()))
    }
}

/// The rotation settings of the rcParams.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RotationSettings {
    style: RotationStyle,
    /// The trackball diameter in units of the axes box.
    size: f64,
    /// The trackball border width in units of the axes box.
    border: f64,
    /// The step angles snap to with control held (0 disables snapping).
    snap: f64,
}

impl RotationSettings {
    /// The `axes3d.*` rotation settings.
    pub(crate) fn from_rc(rc: &RcParams) -> Self {
        Self {
            style: rc.axes3d_mouserotationstyle,
            size: rc.axes3d_trackballsize,
            border: rc.axes3d_trackballborder,
            snap: rc.axes3d_snap_rotation,
        }
    }
}

/// What dragging with a button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MouseAction {
    /// Turn the view.
    Rotate,
    /// Move the limits across the screen.
    Pan,
    /// Scale the limits about their centre.
    Zoom,
}

/// The mouse buttons that rotate, pan and zoom a 3-D axes (matplotlib's
/// `mouse_init`); a button listed for several actions does the first.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MouseButtons {
    /// The buttons turning the view (the left one by default).
    pub rotate: Vec<PointerButton>,
    /// The buttons panning (the middle one by default).
    pub pan: Vec<PointerButton>,
    /// The buttons zooming (the right one by default).
    pub zoom: Vec<PointerButton>,
}

impl Default for MouseButtons {
    fn default() -> Self {
        Self {
            rotate: vec![PointerButton::Left],
            pan: vec![PointerButton::Middle],
            zoom: vec![PointerButton::Right],
        }
    }
}

impl MouseButtons {
    /// No button does anything (matplotlib's `disable_mouse_rotation`).
    pub fn none() -> Self {
        Self {
            rotate: Vec::new(),
            pan: Vec::new(),
            zoom: Vec::new(),
        }
    }

    /// What dragging with `button` does.
    pub(crate) fn action(&self, button: PointerButton) -> Option<MouseAction> {
        [
            (&self.rotate, MouseAction::Rotate),
            (&self.pan, MouseAction::Pan),
            (&self.zoom, MouseAction::Zoom),
        ]
        .into_iter()
        .find_map(|(buttons, action)| buttons.contains(&button).then_some(action))
    }
}

/// The point on the virtual trackball under `(x, y)` (in units of the
/// view window, centred), a sphere whose edge is softened by a section of
/// a cylinder (matplotlib's `_arcball`).
fn arcball(x: f64, y: f64, settings: &RotationSettings) -> Vec3 {
    let s = settings.size / 2.0;
    let b = settings.border / s;
    let (x, y) = (x / s, y / s);
    let r2 = x * x + y * y;
    let r = r2.sqrt();
    let ra = 1.0 + b;
    let a = b * (1.0 + b / 2.0);
    let ri = 2.0 / (ra + 1.0 / ra);
    if r < ri {
        [(1.0 - r2).sqrt(), x, y]
    } else if r < ra {
        let dr = ra - r;
        let p = [a - ((a + dr) * (a - dr)).sqrt(), x, y];
        let n = norm(p);
        p.map(|c| c / n)
    } else {
        [0.0, x / r, y / r]
    }
}

/// The view after the pointer moved from `start` to `end`, in the
/// projected coordinates of a view window `size` wide and high; `None`
/// when it did not move. With `snap`, the angles round to the
/// `axes3d.snap_rotation` step.
pub(crate) fn rotated(
    angles: ViewAngles,
    start: Point,
    end: Point,
    size: (f64, f64),
    settings: &RotationSettings,
    snap: bool,
) -> Option<ViewAngles> {
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    if dx == 0.0 && dy == 0.0 {
        return None;
    }
    let (w, h) = size;
    let (elev, azim, roll) = if settings.style == RotationStyle::Azel {
        let (sin, cos) = angles.roll.to_radians().sin_cos();
        let delev = -(dy / h) * 180.0 * cos + (dx / w) * 180.0 * sin;
        let dazim = -(dy / h) * 180.0 * sin - (dx / w) * 180.0 * cos;
        (angles.elev + delev, angles.azim + dazim, angles.roll)
    } else {
        let q = Quaternion::from_cardan_angles(
            angles.elev.to_radians(),
            angles.azim.to_radians(),
            angles.roll.to_radians(),
        );
        let dq = match settings.style {
            RotationStyle::Trackball => {
                let k = [0.0, -dy / h, dx / w];
                let nk = norm(k);
                let th = nk / settings.size;
                let (sin, cos) = th.sin_cos();
                Quaternion::new(cos, k.map(|c| c * sin / nk))
            }
            style => {
                let current = arcball(start.x / w, start.y / h, settings);
                let new = arcball(end.x / w, end.y / h, settings);
                if style == RotationStyle::Sphere {
                    Quaternion::rotate_from_to(current, new)
                } else {
                    Quaternion::new(0.0, new).mul(Quaternion::new(0.0, current.map(|c| -c)))
                }
            }
        };
        let (elev, azim, roll) = dq.mul(q).as_cardan_angles();
        (elev.to_degrees(), azim.to_degrees(), roll.to_degrees())
    };
    let snapped = |angle: f64| {
        if snap && settings.snap > 0.0 {
            settings.snap * (angle / settings.snap).round_ties_even()
        } else {
            angle
        }
    };
    Some(ViewAngles {
        elev: snapped(elev),
        azim: snapped(azim),
        roll: snapped(roll),
    })
}

impl Axes3D {
    /// Chooses the mouse buttons that rotate, pan and zoom the axes
    /// (matplotlib's `mouse_init`).
    pub fn mouse_init(&self, buttons: MouseButtons) -> &Self {
        self.with_mut(|axes| axes.mouse = buttons);
        self
    }

    /// Stops every mouse button from rotating, panning or zooming the
    /// axes (matplotlib's `disable_mouse_rotation`).
    pub fn disable_mouse_rotation(&self) -> &Self {
        self.mouse_init(MouseButtons::none())
    }

    /// The buttons that rotate, pan and zoom the axes.
    pub fn mouse_buttons(&self) -> MouseButtons {
        self.with(|axes| axes.mouse.clone())
    }

    /// The width and height of the view window in projected coordinates
    /// (matplotlib's `_pseudo_w` and `_pseudo_h`).
    pub(crate) fn view_window_size() -> (f64, f64) {
        let (lo, hi) = super::camera::Camera::view_window();
        (hi - lo, hi - lo)
    }

    /// Turns the view, and that of the axes sharing it, as dragging the
    /// rotate button from `start` to `end` does (projected coordinates of
    /// the view window, as matplotlib's `event.xdata`), following
    /// `axes3d.mouserotationstyle`; `snap` (control held) rounds the
    /// angles to `axes3d.snap_rotation`.
    pub fn drag_rotate(&self, start: Point, end: Point, snap: bool) -> &Self {
        let settings = RotationSettings::from_rc(&crate::rc::params());
        let size = Self::view_window_size();
        if let Some(view) = rotated(self.view(), start, end, size, &settings, snap) {
            self.view_init_shared(view.elev, view.azim, view.roll);
        }
        self
    }

    /// Zooms as dragging the zoom button from height `start_y` to `end_y`
    /// does (projected coordinates): dragging down zooms in.
    ///
    /// # Errors
    ///
    /// When the drag makes a limit infinite or NaN.
    pub fn drag_zoom(&self, start_y: f64, end_y: f64) -> Result<&Self, Axes3DError> {
        let (_, h) = Self::view_window_size();
        let scale = h / (h - (end_y - start_y));
        self.scale_limits([scale; 3])
    }

    /// Scales the limits of each axis about their centre, by more than 1
    /// to zoom out, in transformed coordinates (matplotlib's
    /// `_scale_axis_limits`).
    ///
    /// # Errors
    ///
    /// When a limit becomes infinite or NaN.
    pub fn scale_limits(&self, scale: [f64; 3]) -> Result<&Self, Axes3DError> {
        let limits = self.with(Axes3DState::scaled_limits);
        let scaled = |i: usize| {
            let (lo, hi) = (limits[2 * i], limits[2 * i + 1]);
            let (center, range) = (f64::midpoint(hi, lo), hi - lo);
            (
                center - range * scale[i] / 2.0,
                center + range * scale[i] / 2.0,
            )
        };
        self.set_limits_from_transformed([scaled(0), scaled(1), scaled(2)])
    }

    /// Moves the view by `(du, dv)` in projected coordinates, as panning
    /// does (matplotlib's `drag_pan`): the motion on the screen becomes a
    /// shift of the limits along the view axes, in transformed
    /// coordinates.
    ///
    /// # Errors
    ///
    /// When a limit becomes infinite or NaN.
    pub fn pan_view(&self, du: f64, dv: f64) -> Result<&Self, Axes3DError> {
        if du == 0.0 && dv == 0.0 {
            return Ok(self);
        }
        let (frame, aspect, limits) = self.with(|axes| {
            (
                axes.camera.view_frame(),
                axes.camera.stored_box_aspect(),
                axes.scaled_limits(),
            )
        });
        let shift = |j: usize| {
            let r = |i: usize| -frame[i][j] / aspect[j] * DIST;
            r(0) * du + r(1) * dv + r(2) * 0.0
        };
        let moved = |i: usize| {
            let (lo, hi) = (limits[2 * i], limits[2 * i + 1]);
            let delta = (hi - lo) * shift(i);
            (lo + delta, hi + delta)
        };
        self.set_limits_from_transformed([moved(0), moved(1), moved(2)])
    }

    /// Sets the limits of the three axes from transformed coordinates,
    /// keeping their autoscaling (matplotlib's
    /// `_set_lims_from_transformed`).
    fn set_limits_from_transformed(&self, limits: [(f64, f64); 3]) -> Result<&Self, Axes3DError> {
        self.with_mut(|axes| axes.set_lims_from_transformed(limits))?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn settings(style: RotationStyle) -> RotationSettings {
        RotationSettings {
            style,
            ..RotationSettings::from_rc(&RcParams::matplotlib())
        }
    }

    #[test]
    fn rotation_styles_round_trip_their_names() {
        for style in ["azel", "trackball", "sphere", "arcball"] {
            assert_eq!(
                style.parse::<RotationStyle>().map(|s| s.to_string()),
                Ok(style.to_owned())
            );
        }
        assert!("Arcball".parse::<RotationStyle>().is_err());
    }

    #[test]
    fn azel_drags_turn_azimuth_and_elevation() {
        let start = Point::new(0.0, 0.0);
        let size = Axes3D::view_window_size();
        let right = Point::new(size.0 / 2.0, 0.0);
        let view = rotated(
            ViewAngles::default(),
            start,
            right,
            size,
            &settings(RotationStyle::Azel),
            false,
        )
        .unwrap();
        assert!((view.azim - (-60.0 - 90.0)).abs() < 1e-9, "{view:?}");
        assert_eq!(view.elev, 30.0);
        let same = rotated(
            ViewAngles::default(),
            start,
            start,
            size,
            &settings(RotationStyle::Azel),
            false,
        );
        assert_eq!(same, None);
    }

    #[test]
    fn every_style_turns_the_view_and_snaps() {
        let size = Axes3D::view_window_size();
        let (start, end) = (Point::new(0.01, -0.02), Point::new(0.03, 0.015));
        for style in [
            RotationStyle::Azel,
            RotationStyle::Trackball,
            RotationStyle::Sphere,
            RotationStyle::Arcball,
        ] {
            let free = rotated(
                ViewAngles::default(),
                start,
                end,
                size,
                &settings(style),
                false,
            )
            .unwrap();
            assert_ne!(free, ViewAngles::default(), "{style:?}");
            let snapped = rotated(
                ViewAngles::default(),
                start,
                end,
                size,
                &settings(style),
                true,
            )
            .unwrap();
            for angle in [snapped.elev, snapped.azim, snapped.roll] {
                assert_eq!(angle % 5.0, 0.0, "{style:?}: {snapped:?}");
            }
        }
    }

    #[test]
    fn arcball_points_lie_on_the_unit_sphere() {
        let settings = settings(RotationStyle::Arcball);
        for (x, y) in [(0.0, 0.0), (0.2, 0.1), (0.35, 0.0), (2.0, 2.0)] {
            let p = arcball(x, y, &settings);
            assert!((norm(p) - 1.0).abs() < 1e-12, "({x}, {y}) → {p:?}");
        }
    }

    #[test]
    fn buttons_choose_actions() {
        let buttons = MouseButtons::default();
        assert_eq!(
            buttons.action(PointerButton::Left),
            Some(MouseAction::Rotate)
        );
        assert_eq!(
            buttons.action(PointerButton::Right),
            Some(MouseAction::Zoom)
        );
        assert_eq!(MouseButtons::none().action(PointerButton::Middle), None);
        assert_eq!("sphere".parse(), Ok(RotationStyle::Sphere));
        assert!("spin".parse::<RotationStyle>().is_err());
    }

    #[test]
    fn zooming_and_panning_move_the_limits() {
        let ax = Figure::new().subplot_3d();
        ax.set_xlim(0.0, 10.0).unwrap();
        ax.drag_zoom(0.0, -0.0925).unwrap();
        let (lo, hi) = ax.xlim();
        assert!(
            (hi - lo - 20.0 / 3.0).abs() < 1e-9,
            "zoomed in to {lo}..{hi}"
        );
        let before = (ax.xlim(), ax.ylim(), ax.zlim());
        ax.pan_view(0.01, 0.0).unwrap();
        assert_ne!(before, (ax.xlim(), ax.ylim(), ax.zlim()));
        assert!(ax.scale_limits([f64::INFINITY; 3]).is_err());
    }
}
