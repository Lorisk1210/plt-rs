//! The arrows drawn along streamlines: straight `FancyArrowPatch`es of the
//! `ArrowStyle._Curve` family, shrunk by 2 points at both ends (matplotlib's
//! `ConnectionStyle.Arc3`, `_Base._clip` and `_Curve.transmute`).
//!
//! This is a self-contained helper for streamplot; it covers only what
//! streamplot's arrows use (a straight connection, no patches to clip to,
//! solid unhatched edges).

use std::rc::Rc;
use std::str::FromStr;

use plt_core::bezier::{inside_circle, split_path_inout};
use plt_core::{Affine2D, Path, PathCode, Point};
use plt_render::{CapStyle, JoinStyle};

use crate::artist::{Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, RawHandle};
use crate::colors::Color;
use crate::rc::RcParams;
use crate::transforms::{ResolvedTransform, Transform, TransformContext};

/// An arrow head of a [`StreamArrowStyle`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StreamArrowHead {
    /// Two strokes (`>`).
    Open,
    /// A filled triangle (`|>`).
    Filled,
}

/// The heads of streamplot arrows: matplotlib's line-like arrow styles
/// `"-"`, `"->"`, `"-|>"`, `"<-"`, `"<|-"`, `"<->"` and `"<|-|>"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StreamArrowStyle {
    /// The head at the tail end.
    pub begin: Option<StreamArrowHead>,
    /// The head at the tip end.
    pub end: Option<StreamArrowHead>,
}

impl Default for StreamArrowStyle {
    /// `"-|>"`.
    fn default() -> Self {
        Self {
            begin: None,
            end: Some(StreamArrowHead::Filled),
        }
    }
}

/// An arrow style name that is not one of [`StreamArrowStyle`]'s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownArrowStyle(pub String);

impl std::fmt::Display for UnknownArrowStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown streamplot arrow style {:?}", self.0)
    }
}

impl std::error::Error for UnknownArrowStyle {}

impl FromStr for StreamArrowStyle {
    type Err = UnknownArrowStyle;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let unknown = || UnknownArrowStyle(name.to_owned());
        let (begin, end) = name.split_once('-').ok_or_else(unknown)?;
        let begin = match begin {
            "" => None,
            "<" => Some(StreamArrowHead::Open),
            "<|" => Some(StreamArrowHead::Filled),
            _ => return Err(unknown()),
        };
        let end = match end {
            "" => None,
            ">" => Some(StreamArrowHead::Open),
            "|>" => Some(StreamArrowHead::Filled),
            _ => return Err(unknown()),
        };
        Ok(Self { begin, end })
    }
}

/// Head length relative to the mutation scale.
const HEAD_LENGTH: f64 = 0.4;
/// Head width relative to the mutation scale.
const HEAD_WIDTH: f64 = 0.2;
/// How far the arrow is shortened at each end, in points.
const SHRINK: f64 = 2.0;
/// Precision of the shortening, in pixels.
const SHRINK_TOLERANCE: f64 = 0.01;

/// How a streamplot arrow looks.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ArrowLook {
    pub(crate) style: StreamArrowStyle,
    /// Scale of the heads in points.
    pub(crate) mutation_scale: f64,
    /// Stroke width in points.
    pub(crate) linewidth: f64,
    /// Face and edge colour.
    pub(crate) color: Color,
}

/// State of one streamplot arrow from `tail` to `head` (in the artist
/// transform's coordinates).
#[derive(Debug)]
pub(crate) struct StreamArrowArtist {
    base: ArtistBase,
    tail: Point,
    head: Point,
    look: ArrowLook,
    rc: Rc<RcParams>,
}

impl StreamArrowArtist {
    pub(crate) fn new(
        (tail, head): (Point, Point),
        look: ArrowLook,
        transform: Transform,
        zorder: f64,
    ) -> Self {
        let mut base = ArtistBase::with_zorder(zorder);
        base.transform = Some(transform);
        Self {
            base,
            tail,
            head,
            look,
            rc: crate::rc::params(),
        }
    }

    /// The paths in display coordinates, each with whether it is filled.
    fn display_paths(&self, ctx: &DrawContext<'_>) -> Option<Vec<(Path, bool)>> {
        let transform = ctx.resolve(self.base.transform.as_ref()?)?;
        Some(self.paths_through(&transform, ctx.points_to_pixels(1.0)))
    }

    /// The paths with the ends placed by `transform` (to display) and the
    /// sizes at `dpi_cor` pixels per point.
    fn paths_through(&self, transform: &ResolvedTransform, dpi_cor: f64) -> Vec<(Path, bool)> {
        let a = transform.transform_point(self.tail);
        let b = transform.transform_point(self.head);
        let connection = shrunk(arc3(a, b), (a, b), SHRINK * dpi_cor);
        transmute(
            &connection,
            self.look.style,
            self.look.mutation_scale * dpi_cor,
            self.look.linewidth * dpi_cor,
        )
    }

    /// The points bounding the arrow in the data coordinates of `axes`
    /// (the ends and axis extrema of its outline), laid out in display
    /// coordinates in the axes' view of the moment, or `None` when the
    /// arrow is not placed in data coordinates (matplotlib's
    /// `_update_patch_limits` on a `FancyArrowPatch`, whose path goes
    /// through display coordinates). Like a patch never drawn, it takes
    /// its sizes in points for pixels (`_dpi_cor` is 1 until the first
    /// draw).
    pub(crate) fn outline_data_points(
        &self,
        ctx: &dyn TransformContext,
        axes: ArtistId,
    ) -> Option<Vec<Point>> {
        let transform = self.base.transform.as_ref()?;
        if transform.data_directions(axes) == (false, false) {
            return None;
        }
        let to_display = transform.resolve(ctx).ok()?;
        let to_data = ctx.data_transform(axes)?.inverted()?;
        let points = self
            .paths_through(&to_display, 1.0)
            .iter()
            .flat_map(|(path, _)| path.beziers().collect::<Vec<_>>())
            .flat_map(|curve| {
                let mut ts = vec![0.0];
                ts.extend(curve.axis_aligned_extrema());
                ts.push(1.0);
                ts.into_iter().map(move |t| curve.eval(t))
            })
            .map(|p| to_data.transform_point(p))
            .collect();
        Some(points)
    }
}

/// A straight quadratic Bézier from `a` to `b` (matplotlib's
/// `ConnectionStyle.Arc3` with `rad = 0`).
fn arc3(a: Point, b: Point) -> Path {
    let middle = a.midpoint(b);
    Path::new(
        vec![a, middle, b],
        vec![PathCode::MoveTo, PathCode::Curve3, PathCode::Curve3],
    )
    .expect("three vertices with three codes starting with a move")
}

/// `path` with the parts within `shrink` pixels of `a` and of `b` cut off
/// (left whole where it does not leave a circle), as matplotlib's
/// `ConnectionStyle._Base._clip`.
fn shrunk(path: Path, (a, b): (Point, Point), shrink: f64) -> Path {
    let path = match split_path_inout(&path, inside_circle(a, shrink), SHRINK_TOLERANCE, false) {
        Ok((_, outside)) => outside,
        Err(_) => path,
    };
    match split_path_inout(&path, inside_circle(b, shrink), SHRINK_TOLERANCE, false) {
        Ok((inside, _)) => inside,
        Err(_) => path,
    }
}

/// The shaft and heads along `path` (matplotlib's `_Curve.transmute`),
/// each with whether it is filled.
fn transmute(
    path: &Path,
    style: StreamArrowStyle,
    mutation_size: f64,
    linewidth: f64,
) -> Vec<(Path, bool)> {
    let vertices = path.vertices();
    let n = vertices.len();
    let head = HeadGeometry::new(mutation_size, linewidth);
    let begin = style
        .begin
        .filter(|_| vertices[0] != vertices[1])
        .map(|kind| (kind, head.wedge(vertices[1], vertices[0])));
    let end = style
        .end
        .filter(|_| vertices[n - 2] != vertices[n - 1])
        .map(|kind| (kind, head.wedge(vertices[n - 2], vertices[n - 1])));
    let pad = |wedge: Option<&(StreamArrowHead, Wedge)>| wedge.map_or((0.0, 0.0), |(_, w)| w.pad);
    let (begin_pad, end_pad) = (pad(begin.as_ref()), pad(end.as_ref()));
    let mut shaft_vertices = vertices.to_vec();
    shaft_vertices[0] = Point::new(vertices[0].x + begin_pad.0, vertices[0].y + begin_pad.1);
    shaft_vertices[n - 1] =
        Point::new(vertices[n - 1].x + end_pad.0, vertices[n - 1].y + end_pad.1);
    let shaft = Path::new(shaft_vertices, path.resolved_codes())
        .expect("the shaft keeps the connection's codes");
    let mut paths = vec![(shaft, false)];
    paths.extend(
        [begin, end]
            .into_iter()
            .flatten()
            .map(|(kind, wedge)| wedge.path(kind)),
    );
    paths
}

/// The proportions of an arrow head in pixels.
struct HeadGeometry {
    distance: f64,
    cos: f64,
    sin: f64,
    linewidth: f64,
}

/// One arrow head: its three corners and how far its tip is pulled back
/// so the round-joined stroke does not overshoot the end point.
struct Wedge {
    corners: [Point; 3],
    pad: (f64, f64),
}

impl HeadGeometry {
    fn new(mutation_size: f64, linewidth: f64) -> Self {
        let length = HEAD_LENGTH * mutation_size;
        let width = HEAD_WIDTH * mutation_size;
        let distance = length.hypot(width);
        Self {
            distance,
            cos: length / distance,
            sin: width / distance,
            linewidth,
        }
    }

    /// The head at `to` of the segment from `from` (matplotlib's
    /// `_get_arrow_wedge`).
    fn wedge(&self, from: Point, to: Point) -> Wedge {
        let (dx, dy) = (from.x - to.x, from.y - to.y);
        let distance = match dx.hypot(dy) {
            0.0 => 1.0,
            d => d,
        };
        let pad_projected = 0.5 * self.linewidth / self.sin;
        let pad = (pad_projected * dx / distance, pad_projected * dy / distance);
        let (dx, dy) = (dx / distance * self.distance, dy / distance * self.distance);
        let (cos, sin) = (self.cos, self.sin);
        let (dx1, dy1) = (cos * dx + sin * dy, -sin * dx + cos * dy);
        let (dx2, dy2) = (cos * dx - sin * dy, sin * dx + cos * dy);
        let tip = Point::new(to.x + pad.0, to.y + pad.1);
        Wedge {
            corners: [
                Point::new(tip.x + dx1, tip.y + dy1),
                tip,
                Point::new(tip.x + dx2, tip.y + dy2),
            ],
            pad,
        }
    }
}

impl Wedge {
    /// The head as a path: a closed triangle when filled, two strokes
    /// otherwise.
    fn path(&self, kind: StreamArrowHead) -> (Path, bool) {
        match kind {
            StreamArrowHead::Filled => (
                Path::closed([&self.corners[..], &[Point::ORIGIN]].concat()),
                true,
            ),
            StreamArrowHead::Open => (Path::polyline(self.corners.to_vec()), false),
        }
    }
}

impl Artist for StreamArrowArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let Some(paths) = self.display_paths(ctx) else {
            ctx.report(self.base.id(), "its coordinate system no longer exists");
            return;
        };
        let color = match self.look.color.resolve_alpha(self.base.alpha) {
            Ok(color) => color,
            Err(err) => return ctx.report(self.base.id(), err),
        };
        let mut gc = ctx.base_gc(&self.base);
        gc.linewidth = if color.is_transparent() {
            0.0
        } else {
            self.look.linewidth
        };
        gc.cap = CapStyle::Round;
        gc.join = JoinStyle::Round;
        gc.antialiased = self.rc.patch_antialiased;
        gc.foreground = color;
        let face = (!color.is_transparent()).then_some(color);
        ctx.renderer().open_group("patch", self.base.gid.as_deref());
        for (path, filled) in &paths {
            let fill = if *filled { face } else { None };
            ctx.renderer()
                .draw_path(&gc, path, &Affine2D::IDENTITY, fill);
        }
        ctx.renderer().close_group("patch");
    }
}

/// A handle to one arrow of a streamplot (matplotlib's `FancyArrowPatch`
/// in `StreamplotSet.arrows`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamArrow {
    raw: RawHandle,
}

impl ArtistHandle for StreamArrow {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl StreamArrow {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// The tail and head of the arrow in data coordinates.
    pub fn positions(&self) -> (Point, Point) {
        self.raw
            .with::<StreamArrowArtist, _>(|arrow| (arrow.tail, arrow.head))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn outlines_count_in_data_coordinates_only() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        let look = ArrowLook {
            style: StreamArrowStyle::default(),
            mutation_scale: 10.0,
            linewidth: 1.0,
            color: Color::from("C0"),
        };
        let arrow = |transform| {
            StreamArrowArtist::new(
                (Point::new(4.0, 5.0), Point::new(6.0, 5.0)),
                look.clone(),
                transform,
                2.0,
            )
        };
        let state = fig.raw().figure_state().borrow();
        let points = arrow(ax.trans_data())
            .outline_data_points(&*state, ax.id())
            .unwrap();
        let (x_min, x_max) = points
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.x), hi.max(p.x))
            });
        // Shrunk by 2 points at both ends, the head at the far end.
        assert!(
            x_min > 4.0 && x_max < 6.0 && x_max > 5.85,
            "{x_min} {x_max}"
        );
        assert!(points.iter().any(|p| p.y > 5.0) && points.iter().any(|p| p.y < 5.0));
        assert_eq!(
            arrow(ax.trans_axes()).outline_data_points(&*state, ax.id()),
            None
        );
    }

    #[test]
    fn styles_parse_like_matplotlib() {
        assert_eq!(
            "-|>".parse::<StreamArrowStyle>().unwrap(),
            StreamArrowStyle::default()
        );
        let open: StreamArrowStyle = "->".parse().unwrap();
        assert_eq!(open.end, Some(StreamArrowHead::Open));
        let both: StreamArrowStyle = "<|-|>".parse().unwrap();
        assert_eq!(both.begin, Some(StreamArrowHead::Filled));
        let line: StreamArrowStyle = "-".parse().unwrap();
        assert_eq!((line.begin, line.end), (None, None));
        assert!("simple".parse::<StreamArrowStyle>().is_err());
        assert!("-]".parse::<StreamArrowStyle>().is_err());
    }

    #[test]
    fn arrows_are_shrunk_at_both_ends() {
        let (a, b) = (Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        let path = shrunk(arc3(a, b), (a, b), 2.0);
        let v = path.vertices();
        assert!((v[0].x - 2.0).abs() < 0.01 && (v[2].x - 8.0).abs() < 0.01);
        // Shorter than the shrinking: left alone.
        let (c, d) = (Point::new(0.0, 0.0), Point::new(1.0, 0.0));
        assert_eq!(shrunk(arc3(c, d), (c, d), 2.0).vertices()[2], d);
    }

    #[test]
    fn filled_heads_close_and_pull_back_the_tip() {
        let path = arc3(Point::new(0.0, 0.0), Point::new(20.0, 0.0));
        let paths = transmute(&path, StreamArrowStyle::default(), 10.0, 1.0);
        assert_eq!(paths.len(), 2);
        let (head, filled) = &paths[1];
        assert!(*filled);
        assert_eq!(head.code_at(3), PathCode::ClosePoly);
        let pad = 0.5 / (2.0 / 20.0_f64.sqrt());
        assert!((head.vertices()[1].x - (20.0 - pad)).abs() < 1e-12);
        assert!(
            (head.vertices()[0].y - head.vertices()[2].y).abs() > 3.9,
            "head width"
        );
        assert_eq!(paths[0].0.vertices()[2], head.vertices()[1]);
    }

    #[test]
    fn degenerate_arrows_have_no_head() {
        let p = Point::new(3.0, 4.0);
        let paths = transmute(&arc3(p, p), "<->".parse().unwrap(), 10.0, 1.0);
        assert_eq!(paths.len(), 1, "no room for heads");
    }
}
