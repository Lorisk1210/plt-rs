//! `FancyArrowPatch`: an arrow of [`ArrowStyle`] along a connection between
//! two points, sized in display space.

use pltrs_core::{Path, Point};
use pltrs_render::{CapStyle, JoinStyle};

use super::arrow_style::ArrowStyle;
use super::connection_style::{ClipRegion, ConnectionStyle};
use super::{DisplayPath, PatchError, PatchShape, PatchStyle, ShapeFrame};
use crate::artist::{ArtistBase, ArtistId};

/// How an arrow is built between its ends: the shared part of
/// [`FancyArrowPatch`], [`ConnectionPatch`](super::ConnectionPatch) and
/// annotation arrows.
#[derive(Clone, Debug, PartialEq)]
pub struct ArrowSpec {
    /// Heads, brackets and body.
    pub arrowstyle: ArrowStyle,
    /// The path between the ends.
    pub connectionstyle: ConnectionStyle,
    /// Shortening at the start, in points.
    pub shrink_a: f64,
    /// Shortening at the end, in points.
    pub shrink_b: f64,
    /// Scale of the arrow style's sizes, in points.
    pub mutation_scale: f64,
    /// Vertical squeeze applied while the arrow is built.
    pub mutation_aspect: f64,
    /// Region the start is clipped to, in display pixels.
    pub patch_a: Option<ClipRegion>,
    /// Region the end is clipped to, in display pixels.
    pub patch_b: Option<ClipRegion>,
}

impl Default for ArrowSpec {
    /// matplotlib's `FancyArrowPatch` defaults: a `simple` arrow on a
    /// straight `arc3` connection, shrunk by 2 points at both ends.
    fn default() -> Self {
        Self {
            arrowstyle: ArrowStyle::simple(),
            connectionstyle: ConnectionStyle::default(),
            shrink_a: 2.0,
            shrink_b: 2.0,
            mutation_scale: 1.0,
            mutation_aspect: 1.0,
            patch_a: None,
            patch_b: None,
        }
    }
}

impl ArrowSpec {
    /// The connection from `a` to `b` (display pixels), clipped and shrunk;
    /// `dpi_cor` converts points to pixels.
    pub fn route(&self, a: Point, b: Point, dpi_cor: f64) -> Result<Path, PatchError> {
        self.connectionstyle.route(
            (a, b),
            (self.shrink_a * dpi_cor, self.shrink_b * dpi_cor),
            self.patch_a.as_ref(),
            self.patch_b.as_ref(),
        )
    }

    /// The arrow pieces along `path` (display pixels) for an edge of
    /// `linewidth` points.
    pub fn display_paths(
        &self,
        path: &Path,
        dpi_cor: f64,
        linewidth: f64,
    ) -> Result<Vec<DisplayPath>, PatchError> {
        Ok(self
            .arrowstyle
            .paths(
                path,
                self.mutation_scale * dpi_cor,
                linewidth * dpi_cor,
                self.mutation_aspect,
            )?
            .into_iter()
            .map(|(path, filled)| DisplayPath::display(path, filled))
            .collect())
    }
}

/// Where a [`FancyArrowPatch`] runs, in the coordinates of its transform.
#[derive(Clone, Debug, PartialEq)]
pub enum ArrowRoute {
    /// From the first point to the second through the connection style.
    Between(Point, Point),
    /// Along a path; connection style, clipping and shrinking are ignored.
    Along(Path),
}

/// An arrow whose ends are fixed in data (or any) coordinates but whose
/// head and body are sized in display space (matplotlib's
/// `FancyArrowPatch`); draws an edge with round caps and joins by default.
#[derive(Clone, Debug, PartialEq)]
pub struct FancyArrowPatch {
    /// Where the arrow runs.
    pub route: ArrowRoute,
    /// How it is built.
    pub spec: ArrowSpec,
    /// The patches whose outlines, as drawn, clip the start and the end
    /// of the connection (matplotlib's `patchA` and `patchB`); they take
    /// precedence over the fixed regions of [`ArrowSpec`].
    pub patches: (Option<ArtistId>, Option<ArtistId>),
}

impl FancyArrowPatch {
    /// A default (`simple`) arrow from `a` to `b`.
    pub fn between(a: impl Into<Point>, b: impl Into<Point>) -> Self {
        Self::on(ArrowRoute::Between(a.into(), b.into()))
    }

    /// A default arrow along `path`.
    pub fn along(path: Path) -> Self {
        Self::on(ArrowRoute::Along(path))
    }

    fn on(route: ArrowRoute) -> Self {
        Self {
            route,
            spec: ArrowSpec::default(),
            patches: (None, None),
        }
    }

    /// The same arrow clipped to the outlines of the patches (or the
    /// boxes of the texts) `a` at its start and `b` at its end
    /// (matplotlib's `patchA` and `patchB`).
    #[must_use]
    pub fn with_patches(mut self, a: Option<ArtistId>, b: Option<ArtistId>) -> Self {
        self.patches = (a, b);
        self
    }

    /// The same arrow in another style.
    #[must_use]
    pub fn with_arrowstyle(mut self, arrowstyle: ArrowStyle) -> Self {
        self.spec.arrowstyle = arrowstyle;
        self
    }

    /// The same arrow on another connection.
    #[must_use]
    pub fn with_connectionstyle(mut self, connectionstyle: ConnectionStyle) -> Self {
        self.spec.connectionstyle = connectionstyle;
        self
    }

    /// The same arrow shortened by other amounts (points).
    #[must_use]
    pub fn with_shrink(mut self, shrink_a: f64, shrink_b: f64) -> Self {
        self.spec.shrink_a = shrink_a;
        self.spec.shrink_b = shrink_b;
        self
    }

    /// The same arrow at another mutation scale (points).
    #[must_use]
    pub fn with_mutation_scale(mut self, mutation_scale: f64) -> Self {
        self.spec.mutation_scale = mutation_scale;
        self
    }
}

/// Round caps and joins, which `FancyArrowPatch` and its subclasses use
/// unless told otherwise.
pub(crate) fn round_ends(style: &mut PatchStyle) {
    style.capstyle = CapStyle::Round;
    style.joinstyle = JoinStyle::Round;
}

impl PatchShape for FancyArrowPatch {
    const EDGE_DEFAULT: bool = true;

    /// The route in the patch's coordinates (used for data limits).
    fn path(&self) -> Path {
        match &self.route {
            ArrowRoute::Between(a, b) => Path::polyline(vec![*a, *b]),
            ArrowRoute::Along(path) => path.clone(),
        }
    }

    fn init_style(&self, style: &mut PatchStyle, _base: &mut ArtistBase) {
        round_ends(style);
    }

    /// `FancyArrowPatch._get_path_in_displaycoord`.
    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let dpi_cor = frame.dpi_cor;
        let path = match &self.route {
            ArrowRoute::Between(a, b) => {
                let region = |patch: Option<ArtistId>| {
                    let artist = frame.layout.store().get(patch?)?;
                    artist.connection_region(&frame.layout)
                };
                let spec = ArrowSpec {
                    patch_a: region(self.patches.0).or_else(|| self.spec.patch_a.clone()),
                    patch_b: region(self.patches.1).or_else(|| self.spec.patch_b.clone()),
                    ..self.spec.clone()
                };
                spec.route(
                    frame.transform.transform_point(*a),
                    frame.transform.transform_point(*b),
                    dpi_cor,
                )?
            }
            ArrowRoute::Along(path) => frame.transform.transform_path(path),
        };
        self.spec.display_paths(&path, dpi_cor, frame.linewidth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::patches::CurveEnd;

    /// The display extent of `arrow` in an axes over `(0, 10)²` spanning a
    /// 1000 × 1000 px, 72 dpi canvas (one data unit is 100 px).
    fn extent(arrow: FancyArrowPatch) -> pltrs_core::Bbox {
        let fig = Figure::with_size(1000.0 / 72.0, 1000.0 / 72.0, 72.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        ax.add_patch(arrow).window_extent().expect("an extent")
    }

    #[test]
    fn arrows_are_shrunk_in_points_at_both_ends() {
        let plain = FancyArrowPatch::between((1.0, 5.0), (9.0, 5.0))
            .with_arrowstyle(ArrowStyle::curve(CurveEnd::Plain, CurveEnd::Plain))
            .with_shrink(10.0, 20.0);
        let bounds = extent(plain);
        assert!((bounds.x0() - 110.0).abs() < 0.02, "{bounds:?}");
        assert!((bounds.x1() - 880.0).abs() < 0.02);
    }

    #[test]
    fn patches_clip_the_ends_where_they_are_drawn() {
        let fig = Figure::with_size(1000.0 / 72.0, 1000.0 / 72.0, 72.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 10.0)
            .and_then(|ax| ax.set_ylim(0.0, 10.0))
            .expect("finite limits");
        // Unstroked circles of radius 1 at both ends: 100 px.
        let start = ax.add_patch(crate::patches::Circle::new((1.0, 5.0), 1.0));
        let end = ax.add_patch(crate::patches::Circle::new((9.0, 5.0), 1.0));
        let arrow = FancyArrowPatch::between((1.0, 5.0), (9.0, 5.0))
            .with_arrowstyle(ArrowStyle::curve(CurveEnd::Plain, CurveEnd::Plain))
            .with_shrink(0.0, 0.0)
            .with_patches(Some(start.id()), Some(end.id()));
        let bounds = ax.add_patch(arrow).window_extent().expect("an extent");
        assert!((bounds.x0() - 200.0).abs() < 1.0, "{bounds:?}");
        assert!((bounds.x1() - 800.0).abs() < 1.0);
    }

    #[test]
    fn arrows_are_measured_at_one_pixel_per_point_until_drawn() {
        // matplotlib's `_dpi_cor` is 1 until the arrow is drawn.
        let fig = Figure::with_size(2.0, 2.0, 144.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        let simple = ArrowStyle::Simple {
            head_length: 10.0,
            head_width: 10.0,
            tail_width: 2.0,
        };
        let arrow = ax.add_patch(
            FancyArrowPatch::between((0.2, 0.5), (0.8, 0.5))
                .with_arrowstyle(simple)
                .with_shrink(0.0, 0.0),
        );
        let height = || arrow.window_extent().expect("an extent").height();
        assert!((height() - 10.0).abs() < 1e-6, "{}", height());
        fig.layout();
        assert!((height() - 20.0).abs() < 1e-6, "{}", height());
    }

    #[test]
    fn heads_scale_with_the_mutation_scale() {
        let arrow = |scale: f64| {
            FancyArrowPatch::between((1.0, 5.0), (9.0, 5.0))
                .with_arrowstyle("-|>".parse().expect("valid style"))
                .with_mutation_scale(scale)
        };
        let small = extent(arrow(10.0));
        let large = extent(arrow(40.0));
        assert!(large.height() > 3.0 * small.height());
    }

    #[test]
    fn paths_are_followed_without_shrinking() {
        let path = Path::polyline(vec![Point::new(1.0, 1.0), Point::new(1.0, 9.0)]);
        let along = FancyArrowPatch::along(path)
            .with_arrowstyle(ArrowStyle::curve(CurveEnd::Plain, CurveEnd::Plain));
        let bounds = extent(along);
        assert!((bounds.y0() - 100.0).abs() < 1e-9 && (bounds.y1() - 900.0).abs() < 1e-9);
    }

    #[test]
    fn styles_needing_a_quadratic_curve_report_other_paths() {
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.1, 0.1, 0.8, 0.8])
            .expect("a valid rectangle");
        let angled = FancyArrowPatch::between((0.0, 0.0), (1.0, 1.0))
            .with_connectionstyle("angle".parse().expect("valid style"));
        ax.add_patch(angled);
        let mut renderer = crate::testing::RecordingRenderer::new(640.0, 480.0, 100.0);
        let errors = fig
            .draw(&mut renderer)
            .expect_err("simple arrows need arcs");
        assert!(errors.to_string().contains("quadratic"));
    }
}
