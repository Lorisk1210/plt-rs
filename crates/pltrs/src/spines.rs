//! Axes spines: the lines framing the data area (matplotlib.spines).

mod made;
mod patch;
mod style;

use std::rc::Rc;

use pltrs_core::{Affine2D, Bbox, Path, Point};
use pltrs_render::CapStyle;

use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, LayoutContext, RawHandle, TypedHandle,
};
use crate::axes::AxesState;
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::patches::PatchStyle;
use crate::polar::PolarView;
use crate::projections::OUTLINE_SPINE;
use crate::rc::RcParams;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, Transform, program_calls, unit_to_bbox};

pub use patch::SpinePatch;

/// Which edge of the axes a spine frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpineSide {
    /// The left edge (frames the y axis).
    Left,
    /// The right edge.
    Right,
    /// The bottom edge (frames the x axis).
    Bottom,
    /// The top edge.
    Top,
}

pltrs_core::archive_enum!(SpineSide {
    Left,
    Right,
    Bottom,
    Top,
});

impl SpineSide {
    /// All four sides in matplotlib's order.
    pub const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Bottom, Self::Top];

    /// matplotlib's name (`"left"`, ...).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Top => "top",
        }
    }

    /// The side matplotlib calls `name`, if any.
    ///
    /// ```
    /// use pltrs::spines::SpineSide;
    /// assert_eq!(SpineSide::from_name("top"), Some(SpineSide::Top));
    /// assert_eq!(SpineSide::from_name("middle"), None);
    /// ```
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|side| side.name() == name)
    }

    /// The side of axis `name` at its low (bottom, left) or high (top,
    /// right) end.
    pub const fn of(name: AxisName, high: bool) -> Self {
        match (name, high) {
            (AxisName::X, false) => Self::Bottom,
            (AxisName::X, true) => Self::Top,
            (AxisName::Y, false) => Self::Left,
            (AxisName::Y, true) => Self::Right,
        }
    }

    /// The axis that can be drawn along this side.
    pub const fn axis(self) -> AxisName {
        if self.is_vertical() {
            AxisName::Y
        } else {
            AxisName::X
        }
    }

    /// Whether this is the high side (top or right) of its axis.
    pub const fn is_high(self) -> bool {
        matches!(self, Self::Right | Self::Top)
    }

    const fn is_vertical(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    /// Default position across the axes (0 or 1 in axes fractions).
    const fn default_fraction(self) -> f64 {
        match self {
            Self::Left | Self::Bottom => 0.0,
            Self::Right | Self::Top => 1.0,
        }
    }
}

/// Where a spine sits (matplotlib's `set_position`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpinePosition {
    /// Moved outwards from the axes edge by this many points.
    Outward(f64),
    /// At this fraction of the axes (`'center'` is `Axes(0.5)`).
    Axes(f64),
    /// At this data coordinate (`'zero'` is `Data(0.0)`).
    Data(f64),
}

pltrs_core::archive_enum!(SpinePosition {
    Outward(v0),
    Axes(v0),
    Data(v0),
});

impl Default for SpinePosition {
    fn default() -> Self {
        Self::Outward(0.0)
    }
}

/// State of one spine.
#[derive(Clone, Debug)]
pub(crate) struct SpineArtist {
    base: ArtistBase,
    /// The side the spine frames, or for a spine of no side
    /// ([`Axes::add_spine`](crate::axes::Axes::add_spine)) the side it
    /// stands against.
    pub(crate) side: SpineSide,
    /// Whether it is one of the four spines of the sides, which the axes
    /// draw their axes on; not a spine of no side.
    frames_side: bool,
    pub(crate) style: PatchStyle,
    pub(crate) position: SpinePosition,
    /// Data interval the spine spans; `None` follows the view limits.
    pub(crate) bounds: Option<(f64, f64)>,
    /// The shape the spine is drawn with.
    pub(crate) patch: SpinePatch,
    /// Whether the program made the spine and nothing placed it yet
    /// ([`Spine::leave_unplaced`]): matplotlib's spine then still has the
    /// transform it was made with, `transData`, which only its position
    /// replaces, so it is drawn in data coordinates. An axes places the
    /// spines of its sides as it makes the ticks of its axes, so a spine
    /// that takes the place of one before the axes was ever drawn counts
    /// as placed ([`Axes::replace_spine`](crate::axes::Axes::replace_spine)).
    pub(crate) unplaced: bool,
    rc: Rc<RcParams>,
}

pltrs_core::archive_struct!(SpineArtist {
    base,
    side,
    frames_side,
    style,
    position,
    bounds,
    patch,
    unplaced,
    rc
});

impl SpineArtist {
    /// A spine styled from `rc` (`axes.edgecolor`, `axes.linewidth`,
    /// projecting caps, z-order 2.5).
    pub(crate) fn new(side: SpineSide, axes: ArtistId, rc: Rc<RcParams>) -> Self {
        let mut style = PatchStyle::from_rc(&rc);
        style.facecolor = Some(Color::none());
        style.edgecolor = Some(rc.axes_edgecolor.clone());
        style.linewidth = Some(rc.axes_linewidth);
        style.capstyle = CapStyle::Projecting;
        let visible = match side {
            SpineSide::Left => rc.axes_spines_left,
            SpineSide::Right => rc.axes_spines_right,
            SpineSide::Bottom => rc.axes_spines_bottom,
            SpineSide::Top => rc.axes_spines_top,
        };
        let mut base = ArtistBase::with_zorder(2.5);
        base.visible = visible;
        base.parent = Some(axes);
        let mut spine = Self {
            base,
            side,
            frames_side: true,
            style,
            position: SpinePosition::default(),
            bounds: None,
            patch: SpinePatch::Line,
            unplaced: false,
            rc,
        };
        spine.place(SpinePosition::default());
        spine
    }

    /// Whether the spine is drawn in data coordinates: a line the program
    /// made that nothing placed nor gave a transform (see
    /// [`SpineArtist::unplaced`]).
    fn in_data_coordinates(&self, axes: ArtistId) -> bool {
        self.unplaced
            && matches!(self.patch, SpinePatch::Line)
            && self.base.transform.as_ref() == Some(&self.spine_transform(axes))
    }

    /// Whether it is one of the four spines of the sides (not a spine of
    /// no side), which an axis is drawn on.
    pub(crate) fn frames_side(&self) -> bool {
        self.frames_side
    }

    /// Moves the spine to `position`, which gives it the transform of
    /// that position, whatever transform it was given (matplotlib's
    /// `set_position` sets the transform to `get_spine_transform()`).
    fn place(&mut self, position: SpinePosition) {
        self.unplaced = false;
        self.position = position;
        self.base.transform = self.base.parent.map(|axes| self.spine_transform(axes));
    }

    /// The coordinate system of the spine path: axes fractions across the
    /// spine, data along it, adjusted by the spine position
    /// (matplotlib's `get_spine_transform`).
    pub(crate) fn spine_transform(&self, axes: ArtistId) -> Transform {
        let data = Transform::Data(axes);
        let across = Transform::Axes(axes);
        let base = if self.side.is_vertical() {
            Transform::blended(across, data.clone())
        } else {
            Transform::blended(data.clone(), across)
        };
        match self.position {
            SpinePosition::Outward(0.0) => base,
            SpinePosition::Outward(points) => {
                let (dx, dy) = match self.side {
                    SpineSide::Left => (-points, 0.0),
                    SpineSide::Right => (points, 0.0),
                    SpineSide::Bottom => (0.0, -points),
                    SpineSide::Top => (0.0, points),
                };
                base.offset_points(dx, dy)
            }
            SpinePosition::Axes(fraction) => {
                let pin = if self.side.is_vertical() {
                    Affine2D::from_values(0.0, 0.0, 0.0, 1.0, fraction, 0.0)
                } else {
                    Affine2D::from_values(1.0, 0.0, 0.0, 0.0, 0.0, fraction)
                };
                Transform::Affine(pin).then(base)
            }
            SpinePosition::Data(value) => {
                let amount = value - self.side.default_fraction();
                if self.side.is_vertical() {
                    Transform::blended(
                        Transform::Affine(Affine2D::translation(amount, 0.0)).then(data.clone()),
                        data,
                    )
                } else {
                    Transform::blended(
                        data.clone(),
                        Transform::Affine(Affine2D::translation(0.0, amount)).then(data),
                    )
                }
            }
        }
    }

    /// The two-point spine path spanning `(low, high)` along the spine.
    fn path(&self, (low, high): (f64, f64)) -> Path {
        let at = self.side.default_fraction();
        let (start, end) = if self.side.is_vertical() {
            (Point::new(at, low), Point::new(at, high))
        } else {
            (Point::new(low, at), Point::new(high, at))
        };
        Path::polyline(vec![start, end])
    }
}

impl Artist for SpineArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        // The program's scales answer from the calls matplotlib's draw
        // makes (see `axes::transform_sites`).
        let Some(outline) = program_calls::quietly(|| self.outline_through(ctx.layout())) else {
            return;
        };
        // The program's transform runs with the figure free.
        ctx.free_for(outline.transform());
        let paths = [program_calls::quietly(|| outline.split())];
        if let Err(err) =
            crate::patches::draw_patch_paths(ctx, &self.base, &self.style, &self.rc, false, &paths)
        {
            ctx.report(self.base.id(), err);
        }
    }

    /// The spine's extent widened by the tick marks of a visible axis
    /// drawn on it (matplotlib's `Spine.get_window_extent`).
    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        crate::axes::spine_extent_with_ticks(*ctx, self).or_else(|| self.line_extent(*ctx))
    }
}

impl SpineArtist {
    /// The extent of the spine line alone, in display pixels.
    pub(crate) fn line_extent(&self, ctx: LayoutContext<'_>) -> Option<Bbox> {
        program_calls::quietly(|| Some(self.outline_through(ctx)?.extent()))
    }
}

impl SpineArtist {
    /// The spine path (non-affine part applied) and its display affine,
    /// spanning the bounds or else the parent axes' view interval (or the
    /// interval a custom projection gives).
    fn display_path(&self, ctx: LayoutContext<'_>) -> Option<(Path, Affine2D)> {
        Some(self.outline_through(ctx)?.split())
    }

    /// The spine path in its coordinates and the transform drawing it (see
    /// [`display_path`](Self::display_path)), which a draw applies once
    /// it let the figure go.
    pub(crate) fn outline_through(&self, ctx: LayoutContext<'_>) -> Option<OutlineThrough> {
        let axes_id = self.base.parent?;
        if let SpinePatch::Path(path) = &self.patch {
            // The program's path through the transform it gave the spine.
            let transform = self
                .base
                .transform
                .clone()
                .unwrap_or(Transform::Data(axes_id));
            return Some(OutlineThrough::new(path.clone(), ctx.resolve(&transform)?));
        }
        if let Some(outline) = self.patch.outline() {
            let resolved = ctx.resolve(&Transform::Axes(axes_id))?;
            return Some(OutlineThrough::new(outline, resolved));
        }
        let axes = ctx.store().get_as::<AxesState>(axes_id)?;
        if let Some(polar) = axes.polar_view() {
            return self.polar_path(&polar, axes_id, ctx);
        }
        if !axes.projection.is_rectangular() {
            // matplotlib's single spine around a map or another outline.
            if self.side != OUTLINE_SPINE {
                return None;
            }
            let frame = unit_to_bbox(&ctx.axes_extent(axes_id)?);
            return Some(OutlineThrough::new(
                axes.patch_path(),
                ResolvedTransform::affine(frame),
            ));
        }
        let interval = self
            .bounds
            .unwrap_or_else(|| axes.spine_interval(self.side));
        // The transform of its position, or one the program gave it; the
        // data's for a spine of the program's that has neither.
        let transform = if self.in_data_coordinates(axes_id) {
            Transform::Data(axes_id)
        } else {
            (self.base.transform.clone()).unwrap_or_else(|| self.spine_transform(axes_id))
        };
        Some(OutlineThrough::new(
            self.path(interval),
            ctx.resolve(&transform)?,
        ))
    }
}

/// A spine's path in its own coordinates and the transform drawing it.
pub(crate) struct OutlineThrough {
    path: Path,
    transform: ResolvedTransform,
}

impl OutlineThrough {
    fn new(path: Path, transform: ResolvedTransform) -> Self {
        Self { path, transform }
    }

    /// The transform drawing the path.
    pub(crate) fn transform(&self) -> &ResolvedTransform {
        &self.transform
    }

    /// The path with the transform's non-affine part applied, and the
    /// affine part left for the renderer.
    fn split(&self) -> (Path, Affine2D) {
        let (path, affine) = self.transform.split_path(&self.path);
        (path.into_owned(), affine)
    }

    /// The extent of the path in display pixels.
    pub(crate) fn extent(&self) -> Bbox {
        let (path, affine) = self.split();
        path.transformed(&affine).extents()
    }
}

impl SpineArtist {
    /// The spine of polar axes: the outer (`Top`) and inner (`Bottom`)
    /// arcs, and the radial lines at the start (`Left`) and end (`Right`)
    /// of a sector (matplotlib's `PolarAxes._gen_axes_spines`). `None`
    /// when the geometry hides it.
    fn polar_path(
        &self,
        polar: &PolarView<'_>,
        axes_id: ArtistId,
        ctx: LayoutContext<'_>,
    ) -> Option<OutlineThrough> {
        if !polar.spine_shown(self.side) {
            return None;
        }
        let view = polar.view();
        if !self.side.is_vertical() {
            let bounds = self.bounds.unwrap_or(view.interval_x());
            let (path, affine) = polar.arc_spine(self.side, bounds, &ctx.axes_extent(axes_id)?);
            return Some(OutlineThrough::new(path, ResolvedTransform::affine(affine)));
        }
        let (low, high) = self.bounds.unwrap_or(view.interval_y());
        let at = self.side.default_fraction();
        let line = Path::polyline(vec![Point::new(at, low), Point::new(at, high)]);
        let resolved = ResolvedTransform::affine(polar.theta_fraction())
            .then(ctx.resolve(&Transform::Data(axes_id))?);
        Some(OutlineThrough::new(line, resolved))
    }
}

/// A handle to one spine of an axes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spine {
    raw: RawHandle,
}

impl ArtistHandle for Spine {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for Spine {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<SpineArtist>().then(|| Self::from_raw(raw.clone()))
    }
}

impl Spine {
    /// The handle of the spine behind `raw`.
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// Which edge this spine frames (for a spine of no side, see
    /// [`Axes::add_spine`](crate::axes::Axes::add_spine), the edge it
    /// stands against).
    pub fn side(&self) -> SpineSide {
        self.raw.with::<SpineArtist, _>(|s| s.side)
    }

    /// Whether this is one of the four spines of the sides of its axes,
    /// not a spine of no side
    /// ([`Axes::add_spine`](crate::axes::Axes::add_spine)).
    pub fn frames_side(&self) -> bool {
        self.raw.with::<SpineArtist, _>(SpineArtist::frames_side)
    }

    /// Moves the spine (outwards, to an axes fraction or to a data value);
    /// a line spine is then drawn in the coordinates of its position
    /// again, whatever transform it was given (matplotlib's
    /// `set_position`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpinePosition;
    /// use pltrs::transforms::Transform;
    /// let ax = Figure::new().subplot();
    /// let left = &ax.spines().left;
    /// left.set_transform(Transform::Display);
    /// left.set_position(SpinePosition::Axes(0.5));
    /// assert_eq!(left.transform(), left.spine_transform());
    /// ```
    pub fn set_position(&self, position: SpinePosition) -> &Self {
        self.raw.with_mut::<SpineArtist, _>(|s| s.place(position));
        self
    }

    /// Leaves the spine, a line the program made, as matplotlib's is until
    /// something places it: with the transform it was made with,
    /// `transData`, so that it is drawn in data coordinates, until
    /// [`set_position`](Self::set_position) (or the axes, making its ticks
    /// for the first time with the spine on one of its sides) places it.
    /// A spine given a transform of its own keeps that.
    pub fn leave_unplaced(&self) -> &Self {
        self.raw.with_mut::<SpineArtist, _>(|s| s.unplaced = true);
        self
    }

    /// Whether the spine was left unplaced ([`leave_unplaced`](Self::leave_unplaced))
    /// and nothing placed it since.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// use pltrs::spines::{SpinePosition, SpineSide};
    /// let fig = Figure::with_size(6.4, 4.8, 100.0);
    /// let ax = fig.subplot();
    /// ax.set_xlim(0.0, 4.0)?;
    /// let extra = ax.new_spine(SpineSide::Right);
    /// ax.add_spine(&extra);
    /// assert!(!extra.is_unplaced());
    /// extra.leave_unplaced();
    /// assert!(extra.is_unplaced());
    /// let frame = ax.window_extent().expect("laid out");
    /// let at_one = extra.window_extent().expect("drawn").x0();
    /// let quarter = frame.x0() + frame.width() / 4.0;
    /// assert!((at_one - quarter).abs() < 1e-9, "at x = 1 of the data");
    /// extra.set_position(SpinePosition::Outward(0.0));
    /// assert!(!extra.is_unplaced());
    /// let placed = extra.window_extent().expect("drawn").x0();
    /// assert!((placed - frame.x1()).abs() < 1e-9, "on the right side");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn is_unplaced(&self) -> bool {
        self.raw.with::<SpineArtist, _>(|s| s.unplaced)
    }

    /// The spine position.
    pub fn position(&self) -> SpinePosition {
        self.raw.with::<SpineArtist, _>(|s| s.position)
    }

    /// Limits the spine to a data interval (`None` spans the view).
    pub fn set_bounds(&self, bounds: Option<(f64, f64)>) -> &Self {
        self.raw.with_mut::<SpineArtist, _>(|s| s.bounds = bounds);
        self
    }

    /// Limits the spine to the data interval from `low` to `high`, an end
    /// given as `None` keeping its current value: the bound set before, or
    /// else the view limit of the spine's axis at the time of the call
    /// (matplotlib's `set_bounds(low, high)`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.set_xlim(0.0, 10.0)?;
    /// let bottom = &ax.spines().bottom;
    /// bottom.set_bound_ends(Some(2.0), None);
    /// assert_eq!(bottom.bounds(), Some((2.0, 10.0)));
    /// bottom.set_bound_ends(None, Some(5.0));
    /// assert_eq!(bottom.bounds(), Some((2.0, 5.0)));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_bound_ends(&self, low: Option<f64>, high: Option<f64>) -> &Self {
        let current = self.bounds().or_else(|| {
            let ax = self.axes()?;
            Some(match self.side().axis() {
                AxisName::X => ax.xlim(),
                AxisName::Y => ax.ylim(),
            })
        });
        if let Some((old_low, old_high)) = current {
            self.set_bounds(Some((low.unwrap_or(old_low), high.unwrap_or(old_high))));
        }
        self
    }

    /// The data interval the spine is limited to, `None` while it spans
    /// the view (matplotlib's `get_bounds`).
    pub fn bounds(&self) -> Option<(f64, f64)> {
        self.raw.with::<SpineArtist, _>(|s| s.bounds)
    }

    /// The display box of the spine line alone in the figure's current
    /// layout (laid out first), without its ticks (matplotlib's
    /// `get_transform().transform_path(get_path()).get_extents()`).
    pub fn line_extent(&self) -> Option<Bbox> {
        Some(self.display_path()?.extents())
    }

    /// The spine line in display pixels in the figure's current layout
    /// (laid out first), `None` where the geometry hides it (matplotlib's
    /// `get_transform().transform_path(get_path())`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::with_size(4.0, 3.0, 100.0);
    /// let ax = fig.subplot();
    /// let left = ax.spines().left.display_path().expect("a drawn spine");
    /// let frame = ax.window_extent().expect("an extent");
    /// assert!((left.extents().x0() - frame.x0()).abs() < 1e-9);
    /// ```
    pub fn display_path(&self) -> Option<Path> {
        let state = self.raw.figure_state().prepared();
        let ctx = LayoutContext::new(&state);
        let spine = state.store.get_as::<SpineArtist>(self.raw.id())?;
        let (path, affine) = spine.display_path(ctx)?;
        Some(path.transformed(&affine))
    }

    /// Sets the spine colour.
    pub fn set_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.raw
            .with_mut::<SpineArtist, _>(|s| s.style.edgecolor = Some(color));
        self
    }

    /// The spine colour (matplotlib's `get_edgecolor`).
    pub fn edgecolor(&self) -> Color {
        self.raw.with::<SpineArtist, _>(|s| {
            s.style
                .edgecolor
                .clone()
                .unwrap_or_else(|| s.rc.axes_edgecolor.clone())
        })
    }

    /// Sets the spine width in points.
    pub fn set_linewidth(&self, width: f64) -> &Self {
        self.raw
            .with_mut::<SpineArtist, _>(|s| s.style.linewidth = Some(width));
        self
    }

    /// Sets the spine line style (solid, dashed, ...).
    pub fn set_linestyle(&self, style: LineStyle) -> &Self {
        self.raw
            .with_mut::<SpineArtist, _>(|s| s.style.linestyle = style);
        self
    }

    /// Sets how the spine ends are drawn (projecting by default).
    pub fn set_capstyle(&self, capstyle: CapStyle) -> &Self {
        self.raw
            .with_mut::<SpineArtist, _>(|s| s.style.capstyle = capstyle);
        self
    }
}

/// The four spines of an axes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spines {
    /// Left spine.
    pub left: Spine,
    /// Right spine.
    pub right: Spine,
    /// Bottom spine.
    pub bottom: Spine,
    /// Top spine.
    pub top: Spine,
}

impl Spines {
    /// The spine on `side`.
    pub fn get(&self, side: SpineSide) -> &Spine {
        match side {
            SpineSide::Left => &self.left,
            SpineSide::Right => &self.right,
            SpineSide::Bottom => &self.bottom,
            SpineSide::Top => &self.top,
        }
    }

    /// All four spines.
    pub fn iter(&self) -> impl Iterator<Item = &Spine> {
        [&self.left, &self.right, &self.bottom, &self.top].into_iter()
    }

    /// Shows or hides all four spines (matplotlib's
    /// `ax.spines[:].set_visible(visible)`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.spines().set_visible(false);
    /// assert!(ax.spines().iter().all(|spine| !spine.visible()));
    /// ```
    pub fn set_visible(&self, visible: bool) -> &Self {
        for spine in self.iter() {
            spine.set_visible(visible);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Bbox;

    use super::*;
    use crate::figure::Figure;
    use crate::testing::{Call, RecordingRenderer};

    fn spine_segments(fig: &Figure) -> Vec<Vec<Point>> {
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        renderer
            .calls
            .into_iter()
            .filter_map(|call| match call {
                Call::Path {
                    points, fill: None, ..
                } if points.len() == 2 => Some(points),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn default_spines_frame_the_axes() {
        let fig = Figure::new();
        fig.subplot();
        let segments = spine_segments(&fig);
        assert_eq!(segments.len(), 4);
        let left = Bbox::from_points(segments[0].iter().copied());
        assert!(
            (left.x0() - 80.0).abs() < 1e-9
                && (left.y0() - 52.8).abs() < 1e-9
                && (left.y1() - 422.4).abs() < 1e-9
        );
    }

    #[test]
    fn positions_move_spines() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(-1.0, 1.0).expect("finite limits");
        let spines = ax.spines();
        spines.left.set_position(SpinePosition::Data(0.0));
        spines.bottom.set_position(SpinePosition::Outward(10.0));
        spines.top.set_visible(false);
        let segments = spine_segments(&fig);
        assert_eq!(segments.len(), 3);
        assert!(
            (segments[0][0].x - 328.0).abs() < 1e-9,
            "left spine at x = 0"
        );
        assert!(
            (segments[2][0].y - (52.8 - 10.0 * 100.0 / 72.0)).abs() < 1e-9,
            "bottom spine 10pt out"
        );
    }

    #[test]
    fn missing_bound_ends_take_the_autoscaled_view() {
        // matplotlib's test_spine_set_bounds_with_none.
        let fig = Figure::new();
        let ax = fig.subplot();
        let x = crate::numeric::linspace(0.0, 10.0, 100);
        let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
        ax.plot(x, y).unwrap();
        let (xlim, ylim) = (ax.xlim(), ax.ylim());
        let spines = ax.spines();
        assert_eq!(spines.bottom.bounds(), None);
        spines.bottom.set_bound_ends(Some(2.0), None);
        spines.left.set_bound_ends(None, None);
        assert_eq!(spines.bottom.bounds(), Some((2.0, xlim.1)));
        assert_eq!(spines.left.bounds(), Some(ylim));
    }

    #[test]
    fn line_extents_leave_out_the_ticks() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let left = &ax.spines().left;
        let line = left.line_extent().expect("a drawn spine");
        assert!((line.x0() - 80.0).abs() < 1e-9 && line.width() == 0.0);
        let with_ticks = left.window_extent().expect("a drawn spine");
        assert!(with_ticks.x0() < line.x0());
    }

    #[test]
    fn spines_take_a_line_style() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.spines()
            .left
            .set_linestyle(LineStyle::Dashed)
            .set_linewidth(2.0);
        let style = ax
            .spines()
            .left
            .raw()
            .with::<SpineArtist, _>(|spine| spine.style.clone());
        assert_eq!(style.linestyle, LineStyle::Dashed);
        assert_eq!(style.linewidth, Some(2.0));
    }

    #[test]
    fn spines_project_their_caps_unless_told_otherwise() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let cap = |spine: &Spine| spine.raw().with::<SpineArtist, _>(|s| s.style.capstyle);
        assert_eq!(cap(&ax.spines().left), CapStyle::Projecting);
        ax.spines().left.set_capstyle(CapStyle::Round);
        assert_eq!(cap(&ax.spines().left), CapStyle::Round);
        assert_eq!(cap(&ax.spines().right), CapStyle::Projecting);
    }
}
