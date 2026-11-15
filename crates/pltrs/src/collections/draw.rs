//! Rendering a collection (matplotlib's `Collection.draw`).

use std::borrow::Cow;

use pltrs_core::{Affine2D, Path, PathCode, Point, Rgba};
use pltrs_render::{Dashes, GraphicsContext, Hatch, PathCollection};

use super::gaps::inverse_paths_and_dashes;
use super::spec::{EllipseUnits, Ellipses};
use super::style::{
    broadcast_widths_and_dashes, resolve_colors, resolve_gap_colors, resolve_hatch_colors,
};
use super::{CollectionArtist, CollectionKind, cycled};
use crate::artist::{Artist, ArtistId, DrawContext, LayoutContext};
use crate::axes::AxesState;
use crate::patheffects::with_path_effects;
use crate::transforms::Transform;

/// Where a collection's items are for one draw: the shapes after the
/// non-affine part of the transform (the shapes themselves under an
/// affine transform), its affine rest, the per-item shape transforms, and
/// the offsets after the non-affine part of the offset transform with its
/// affine rest (matplotlib's `_prepare_points`).
pub(super) struct Geometry<'a> {
    paths: Cow<'a, [Path]>,
    master: Affine2D,
    transforms: Vec<Affine2D>,
    offsets: Vec<Point>,
    offset_affine: Affine2D,
}

impl<'a> Geometry<'a> {
    /// The geometry of `shapes` in the coordinates of `transform`, placed
    /// at `offsets` (in data order for picking, in drawing order for a
    /// draw).
    pub(super) fn new(
        c: &CollectionArtist,
        layout: LayoutContext<'_>,
        (shapes, transform): (&'a [Path], &Transform),
        offsets: &[Point],
    ) -> Result<Self, String> {
        let resolved = layout
            .resolve(transform)
            .ok_or("its coordinate system no longer exists")?;
        let paths = if resolved.is_affine() {
            Cow::Borrowed(shapes)
        } else {
            Cow::Owned(resolved.split_members(shapes).0)
        };
        let dpi = layout.dpi();
        let mut master = resolved.affine_part();
        let transforms = match (&c.ellipses, c.drawn_sizes()) {
            (Some(ellipses), _) => {
                let axes = c.base().parent;
                let scales = ellipse_scales(ellipses.units, layout, axes)
                    .ok_or("its axes no longer exist")?;
                if let Some(data) = scales.data {
                    master = data;
                }
                ellipse_transforms(ellipses, scales.scale)
            }
            (None, sizes) => c.item_transforms(sizes, dpi),
        };
        let (offsets, offset_affine) = match &c.offset_transform {
            Some(t) => {
                let r = layout
                    .resolve(t)
                    .ok_or("its offset coordinate system no longer exists")?;
                // All at once: a map of the program's is called once, as
                // matplotlib maps the offsets array.
                let mut placed = offsets.to_vec();
                if c.offsets_masked {
                    r.transform_points_non_affine_masked(&mut placed);
                } else {
                    r.transform_points_non_affine(&mut placed);
                }
                (placed, r.affine_part())
            }
            None => (vec![Point::ORIGIN], Affine2D::IDENTITY),
        };
        Ok(Self {
            paths,
            master,
            transforms,
            offsets,
            offset_affine,
        })
    }

    /// The offsets in display space.
    fn display_offsets(&self) -> Vec<Point> {
        (self.offsets.iter())
            .map(|&p| self.offset_affine.transform_point(p))
            .collect()
    }

    /// The items within `radius` pixels of display point `at` — on their
    /// outline stroked `2 * radius` wide, or inside them when `filled`
    /// (matplotlib's `point_in_path_collection`).
    pub(super) fn hits(&self, at: Point, radius: f64, filled: bool) -> Vec<usize> {
        if self.paths.is_empty() {
            return Vec::new();
        }
        let count = self.paths.len().max(self.offsets.len());
        (0..count)
            .filter(|&i| {
                let path = &self.paths[i % self.paths.len()];
                // No offsets (matplotlib's `Noffsets == 0`) leave the
                // paths where they are.
                let offset = cycled(&self.offsets, i)
                    .map(|&p| self.offset_affine.transform_point(p))
                    .unwrap_or_default();
                let transform = cycled(&self.transforms, i)
                    .copied()
                    .unwrap_or_default()
                    .then(self.master)
                    .then(Affine2D::translation(offset.x, offset.y));
                if filled {
                    path.contains_point(at, Some(&transform), radius)
                } else {
                    path.stroke_contains_point(at, &transform, radius)
                }
            })
            .collect()
    }
}

/// A collection resolved for one draw: its geometry and per-item styles.
struct Batch<'a> {
    geometry: Geometry<'a>,
    faces: Vec<Rgba>,
    /// The edge colours, `None` where they are the face colours (as a
    /// scatter's edges follow its faces), kept once.
    edges: Option<Vec<Rgba>>,
    /// The colours of the gaps of dashed edges (`None` = empty gaps).
    gaps: Option<Vec<Rgba>>,
    widths: Vec<f64>,
    dashes: Vec<Dashes>,
    antialiased: Vec<bool>,
    /// The links of the items, cycled (`[None]` when none are set).
    urls: Vec<Option<String>>,
}

impl<'a> Batch<'a> {
    fn new(
        c: &CollectionArtist,
        ctx: &DrawContext<'_>,
        shapes: &'a [Path],
        transform: &Transform,
    ) -> Result<Self, String> {
        let geometry = Geometry::new(c, ctx.layout(), (shapes, transform), c.drawn_offsets())?;
        let (faces, edges) = resolve_colors(c)?;
        let edges = (edges != faces).then_some(edges);
        let gaps = resolve_gap_colors(c)?;
        let rc = c.rc();
        let line_defaults = c.kind == CollectionKind::Line;
        let widths = c.drawn_linewidths();
        let (widths, dashes) = broadcast_widths_and_dashes(&widths, &c.linestyles, rc);
        let antialiased = c.antialiased.clone().unwrap_or_else(|| {
            vec![if line_defaults {
                rc.lines_antialiased
            } else {
                rc.patch_antialiased
            }]
        });
        let urls = if c.urls.is_empty() {
            vec![None]
        } else {
            c.urls.clone()
        };
        Ok(Self {
            geometry,
            faces,
            edges,
            gaps,
            widths,
            dashes,
            antialiased,
            urls,
        })
    }

    /// The edge colours.
    fn edges(&self) -> &[Rgba] {
        self.edges.as_deref().unwrap_or(&self.faces)
    }

    /// The shape transform for drawing every item as the same marker, when
    /// matplotlib's single-path optimisation applies: one shape, style,
    /// colour pair and link, solid, unhatched and smaller than the canvas.
    fn single_marker(&self, hatched: bool, canvas: (f64, f64)) -> Option<Affine2D> {
        let uniform = self.geometry.paths.len() == 1
            && self.geometry.transforms.len() <= 1
            && self.faces.len() == 1
            && self.edges().len() == 1
            && self.widths.len() == 1
            && self.dashes.iter().all(Dashes::is_solid)
            && self.antialiased.len() == 1
            && self.urls.len() == 1
            && !hatched;
        if !uniform {
            return None;
        }
        let combined = cycled(&self.geometry.transforms, 0)
            .copied()
            .unwrap_or_default()
            .then(self.geometry.master);
        let path = self.geometry.paths[0].transformed(&combined);
        // matplotlib's `get_extents` takes NumPy minima and maxima, which a
        // non-finite vertex turns NaN or infinite: no shortcut then.
        let finite = path
            .vertices()
            .iter()
            .zip(path.resolved_codes())
            .all(|(vertex, code)| code == PathCode::ClosePoly || vertex.is_finite());
        let extents = path.extents();
        (finite && extents.width() < canvas.0 && extents.height() < canvas.1).then_some(combined)
    }
}

/// Draws the collection in its SVG group, which matplotlib opens for a
/// collection of no item too (an empty scatter plot, a line collection
/// without segments).
pub(super) fn draw_collection(
    c: &CollectionArtist,
    ctx: &mut DrawContext<'_>,
) -> Result<(), String> {
    warn_of_negative_sizes(c);
    c.resolve_colors_again();
    let (shapes, transform) = c.drawn_shapes(ctx.layout())?;
    let name = c.class_name.unwrap_or(match c.kind {
        CollectionKind::Generic => "Collection",
        CollectionKind::Patch => "PatchCollection",
        CollectionKind::Path => "PathCollection",
        CollectionKind::Poly => "PolyCollection",
        CollectionKind::Line => "LineCollection",
        CollectionKind::QuadMesh => "QuadMesh",
        CollectionKind::TriMesh => "TriMesh",
    });
    ctx.renderer().open_group(name, c.base().gid.as_deref());
    let drawn = if shapes.is_empty() {
        Ok(())
    } else {
        draw_shapes(c, ctx, &shapes, &transform)
    };
    ctx.renderer().close_group(name);
    drawn
}

/// Warns, as numpy does, of the square root of a negative marker area
/// that matplotlib takes in each of its draws the draw running now stands
/// for (`_CollectionWithSizes.draw` scales the items again,
/// `set_sizes(sizes, dpi)`).
fn warn_of_negative_sizes(c: &CollectionArtist) {
    if c.drawn_sizes()
        .is_some_and(|sizes| sizes.iter().any(|&area| area < 0.0))
    {
        for _ in 0..crate::text::font_notes::draws_of_print() {
            crate::warnings::warn_runtime("invalid value encountered in sqrt");
        }
    }
}

/// Draws the items of `shapes` through the collection's path effects,
/// using `draw_markers` when every item shares one shape and style
/// (matplotlib's single-path optimisation); a collection placed at
/// offsets but without any draws nothing.
fn draw_shapes(
    c: &CollectionArtist,
    ctx: &mut DrawContext<'_>,
    shapes: &[Path],
    transform: &Transform,
) -> Result<(), String> {
    let batch = Batch::new(c, ctx, shapes, transform)?;
    let hatches = resolve_hatch_colors(c, batch.edges()).map_err(|e| e.to_string())?;
    let gc = graphics_context(c, ctx, &hatches);
    if batch.geometry.offsets.is_empty() {
        return Ok(());
    }
    with_path_effects(ctx, &c.base().path_effects, |ctx| {
        draw_items(ctx, gc, &batch, &hatches, c.hatch.is_some());
    })
    .map_err(|err| err.to_string())
}

/// Draws the items of a collection with at least one offset, each hatched
/// in its colour of `hatches`.
fn draw_items(
    ctx: &mut DrawContext<'_>,
    mut gc: GraphicsContext,
    batch: &Batch<'_>,
    hatches: &[Rgba],
    hatched: bool,
) {
    let canvas = ctx.renderer().canvas_size();
    if let Some(combined) = batch.single_marker(hatched, canvas) {
        gc.foreground = batch.edges()[0];
        gc.linewidth = batch.widths[0];
        gc.dashes.clone_from(&batch.dashes[0]);
        gc.antialiased = batch.antialiased[0];
        gc.url.clone_from(&batch.urls[0]);
        // Agg composes the offsets' affine transform with its own flip
        // into the device, so the rows markers land on depend on it.
        let geometry = &batch.geometry;
        let positions = Path::polyline(geometry.offsets.clone());
        let fill = Some(batch.faces[0]);
        ctx.renderer().draw_markers(
            &gc,
            &geometry.paths[0],
            &combined,
            &positions,
            &geometry.offset_affine,
            fill,
        );
    } else {
        let offsets = batch.geometry.display_offsets();
        let collection = PathCollection {
            master_transform: batch.geometry.master,
            paths: &batch.geometry.paths,
            path_transforms: &batch.geometry.transforms,
            offsets: &offsets,
            facecolors: &batch.faces,
            edgecolors: batch.edges(),
            hatchcolors: hatches,
            linewidths: &batch.widths,
            dashes: &batch.dashes,
            antialiased: &batch.antialiased,
            urls: &batch.urls,
        };
        if let Some(gaps) = &batch.gaps {
            // The gaps first, with transparent faces, as matplotlib does.
            let (paths, dashes) = inverse_paths_and_dashes(&batch.geometry.paths, &batch.dashes);
            let gap_pass = PathCollection {
                paths: &paths,
                facecolors: &[Rgba::TRANSPARENT],
                edgecolors: gaps,
                dashes: &dashes,
                ..collection
            };
            ctx.renderer().draw_path_collection(&gc, &gap_pass);
        }
        ctx.renderer().draw_path_collection(&gc, &collection);
    }
}

/// The shared graphics state: clipping, hatch (in the first of
/// `hatches`; items take theirs from the collection), caps and joins.
fn graphics_context(
    c: &CollectionArtist,
    ctx: &DrawContext<'_>,
    hatches: &[Rgba],
) -> GraphicsContext {
    let mut gc = ctx.base_gc(c.base());
    gc.alpha = None;
    // matplotlib only hands the renderer non-empty hatches.
    if let Some(pattern) = c.hatch.as_ref().filter(|p| !p.is_empty()) {
        gc.hatch = Some(Hatch {
            pattern: pattern.clone(),
            color: hatches.first().copied().unwrap_or(Rgba::BLACK),
            linewidth: c.hatch_linewidth,
        });
    }
    if let Some(cap) = c.capstyle {
        gc.cap = cap;
    }
    if let Some(join) = c.joinstyle {
        gc.join = join;
    }
    gc
}

/// How the ellipses of a collection are scaled at draw time.
pub(super) struct EllipseScales {
    /// Pixels per unit along both axes.
    pub(super) scale: f64,
    /// For data units: the data transform without its translation, which
    /// replaces the collection's transform.
    data: Option<Affine2D>,
}

/// `EllipseCollection._set_transforms`: the scale of `units` in the axes
/// `axes`.
pub(super) fn ellipse_scales(
    units: EllipseUnits,
    layout: LayoutContext<'_>,
    axes: Option<ArtistId>,
) -> Option<EllipseScales> {
    let plain = |scale: f64| Some(EllipseScales { scale, data: None });
    match units {
        EllipseUnits::Inches => return plain(layout.dpi()),
        EllipseUnits::Points => return plain(layout.dpi() / 72.0),
        EllipseUnits::Dots => return plain(1.0),
        _ => {}
    }
    let axes = axes?;
    let bbox = layout.axes_extent(axes)?;
    let view = layout.store().get_as::<AxesState>(axes)?.view_lim;
    match units {
        EllipseUnits::Xy => {
            let m = layout
                .resolve(&crate::transforms::Transform::Data(axes))?
                .affine_part()
                .matrix();
            Some(EllipseScales {
                scale: 1.0,
                data: Some(Affine2D::from_values(
                    m[0][0], m[1][0], m[0][1], m[1][1], 0.0, 0.0,
                )),
            })
        }
        EllipseUnits::X => plain(bbox.width() / view.width()),
        EllipseUnits::Y => plain(bbox.height() / view.height()),
        EllipseUnits::Width => plain(bbox.width()),
        _ => plain(bbox.height()),
    }
}

/// One affine per width: the unit circle stretched to its half axes
/// (scaled by `scale`) and rotated, a single height or angle serving every
/// ellipse as numpy broadcasts it.
fn ellipse_transforms(ellipses: &Ellipses, scale: f64) -> Vec<Affine2D> {
    let (heights, angles) = (&ellipses.half_heights, &ellipses.angles);
    if heights.is_empty() || angles.is_empty() {
        return Vec::new();
    }
    ellipses
        .half_widths
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let (h, angle) = (heights[i % heights.len()], angles[i % angles.len()]);
            let (w, h) = (w * scale, h * scale);
            let (sin, cos) = (angle.sin(), angle.cos());
            Affine2D::from_values(w * cos, w * sin, h * -sin, h * cos, 0.0, 0.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::HatchColors;

    fn square(alpha: Option<f64>) -> CollectionArtist {
        let path = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
        let mut collection =
            CollectionArtist::new(CollectionKind::Poly, vec![path], crate::rc::params());
        collection.base_mut().alpha = alpha;
        collection
    }

    #[test]
    fn hatches_take_the_edges_or_the_faded_default_edge_colour() {
        let edges = [Rgba::new(1.0, 0.0, 0.0, 0.5), Rgba::BLACK];
        assert_eq!(
            resolve_hatch_colors(&square(Some(0.5)), &edges),
            Ok(edges.to_vec())
        );
        let black = resolve_hatch_colors(&square(Some(0.5)), &[]).unwrap();
        assert_eq!(
            black,
            [Rgba::new(0.0, 0.0, 0.0, 0.5)],
            "matplotlib fades the default"
        );
        assert_eq!(resolve_hatch_colors(&square(None), &[]).unwrap()[0].a, 1.0);
    }

    #[test]
    fn edges_following_the_faces_are_the_faces_kept_once() {
        let faces = vec![Rgba::new(1.0, 0.0, 0.0, 1.0), Rgba::BLACK];
        let batch = |edges: Option<Vec<Rgba>>| Batch {
            geometry: Geometry {
                paths: Vec::new().into(),
                master: Affine2D::IDENTITY,
                transforms: Vec::new(),
                offsets: Vec::new(),
                offset_affine: Affine2D::IDENTITY,
            },
            faces: faces.clone(),
            edges,
            gaps: None,
            widths: vec![1.0],
            dashes: vec![Dashes::default()],
            antialiased: vec![true],
            urls: vec![None],
        };
        assert_eq!(batch(None).edges(), faces.as_slice());
        let own = vec![Rgba::BLACK];
        assert_eq!(batch(Some(own.clone())).edges(), own.as_slice());
    }

    #[test]
    fn items_without_offsets_are_hit_where_their_paths_are() {
        // Found by fuzzing: a collection whose offsets were emptied
        // divided by zero on a pick.
        let path = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)]);
        let geometry = Geometry {
            paths: vec![path].into(),
            master: Affine2D::IDENTITY,
            transforms: Vec::new(),
            offsets: Vec::new(),
            offset_affine: Affine2D::IDENTITY,
        };
        assert_eq!(geometry.hits(Point::new(5.0, 0.0), 1.0, false), [0]);
        assert_eq!(
            geometry.hits(Point::new(5.0, 9.0), 1.0, false),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn an_empty_scatter_draws_nothing() {
        use crate::axes::{ScatterColors, ScatterOptions};
        use crate::collections::EdgeColors;
        use crate::figure::Figure;
        use crate::lines::LineStyle;
        use crate::testing::RecordingRenderer;

        // Dashed edges and an invisible face rule out the single-marker
        // shortcut, which has no positions to draw at anyway.
        let calls = |linestyle: Option<LineStyle>, face: &str| {
            let fig = Figure::new();
            let ax = fig.subplot();
            if let Some(linestyle) = linestyle {
                let options = ScatterOptions {
                    colors: ScatterColors::Single(face.into()),
                    edgecolors: Some(EdgeColors::Colors(vec!["red".into()])),
                    ..ScatterOptions::default()
                };
                ax.scatter_with(Vec::<f64>::new(), Vec::<f64>::new(), options)
                    .unwrap()
                    .set_linestyles(vec![linestyle]);
            }
            ax.set_xlim(0.0, 1.0)
                .and_then(|ax| ax.set_ylim(0.0, 1.0))
                .expect("finite limits");
            let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
            fig.draw(&mut renderer).unwrap();
            renderer.calls
        };
        let without = calls(None, "none");
        assert_eq!(calls(Some(LineStyle::Dashed), "blue"), without);
        assert_eq!(calls(Some(LineStyle::Solid), "none"), without);
    }

    #[test]
    fn an_empty_collection_still_writes_its_group() {
        use crate::collections::CollectionSpec;
        use crate::figure::Figure;

        let fig = Figure::new();
        fig.subplot()
            .add_collection(CollectionSpec::lines(Vec::new()));
        let svg = fig.to_svg().unwrap();
        assert!(svg.contains("<g id=\"LineCollection_1\"/>"), "{svg}");
    }

    #[test]
    fn one_ellipse_per_width_with_heights_and_angles_broadcast() {
        let ellipses = Ellipses {
            half_widths: vec![1.0, 2.0],
            half_heights: vec![3.0],
            angles: vec![0.0],
            units: EllipseUnits::Xy,
        };
        let transforms = ellipse_transforms(&ellipses, 2.0);
        assert_eq!(
            transforms,
            [Affine2D::scaling(2.0, 6.0), Affine2D::scaling(4.0, 6.0)]
        );
        let no_heights = Ellipses {
            half_heights: Vec::new(),
            ..ellipses
        };
        assert_eq!(ellipse_transforms(&no_heights, 1.0), Vec::<Affine2D>::new());
    }

    #[test]
    fn item_urls_link_each_item_in_svg() {
        use crate::figure::Figure;

        let fig = Figure::new();
        let points = fig
            .subplot()
            .scatter([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])
            .unwrap();
        let links = |svg: &str| svg.matches("xlink:href=\"https://").count();
        let plain = fig.to_svg().unwrap();
        assert_eq!(links(&plain), 0);
        points.set_urls(vec![
            Some("https://example.com/a".into()),
            Some("https://example.com/b".into()),
            None,
        ]);
        assert_eq!(points.urls().len(), 3);
        let linked = fig.to_svg().unwrap();
        assert_eq!(links(&linked), 2, "one link per item with a url");
        points.set_urls(vec![Some("https://example.com/all".into())]);
        let one = fig.to_svg().unwrap();
        assert_eq!(links(&one), 1, "one url links all the markers at once");
    }

    #[test]
    fn explicit_hatch_colours_take_the_alpha() {
        crate::rc::context(|| {
            crate::rc::set("hatch.color", "blue").unwrap();
            let colors = resolve_hatch_colors(&square(Some(0.25)), &[Rgba::BLACK]).unwrap();
            assert_eq!(colors, [Rgba::new(0.0, 0.0, 1.0, 0.25)]);
        });
        let mut both = square(None);
        both.hatchcolors = HatchColors::Colors(vec!["red".into(), "lime".into()]);
        let colors = resolve_hatch_colors(&both, &[Rgba::BLACK]).unwrap();
        assert_eq!(colors[1], Rgba::new(0.0, 1.0, 0.0, 1.0));
    }
}
