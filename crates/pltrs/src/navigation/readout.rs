//! Which axes is under the pointer and the data coordinates there, as a
//! status bar shows them (matplotlib's `inaxes` and `format_coord`).

use std::any::Any;
use std::f64::consts::{PI, TAU};
use std::rc::Rc;

use pltrs_core::{Bbox, Point};

use crate::artist::{ArtistHandle, LayoutContext};
use crate::axes::{Axes, AxesState, AxisName};
use crate::figure::{Figure, FigureState};
use crate::ticker::{Formatter, ScalarFormatter, TickContext, g_sig_digits, pyfmt};
use crate::transforms::{ResolvedTransform, TransformContext};

/// The axes under display point `at`: the visible axes whose background
/// contains it, the highest z-order first and, among equals, the last
/// created, which is drawn on top (matplotlib's `canvas.inaxes`).
pub fn axes_at(figure: &Figure, at: Point) -> Option<Axes> {
    figure
        .axes()
        .into_iter()
        .filter(|ax| ax.visible() && contains(ax, at))
        .fold(None, |best: Option<Axes>, ax| match best {
            Some(best) if best.zorder() > ax.zorder() => Some(best),
            _ => Some(ax),
        })
}

impl Axes {
    /// Whether display point `at` (pixels from the figure's bottom-left
    /// corner) is over the axes background: its box, or the wedge of polar
    /// axes (matplotlib's `contains_point`).
    pub fn contains_point(&self, at: Point) -> bool {
        contains(self, at)
    }
}

/// Whether the background of `ax` contains display point `at`
/// (matplotlib's `in_axes`): its box, or the wedge of polar axes.
pub(super) fn contains(ax: &Axes, at: Point) -> bool {
    let figure = ax.raw().figure_state();
    let mut state = figure.borrow_mut();
    state.prepare();
    LayoutContext::new(&state).axes_contains(ax.id(), at)
}

/// What a status bar shows of the data under display point `at` in `ax`:
/// the cursor text of the topmost visible artist of `ax` that has
/// `mouseover` set and contains the point, the last added among equal
/// z-orders (the second line of matplotlib's `_mouse_event_to_message`).
pub(super) fn cursor_text(ax: &Axes, at: Point) -> Option<String> {
    let figure = ax.raw().figure_state();
    let mut state = figure.borrow_mut();
    state.prepare();
    let layout = LayoutContext::new(&state);
    let store = layout.store();
    let topmost = store
        .get(ax.id())?
        .children()
        .into_iter()
        .filter_map(|id| store.get(id))
        .filter(|artist| {
            let base = artist.base();
            base.mouseover && base.visible && artist.pick_hit(&layout, at, None).is_some()
        })
        .reduce(|best, artist| {
            if best.base().zorder > artist.base().zorder {
                best
            } else {
                artist
            }
        })?;
    let text = topmost.cursor_text(&layout, at)?;
    let text = text.trim_end();
    (!text.is_empty()).then(|| text.to_owned())
}

/// The display box of `ax`, brought up to date.
pub(super) fn display_bbox(ax: &Axes) -> Option<Bbox> {
    let figure = ax.raw().figure_state();
    let mut state = figure.borrow_mut();
    state.prepare();
    state.axes_bbox(ax.id())
}

/// The data → display transform of `ax`, brought up to date.
pub(super) fn data_transform(ax: &Axes) -> Option<ResolvedTransform> {
    let figure = ax.raw().figure_state();
    let mut state = figure.borrow_mut();
    state.prepare();
    let state: &FigureState = &state;
    state.data_transform(ax.id())
}

/// The data coordinates of `ax` (and of its twins) at display point `at`
/// (matplotlib's `format_coord`); empty when they cannot be formatted.
pub(super) fn format_coord(ax: &Axes, at: Point) -> String {
    if ax.as_polar().is_some() {
        return data_transform(ax)
            .and_then(|trans| trans.inverted())
            .and_then(|inverse| polar_coord(ax, inverse.transform_point(at)))
            .unwrap_or_default();
    }
    if let Some(text) = custom_coord_at(ax, at) {
        return text;
    }
    let mut twins = ax.twins();
    twins.sort_by(|a, b| a.zorder().total_cmp(&b.zorder()));
    let pairs: Option<Vec<String>> = twins
        .iter()
        .map(|twin| {
            let data = data_transform(twin)?.inverted()?.transform_point(at);
            coordinate_pair(twin, data)
        })
        .collect();
    pairs.map_or_else(String::new, |pairs| {
        format!("(x, y) = {}", pairs.join(" | "))
    })
}

/// The data point `data` of `ax` as `(x, y)`, each coordinate formatted
/// by [`format_data`].
fn coordinate_pair(ax: &Axes, data: Point) -> Option<String> {
    let x = format_data(ax, AxisName::X, data.x)?;
    let y = format_data(ax, AxisName::Y, data.y)?;
    Some(format!("({x}, {y})"))
}

/// `value` along axis `name` of `ax`, formatted by the axis' major
/// formatter in its short form; a `ScalarFormatter` gives as many digits
/// as a screen pixel resolves around the value (matplotlib's
/// `format_xdata`/`format_ydata` with `format_data_short`, which measures
/// the pixel in the axis transform).
fn format_data(ax: &Axes, name: AxisName, value: f64) -> Option<String> {
    let resolution = ax.pixel_resolution(name, value)?;
    ax.raw().with::<AxesState, _>(|state| {
        let axis = state.axis(name);
        let custom = match name {
            AxisName::X => state.fmt_data.0.as_ref(),
            AxisName::Y => state.fmt_data.1.as_ref(),
        };
        let formatter = custom.unwrap_or(&axis.major_formatter);
        if (formatter.as_ref() as &dyn Any).is::<ScalarFormatter>() {
            return ScalarFormatter::format_data_resolved(value, resolution, &axis.settings).ok();
        }
        let ((low, high), minpos) = state.data_interval(name);
        let info = axis.info(&crate::axes::axis::AxisGeometry {
            view: state.view_interval(name),
            data: (low, high),
            minpos,
            length_points: 0.0,
            tick_space: None,
            tick_floor: None,
        });
        formatter
            .format_data_short(value, &TickContext::new(&info, &axis.settings))
            .ok()
    })
}

impl Axes {
    /// The data size of a screen pixel along axis `name` around `value`:
    /// the farthest the values one pixel to either side lie from it
    /// (matplotlib's `format_data_short`, which measures the pixel in the
    /// axis transform). The figure is laid out first; `None` when the
    /// axes has no extent.
    pub(crate) fn pixel_resolution(&self, name: AxisName, value: f64) -> Option<f64> {
        let axis_transform = match name {
            AxisName::X => self.trans_xaxis(),
            AxisName::Y => self.trans_yaxis(),
        };
        let trans = self.figure().resolve_transform(&axis_transform).ok()?;
        let inverse = trans.inverted()?;
        let (along, step) = match name {
            AxisName::X => (Point::new(value, 0.0), Point::new(1.0, 0.0)),
            AxisName::Y => (Point::new(0.0, value), Point::new(0.0, 1.0)),
        };
        let pick = |p: Point| match name {
            AxisName::X => p.x,
            AxisName::Y => p.y,
        };
        let at = trans.transform_point(along);
        Some(
            [at - step, at + step]
                .into_iter()
                .map(|p| (pick(inverse.transform_point(p)) - value).abs())
                .fold(0.0, f64::max),
        )
    }

    /// `value` along `axis` as a status bar shows it (matplotlib's
    /// `format_xdata` and `format_ydata`, as `Axes3D::format_data` for
    /// 3-D axes): in the short form of the formatter set with
    /// [`set_fmt_data`](Self::set_fmt_data), else of the axis' major
    /// formatter, which for a `ScalarFormatter` has as many digits as a
    /// screen pixel resolves. The figure is laid out first; `None` when the
    /// formatter fails or the axes has no extent.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.set_xlim(0.0, 10.0)?;
    /// assert_eq!(ax.format_data(pltrs::axes::AxisName::X, 2.5).as_deref(), Some("2.50"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn format_data(&self, axis: AxisName, value: f64) -> Option<String> {
        format_data(self, axis, value)
    }

    /// Formats the coordinates along `axis` that a status bar shows (see
    /// [`format_data`](Self::format_data) and
    /// [`format_coord`](Self::format_coord)) with the short form of
    /// `formatter`, or with the axis' major formatter again for `None`
    /// (matplotlib's `fmt_xdata` and `fmt_ydata`).
    ///
    /// ```
    /// use std::rc::Rc;
    /// use pltrs::axes::AxisName;
    /// use pltrs::prelude::*;
    /// use pltrs::ticker::FuncFormatter;
    /// let ax = Figure::new().subplot();
    /// let foo = Rc::new(FuncFormatter::new(|_, _| "foo".into()));
    /// ax.set_fmt_data(AxisName::X, Some(foo.clone()))
    ///     .set_fmt_data(AxisName::Y, Some(foo));
    /// assert_eq!(ax.format_coord(42.0, 24.0), "(x, y) = (foo, foo)");
    /// ```
    pub fn set_fmt_data(&self, axis: AxisName, formatter: Option<Rc<dyn Formatter>>) -> &Self {
        self.raw().with_mut::<AxesState, _>(|state| match axis {
            AxisName::X => state.fmt_data.0 = formatter,
            AxisName::Y => state.fmt_data.1 = formatter,
        });
        self
    }

    /// The data point `(x, y)` as a status bar shows it (matplotlib's
    /// `format_coord`): `(x, y) = (…, …)` with each coordinate as
    /// [`format_data`](Self::format_data) gives it, followed by the same
    /// screen point in the data of each twin, in z-order; θ and r for
    /// polar axes, longitude and latitude for geographic ones, and what a
    /// custom projection makes of it. Empty when a coordinate cannot be
    /// formatted.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 1.0)?;
    /// assert_eq!(ax.format_coord(2.5, 0.25), "(x, y) = (2.50, 0.250)");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn format_coord(&self, x: f64, y: f64) -> String {
        if let Some(text) = custom_coord(self, x, y) {
            return text;
        }
        if self.as_polar().is_some() {
            return polar_coord(self, Point::new(x, y)).unwrap_or_default();
        }
        if self.twins().len() == 1 {
            return coordinate_pair(self, Point::new(x, y))
                .map_or_else(String::new, |pair| format!("(x, y) = {pair}"));
        }
        data_transform(self)
            .map(|trans| trans.transform_point(Point::new(x, y)))
            .map_or_else(String::new, |at| format_coord(self, at))
    }
}

/// The status-bar text the custom projection of `ax` gives for its data
/// point `(x, y)`, if it has one that formats it.
fn custom_coord(ax: &Axes, x: f64, y: f64) -> Option<String> {
    ax.raw()
        .with::<AxesState, _>(|state| state.projection.custom()?.format_coord(x, y))
}

/// [`custom_coord`] of the data point of `ax` at display point `at`.
fn custom_coord_at(ax: &Axes, at: Point) -> Option<String> {
    let custom = ax
        .raw()
        .with::<AxesState, _>(|state| state.projection.custom().is_some());
    if !custom {
        return None;
    }
    let data = data_transform(ax)?.inverted()?.transform_point(at);
    custom_coord(ax, data.x, data.y)
}

/// matplotlib's `PolarAxes.format_coord` of the data point `data` (θ, r):
/// θ in half turns and degrees and r, each to the precision a screen
/// pixel resolves, unless formatters set with
/// [`Axes::set_fmt_data`] format them.
fn polar_coord(ax: &Axes, data: Point) -> Option<String> {
    let trans = data_transform(ax)?;
    let inverse = trans.inverted()?;
    let (theta, r) = (data.x, data.y);
    let screen = trans.transform_point(data);
    let (mut delta_t, mut delta_r) = (0.0_f64, 0.0_f64);
    for dx in [-1.0, 0.0, 1.0] {
        for dy in [-1.0, 0.0, 1.0] {
            let near = inverse.transform_point(screen + Point::new(dx, dy));
            delta_t = delta_t.max(((near.x - theta + PI).rem_euclid(TAU) - PI).abs());
            delta_r = delta_r.max((near.y - r).abs());
        }
    }
    let theta = if theta < 0.0 { theta + TAU } else { theta };
    let (custom_theta, custom_r) = ax
        .raw()
        .with::<AxesState, _>(|state| (state.fmt_data.0.is_some(), state.fmt_data.1.is_some()));
    let r_label = if custom_r {
        format_data(ax, AxisName::Y, r)?
    } else {
        pyfmt::general_alternate(r, g_sig_digits(r, delta_r).ok()?)
    };
    if custom_theta {
        return Some(format!(
            "\u{3b8}={}, r={r_label}",
            format_data(ax, AxisName::X, theta)?
        ));
    }
    let (turns, delta_turns) = (theta / PI, delta_t / PI);
    let fixed = |value: f64, delta: f64| {
        let digits = (-delta.log10().floor()).max(0.0);
        digits
            .is_finite()
            .then(|| pyfmt::fixed(value, digits as usize))
    };
    Some(format!(
        "\u{3b8}={}\u{3c0} ({}\u{b0}), r={r_label}",
        fixed(turns, delta_turns)?,
        fixed(turns * 180.0, delta_turns * 180.0)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(ax: &Axes, x: f64, y: f64) -> Point {
        data_transform(ax)
            .unwrap()
            .transform_point(Point::new(x, y))
    }

    #[test]
    fn finds_the_axes_under_the_pointer() {
        let fig = Figure::new();
        let grid = fig.subplots(1, 2).unwrap();
        let right = grid[(0, 1)].clone();
        assert_eq!(
            axes_at(&fig, display(&right, 0.5, 0.5)),
            Some(right.clone())
        );
        assert_eq!(axes_at(&fig, Point::new(1.0, 1.0)), None);
        let twin = right.twinx().expect("rectilinear axes have twins");
        assert_eq!(
            axes_at(&fig, display(&right, 0.5, 0.5)),
            Some(twin),
            "the twin is drawn on top"
        );
    }

    #[test]
    fn images_add_the_value_under_the_pointer() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let values = crate::numeric::Grid::from_rows(vec![vec![0.25, 0.5]]).unwrap();
        let image = ax
            .imshow(values, crate::axes::ImshowOptions::default())
            .unwrap();
        let at = display(&ax, 1.0, 0.0);
        assert_eq!(cursor_text(&ax, at).as_deref(), Some("[0.5000]"));
        assert_eq!(
            cursor_text(&ax, display(&ax, 1.0, 1.0)),
            None,
            "off the image"
        );
        image.set_mouseover(false);
        assert_eq!(cursor_text(&ax, at), None);
    }

    #[test]
    fn coordinates_use_the_screen_resolution() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .expect("finite limits");
        let text = format_coord(&ax, display(&ax, 2.5, 0.25));
        assert_eq!(text, "(x, y) = (2.50, 0.250)");
        let twin = ax.twinx().expect("rectilinear axes have twins");
        twin.set_ylim(0.0, 100.0).expect("finite limits");
        let text = format_coord(&ax, display(&ax, 2.5, 0.25));
        assert_eq!(text, "(x, y) = (2.50, 0.250) | (2.50, 25.0)");
    }

    #[test]
    fn data_points_format_like_the_screen_points_over_them() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .expect("finite limits");
        assert_eq!(ax.format_data(AxisName::X, 2.5).as_deref(), Some("2.50"));
        assert_eq!(ax.format_data(AxisName::Y, 0.25).as_deref(), Some("0.250"));
        assert_eq!(ax.format_coord(2.5, 0.25), "(x, y) = (2.50, 0.250)");
        assert_eq!(
            ax.format_data(AxisName::X, f64::NAN).as_deref(),
            Some("nan")
        );
        let twin = ax.twinx().expect("rectilinear axes have twins");
        twin.set_ylim(0.0, 100.0).expect("finite limits");
        assert_eq!(
            ax.format_coord(2.5, 0.25),
            "(x, y) = (2.50, 0.250) | (2.50, 25.0)"
        );
        let polar = fig.polar_subplot();
        polar.set_rlim(0.0, 2.0).expect("finite limits");
        assert_eq!(
            polar.format_coord(PI / 2.0, 1.0),
            format_coord(&polar, display(&polar, PI / 2.0, 1.0))
        );
        let geo = fig.geo_subplot(crate::geo::GeoProjection::Hammer);
        assert_eq!(geo.format_coord(-PI / 2.0, 0.5), "28.647890°N, 90.000000°W");
    }

    #[test]
    fn geographic_coordinates_in_hemispheres() {
        let fig = Figure::new();
        let geo = fig.geo_subplot(crate::geo::GeoProjection::Hammer);
        let text = format_coord(&geo, display(&geo, -PI / 2.0, 0.5));
        assert_eq!(text, "28.647890°N, 90.000000°W");
    }

    #[test]
    fn polar_coordinates_keep_theta_and_take_custom_formatters() {
        // matplotlib's test_custom_fmt_data, on a laid out figure.
        let fig = Figure::new();
        let polar = fig.polar_subplot();
        let millions: Rc<dyn Formatter> = Rc::new(crate::ticker::FuncFormatter::new(|x, _| {
            format!("${:.1}M", x * 1e-6)
        }));
        polar.set_fmt_data(AxisName::Y, Some(Rc::clone(&millions)));
        let text = polar.format_coord(12.0, 2e7);
        assert!(
            text.starts_with("\u{3b8}=3.819"),
            "θ is not wrapped: {text}"
        );
        assert!(text.ends_with("r=$20.0M"), "{text}");
        polar
            .set_fmt_data(AxisName::X, Some(millions))
            .set_fmt_data(AxisName::Y, None);
        assert_eq!(polar.format_coord(2e5, 1.0), "\u{3b8}=$0.2M, r=1.000");
    }

    #[test]
    fn polar_coordinates_in_turns_and_degrees() {
        let fig = Figure::new();
        let polar = fig.polar_subplot();
        polar.set_rlim(0.0, 2.0).expect("finite limits");
        let text = format_coord(&polar, display(&polar, PI / 2.0, 1.0));
        assert!(text.starts_with("\u{3b8}=0.50"), "{text}");
        assert!(text.contains("(90"), "{text}");
        assert!(text.contains("r=1.00"), "{text}");
    }
}
