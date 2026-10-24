//! Zooming (matplotlib's zoom-to-rectangle tool and
//! `Axes._set_view_from_bbox`).

use plt_core::{Bbox, Point};

use super::pan::bbox_through;
use super::{DragKey, PointerButton, Rubberband, ZoomDirection};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxesState, AxisName};
use crate::colorbar::Colorbar;
use crate::figure::{Figure, FigureState};
use crate::orientation::Orientation;
use crate::projections::{FrozenView, Projection, ZoomBox};
use crate::transforms::{ResolvedTransform, TransformContext};

/// Drags shorter than this many pixels cancel a zoom.
const MIN_DRAG: f64 = 5.0;

impl Axes {
    /// Whether the zoom-to-rectangle tool works on this axes (not on
    /// polar or geographic axes, nor on those of custom projections
    /// refusing it; matplotlib's `can_zoom`).
    pub fn can_zoom(&self) -> bool {
        self.as_polar().is_none() && self.custom_projection_allows(Projection::can_zoom)
    }

    /// Whether the pan/zoom tool works on this axes (not on geographic
    /// axes, nor on those of custom projections refusing it;
    /// matplotlib's `can_pan`).
    pub fn can_pan(&self) -> bool {
        self.custom_projection_allows(Projection::can_pan)
    }

    /// Whether the custom projection of this axes, if any, allows what
    /// `allows` asks it.
    fn custom_projection_allows(&self, allows: impl FnOnce(&dyn Projection) -> bool) -> bool {
        self.raw()
            .with::<AxesState, _>(|state| state.projection.custom().is_none_or(allows))
    }

    /// Whether the navigation toolbar acts on this axes and reports its
    /// coordinates (matplotlib's `get_navigate`).
    pub fn navigate(&self) -> bool {
        self.raw().with::<AxesState, _>(|state| state.navigate)
    }

    /// Lets the navigation toolbar act on this axes or not (matplotlib's
    /// `set_navigate`; widgets turn it off).
    pub fn set_navigate(&self, navigate: bool) -> &Self {
        self.raw()
            .with_mut::<AxesState, _>(|state| state.navigate = navigate);
        self
    }

    /// Sets whether panning and zooming on this axes also act on the axes
    /// below it (at a lower or equal z-order): `Some(true)` always,
    /// `Some(false)` never, `None` (matplotlib's `"auto"`) when its
    /// background is hidden (matplotlib's `set_forward_navigation_events`).
    pub fn set_forward_navigation_events(&self, forward: Option<bool>) -> &Self {
        self.raw()
            .with_mut::<AxesState, _>(|state| state.forward_navigation = forward);
        self
    }

    /// Whether panning and zooming reach the axes below; `None` is
    /// `"auto"`.
    pub fn forward_navigation_events(&self) -> Option<bool> {
        self.raw()
            .with::<AxesState, _>(|state| state.forward_navigation)
    }

    /// Whether this axes keeps pan and zoom from the axes below
    /// (matplotlib's `_capture_events`).
    pub(super) fn captures_navigation(&self) -> bool {
        self.forward_navigation_events()
            .map_or_else(|| self.patch_visible(), |forward| !forward)
    }
}

/// The axes box and data transform of `ax`, brought up to date.
fn geometry(ax: &Axes) -> Option<(Bbox, ResolvedTransform)> {
    let figure = ax.raw().figure_state();
    let mut state = figure.borrow_mut();
    state.prepare();
    let state: &FigureState = &state;
    Some((state.axes_bbox(ax.id())?, state.data_transform(ax.id())?))
}

/// The rectangle to show while dragging from `start` to `at` over `ax`
/// (matplotlib's `drag_zoom`): clipped to the axes, spanning its height
/// with `x` held or its width with `y`.
pub(super) fn rubberband(ax: &Axes, start: Point, at: Point, key: DragKey) -> Option<Rubberband> {
    let key = colorbar_key(ax).unwrap_or(key);
    let (bbox, _) = geometry(ax)?;
    let clip = |p: Point| {
        Point::new(
            p.x.clamp(bbox.xmin(), bbox.xmax()),
            p.y.clamp(bbox.ymin(), bbox.ymax()),
        )
    };
    let (mut a, mut b) = (clip(start), clip(at));
    match key {
        DragKey::X => (a.y, b.y) = bbox.interval_y(),
        DragKey::Y => (a.x, b.x) = bbox.interval_x(),
        _ => {}
    }
    Some(Rubberband {
        bbox: Bbox::from_extents(a.x, a.y, b.x, b.y),
    })
}

/// The key a zoom starting on `ax` acts as if held: one on a colorbar
/// spans its whole width and acts along the colour scale only
/// (matplotlib's `_zoom_info.cbar`).
fn colorbar_key(ax: &Axes) -> Option<DragKey> {
    Colorbar::in_axes(ax).map(|colorbar| match colorbar.orientation() {
        Orientation::Horizontal => DragKey::X,
        Orientation::Vertical => DragKey::Y,
    })
}

/// Which axes of each zoomed axes change: twins zoomed through an earlier
/// axes and the axis a held key excludes stay put.
#[derive(Clone, Copy, Debug)]
struct Changes {
    x: bool,
    y: bool,
}

/// For each axes, whether its x and y should change, skipping axes
/// sharing x or y with an earlier one (matplotlib's `twinx`/`twiny`).
fn changes(figure: &Figure, axes: &[Axes], key: DragKey) -> Vec<Changes> {
    let state = figure.state();
    axes.iter()
        .enumerate()
        .map(|(i, ax)| {
            let shares = |name: AxisName| {
                axes[..i]
                    .iter()
                    .any(|prev| state.siblings(name, ax.id()).contains(&prev.id()))
            };
            Changes {
                x: !shares(AxisName::X) && key != DragKey::Y,
                y: !shares(AxisName::Y) && key != DragKey::X,
            }
        })
        .collect()
}

/// Whether a zoom box dragged from `start` to `at` is too small to count:
/// under 5 pixels across a side that `key` does not fix (matplotlib's
/// cancelled `release_zoom`).
pub(super) fn is_click(start: Point, at: Point, key: DragKey) -> bool {
    ((at.x - start.x).abs() < MIN_DRAG && key != DragKey::Y)
        || ((at.y - start.y).abs() < MIN_DRAG && key != DragKey::X)
}

/// Zooms `axes` to the box dragged from `start` to `at` with `button`;
/// returns whether the view changed (matplotlib's `release_zoom`).
pub(super) fn zoom_to_rect(
    figure: &Figure,
    axes: &[Axes],
    start: Point,
    at: Point,
    button: PointerButton,
    key: DragKey,
) -> bool {
    let key = axes.first().and_then(colorbar_key).unwrap_or(key);
    if is_click(start, at, key) {
        return false;
    }
    let direction = if button == PointerButton::Left {
        ZoomDirection::In
    } else {
        ZoomDirection::Out
    };
    let corners = Bbox::from_extents(start.x, start.y, at.x, at.y);
    for (ax, change) in axes.iter().zip(changes(figure, axes, key)) {
        set_view_from_bbox(ax, corners, None, direction, change);
    }
    true
}

/// Zooms `axes` by `scale` (> 1 zooms in) around display point `at`
/// (matplotlib's `_set_view_from_bbox((x, y, scale))`).
pub(super) fn zoom_around(figure: &Figure, axes: &[Axes], at: Point, scale: f64) {
    let (direction, factor) = if scale > 1.0 {
        (ZoomDirection::In, scale)
    } else {
        (ZoomDirection::Out, 1.0 / scale)
    };
    for (ax, change) in axes.iter().zip(changes(figure, axes, DragKey::None)) {
        let Some((_, trans)) = geometry(ax) else {
            continue;
        };
        let limits = ax.raw().with::<AxesState, _>(|state| state.view_lim);
        let view = bbox_through(&limits, &trans);
        let (center, half) = (view.center(), (view.width() / 2.0, view.height() / 2.0));
        let focus = Point::new(
            (at.x * (factor - 1.0) + center.x) / factor,
            (at.y * (factor - 1.0) + center.y) / factor,
        );
        let corners = Bbox::from_extents(
            focus.x - half.0 / factor,
            focus.y - half.1 / factor,
            focus.x + half.0 / factor,
            focus.y + half.1 / factor,
        );
        set_view_from_bbox(ax, corners, Some((at, scale)), direction, change);
    }
}

/// matplotlib's `_prepare_view_from_bbox` + `_set_view_from_bbox` for the
/// box `corners` in display coordinates (made around the pointer for the
/// `wheel`), or the override of a projection defined by the program; on a
/// colorbar, the bound along the colour scale becomes the norm's limits
/// (`Colorbar._set_view_from_bbox`).
fn set_view_from_bbox(
    ax: &Axes,
    corners: Bbox,
    wheel: Option<(Point, f64)>,
    direction: ZoomDirection,
    change: Changes,
) {
    let Some((bbox, trans)) = geometry(ax) else {
        return;
    };
    let Some(inverse) = trans.inverted() else {
        return;
    };
    let limits = ax.raw().with::<AxesState, _>(|state| state.view_lim);
    let view = FrozenView::new(limits, bbox, trans, inverse);
    let zoom = ZoomBox::new(corners, wheel, direction, (change.x, change.y), view);
    if let Some(limits) = ax
        .with_custom_projection_mut(|projection| projection.zoom_to_box(&zoom))
        .flatten()
    {
        limits.apply(ax);
        return;
    }
    let colorbar = Colorbar::in_axes(ax);
    let inverse = zoom.view().to_data();
    let start = inverse.transform_point(Point::new(corners.x0(), corners.y0()));
    let stop = inverse.transform_point(Point::new(corners.x1(), corners.y1()));
    for (name, (a, b)) in [
        (AxisName::X, (start.x, stop.x)),
        (AxisName::Y, (start.y, stop.y)),
    ] {
        if !zoom.changes(name) {
            continue;
        }
        let (low0, high0) = bound(ax, name);
        let (low, high) = (a.min(b).clamp(low0, high0), a.max(b).clamp(low0, high0));
        let bounds = match direction {
            ZoomDirection::In => (low, high),
            ZoomDirection::Out => unzoom(ax, name, (low0, high0), (low, high)),
        };
        match &colorbar {
            Some(colorbar) if colorbar.long_axis().name() == name => colorbar.set_view(bounds),
            Some(_) => {}
            None => set_bound(ax, name, bounds),
        }
    }
}

/// The view shrunk into `(low, high)` from `(low0, high0)`, in the axis'
/// scale space.
fn unzoom(
    ax: &Axes,
    name: AxisName,
    (low0, high0): (f64, f64),
    (low, high): (f64, f64),
) -> (f64, f64) {
    ax.raw().with::<AxesState, _>(|state| {
        let axis = state.axis(name);
        let (s_low0, s_high0) = (axis.forward(low0), axis.forward(high0));
        let (s_low, s_high) = (axis.forward(low), axis.forward(high));
        let factor = (s_high0 - s_low0) / (s_high - s_low);
        (
            axis.inverse(s_low0 - factor * (s_low - s_low0)),
            axis.inverse(s_high0 + factor * (s_high0 - s_high)),
        )
    })
}

/// The view interval of one axis, increasing (matplotlib's `get_xbound`).
fn bound(ax: &Axes, name: AxisName) -> (f64, f64) {
    let (a, b) = match name {
        AxisName::X => ax.xlim(),
        AxisName::Y => ax.ylim(),
    };
    (a.min(b), a.max(b))
}

/// Sets the view interval keeping the axis direction and turns its
/// autoscaling off (matplotlib's `set_xbound` + `set_autoscalex_on`).
fn set_bound(ax: &Axes, name: AxisName, (low, high): (f64, f64)) {
    let inverted = ax.raw().with::<AxesState, _>(|state| {
        let (a, b) = state.view_interval(name);
        a > b
    });
    let (start, end) = if inverted { (high, low) } else { (low, high) };
    // Zooming out of a degenerate box gives limits that are not finite;
    // they leave the axis as it is, where matplotlib's event handler
    // raises.
    let _ = ax.set_limits(name, start, end);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(ax: &Axes, x: f64, y: f64) -> Point {
        geometry(ax).unwrap().1.transform_point(Point::new(x, y))
    }

    #[test]
    fn dragging_a_box_zooms_into_it() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(10.0, 0.0))
            .expect("finite limits");
        let (a, b) = (display(&ax, 2.0, 3.0), display(&ax, 4.0, 7.0));
        assert!(zoom_to_rect(
            &fig,
            std::slice::from_ref(&ax),
            a,
            b,
            PointerButton::Left,
            DragKey::None
        ));
        let (x0, x1) = ax.xlim();
        assert!((x0 - 2.0).abs() < 1e-9 && (x1 - 4.0).abs() < 1e-9);
        let (y0, y1) = ax.ylim();
        assert!(
            (y0 - 7.0).abs() < 1e-9 && (y1 - 3.0).abs() < 1e-9,
            "inversion kept"
        );
    }

    #[test]
    fn tiny_drags_cancel_and_keys_fix_an_axis() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let a = display(&ax, 2.0, 2.0);
        let near = Point::new(a.x + 2.0, a.y + 50.0);
        assert!(!zoom_to_rect(
            &fig,
            std::slice::from_ref(&ax),
            a,
            near,
            PointerButton::Left,
            DragKey::None
        ));
        let b = display(&ax, 5.0, 6.0);
        zoom_to_rect(
            &fig,
            std::slice::from_ref(&ax),
            a,
            b,
            PointerButton::Left,
            DragKey::X,
        );
        let (x0, x1) = ax.xlim();
        assert!((x0 - 2.0).abs() < 1e-9 && (x1 - 5.0).abs() < 1e-9);
        assert_eq!(ax.ylim(), (0.0, 10.0));
    }

    #[test]
    fn zooming_out_and_around_a_point() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let (a, b) = (display(&ax, 0.0, 0.0), display(&ax, 5.0, 5.0));
        zoom_to_rect(
            &fig,
            std::slice::from_ref(&ax),
            a,
            b,
            PointerButton::Right,
            DragKey::None,
        );
        let (x0, x1) = ax.xlim();
        assert!((x0 - 0.0).abs() < 1e-9 && (x1 - 20.0).abs() < 1e-9);
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        zoom_around(&fig, std::slice::from_ref(&ax), display(&ax, 0.0, 0.0), 2.0);
        let (x0, x1) = ax.xlim();
        assert!(
            x0.abs() < 1e-9 && (x1 - 5.0).abs() < 1e-9,
            "the focus stays put"
        );
    }

    #[test]
    fn rubberband_spans_the_axes_with_a_key() {
        let ax = Figure::new().subplot();
        let (bbox, _) = geometry(&ax).unwrap();
        let band = rubberband(&ax, bbox.center(), Point::new(-50.0, 0.0), DragKey::X).unwrap();
        assert_eq!(band.bbox.x1(), bbox.x0(), "clipped to the axes");
        assert_eq!(band.bbox.interval_y(), bbox.interval_y());
    }
}
