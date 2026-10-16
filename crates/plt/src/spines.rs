//! Axes spines: the lines framing the data area (matplotlib.spines).

use std::rc::Rc;

use plt_core::{Affine2D, Path, Point};
use plt_render::CapStyle;

use crate::artist::{Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, RawHandle};
use crate::axes::AxesState;
use crate::colors::Color;
use crate::patches::PatchStyle;
use crate::rc::RcParams;
use crate::transforms::Transform;

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

impl Default for SpinePosition {
    fn default() -> Self {
        Self::Outward(0.0)
    }
}

/// State of one spine.
#[derive(Debug)]
pub(crate) struct SpineArtist {
    base: ArtistBase,
    pub(crate) side: SpineSide,
    pub(crate) style: PatchStyle,
    pub(crate) position: SpinePosition,
    /// Data interval the spine spans; `None` follows the view limits.
    pub(crate) bounds: Option<(f64, f64)>,
    rc: Rc<RcParams>,
}

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
        Self {
            base,
            side,
            style,
            position: SpinePosition::default(),
            bounds: None,
            rc,
        }
    }

    /// The coordinate system of the spine path: axes fractions across the
    /// spine, data along it, adjusted by the spine position
    /// (matplotlib's `get_spine_transform`).
    fn spine_transform(&self, axes: ArtistId) -> Transform {
        let data = Transform::Data(axes);
        let across = Transform::Axes(axes);
        let base = if self.side.is_vertical() {
            Transform::blended(across, data.clone())
        } else {
            Transform::blended(data.clone(), across)
        };
        match self.position {
            SpinePosition::Outward(points) if points == 0.0 => base,
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
        let Some(axes_id) = self.base.parent else {
            return;
        };
        let Some(axes) = ctx.layout().store().get_as::<AxesState>(axes_id) else {
            return;
        };
        let interval = self.bounds.unwrap_or(if self.side.is_vertical() {
            axes.view_lim.interval_y()
        } else {
            axes.view_lim.interval_x()
        });
        let Some(resolved) = ctx.resolve(&self.spine_transform(axes_id)) else {
            return;
        };
        let outline = self.path(interval);
        let (path, affine) = resolved.split_path(&outline);
        let paths = [(path.into_owned(), affine)];
        if let Err(err) =
            crate::patches::draw_patch_paths(ctx, &self.base, &self.style, &self.rc, false, &paths)
        {
            ctx.report(self.base.id(), err);
        }
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

impl Spine {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// Which edge this spine frames.
    pub fn side(&self) -> SpineSide {
        self.raw.with::<SpineArtist, _>(|s| s.side)
    }

    /// Moves the spine (outwards, to an axes fraction or to a data value).
    pub fn set_position(&self, position: SpinePosition) -> &Self {
        self.raw
            .with_mut::<SpineArtist, _>(|s| s.position = position);
        self
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

    /// Sets the spine colour.
    pub fn set_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.raw
            .with_mut::<SpineArtist, _>(|s| s.style.edgecolor = Some(color));
        self
    }

    /// Sets the spine width in points.
    pub fn set_linewidth(&self, width: f64) -> &Self {
        self.raw
            .with_mut::<SpineArtist, _>(|s| s.style.linewidth = Some(width));
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
}

#[cfg(test)]
mod tests {
    use plt_core::Bbox;

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
        ax.set_xlim(-1.0, 1.0);
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
}
